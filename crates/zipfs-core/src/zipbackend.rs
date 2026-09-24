//! The ZIP backend: a memory-mapped file plus the parsed central directory.
//!
//! All the fuss with name encodings lives here and does not leak out: further
//! up the stack, entries are already described by the format-independent
//! `EntryMeta`.

use std::borrow::Cow;
use std::fs::File;

use anyhow::{Context, Result};
use memmap2::Mmap;
use zipmount_i18n::t;

use crate::entry::EntryMeta;
use crate::names::{decode_name, NameEncoding};
use crate::secret::Secret;
use crate::zipcrypt::decrypt_entry;
use crate::zipfmt::{self, RawEntry};

pub struct ZipBackend {
    mmap: Mmap,
    entries: Vec<RawEntry>,
    password: Option<Secret>,
}

impl ZipBackend {
    pub fn open(
        file: &File,
        encoding: NameEncoding,
        password: Option<Secret>,
    ) -> Result<(Self, Vec<EntryMeta>)> {
        // Mapping into memory gives every thread concurrent reads without a
        // pool of handles, and the OS takes care of caching compressed data.
        //
        // SAFETY: a mapping is unsound if the file is changed from outside
        // while in use. The volume is read-only and the archive must not change
        // while mounted; that is a stated limitation, not an assumption.
        let mmap = unsafe { Mmap::map(file) }.with_context(|| t!("core-mmap-failed"))?;

        let entries =
            zipfmt::parse_central_directory(&mmap).with_context(|| t!("core-zip-structure"))?;

        let meta = entries
            .iter()
            .map(|raw| EntryMeta {
                path: decode_name(
                    &raw.name_raw,
                    raw.unicode_path.as_deref(),
                    raw.has_utf8_flag(),
                    encoding,
                ),
                is_dir: raw.is_dir(),
                size: raw.uncomp_size,
                mtime: raw.mtime_filetime(),
            })
            .collect();

        Ok((
            Self {
                mmap,
                entries,
                password,
            },
            meta,
        ))
    }

    pub fn data(&self) -> &[u8] {
        &self.mmap
    }

    pub fn entry(&self, index: u32) -> &RawEntry {
        &self.entries[index as usize]
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// The entry's CRC32 from the central directory — a ready reference for
    /// checking reads. `None` for entries whose format does not keep it
    /// (WinZip AE-2).
    pub fn crc32(&self, index: u32) -> Option<u32> {
        self.entries[index as usize].stored_crc32()
    }

    pub fn has_password(&self) -> bool {
        self.password.is_some()
    }

    /// The entry's compressed bytes, already decrypted if it was encrypted.
    ///
    /// For an ordinary entry this is a slice of the mapped file, no copying;
    /// for an encrypted one, a fresh buffer, because a read-only mapping has
    /// nowhere to hold the decryption.
    pub fn compressed_bytes(&self, index: u32) -> Result<Cow<'_, [u8]>> {
        let entry = self.entry(index);
        let data_off = zipfmt::data_offset(&self.mmap, entry.local_header_offset)?;
        let start = usize::try_from(data_off).context("data offset does not fit in usize")?;
        let len = usize::try_from(entry.comp_size).context("entry size does not fit in usize")?;
        let raw = self
            .mmap
            .get(start..start + len)
            .with_context(|| t!("core-entry-out-of-bounds"))?;

        if entry.is_encrypted() {
            let plain = decrypt_entry(raw, entry, self.password.as_ref())?;
            Ok(Cow::Owned(plain))
        } else {
            Ok(Cow::Borrowed(raw))
        }
    }
}
