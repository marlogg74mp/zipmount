//! The 7z backend.
//!
//! The main difference from zip is the unit of access. Zip compresses every
//! entry on its own, so it can be read file by file. 7z joins entries into
//! solid blocks: getting one file out means expanding its block from the
//! start. Hence two modes of work:
//!
//! * random access (a mounted drive) — through a cache of expanded blocks;
//! * full scans (search, verification) — streaming through a block, no cache,
//!   at constant memory. For a solid archive this is also the best read order.
//!
//! Contrary to expectation, solid does not necessarily mean slow: with
//! well-compressible data (logs compress tens of times) the decoder reads few
//! input bytes, and a block of a couple of gigabytes expands in a fraction of
//! a second.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;
use sevenz_rust2::{Archive as SzArchive, BlockDecoder, Password};
use zipmount_i18n::t;

use crate::blockcache::{BlockCache, BlockCacheStats, DecodedBlock};
use crate::entry::EntryMeta;
use crate::secret::{PasswordError, Secret};

pub struct SevenZBackend {
    path: PathBuf,
    archive: SzArchive,
    cache: BlockCache,
    /// A mutex per block: two threads must not expand the same block at once.
    /// During a parallel search that saves gigabytes of work.
    locks: Vec<Mutex<()>>,
    threads: u32,
    password: Password,
}

impl SevenZBackend {
    pub fn open(
        path: &Path,
        block_budget: usize,
        password: Option<Secret>,
    ) -> Result<(Self, Vec<EntryMeta>)> {
        let had_password = password.is_some();
        let password = match &password {
            Some(p) => Password::from(p.as_str().as_ref()),
            None => Password::empty(),
        };

        // A 7z header can itself be encrypted (the archiver's -mhe switch), so
        // the password is needed to parse the table of contents already, not
        // only to read data.
        let mut file = File::open(path)
            .with_context(|| t!("core-open-failed", path = path.display().to_string()))?;
        let archive = SzArchive::read(&mut file, &password).map_err(|e| match e {
            // The crate knows these two for sure; its own text for them is
            // English and adds nothing to ours.
            sevenz_rust2::Error::PasswordRequired => PasswordError::missing().into(),
            sevenz_rust2::Error::MaybeBadPassword(_) if had_password => {
                PasswordError::wrong().into()
            }
            // Anything else cannot be told apart from a damaged archive, so
            // the message offers both, and the error is marked as a password
            // one — the menu's search then asks for it.
            other => {
                let message = if had_password {
                    t!(
                        "core-7z-structure-password",
                        path = path.display().to_string()
                    )
                } else {
                    t!("core-7z-structure", path = path.display().to_string())
                };
                anyhow::Error::new(other).context(PasswordError::with_message(message))
            }
        })?;

        let meta = archive
            .files
            .iter()
            .map(|f| EntryMeta {
                path: f.name.clone(),
                is_dir: f.is_directory,
                size: f.size,
                mtime: if f.has_last_modified_date {
                    u64::from(f.last_modified_date)
                } else {
                    0
                },
            })
            .collect();

        let block_count = archive.blocks.len();
        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);

        Ok((
            Self {
                path: path.to_path_buf(),
                archive,
                cache: BlockCache::new(block_budget),
                locks: (0..block_count).map(|_| Mutex::new(())).collect(),
                threads,
                password,
            },
            meta,
        ))
    }

    pub fn block_count(&self) -> usize {
        self.archive.blocks.len()
    }

    pub fn is_solid(&self) -> bool {
        self.archive.is_solid
    }

    pub fn entry_count(&self) -> usize {
        self.archive.files.len()
    }

    /// The entry's CRC32 from the archive header — the reference for checking
    /// reads. `None` if the archiver did not store one.
    pub fn crc32(&self, index: u32) -> Option<u32> {
        let f = &self.archive.files[index as usize];
        f.has_crc.then_some(f.crc as u32)
    }

    pub fn block_of(&self, index: u32) -> Option<usize> {
        *self
            .archive
            .stream_map
            .file_block_index
            .get(index as usize)?
    }

    pub fn cache_stats(&self) -> BlockCacheStats {
        self.cache.stats()
    }

    /// An entry's contents. The first access expands its whole block into the
    /// cache; later ones hit the cache.
    pub fn entry_bytes(&self, index: u32) -> Result<Arc<[u8]>> {
        let Some(block) = self.block_of(index) else {
            // A directory or an empty file: no data stream of its own.
            return Ok(Arc::from(Vec::new()));
        };

        if let Some(found) = self.cache.get(block).and_then(|b| b.get(index)) {
            return Ok(found);
        }

        let _guard = self.locks[block]
            .lock()
            .expect("block lock poisoned by a panic");

        // While we waited for the mutex, another thread may have expanded it.
        if let Some(found) = self.cache.get(block).and_then(|b| b.get(index)) {
            return Ok(found);
        }

        let decoded = self.decode_block(block)?;
        self.cache.insert(block, decoded.clone());

        decoded
            .get(index)
            .with_context(|| format!("entry {index} is missing from block {block}"))
    }

    /// Expands a whole block into memory.
    fn decode_block(&self, block: usize) -> Result<Arc<DecodedBlock>> {
        let mut entries: FxHashMap<u32, Arc<[u8]>> = FxHashMap::default();
        let start = self.archive.stream_map.block_first_file_index[block] as u32;
        let mut index = start;

        self.for_each_in_block(block, |_entry_index, data| {
            entries.insert(index, Arc::from(data));
            index += 1;
            Ok(())
        })
        .with_context(|| t!("core-7z-block-failed", block = block))?;

        Ok(Arc::new(DecodedBlock::new(entries)))
    }

    /// A streaming pass through a block: each entry's contents go to a
    /// temporary buffer and are forgotten right away. Memory does not grow
    /// with the block's size, so this is the main path for search and
    /// verification.
    ///
    /// The crate checks every entry's CRC on the way, so an error from here
    /// can also mean damaged data.
    pub fn for_each_in_block<F>(&self, block: usize, mut visit: F) -> Result<()>
    where
        F: FnMut(u32, Vec<u8>) -> Result<()>,
    {
        let mut file = File::open(&self.path)
            .with_context(|| t!("core-open-failed", path = self.path.display().to_string()))?;
        let decoder = BlockDecoder::new(
            self.threads,
            block,
            &self.archive,
            &self.password,
            &mut file,
        );

        let start = self.archive.stream_map.block_first_file_index[block] as u32;
        let mut index = start;
        // The callback's own error is carried out separately: the crate's
        // error type cannot hold an anyhow one.
        let mut failure: Option<anyhow::Error> = None;

        let result = decoder.for_each_entries(&mut |entry, reader| {
            let mut buf = Vec::new();
            if entry.has_stream && entry.size > 0 {
                buf.reserve_exact(entry.size as usize);
                reader.read_to_end(&mut buf)?;
            }
            let current = index;
            index += 1;
            match visit(current, buf) {
                Ok(()) => Ok(true),
                Err(e) => {
                    failure = Some(e);
                    Ok(false) // stop the walk; the error is returned outside
                }
            }
        });

        if let Some(e) = failure {
            return Err(e);
        }
        result.map_err(|e| {
            anyhow::anyhow!(t!(
                "core-7z-block-error",
                block = block,
                error = e.to_string()
            ))
        })?;
        Ok(())
    }
}
