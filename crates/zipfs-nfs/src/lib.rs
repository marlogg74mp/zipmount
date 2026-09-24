//! Mounting an archive through a local NFS server, for macOS.
//!
//! macOS has no FUSE of its own, and macFUSE asks the user to lower the
//! system's security on Apple Silicon. Its NFS client, though, is built in:
//! this crate serves the archive as an NFSv3 export on 127.0.0.1, and the
//! system mounts it with `mount_nfs` like any network share — no kernel
//! extension, no administrator rights, and Finder shows it as a volume.
//!
//! The filesystem ([`ZipNfs`]) builds on any Unix, so its tests run on Linux
//! as well; mounting it is macOS only.
#![cfg(unix)]

#[cfg(target_os = "macos")]
mod mount;
#[cfg(target_os = "macos")]
pub use mount::{mounts, Mount, MountOptions, MountRecord, Unmounter};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use nfsserve::nfs::{
    fattr3, fileid3, filename3, ftype3, nfspath3, nfsstat3, nfstime3, sattr3, specdata3,
};
use nfsserve::vfs::{DirEntry, NFSFileSystem, ReadDirResult, VFSCapabilities};
use zipfs_core::{Archive, ContentCache, FileHandle};

/// 1980-01-01, the fallback time for entries without a date.
const FALLBACK_SECS: u32 = 315_532_800;

/// Seconds between 1601-01-01 (FILETIME, what the core stores) and 1970-01-01.
const FILETIME_TO_UNIX_SECS: u64 = 11_644_473_600;

/// Readers kept between requests. NFS has no open and close: every read
/// names the file afresh, and reopening would throw away what a reader has
/// built — the checkpoints of a large deflate stream, say.
const KEPT_READERS: usize = 64;

/// NFS file ids start at 1 for the root; the core's node ids start at 0.
fn id_of(node: u32) -> fileid3 {
    u64::from(node) + 1
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn nfs_time(filetime: u64) -> nfstime3 {
    match (filetime / 10_000_000).checked_sub(FILETIME_TO_UNIX_SECS) {
        Some(secs) if filetime != 0 => nfstime3 {
            seconds: u32::try_from(secs).unwrap_or(u32::MAX),
            nseconds: (filetime % 10_000_000) as u32 * 100,
        },
        _ => nfstime3 {
            seconds: FALLBACK_SECS,
            nseconds: 0,
        },
    }
}

/// node → (reader, when last used), and the clock those times come from.
type Readers = (HashMap<u32, (Arc<Mutex<FileHandle>>, u64)>, u64);

struct Inner {
    archive: Archive,
    cache: ContentCache,
    uid: u32,
    gid: u32,
    readers: Mutex<Readers>,
}

impl Inner {
    fn node_of(&self, id: fileid3) -> Result<u32, nfsstat3> {
        let node = id
            .checked_sub(1)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(nfsstat3::NFS3ERR_STALE)?;
        if (node as usize) < self.archive.tree().node_count() {
            Ok(node)
        } else {
            Err(nfsstat3::NFS3ERR_STALE)
        }
    }

    fn attr(&self, node: u32) -> fattr3 {
        let info = self.archive.tree().node(node);
        let time = nfs_time(info.mtime);
        fattr3 {
            ftype: if info.is_dir {
                ftype3::NF3DIR
            } else {
                ftype3::NF3REG
            },
            mode: if info.is_dir { 0o555 } else { 0o444 },
            nlink: if info.is_dir { 2 } else { 1 },
            uid: self.uid,
            gid: self.gid,
            size: info.size,
            used: info.size,
            rdev: specdata3::default(),
            fsid: 0,
            fileid: id_of(node),
            atime: time,
            mtime: time,
            ctime: time,
        }
    }

    fn reader(&self, node: u32) -> Result<Arc<Mutex<FileHandle>>, nfsstat3> {
        {
            let mut guard = lock(&self.readers);
            let (readers, clock) = &mut *guard;
            *clock += 1;
            if let Some((reader, used)) = readers.get_mut(&node) {
                *used = *clock;
                return Ok(reader.clone());
            }
        }
        // Opened outside the lock: for 7z this expands a solid block, which
        // can take a second, and other files need not wait for it.
        let handle = FileHandle::open(&self.archive, node).map_err(|e| {
            if zipfs_core::needs_password(&e) {
                nfsstat3::NFS3ERR_ACCES
            } else {
                nfsstat3::NFS3ERR_IO
            }
        })?;
        let reader = Arc::new(Mutex::new(handle));
        let mut guard = lock(&self.readers);
        let (readers, clock) = &mut *guard;
        if readers.len() >= KEPT_READERS {
            if let Some(oldest) = readers
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(n, _)| *n)
            {
                readers.remove(&oldest);
            }
        }
        readers.insert(node, (reader.clone(), *clock));
        Ok(reader)
    }

    /// Reads up to `count` bytes; the flag says whether that reached the end.
    fn read(&self, node: u32, offset: u64, count: u32) -> Result<(Vec<u8>, bool), nfsstat3> {
        if self.archive.tree().node(node).is_dir {
            return Err(nfsstat3::NFS3ERR_ISDIR);
        }
        let reader = self.reader(node)?;
        let mut reader = lock(&reader);
        let size = reader.size();
        let wanted = u64::from(count).min(size.saturating_sub(offset)) as usize;
        let mut buf = vec![0u8; wanted];
        let mut filled = 0;
        while filled < wanted {
            match reader.read_at(
                &self.archive,
                &self.cache,
                offset + filled as u64,
                &mut buf[filled..],
            ) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(_) => return Err(nfsstat3::NFS3ERR_IO),
            }
        }
        buf.truncate(filled);
        let eof = offset + filled as u64 >= size;
        Ok((buf, eof))
    }
}

