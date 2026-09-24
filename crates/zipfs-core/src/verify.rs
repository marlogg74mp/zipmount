//! End-to-end read verification against the archive's own checksums.
//!
//! The table of contents of both zip and 7z keeps a CRC32 of every entry — a
//! ready reference that needs neither a third-party archiver nor stored
//! samples.
//!
//! The method is fitted to the format:
//!
//! * **zip** — read through the same `FileHandle` the filesystem uses, so
//!   errors in all three read paths are caught at once, seeking backwards in
//!   the streaming decoder included;
//! * **7z** — stream through the solid blocks: reading file by file out of
//!   order would expand a block again for every entry.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use anyhow::{Context, Result};
use rayon::prelude::*;
use zipmount_i18n::t;

use crate::archive::{Archive, Backend};
use crate::cache::ContentCache;
use crate::reader::FileHandle;
use crate::zipfmt::crc32;

const CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct VerifyOptions {
    /// Read a file in shuffled order rather than sequentially. That tests
    /// seeking backwards in the streaming decoder — the most fragile read
    /// path. Applies to zip; in 7z an entry sits in memory whole anyway.
    pub random_access: bool,
    pub cache_budget: usize,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            random_access: false,
            cache_budget: 128 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Default)]
pub struct VerifyOutcome {
    pub files_ok: u64,
    pub files_without_checksum: u64,
    pub bytes_read: u64,
    pub failures: Vec<Failure>,
}

#[derive(Debug, Clone)]
pub struct Failure {
    pub path: String,
    pub reason: String,
}

pub fn verify(archive: &Archive, opts: VerifyOptions) -> Result<VerifyOutcome> {
    match archive.backend() {
        Backend::SevenZ(_) => verify_sevenz(archive),
        #[cfg(feature = "rar")]
        Backend::Rar(_) => verify_rar(archive),
        // zip and tar are read file by file through the same FileHandle the
        // filesystem uses.
        _ => verify_per_file(archive, opts),
    }
}

fn verify_per_file(archive: &Archive, opts: VerifyOptions) -> Result<VerifyOutcome> {
    let tree = archive.tree();
    let cache = ContentCache::new(opts.cache_budget);
    let files_ok = AtomicU64::new(0);
    let without_crc = AtomicU64::new(0);
    let bytes_read = AtomicU64::new(0);
    let failures: Mutex<Vec<Failure>> = Mutex::new(Vec::new());

    tree.file_ids()
        .par_iter()
        .for_each(|&id| match check_entry(archive, &cache, id, opts) {
            Ok((n, checked)) => {
                if checked {
                    files_ok.fetch_add(1, Ordering::Relaxed);
                } else {
                    without_crc.fetch_add(1, Ordering::Relaxed);
                }
                bytes_read.fetch_add(n, Ordering::Relaxed);
            }
            Err(reason) => failures
                .lock()
                .expect("failure list poisoned by a panic")
                .push(Failure {
                    path: tree.path_of(id),
                    reason: format!("{reason:#}"),
                }),
        });

    Ok(finish(
        files_ok.into_inner(),
        without_crc.into_inner(),
        bytes_read.into_inner(),
        failures,
    ))
}

