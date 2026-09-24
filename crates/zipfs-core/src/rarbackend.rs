//! The RAR backend. Built only with the `rar` feature — see the README on why
//! official builds leave it out.
//!
//! RAR is closed, and the only complete decompression implementation is
//! RARLAB's UnRAR source. The `unrar` crate wraps it: the crate is MIT, while
//! the UnRAR terms allow using the code to read archives without restriction
//! but forbid building a compatible packer on it. We only read — but that
//! restriction is what keeps it out of GPL builds; the other price is C++ in
//! the build.
//!
//! In access model RAR is closest to 7z: an archive can be solid, and then
//! decompressing one file means going through the whole preceding stream.
//! Hence the same two modes as there:
//!
//! * random access — through a cache; in a solid archive one miss fills the
//!   whole cache, because one pass is cheaper than a pass per file;
//! * full scans (search, verification) — one sequential pass without a
//!   cache, at constant memory.
//!
//! An ordinary (non-solid) archive is simpler: an entry is read by address,
//! and skipping the previous files does not require decompressing them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use unrar::Archive as RarArchive;
use zipmount_i18n::t;

use crate::cache::ContentCache;
use crate::entry::EntryMeta;
use crate::secret::{PasswordError, Secret};
use crate::zipfmt::dos_to_filetime;

pub struct RarBackend {
    path: PathBuf,
    password: Option<Secret>,
    entries: Vec<RarEntry>,
    solid: bool,
    has_encrypted_headers: bool,
    /// RAR5 or the older RAR4: that decides whether an encrypted entry's
    /// checksum can be trusted.
    rar5: bool,
    cache: ContentCache,
    /// For a solid archive: the cache is already filled by a full pass.
    filled: AtomicBool,
    /// A pass over a solid archive is long, and running it from several
    /// threads at once is pure waste.
    fill_lock: Mutex<()>,
}

#[derive(Debug, Clone)]
pub struct RarEntry {
    pub path: String,
    pub size: u64,
    pub crc32: u32,
    pub mtime: u64,
    pub is_dir: bool,
    pub encrypted: bool,
}