/// The archive as an NFS filesystem, read-only.
pub struct ZipNfs {
    inner: Arc<Inner>,
}

impl ZipNfs {
    pub fn new(archive: Archive, cache_budget: usize) -> Self {
        // SAFETY: getuid and getgid cannot fail and touch no memory of ours.
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        Self {
            inner: Arc::new(Inner {
                archive,
                cache: ContentCache::new(cache_budget),
                uid,
                gid,
                readers: Mutex::new((HashMap::new(), 0)),
            }),
        }
    }
}

#[async_trait]
impl NFSFileSystem for ZipNfs {
    fn capabilities(&self) -> VFSCapabilities {
        VFSCapabilities::ReadOnly
    }

    fn root_dir(&self) -> fileid3 {
        id_of(zipfs_core::ROOT)
    }

    async fn lookup(&self, dirid: fileid3, filename: &filename3) -> Result<fileid3, nfsstat3> {
        let dir = self.inner.node_of(dirid)?;
        let tree = self.inner.archive.tree();
        if !tree.node(dir).is_dir {
            return Err(nfsstat3::NFS3ERR_NOTDIR);
        }
        let name = std::str::from_utf8(filename).map_err(|_| nfsstat3::NFS3ERR_NOENT)?;
        match name {
            "." => Ok(dirid),
            ".." => Ok(id_of(tree.node(dir).parent)),
            _ => tree
                .lookup_child(dir, name)
                .map(id_of)
                .ok_or(nfsstat3::NFS3ERR_NOENT),
        }
    }

    async fn getattr(&self, id: fileid3) -> Result<fattr3, nfsstat3> {
        Ok(self.inner.attr(self.inner.node_of(id)?))
    }

    async fn read(
        &self,
        id: fileid3,
        offset: u64,
        count: u32,
    ) -> Result<(Vec<u8>, bool), nfsstat3> {
        let node = self.inner.node_of(id)?;
        let inner = self.inner.clone();
        // Decompression is blocking work; on the runtime's own threads it
        // would stall every other request, listings included.
        tokio::task::spawn_blocking(move || inner.read(node, offset, count))
            .await
            .map_err(|_| nfsstat3::NFS3ERR_IO)?
    }

    async fn readdir(
        &self,
        dirid: fileid3,
        start_after: fileid3,
        max_entries: usize,
    ) -> Result<ReadDirResult, nfsstat3> {
        let dir = self.inner.node_of(dirid)?;
        let tree = self.inner.archive.tree();
        let info = tree.node(dir);
        if !info.is_dir {
            return Err(nfsstat3::NFS3ERR_NOTDIR);
        }
        // The cookie is the id of the last entry the client got.
        let start = if start_after == 0 {
            0
        } else {
            info.children
                .iter()
                .position(|&child| id_of(child) == start_after)
                .map(|at| at + 1)
                .ok_or(nfsstat3::NFS3ERR_BAD_COOKIE)?
        };
        let entries: Vec<DirEntry> = info.children[start..]
            .iter()
            .take(max_entries)
            .map(|&child| DirEntry {
                fileid: id_of(child),
                name: tree.node(child).name.as_bytes().into(),
                attr: self.inner.attr(child),
            })
            .collect();
        let end = start + entries.len() >= info.children.len();
        Ok(ReadDirResult { entries, end })
    }

    async fn setattr(&self, _id: fileid3, _setattr: sattr3) -> Result<fattr3, nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn write(&self, _id: fileid3, _offset: u64, _data: &[u8]) -> Result<fattr3, nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn create(
        &self,
        _dirid: fileid3,
        _filename: &filename3,
        _attr: sattr3,
    ) -> Result<(fileid3, fattr3), nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn create_exclusive(
        &self,
        _dirid: fileid3,
        _filename: &filename3,
    ) -> Result<fileid3, nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn mkdir(
        &self,
        _dirid: fileid3,
        _dirname: &filename3,
    ) -> Result<(fileid3, fattr3), nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn remove(&self, _dirid: fileid3, _filename: &filename3) -> Result<(), nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn rename(
        &self,
        _from_dirid: fileid3,
        _from_filename: &filename3,
        _to_dirid: fileid3,
        _to_filename: &filename3,
    ) -> Result<(), nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn symlink(
        &self,
        _dirid: fileid3,
        _linkname: &filename3,
        _symlink: &nfspath3,
        _attr: &sattr3,
    ) -> Result<(fileid3, fattr3), nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }

    async fn readlink(&self, _id: fileid3) -> Result<nfspath3, nfsstat3> {
        // There are no symbolic links in the tree.
        Err(nfsstat3::NFS3ERR_INVAL)
    }
}

#[cfg(test)]
mod tests;
