//! Mounting an archive as a directory through FUSE, on Linux.
//!
//! The counterpart of `zipfs-mount`, which does the same through WinFsp. The
//! volume is read-only: it is mounted `ro`, so the kernel refuses writes with
//! EROFS before they ever reach us, and every modifying operation of the
//! trait is left at its default.
//!
//! No libfuse: `fuser` talks to /dev/fuse itself and mounts through the
//! `fusermount3` helper, which every distribution with FUSE installs.
#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use fuser::{
    Config, Errno, FileAttr, FileHandle as Fh, FileType, Filesystem, FopenFlags, Generation,
    INodeNo, LockOwner, MountOption, OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyStatfs, Request, Session, SessionUnmounter,
};
use zipfs_core::{Archive, ContentCache, FileHandle};
use zipmount_i18n::t;

/// The archive never changes while mounted, so the kernel may keep what it
/// learned for as long as it likes.
const TTL: Duration = Duration::from_secs(3600);

const BLOCK_SIZE: u32 = 4096;

/// 1980-01-01, the fallback time for entries without a date: the earliest
/// date a zip can hold, and less surprising than 1970.
const FALLBACK_SECS: u64 = 315_532_800;

/// Seconds between 1601-01-01 (FILETIME, what the core stores) and 1970-01-01.
const FILETIME_TO_UNIX_SECS: u64 = 11_644_473_600;

/// FUSE inode numbers start at 1 for the root; the core's node ids start at 0.
fn ino_of(node: u32) -> INodeNo {
    INodeNo(u64::from(node) + 1)
}

/// A mutex that keeps working after a panic elsewhere: the state it guards
/// (a table of handles, a reader's position) stays usable.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn system_time(filetime: u64) -> SystemTime {
    match (filetime / 10_000_000).checked_sub(FILETIME_TO_UNIX_SECS) {
        Some(secs) if filetime != 0 => {
            UNIX_EPOCH + Duration::new(secs, (filetime % 10_000_000) as u32 * 100)
        }
        _ => UNIX_EPOCH + Duration::from_secs(FALLBACK_SECS),
    }
}

struct ZipFs {
    archive: Archive,
    cache: ContentCache,
    uid: u32,
    gid: u32,
    total_bytes: u64,
    /// Open files. A handle keeps a reader's position (a deflate stream, say),
    /// and reads of one file may arrive on several threads at once.
    handles: Mutex<HashMap<u64, Arc<Mutex<FileHandle>>>>,
    next_handle: AtomicU64,
}

impl ZipFs {
    fn node_of(&self, ino: INodeNo) -> Option<u32> {
        let node = u32::try_from(ino.0.checked_sub(1)?).ok()?;
        ((node as usize) < self.archive.tree().node_count()).then_some(node)
    }

    fn attr(&self, node: u32) -> FileAttr {
        let info = self.archive.tree().node(node);
        let time = system_time(info.mtime);
        FileAttr {
            ino: ino_of(node),
            size: info.size,
            blocks: info.size.div_ceil(512),
            atime: time,
            mtime: time,
            ctime: time,
            crtime: time,
            kind: if info.is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            },
            perm: if info.is_dir { 0o555 } else { 0o444 },
            nlink: if info.is_dir { 2 } else { 1 },
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: BLOCK_SIZE,
            flags: 0,
        }
    }

    fn handle(&self, fh: Fh) -> Option<Arc<Mutex<FileHandle>>> {
        lock(&self.handles).get(&fh.0).cloned()
    }
}

