//! Mounting an archive as a Windows volume through WinFsp.
//!
//! Windows only; `zipfs-fuse` does the same job on Linux.
#![cfg(windows)]

mod fs;
mod security;

use anyhow::{Context, Result};
use winfsp::host::{FileSystemHost, VolumeParams};
use zipfs_core::Archive;
use zipmount_i18n::t;

pub use fs::ZipFs;

/// Volume settings.
pub struct MountOptions {
    /// Mount point: a letter like `Z:` or a path to an empty NTFS directory.
    pub mountpoint: String,
    pub label: String,
    pub cache_budget: usize,
}

/// A mounted volume. Unmounted on drop.
pub struct Mount {
    host: FileSystemHost<ZipFs>,
}

impl Mount {
    pub fn new(archive: Archive, opts: MountOptions) -> Result<Self> {
        let security = security::default_descriptor()?;
        let context = ZipFs::new(archive, opts.cache_budget, opts.label, security);

        let mut params = VolumeParams::new();
        params
            .filesystem_name("ZipFS")
            .sector_size(512)
            .sectors_per_allocation_unit(1)
            .max_component_length(255)
            // Windows expects case-insensitive lookup that preserves case.
            .case_sensitive_search(false)
            .case_preserved_names(true)
            .unicode_on_disk(true)
            .read_only_volume(true)
            // WinFsp does the pattern filtering: we hand over the whole directory.
            .pass_query_directory_pattern(false)
            .persistent_acls(false)
            // The archive's contents never change, so metadata can be cached for
            // long. But NOT with u32::MAX: that turns on the WinFsp cache
            // manager, in which enumeration with a pattern behaves differently.
            .file_info_timeout(10_000);

        // The type is spelled out: FileSystemHost has two locking strategies,
        // and without the annotation the call to start() is ambiguous.
        // FineGuard lets operations run in parallel, which is what we want.
        let mut host: FileSystemHost<ZipFs> = FileSystemHost::new(params, context)
            .map_err(|e| anyhow::anyhow!(t!("mount-err-create", error = format!("{e:?}"))))?;

        host.mount(opts.mountpoint.as_str()).map_err(|e| {
            anyhow::anyhow!(t!(
                "mount-err-mount",
                mountpoint = opts.mountpoint.clone(),
                error = format!("{e:?}")
            ))
        })?;
        host.start()
            .map_err(|e| anyhow::anyhow!(t!("mount-err-dispatcher", error = format!("{e:?}"))))?;

        Ok(Self { host })
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        self.host.stop();
        self.host.unmount();
    }
}

/// Initializes WinFsp. Must happen before a volume is created; if the driver
/// is not installed, this is where we find out, with a clear message.
pub fn init() -> Result<winfsp::FspInit> {
    winfsp::winfsp_init().with_context(|| t!("mount-err-no-winfsp"))
}
