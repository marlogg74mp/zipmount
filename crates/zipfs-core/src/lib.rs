//! The ZipMount core: archive parsing, the directory tree, reading and search.
//!
//! The crate knows nothing about WinFsp. That lets the most error-prone part
//! be debugged with ordinary tests and console commands, without touching the
//! filesystem driver.
//!
//! Formats differ in their unit of access — zip compresses every entry on its
//! own, 7z glues entries into solid blocks, tar.gz is one solid stream. The
//! difference hides behind `Archive`, but shows through honestly where
//! performance depends on it: in search and verification.

pub mod archive;
pub mod blockcache;
pub mod cache;
pub mod deflate_index;
pub mod entry;
pub mod names;
#[cfg(feature = "rar")]
pub mod rarbackend;
pub mod reader;
pub mod sanitize;
pub mod search;
pub mod secret;
pub mod sevenz;
pub mod tarbackend;
pub mod tarfmt;
pub mod tree;
pub mod verify;
pub mod zipbackend;
pub mod zipcrypt;
pub mod zipfmt;

pub use archive::{Archive, Backend, OpenOptions};
pub use blockcache::{BlockCacheStats, DEFAULT_BLOCK_BUDGET};
pub use cache::{CacheStats, ContentCache};
pub use entry::{EntryMeta, Format};
pub use names::NameEncoding;
pub use reader::FileHandle;
pub use search::{
    dos_pattern_match, find_by_name, glob_match, grep, FileCount, GrepOptions, GrepOutcome, Match,
};
pub use secret::{needs_password, PasswordError, Secret};
pub use tree::{Node, Tree, TreeStats, ROOT};
pub use verify::{verify, VerifyOptions, VerifyOutcome};