impl Filesystem for ZipFs {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let found = self.node_of(parent).and_then(|dir| {
            let name = name.to_str()?;
            self.archive.tree().lookup_child(dir, name)
        });
        match found {
            Some(node) => reply.entry(&TTL, &self.attr(node), Generation(0)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<Fh>, reply: ReplyAttr) {
        match self.node_of(ino) {
            Some(node) => reply.attr(&TTL, &self.attr(node)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn open(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        let Some(node) = self.node_of(ino) else {
            return reply.error(Errno::ENOENT);
        };
        if self.archive.tree().node(node).is_dir {
            return reply.error(Errno::EISDIR);
        }
        // The reader is created right away, as on Windows: an encrypted entry
        // without a password fails here rather than halfway through a copy.
        match FileHandle::open(&self.archive, node) {
            Ok(handle) => {
                let id = self.next_handle.fetch_add(1, Ordering::Relaxed);
                lock(&self.handles).insert(id, Arc::new(Mutex::new(handle)));
                // The contents cannot change, so pages cached from an earlier
                // open stay valid.
                reply.opened(Fh(id), FopenFlags::FOPEN_KEEP_CACHE);
            }
            Err(e) if zipfs_core::needs_password(&e) => reply.error(Errno::EACCES),
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn read(
        &self,
        _req: &Request,
        _ino: INodeNo,
        fh: Fh,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let Some(handle) = self.handle(fh) else {
            return reply.error(Errno::EBADF);
        };
        let mut handle = lock(&handle);
        let wanted = (size as u64).min(handle.size().saturating_sub(offset)) as usize;
        let mut buf = vec![0u8; wanted];

        // A short reply means end of file to the kernel, so keep reading until
        // the request is filled: a reader may hand out less than asked, at a
        // cache chunk boundary for instance.
        let mut filled = 0;
        while filled < wanted {
            match handle.read_at(
                &self.archive,
                &self.cache,
                offset + filled as u64,
                &mut buf[filled..],
            ) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(_) => return reply.error(Errno::EIO),
            }
        }
        reply.data(&buf[..filled]);
    }

    fn release(
        &self,
        _req: &Request,
        _ino: INodeNo,
        fh: Fh,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        lock(&self.handles).remove(&fh.0);
        reply.ok();
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        match self.node_of(ino) {
            Some(node) if self.archive.tree().node(node).is_dir => {
                reply.opened(Fh(0), FopenFlags::FOPEN_CACHE_DIR);
            }
            Some(_) => reply.error(Errno::ENOTDIR),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: Fh,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let Some(dir) = self.node_of(ino) else {
            return reply.error(Errno::ENOENT);
        };
        let tree = self.archive.tree();
        let info = tree.node(dir);

        // Offsets are positions in this list plus one: the kernel hands back
        // the offset of the last entry it took.
        let dots = [(dir, "."), (info.parent, "..")];
        let entries = dots
            .iter()
            .map(|&(node, name)| (node, FileType::Directory, name))
            .chain(info.children.iter().map(|&child| {
                let node = tree.node(child);
                let kind = if node.is_dir {
                    FileType::Directory
                } else {
                    FileType::RegularFile
                };
                (child, kind, node.name.as_str())
            }));

        for (i, (node, kind, name)) in entries.enumerate().skip(offset as usize) {
            if reply.add(ino_of(node), i as u64 + 1, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
        // Full, with nothing free: the volume is read-only.
        reply.statfs(
            self.total_bytes.div_ceil(u64::from(BLOCK_SIZE)),
            0,
            0,
            self.archive.tree().node_count() as u64,
            0,
            BLOCK_SIZE,
            255,
            BLOCK_SIZE,
        );
    }
}

/// Mount settings.
pub struct MountOptions {
    /// An existing empty directory.
    pub mountpoint: PathBuf,
    /// What `mount` and `df` show as the source; the archive's path.
    pub source: String,
    pub cache_budget: usize,
}

/// A mounted archive. [`Mount::run`] serves it until it is unmounted.
pub struct Mount {
    session: Session<ZipFs>,
    unmounter: Unmounter,
}

/// Unmounts from another thread — a signal handler's, typically.
#[derive(Clone)]
pub struct Unmounter(Arc<Mutex<SessionUnmounter>>);

impl Unmounter {
    pub fn unmount(&self) {
        let _ = lock(&self.0).unmount();
    }
}

impl Mount {
    pub fn new(archive: Archive, opts: MountOptions) -> Result<Self> {
        let total_bytes = archive.tree().stats().total_uncompressed;
        // SAFETY: getuid and getgid cannot fail and touch no memory of ours.
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        let fs = ZipFs {
            archive,
            cache: ContentCache::new(opts.cache_budget),
            uid,
            gid,
            total_bytes,
            handles: Mutex::new(HashMap::new()),
            next_handle: AtomicU64::new(1),
        };

        let mut config = Config::default();
        config.mount_options = vec![
            MountOption::RO,
            MountOption::NoAtime,
            MountOption::NoDev,
            MountOption::NoSuid,
            MountOption::DefaultPermissions,
            // fusermount3 splits its options on commas, and has no escape
            // for one inside a value.
            MountOption::FSName(opts.source.replace(',', "_")),
            MountOption::Subtype("zipmount".into()),
        ];
        // Reads of different files proceed in parallel, as they do on
        // Windows; a 7z solid block being expanded holds up only its own.
        config.n_threads = Some(
            std::thread::available_parallelism()
                .map_or(4, |n| n.get())
                .min(8),
        );
        config.clone_fd = true;

        let mut session = Session::new(fs, &opts.mountpoint, &config).map_err(|e| {
            anyhow::anyhow!(t!(
                "mount-err-fuse",
                mountpoint = opts.mountpoint.display().to_string(),
                error = e.to_string()
            ))
        })?;
        let unmounter = Unmounter(Arc::new(Mutex::new(session.unmount_callable())));
        Ok(Self { session, unmounter })
    }

    pub fn unmounter(&self) -> Unmounter {
        self.unmounter.clone()
    }

    /// Serves requests until the directory is unmounted — by
    /// [`Unmounter::unmount`], or from outside with `fusermount3 -u`.
    pub fn run(self) -> Result<()> {
        self.session.run()?;
        Ok(())
    }
}

/// Whether FUSE can be used here at all: the device and the mount helper.
pub fn available() -> std::result::Result<(), String> {
    if !Path::new("/dev/fuse").exists() {
        return Err("/dev/fuse".into());
    }
    let helper = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join("fusermount3").is_file()))
        .unwrap_or(false);
    if !helper {
        return Err("fusermount3".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetime_converts_to_unix_time() {
        // 2024-01-01 00:00:00 UTC.
        let filetime = (1_704_067_200 + FILETIME_TO_UNIX_SECS) * 10_000_000 + 5;
        assert_eq!(
            system_time(filetime),
            UNIX_EPOCH + Duration::new(1_704_067_200, 500)
        );
    }

    #[test]
    fn missing_and_pre_1970_times_fall_back_to_1980() {
        let fallback = UNIX_EPOCH + Duration::from_secs(FALLBACK_SECS);
        assert_eq!(system_time(0), fallback);
        assert_eq!(system_time(10_000_000), fallback);
    }
}
