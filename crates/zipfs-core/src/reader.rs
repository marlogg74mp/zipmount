//! Reading entry contents.
//!
//! Zip has three read paths, and that is not premature optimization but a
//! direct consequence of scale:
//!
//! * `stored` — a slice of the mapped file, no copying, no decompression;
//! * small `deflate` — decompressed whole into an LRU cache, then slices from
//!   memory;
//! * large `deflate` — a streaming decoder with a checkpoint index: forwards
//!   it streams at constant memory, backwards it jumps to the nearest
//!   checkpoint instead of restarting from the start of the entry.
//!
//! 7z leaves no choice: the unit of access is a solid block, which is expanded
//! whole anyway, so an entry is simply taken from the block cache.

use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use zipmount_i18n::t;

use crate::archive::{Archive, Backend};
use crate::cache::ContentCache;
use crate::deflate_index::{IndexedReader, DEFAULT_SPAN};
use crate::zipbackend::ZipBackend;
use crate::zipfmt::{self, METHOD_DEFLATE, METHOD_STORED};

/// Entries above this size are streamed rather than materialized.
pub const DEFAULT_MATERIALIZE_MAX: u64 = 8 * 1024 * 1024;

pub enum FileHandle {
    Zip(ZipHandle),
    SevenZ {
        data: Arc<[u8]>,
    },
    /// tar: data lies in one contiguous piece, so a range is enough.
    Tar {
        offset: usize,
        len: usize,
    },
}

impl FileHandle {
    pub fn open(archive: &Archive, node: u32) -> Result<Self> {
        Self::open_with_limit(archive, node, DEFAULT_MATERIALIZE_MAX)
    }

    pub fn open_with_limit(archive: &Archive, node: u32, materialize_max: u64) -> Result<Self> {
        let info = archive.tree().node(node);
        if info.is_dir {
            bail!("the node is a directory, not a file");
        }
        let index = archive.entry_index(node)?;

        match archive.backend() {
            Backend::Zip(b) => Ok(FileHandle::Zip(ZipHandle::open(b, index, materialize_max)?)),
            Backend::SevenZ(b) => Ok(FileHandle::SevenZ {
                data: b.entry_bytes(index)?,
            }),
            Backend::Tar(b) => {
                let (offset, len) = b.entry_range(index)?;
                Ok(FileHandle::Tar { offset, len })
            }
            // RAR, like 7z, only hands out an entry whole.
            #[cfg(feature = "rar")]
            Backend::Rar(b) => Ok(FileHandle::SevenZ {
                data: b.entry_bytes(index)?,
            }),
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            FileHandle::Zip(h) => h.size,
            FileHandle::SevenZ { data } => data.len() as u64,
            FileHandle::Tar { len, .. } => *len as u64,
        }
    }

    /// Reads up to `buf.len()` bytes starting at `offset`. Returns how many were
    /// read; 0 means end of file.
    pub fn read_at(
        &mut self,
        archive: &Archive,
        cache: &ContentCache,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<usize> {
        match self {
            FileHandle::Zip(h) => h.read_at(archive.as_zip()?, cache, offset, buf),
            FileHandle::SevenZ { data } => {
                if buf.is_empty() || offset >= data.len() as u64 {
                    return Ok(0);
                }
                let start = offset as usize;
                let end = (start + buf.len()).min(data.len());
                buf[..end - start].copy_from_slice(&data[start..end]);
                Ok(end - start)
            }
            FileHandle::Tar { offset: base, len } => {
                if buf.is_empty() || offset >= *len as u64 {
                    return Ok(0);
                }
                let data = archive.as_tar()?.data();
                let start = *base + offset as usize;
                let end = (start + buf.len()).min(*base + *len);
                buf[..end - start].copy_from_slice(&data[start..end]);
                Ok(end - start)
            }
        }
    }
}

pub struct ZipHandle {
    entry_idx: u32,
    /// The real compression method: encrypted entries carry a placeholder in
    /// the main header, with the method in an extra field.
    method: u16,
    size: u64,
    data_off: u64,
    comp_size: u64,
    materialize_max: u64,
    /// The decrypted compressed stream. `None` for unencrypted entries, whose
    /// data is then read as a slice of the mapped file without copying.
    decrypted: Option<Arc<[u8]>>,
    stream: Option<IndexedReader>,
}

impl ZipHandle {
    fn open(backend: &ZipBackend, index: u32, materialize_max: u64) -> Result<Self> {
        let entry = backend.entry(index);
        let method = entry.effective_method();

        if method != METHOD_STORED && method != METHOD_DEFLATE {
            bail!(t!("core-zip-method", method = method));
        }

        let data_off = zipfmt::data_offset(backend.data(), entry.local_header_offset)?;

        // Decrypt once, at open time: ZipCrypto keys evolve byte by byte, so an
        // arbitrary piece cannot be decrypted apart from the start of the entry
        // at all.
        let decrypted = if entry.is_encrypted() {
            Some(Arc::from(backend.compressed_bytes(index)?.into_owned()))
        } else {
            None
        };

        Ok(Self {
            entry_idx: index,
            method,
            size: entry.uncomp_size,
            data_off,
            comp_size: entry.comp_size,
            materialize_max,
            decrypted,
            stream: None,
        })
    }