/// Returns the bytes read and whether the contents could be checked against
/// a checksum.
fn check_entry(
    archive: &Archive,
    cache: &ContentCache,
    id: u32,
    opts: VerifyOptions,
) -> Result<(u64, bool)> {
    let expected_crc = archive.crc32_of(id);
    let expected_size = archive.tree().node(id).size;

    let mut handle = FileHandle::open(archive, id)?;
    let mut assembled = vec![0u8; expected_size as usize];
    let mut buf = vec![0u8; CHUNK];

    let mut offsets: Vec<u64> = (0..expected_size).step_by(CHUNK).collect();
    if opts.random_access {
        // A deterministic shuffle: reproducibility matters more than the
        // quality of randomness — a failure must repeat.
        let n = offsets.len();
        for i in 0..n {
            let j = (i * 2_654_435_761usize.wrapping_add(id as usize)) % n.max(1);
            offsets.swap(i, j);
        }
    }

    let mut total = 0u64;
    for off in offsets {
        let want = CHUNK.min((expected_size - off) as usize);
        let mut filled = 0usize;
        while filled < want {
            let n = handle.read_at(archive, cache, off + filled as u64, &mut buf[filled..want])?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        if filled != want {
            anyhow::bail!(t!(
                "core-verify-short-read",
                offset = off,
                got = filled,
                want = want
            ));
        }
        assembled[off as usize..off as usize + want].copy_from_slice(&buf[..want]);
        total += want as u64;
    }

    // An entry may have no checksum (WinZip AE-2). Then the read itself is
    // the confirmation: decryption checks the HMAC and fails on a mismatch, so
    // damage cannot slip through silently here.
    match expected_crc {
        Some(expected) => {
            let actual = crc32(&assembled);
            if actual != expected {
                anyhow::bail!(crc_mismatch(actual, expected));
            }
            Ok((total, true))
        }
        None => Ok((total, false)),
    }
}

fn verify_sevenz(archive: &Archive) -> Result<VerifyOutcome> {
    let backend = archive.as_sevenz().context("expected the 7z backend")?;
    let tree = archive.tree();
    let node_of = tree.entry_node_map(backend.entry_count());

    let files_ok = AtomicU64::new(0);
    let without_crc = AtomicU64::new(0);
    let bytes_read = AtomicU64::new(0);
    let failures: Mutex<Vec<Failure>> = Mutex::new(Vec::new());

    (0..backend.block_count())
        .into_par_iter()
        .for_each(|block| {
            let outcome = backend.for_each_in_block(block, |entry_index, data| {
                let Some(node) = node_of.get(entry_index as usize).copied().flatten() else {
                    return Ok(());
                };

                bytes_read.fetch_add(data.len() as u64, Ordering::Relaxed);

                match backend.crc32(entry_index) {
                    Some(expected) => {
                        let actual = crc32(&data);
                        if actual != expected {
                            failures
                                .lock()
                                .expect("failure list poisoned by a panic")
                                .push(Failure {
                                    path: tree.path_of(node),
                                    reason: crc_mismatch(actual, expected),
                                });
                        } else {
                            files_ok.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    None => {
                        without_crc.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Ok(())
            });

            if let Err(e) = outcome {
                failures
                    .lock()
                    .expect("failure list poisoned by a panic")
                    .push(Failure {
                        path: t!("core-verify-block", block = block),
                        reason: format!("{e:#}"),
                    });
            }
        });

    Ok(finish(
        files_ok.into_inner(),
        without_crc.into_inner(),
        bytes_read.into_inner(),
        failures,
    ))
}

#[cfg(feature = "rar")]
fn verify_rar(archive: &Archive) -> Result<VerifyOutcome> {
    let backend = archive.as_rar().context("expected the RAR backend")?;
    let tree = archive.tree();
    let node_of = tree.entry_node_map(backend.entry_count());

    let mut files_ok = 0u64;
    let mut without_crc = 0u64;
    let mut bytes_read = 0u64;
    let mut failures: Vec<Failure> = Vec::new();

    // One pass: pulling entries out one by one from a solid archive would
    // replay the stream for each.
    backend.for_each_entry(|entry_index, data| {
        let Some(node) = node_of.get(entry_index as usize).copied().flatten() else {
            return Ok(());
        };
        bytes_read += data.len() as u64;

        match backend.crc32(entry_index) {
            Some(expected) => {
                let actual = crc32(&data);
                if actual != expected {
                    failures.push(Failure {
                        path: tree.path_of(node),
                        reason: crc_mismatch(actual, expected),
                    });
                } else {
                    files_ok += 1;
                }
            }
            None => {
                without_crc += 1;
            }
        }
        Ok(())
    })?;

    Ok(finish(
        files_ok,
        without_crc,
        bytes_read,
        Mutex::new(failures),
    ))
}

fn crc_mismatch(actual: u32, expected: u32) -> String {
    t!(
        "core-verify-crc",
        actual = format!("{actual:08x}"),
        expected = format!("{expected:08x}")
    )
}

fn finish(
    files_ok: u64,
    files_without_checksum: u64,
    bytes_read: u64,
    failures: Mutex<Vec<Failure>>,
) -> VerifyOutcome {
    let mut failures = failures
        .into_inner()
        .expect("failure list poisoned by a panic");
    failures.sort_by(|a, b| a.path.cmp(&b.path));
    VerifyOutcome {
        files_ok,
        files_without_checksum,
        bytes_read,
        failures,
    }
}