impl RarBackend {
    pub fn open(
        path: &Path,
        cache_budget: usize,
        password: Option<Secret>,
    ) -> Result<(Self, Vec<EntryMeta>)> {
        let (entries, solid, encrypted_headers) = list_entries(path, password.as_ref())?;
        let rar5 = detect_rar5(path);

        let meta = entries
            .iter()
            .map(|e| EntryMeta {
                path: e.path.clone(),
                is_dir: e.is_dir,
                size: e.size,
                mtime: e.mtime,
            })
            .collect();

        Ok((
            Self {
                path: path.to_path_buf(),
                password,
                entries,
                solid,
                has_encrypted_headers: encrypted_headers,
                rar5,
                cache: ContentCache::new(cache_budget),
                filled: AtomicBool::new(false),
                fill_lock: Mutex::new(()),
            },
            meta,
        ))
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entry(&self, index: u32) -> &RarEntry {
        &self.entries[index as usize]
    }

    pub fn is_solid(&self) -> bool {
        self.solid
    }

    pub fn has_encrypted_headers(&self) -> bool {
        self.has_encrypted_headers
    }

    /// The entry's CRC32 from the table of contents, if it can be trusted.
    ///
    /// RAR5 has a subtlety: if a file is encrypted but the table of contents
    /// is not, the format garbles the stored checksum on purpose, so that it
    /// cannot be used to check a guessed password — otherwise guessing would
    /// go on without a single decryption attempt. Comparing with such a value
    /// is pointless; it is not supposed to match.
    ///
    /// With an encrypted table of contents there is nothing to hide and the
    /// checksum is real. RAR4 has no such trick at all; its checksum is always
    /// right.
    pub fn crc32(&self, index: u32) -> Option<u32> {
        let e = &self.entries[index as usize];
        if e.is_dir {
            return None;
        }
        if self.rar5 && e.encrypted && !self.has_encrypted_headers {
            return None;
        }
        Some(e.crc32)
    }

    /// An entry's contents.
    pub fn entry_bytes(&self, index: u32) -> Result<Arc<[u8]>> {
        if self.entries[index as usize].is_dir {
            return Ok(Arc::from(Vec::new()));
        }
        if let Some(found) = self.cache.get(index) {
            return Ok(found);
        }

        if self.solid {
            // Skipping in a solid archive requires decompression anyway, so
            // pulling files out one by one is pointless: one pass fills the
            // whole cache and pays off by the second access.
            self.fill_cache()?;
            if let Some(found) = self.cache.get(index) {
                return Ok(found);
            }
            // The entry did not fit in the cache — read it by address.
        }

        let data = self.read_single(index)?;
        let data: Arc<[u8]> = Arc::from(data);
        self.cache.insert(index, data.clone());
        Ok(data)
    }

    /// One full pass over the archive. Each entry's contents go to a
    /// temporary buffer and are forgotten at once, so memory does not grow.
    pub fn for_each_entry<F>(&self, visit: F) -> Result<()>
    where
        F: FnMut(u32, Vec<u8>) -> Result<()>,
    {
        self.for_each_entry_filtered(|_| true, visit)
    }

    /// The same, but unwanted entries are skipped without reading.
    ///
    /// In an ordinary archive skipping is cheap — no decompression at all. In
    /// a solid one the stream has to be decompressed anyway, but at least the
    /// data is neither copied nor scanned.
    pub fn for_each_entry_filtered<P, F>(&self, want: P, mut visit: F) -> Result<()>
    where
        P: Fn(u32) -> bool,
        F: FnMut(u32, Vec<u8>) -> Result<()>,
    {
        let mut archive = self.open_for_processing()?;
        let mut index = 0u32;

        while let Some(header) = archive.read_header().map_err(|e| self.err(e))? {
            let is_dir = header.entry().is_directory();
            if is_dir || !want(index) {
                archive = header.skip().map_err(|e| self.err(e))?;
            } else {
                let (bytes, next) = header.read().map_err(|e| self.err(e))?;
                archive = next;
                visit(index, bytes)?;
            }
            index += 1;
        }
        Ok(())
    }

    /// Fills the cache with the whole archive in one pass.
    fn fill_cache(&self) -> Result<()> {
        if self.filled.load(Ordering::Acquire) {
            return Ok(());
        }
        let _guard = self.fill_lock.lock().expect("lock poisoned by a panic");
        if self.filled.load(Ordering::Acquire) {
            return Ok(());
        }

        self.for_each_entry(|index, bytes| {
            self.cache.insert(index, Arc::from(bytes));
            Ok(())
        })?;

        self.filled.store(true, Ordering::Release);
        Ok(())
    }

    /// Reads one entry by address: skip the ones before it, read the one wanted.
    fn read_single(&self, index: u32) -> Result<Vec<u8>> {
        let mut archive = self.open_for_processing()?;
        let mut current = 0u32;

        while let Some(header) = archive.read_header().map_err(|e| self.err(e))? {
            if current == index {
                let (bytes, _) = header.read().map_err(|e| self.err(e))?;
                return Ok(bytes);
            }
            archive = header.skip().map_err(|e| self.err(e))?;
            current += 1;
        }

        bail!("entry {index} not found in the archive")
    }

    fn open_for_processing(
        &self,
    ) -> Result<unrar::OpenArchive<unrar::Process, unrar::CursorBeforeHeader>> {
        let opened = match &self.password {
            Some(pw) => {
                let pw = pw.as_str().into_owned();
                RarArchive::with_password(&self.path, &pw).open_for_processing()
            }
            None => RarArchive::new(&self.path).open_for_processing(),
        };
        opened.map_err(|e| archive_error(e, self.password.is_some()))
    }

    /// UnRAR errors say little, and the most common one is about the password.
    fn err(&self, e: unrar::error::UnrarError) -> anyhow::Error {
        archive_error(e, self.password.is_some())
    }

    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }
}

/// The format version from the signature: byte seven is zero in RAR4, one in
/// RAR5.
fn detect_rar5(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 8];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && head.starts_with(b"Rar!")
        && head[6] == 1
}

/// Lists the entries, finding out on the way whether the archive is solid.
fn list_entries(path: &Path, password: Option<&Secret>) -> Result<(Vec<RarEntry>, bool, bool)> {
    let opened = match password {
        Some(pw) => {
            let pw = pw.as_str().into_owned();
            RarArchive::with_password(path, &pw).open_for_listing()
        }
        None => RarArchive::new(path).open_for_listing(),
    };
    let mut archive = opened.map_err(|e| archive_error(e, password.is_some()))?;

    let solid = archive.is_solid();
    let encrypted_headers = archive.has_encrypted_headers();

    let mut entries = Vec::new();
    for item in archive.by_ref() {
        let header = item.map_err(|e| archive_error(e, password.is_some()))?;
        entries.push(RarEntry {
            path: header.filename.to_string_lossy().into_owned(),
            size: header.unpacked_size,
            crc32: header.file_crc,
            // RAR keeps time in DOS format: the high 16 bits are the date.
            mtime: dos_to_filetime((header.file_time >> 16) as u16, header.file_time as u16),
            is_dir: header.is_directory(),
            encrypted: header.is_encrypted(),
        });
    }

    Ok((entries, solid, encrypted_headers))
}

/// UnRAR errors say little, and the most common of them is a wrong password.
fn archive_error(err: unrar::error::UnrarError, had_password: bool) -> anyhow::Error {
    use unrar::error::Code;
    match err.code {
        Code::BadPassword if had_password => PasswordError::wrong().into(),
        Code::BadPassword | Code::MissingPassword => PasswordError::missing().into(),
        other => anyhow::anyhow!(t!("core-rar-error", error = format!("{other:?}"))),
    }
}
