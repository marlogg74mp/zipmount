//! An open archive: the directory tree plus the backend for its format.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use zipmount_i18n::t;

use crate::blockcache::DEFAULT_BLOCK_BUDGET;
use crate::entry::Format;
use crate::names::NameEncoding;
#[cfg(feature = "rar")]
use crate::rarbackend::RarBackend;
use crate::secret::Secret;
use crate::sevenz::SevenZBackend;
use crate::tarbackend::TarBackend;
use crate::tree::Tree;
use crate::zipbackend::ZipBackend;

/// Options for opening an archive.
///
/// A struct rather than an argument list: there are three already, and
/// positional parameters next to a password are a sure way to one day pass
/// the wrong one.
pub struct OpenOptions {
    pub encoding: NameEncoding,
    /// Ceiling for the cache of expanded solid blocks; only matters for 7z.
    pub block_budget: usize,
    pub password: Option<Secret>,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            encoding: NameEncoding::Auto,
            block_budget: DEFAULT_BLOCK_BUDGET,
            password: None,
        }
    }
}

pub enum Backend {
    Zip(ZipBackend),
    /// Boxed: the 7z structure is an order of magnitude larger than the zip
    /// one, and unboxed it alone would set the size of the whole enum.
    SevenZ(Box<SevenZBackend>),
    Tar(Box<TarBackend>),
    #[cfg(feature = "rar")]
    Rar(Box<RarBackend>),
}

pub struct Archive {
    path: PathBuf,
    size: u64,
    format: Format,
    tree: Tree,
    backend: Backend,
}

impl Archive {
    pub fn open(path: impl AsRef<Path>, encoding: NameEncoding) -> Result<Self> {
        Self::open_with(
            path,
            OpenOptions {
                encoding,
                ..Default::default()
            },
        )
    }

    /// Opens an archive and builds the tree from its table of contents.
    ///
    /// File bodies are not read, so opening time depends on the number of
    /// entries, not on the size of the archive.
    pub fn open_with(path: impl AsRef<Path>, opts: OpenOptions) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)
            .with_context(|| t!("core-open-failed", path = path.display().to_string()))?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);

        // 512 bytes: exactly one tar header block, whose magic sits at
        // offset 257.
        let mut head = [0u8; 512];
        let read = file.read(&mut head).unwrap_or(0);
        let format = Format::detect(&head[..read])
            .with_context(|| t!("core-unknown-format", path = path.display().to_string()))?;

        let (backend, meta) = match format {
            Format::Zip => {
                let (b, m) = ZipBackend::open(&file, opts.encoding, opts.password)?;
                (Backend::Zip(b), m)
            }
            Format::SevenZ => {
                let (b, m) = SevenZBackend::open(&path, opts.block_budget, opts.password)?;
                (Backend::SevenZ(Box::new(b)), m)
            }
            Format::Tar => {
                let (b, m) = TarBackend::open_plain(&file)?;
                (Backend::Tar(Box::new(b)), m)
            }
            Format::TarGz => {
                let (b, m) = TarBackend::open_gzip(&path, opts.block_budget)?;
                (Backend::Tar(Box::new(b)), m)
            }
            #[cfg(feature = "rar")]
            Format::Rar => {
                let (b, m) = RarBackend::open(&path, opts.block_budget, opts.password)?;
                (Backend::Rar(Box::new(b)), m)
            }
            // The format is recognised without UnRAR too — by its signature —
            // so that the error says what is going on rather than "unknown
            // format".
            #[cfg(not(feature = "rar"))]
            Format::Rar => bail!(t!("core-rar-not-built", path = path.display().to_string())),
        };

        let tree = Tree::build(&meta);

        Ok(Self {
            path,
            size,
            format,
            tree,
            backend,
        })
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    /// The backend's entry index for a tree node.
    pub fn entry_index(&self, node: u32) -> Result<u32> {
        self.tree
            .node(node)
            .entry
            .context("the node has no matching entry in the archive")
    }

    /// The whole contents of an entry.
    pub fn read_full(&self, node: u32) -> Result<Vec<u8>> {
        let index = self.entry_index(node)?;
        match &self.backend {
            Backend::Zip(b) => crate::reader::read_zip_entry(b, index),
            Backend::SevenZ(b) => Ok(b.entry_bytes(index)?.to_vec()),
            Backend::Tar(b) => Ok(b.entry_bytes(index)?.to_vec()),
            #[cfg(feature = "rar")]
            Backend::Rar(b) => Ok(b.entry_bytes(index)?.to_vec()),
        }
    }

    /// A slice of the entry's contents right inside the archive data, when
    /// the format allows it. Tar keeps data contiguous, so there is no reason
    /// to copy it — search reads it without a single extra allocation.
    pub fn entry_slice(&self, node: u32) -> Option<&[u8]> {
        let index = self.tree.node(node).entry?;
        match &self.backend {
            Backend::Tar(b) => b.entry_bytes(index).ok(),
            _ => None,
        }
    }

    /// The entry's checksum from the archive's table of contents, if the
    /// archiver stored one.
    pub fn crc32_of(&self, node: u32) -> Option<u32> {
        let index = self.tree.node(node).entry?;
        match &self.backend {
            Backend::Zip(b) => b.crc32(index),
            Backend::SevenZ(b) => b.crc32(index),
            // tar keeps no checksums of contents at all: the header carries
            // a checksum of the header alone.
            Backend::Tar(_) => None,
            #[cfg(feature = "rar")]
            Backend::Rar(b) => b.crc32(index),
        }
    }

    /// How many entries are encrypted. `None` if the format does not say so
    /// in its table of contents (7z hides the flag inside the block).
    pub fn encrypted_entry_count(&self) -> Option<usize> {
        match &self.backend {
            Backend::Zip(b) => Some(
                (0..b.entry_count() as u32)
                    .filter(|i| b.entry(*i).is_encrypted())
                    .count(),
            ),
            _ => None,
        }
    }

    pub fn as_zip(&self) -> Result<&ZipBackend> {
        match &self.backend {
            Backend::Zip(b) => Ok(b),
            _ => bail!("the archive is not a zip"),
        }
    }

    #[cfg(feature = "rar")]
    pub fn as_rar(&self) -> Option<&RarBackend> {
        match &self.backend {
            Backend::Rar(b) => Some(b.as_ref()),
            _ => None,
        }
    }

    pub fn as_tar(&self) -> Result<&TarBackend> {
        match &self.backend {
            Backend::Tar(b) => Ok(b),
            _ => bail!("the archive is not a tar"),
        }
    }

    pub fn as_sevenz(&self) -> Option<&SevenZBackend> {
        match &self.backend {
            Backend::SevenZ(b) => Some(b.as_ref()),
            _ => None,
        }
    }
}
