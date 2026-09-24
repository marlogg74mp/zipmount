//! The tar backend, gzip-compressed tar included.
//!
//! The two cases behave fundamentally differently:
//!
//! * **`.tar`** — the ideal case: each file's data lies contiguously, so a
//!   file is read as a slice of the memory-mapped archive. No decompression,
//!   no copying, no cache.
//! * **`.tar.gz`** — one compressed stream with no entry points. A single file
//!   cannot be pulled out without expanding everything before it; moreover,
//!   even learning what the archive contains means going through the whole
//!   stream. So it is expanded once, at open time, and behaves like an
//!   ordinary tar from then on.
//!
//! The same fork as 7z, taken to the limit: there were three solid blocks
//! there, here there is exactly one for the whole archive.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{bail, Context, Result};
use flate2::read::MultiGzDecoder;
use memmap2::Mmap;
use zipmount_i18n::{decimal, t};

use crate::entry::EntryMeta;
use crate::tarfmt::{self, TarEntry};

/// Where the bytes of the tar stream come from.
enum TarSource {
    /// A plain tar: a mapped file, slices without copying.
    Mapped(Mmap),
    /// An expanded tar.gz, whole, in memory.
    Memory(Vec<u8>),
}

pub struct TarBackend {
    source: TarSource,
    entries: Vec<TarEntry>,
    /// Size of the expanded stream — also the memory held, for tar.gz.
    stream_len: u64,
    compressed: bool,
}

impl TarBackend {
    pub fn open_plain(file: &File) -> Result<(Self, Vec<EntryMeta>)> {
        // SAFETY: the same terms as for zip: the archive must not change from
        // outside while it is open.
        let mmap = unsafe { Mmap::map(file) }.with_context(|| t!("core-mmap-failed"))?;
        let entries = tarfmt::parse_tar(&mmap).with_context(|| t!("core-tar-structure"))?;
        let stream_len = mmap.len() as u64;

        let meta = to_meta(&entries);
        Ok((
            Self {
                source: TarSource::Mapped(mmap),
                entries,
                stream_len,
                compressed: false,
            },
            meta,
        ))
    }

    pub fn open_gzip(path: &Path, budget: usize) -> Result<(Self, Vec<EntryMeta>)> {
        let data = decompress_gzip(path, budget)?;

        if !tarfmt::looks_like_tar(&data) {
            bail!(t!("core-gzip-not-tar"));
        }

        let entries = tarfmt::parse_tar(&data).with_context(|| t!("core-tar-structure"))?;
        let stream_len = data.len() as u64;

        let meta = to_meta(&entries);
        Ok((
            Self {
                source: TarSource::Memory(data),
                entries,
                stream_len,
                compressed: true,
            },
            meta,
        ))
    }

    pub fn data(&self) -> &[u8] {
        match &self.source {
            TarSource::Mapped(m) => m,
            TarSource::Memory(v) => v,
        }
    }

    pub fn entry(&self, index: u32) -> &TarEntry {
        &self.entries[index as usize]
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Length of the tar stream. For `.tar.gz` it is also the memory the
    /// expanded archive takes.
    pub fn stream_len(&self) -> u64 {
        self.stream_len
    }

    pub fn is_compressed(&self) -> bool {
        self.compressed
    }

    /// The range of an entry's data in the stream.
    pub fn entry_range(&self, index: u32) -> Result<(usize, usize)> {
        let entry = self.entry(index);
        let start =
            usize::try_from(entry.data_offset).context("data offset does not fit in usize")?;
        let len = usize::try_from(entry.size).context("entry size does not fit in usize")?;
        if start + len > self.data().len() {
            bail!(t!("core-entry-out-of-bounds"));
        }
        Ok((start, len))
    }

    pub fn entry_bytes(&self, index: u32) -> Result<&[u8]> {
        let (start, len) = self.entry_range(index)?;
        Ok(&self.data()[start..start + len])
    }
}

fn to_meta(entries: &[TarEntry]) -> Vec<EntryMeta> {
    entries
        .iter()
        .map(|e| EntryMeta {
            path: e.path.clone(),
            is_dir: e.is_dir,
            size: e.size,
            mtime: e.mtime,
        })
        .collect()
}

/// Expands a gzip in full, making sure not to eat all the memory.
///
/// `MultiGzDecoder` checks the CRC32 of every member of the stream on the
/// way, so a successful decompression here doubles as an integrity check.
fn decompress_gzip(path: &Path, budget: usize) -> Result<Vec<u8>> {
    let file = File::open(path)
        .with_context(|| t!("core-open-failed", path = path.display().to_string()))?;

    let hint = gzip_size_hint(&file).unwrap_or(0) as usize;
    let mut out: Vec<u8> = Vec::with_capacity(hint.min(budget));

    let mut decoder = MultiGzDecoder::new(BufReader::with_capacity(1 << 20, file));
    let mut buf = vec![0u8; 1 << 20];

    loop {
        let read = decoder
            .read(&mut buf)
            .with_context(|| t!("core-gzip-damaged"))?;
        if read == 0 {
            break;
        }
        if out.len() + read > budget {
            const GB: f64 = 1024.0 * 1024.0 * 1024.0;
            bail!(t!(
                "core-targz-too-large",
                used = decimal(out.len() as f64 / GB, 1),
                limit = decimal(budget as f64 / GB, 1)
            ));
        }
        out.extend_from_slice(&buf[..read]);
    }

    Ok(out)
}

/// The last four bytes of a gzip hold the uncompressed size modulo 4 GB. For
/// larger archives that is a lie, so the value only serves as a hint for
/// reserving memory, never as the truth.
fn gzip_size_hint(file: &File) -> Option<u32> {
    let mut file = file.try_clone().ok()?;
    let len = file.metadata().ok()?.len();
    if len < 4 {
        return None;
    }
    file.seek(SeekFrom::End(-4)).ok()?;
    let mut tail = [0u8; 4];
    file.read_exact(&mut tail).ok()?;
    file.seek(SeekFrom::Start(0)).ok()?;
    Some(u32::from_le_bytes(tail))
}
