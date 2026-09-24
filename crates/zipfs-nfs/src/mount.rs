//! Serving and mounting on macOS.
//!
//! The export is named after the archive's own path, so the system's list of
//! mounts shows `127.0.0.1:/Users/me/logs.7z` as the source: which archive is
//! mounted where comes straight from the kernel, with no list of our own to
//! go stale.

use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use nfsserve::tcp::{NFSTcp, NFSTcpListener};
use tokio::runtime::Runtime;
use zipfs_core::Archive;
use zipmount_i18n::t;

use crate::ZipNfs;

/// Mounts from this host: ours, as `127.0.0.1:/<archive path>`.
const SOURCE_PREFIX: &str = "127.0.0.1:";

/// Mount settings.
pub struct MountOptions {
    /// An existing empty directory.
    pub mountpoint: PathBuf,
    /// The archive's absolute path; it names the export.
    pub archive_path: String,
    pub cache_budget: usize,
}

/// A mounted archive, as the system lists it.
pub struct MountRecord {
    pub mountpoint: PathBuf,
    pub archive: PathBuf,
}

/// What `getfsstat` lists as an NFS mount from 127.0.0.1.
pub fn mounts() -> Vec<MountRecord> {
    // SAFETY: a null buffer asks only for the count.
    let count = unsafe { libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT) };
    if count <= 0 {
        return Vec::new();
    }
    // Room for a few mounts appearing in between.
    let mut buf: Vec<libc::statfs> = Vec::with_capacity(count as usize + 8);
    let bytes = (buf.capacity() * std::mem::size_of::<libc::statfs>()) as libc::c_int;
    // SAFETY: the buffer holds `capacity` entries and `bytes` says exactly
    // that; getfsstat fills at most that many and returns how many it did.
    let filled = unsafe { libc::getfsstat(buf.as_mut_ptr(), bytes, libc::MNT_NOWAIT) };
    if filled <= 0 {
        return Vec::new();
    }
    // SAFETY: getfsstat initialized the first `filled` entries.
    unsafe { buf.set_len(filled as usize) };

    let text = |field: &[libc::c_char]| {
        // SAFETY: the kernel NUL-terminates these fixed-size fields.
        unsafe { CStr::from_ptr(field.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    };
    buf.iter()
        .filter(|s| text(&s.f_fstypename) == "nfs")
        .filter_map(|s| {
            let source = text(&s.f_mntfromname);
            let archive = source.strip_prefix(SOURCE_PREFIX)?;
            Some(MountRecord {
                mountpoint: PathBuf::from(text(&s.f_mntonname)),
                archive: PathBuf::from(archive),
            })
        })
        .collect()
}

fn is_mounted(mountpoint: &Path) -> bool {
    mounts().iter().any(|m| m.mountpoint == mountpoint)
}

/// Unmounts from another thread — a signal handler's, typically.
#[derive(Clone)]
pub struct Unmounter {
    mountpoint: PathBuf,
}

impl Unmounter {
    /// Unmounts, forcibly if a file is still open: this is the owner asking
    /// to stop, and a server that is going away cannot serve that file anyway.
    pub fn unmount(&self) {
        let plain = Command::new("/sbin/umount").arg(&self.mountpoint).status();
        if !plain.is_ok_and(|s| s.success()) {
            let _ = Command::new("/sbin/umount")
                .arg("-f")
                .arg(&self.mountpoint)
                .status();
        }
    }
}

/// A mounted archive. [`Mount::run`] serves it until it is unmounted.
pub struct Mount {
    runtime: Runtime,
    mountpoint: PathBuf,
}

impl Mount {
    pub fn new(archive: Archive, opts: MountOptions) -> Result<Self> {
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .min(8);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(threads)
            .enable_all()
            .build()
            .context("cannot start the NFS server's runtime")?;

        let fs = ZipNfs::new(archive, opts.cache_budget);
        let export = opts.archive_path.clone();
        // Port 0: the system picks a free one, so several archives can be
        // mounted at once.
        let port = runtime.block_on(async {
            let mut listener = NFSTcpListener::bind("127.0.0.1:0", fs).await?;
            listener.with_export_name(&export);
            let port = listener.get_listen_port();
            tokio::spawn(async move {
                let _ = listener.handle_forever().await;
            });
            Ok::<u16, std::io::Error>(port)
        })?;

        // mount_nfs talks to the server while the runtime's threads serve it.
        // soft and a short timeout: if this process dies, programs reading
        // the volume get an error instead of hanging. locallocks: file locks
        // stay on this Mac, which is all a read-only volume needs.
        let options = format!(
            "port={port},mountport={port},vers=3,tcp,rdonly,soft,intr,timeo=10,retrans=3,locallocks,actimeo=60"
        );
        let source = format!(
            "{SOURCE_PREFIX}/{}",
            opts.archive_path.trim_start_matches('/')
        );
        let output = Command::new("/sbin/mount_nfs")
            .args(["-o", &options])
            .arg(&source)
            .arg(&opts.mountpoint)
            .output()
            .context("cannot run mount_nfs")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(t!(
                "mount-err-nfs",
                mountpoint = opts.mountpoint.display().to_string(),
                error = stderr.trim().to_string()
            ));
        }

        Ok(Self {
            runtime,
            mountpoint: opts.mountpoint,
        })
    }

    pub fn unmounter(&self) -> Unmounter {
        Unmounter {
            mountpoint: self.mountpoint.clone(),
        }
    }

    /// Serves until the directory is unmounted — by [`Unmounter::unmount`],
    /// or from outside: `umount`, or Eject in Finder.
    pub fn run(self) -> Result<()> {
        // The server runs on the runtime's threads; this one only watches the
        // system's list of mounts.
        while is_mounted(&self.mountpoint) {
            std::thread::sleep(Duration::from_millis(500));
        }
        self.runtime.shutdown_timeout(Duration::from_secs(2));
        Ok(())
    }
}