    fn read_at(
        &mut self,
        backend: &ZipBackend,
        cache: &ContentCache,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<usize> {
        if buf.is_empty() || offset >= self.size {
            return Ok(0);
        }
        let want = buf.len().min((self.size - offset) as usize);
        let buf = &mut buf[..want];

        // Clone the Arc into a local so the slice does not borrow self: mutable
        // access to the streaming decoder is needed below.
        let decrypted = self.decrypted.clone();
        let comp: &[u8] = match &decrypted {
            Some(d) => d,
            None => compressed_slice(backend, self.data_off, self.comp_size)?,
        };

        if self.method == METHOD_STORED {
            let start = offset as usize;
            let src = comp
                .get(start..start + want)
                .with_context(|| t!("core-entry-out-of-bounds"))?;
            buf.copy_from_slice(src);
            return Ok(want);
        }

        if self.size <= self.materialize_max {
            let data = match cache.get(self.entry_idx) {
                Some(d) => d,
                None => {
                    let decoded: Arc<[u8]> = inflate_exact(comp, self.size)?.into();
                    cache.insert(self.entry_idx, decoded.clone());
                    decoded
                }
            };
            let start = offset as usize;
            let end = (start + want).min(data.len());
            if start >= end {
                return Ok(0);
            }
            buf[..end - start].copy_from_slice(&data[start..end]);
            return Ok(end - start);
        }

        if self.stream.is_none() {
            self.stream = Some(IndexedReader::new(DEFAULT_SPAN, self.size)?);
        }
        let stream = self.stream.as_mut().expect("the reader was just created");
        stream.read_at(comp, offset, buf)
    }
}

fn compressed_slice(backend: &ZipBackend, data_off: u64, comp_size: u64) -> Result<&[u8]> {
    let start = usize::try_from(data_off).context("data offset does not fit in usize")?;
    let len = usize::try_from(comp_size).context("entry size does not fit in usize")?;
    backend
        .data()
        .get(start..start + len)
        .with_context(|| t!("core-entry-out-of-bounds"))
}

/// Decompresses a whole zip entry, taking the encryption off first if needed.
pub fn read_zip_entry(backend: &ZipBackend, index: u32) -> Result<Vec<u8>> {
    let entry = backend.entry(index);
    let comp = backend.compressed_bytes(index)?;

    match entry.effective_method() {
        METHOD_STORED => Ok(comp.into_owned()),
        METHOD_DEFLATE => inflate_exact(&comp, entry.uncomp_size),
        other => bail!(t!("core-zip-method", method = other)),
    }
}

/// Decompression into a buffer of known size. libdeflate is noticeably faster
/// than a streaming decoder, so it is used wherever the size is known.
fn inflate_exact(comp: &[u8], size: u64) -> Result<Vec<u8>> {
    if size == 0 {
        return Ok(Vec::new());
    }
    let size = usize::try_from(size).context("entry size does not fit in usize")?;
    let mut out = vec![0u8; size];
    let mut dec = libdeflater::Decompressor::new();
    let written = dec
        .deflate_decompress(comp, &mut out)
        .map_err(|e| anyhow!(t!("core-deflate-failed", error = format!("{e:?}"))))?;
    out.truncate(written);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    /// Text that compresses well but does not degenerate into one repeating
    /// sequence — so that checking offsets makes sense.
    fn sample(len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        let mut i = 0usize;
        while out.len() < len {
            out.extend_from_slice(format!("line {i} of text to search\n").as_bytes());
            i += 1;
        }
        out.truncate(len);
        out
    }

    fn deflate(data: &[u8]) -> Vec<u8> {
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn inflate_exact_round_trips() {
        let data = sample(100_000);
        let comp = deflate(&data);
        assert_eq!(inflate_exact(&comp, data.len() as u64).unwrap(), data);
    }

    #[test]
    fn inflate_exact_handles_empty() {
        assert!(inflate_exact(&[], 0).unwrap().is_empty());
    }
}
