//! The filesystem implementation on top of WinFsp.
//!
//! The volume is read-only: every modifying operation of the trait is left at
//! its default implementation, which refuses, and the volume itself is marked
//! `read_only_volume`. Thanks to that File Explorer hides "Delete" and
//! "Rename" instead of producing obscure errors.

use std::ffi::c_void;
use std::sync::Mutex;

use winfsp::filesystem::{
    DirBuffer, DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, OpenFileInfo,
    VolumeInfo, WideNameInfo,
};
use winfsp::{FspError, Result as FspResult, U16CStr};
use zipfs_core::{Archive, ContentCache, FileHandle};

// NTSTATUS codes. Declared here so as not to pull in the whole `windows`
// crate for a handful of values.
const STATUS_OBJECT_NAME_NOT_FOUND: i32 = 0xC000_0034u32 as i32;
const STATUS_END_OF_FILE: i32 = 0xC000_0011u32 as i32;
const STATUS_NOT_A_DIRECTORY: i32 = 0xC000_0103u32 as i32;
const STATUS_FILE_IS_A_DIRECTORY: i32 = 0xC000_00BAu32 as i32;
const STATUS_IO_DEVICE_ERROR: i32 = 0xC000_0185u32 as i32;

const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;

const SECTOR_SIZE: u64 = 4096;

/// 1980-01-01 as a FILETIME: the fallback time for entries without a date.
/// Zero would show in File Explorer as the year 1601.
const FALLBACK_TIME: u64 = 119_600_064_000_000_000;

pub struct ZipFs {
    archive: Archive,
    cache: ContentCache,
    security: Vec<u8>,
    total_bytes: u64,
    label: String,
}

impl ZipFs {
    pub fn new(archive: Archive, cache_budget: usize, label: String, security: Vec<u8>) -> Self {
        let total_bytes = archive.tree().stats().total_uncompressed;
        Self {
            archive,
            cache: ContentCache::new(cache_budget),
            security,
            total_bytes,
            label,
        }
    }

    fn fill_info(&self, node_id: u32, info: &mut FileInfo) {
        let node = self.archive.tree().node(node_id);

        info.file_attributes = if node.is_dir {
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_READONLY
        } else {
            FILE_ATTRIBUTE_READONLY
        };
        info.reparse_tag = 0;
        info.file_size = node.size;
        info.allocation_size = node.size.div_ceil(SECTOR_SIZE) * SECTOR_SIZE;

        let time = if node.mtime == 0 {
            FALLBACK_TIME
        } else {
            node.mtime
        };
        info.creation_time = time;
        info.last_access_time = time;
        info.last_write_time = time;
        info.change_time = time;

        info.index_number = node_id as u64;
        info.hard_links = 0;
        info.ea_size = 0;
    }

    fn copy_security(&self, buffer: Option<&mut [c_void]>) {
        if let Some(buf) = buffer {
            if buf.len() >= self.security.len() {
                // SAFETY: WinFsp provides the buffer, and it is no shorter than
                // our descriptor.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        self.security.as_ptr(),
                        buf.as_mut_ptr().cast::<u8>(),
                        self.security.len(),
                    );
                }
            }
        }
    }

    fn attributes_of(&self, node_id: u32) -> u32 {
        if self.archive.tree().node(node_id).is_dir {
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_READONLY
        } else {
            FILE_ATTRIBUTE_READONLY
        }
    }
}

/// An open handle. WinFsp calls back in parallel and passes a shared
/// reference here, so the reader's mutable state sits under a mutex.
pub struct ZipFileContext {
    node: u32,
    is_dir: bool,
    dir_buffer: DirBuffer,
    reader: Mutex<Option<FileHandle>>,
    /// The pattern the directory buffer was filled with. It usually stays
    /// the same for the handle's lifetime, but if it changes the buffer must
    /// be rebuilt, or the listing stays filtered by the old pattern.
    filled_pattern: Mutex<Option<String>>,
}

impl FileSystemContext for ZipFs {
    type FileContext = ZipFileContext;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        security_descriptor: Option<&mut [c_void]>,
        _reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> FspResult<FileSecurity> {
        let path = file_name.to_string_lossy();
        let node = self
            .archive
            .tree()
            .resolve(&path)
            .ok_or(FspError::NTSTATUS(STATUS_OBJECT_NAME_NOT_FOUND))?;

        self.copy_security(security_descriptor);

        Ok(FileSecurity {
            reparse: false,
            sz_security_descriptor: self.security.len() as u64,
            attributes: self.attributes_of(node),
        })
    }

    fn open(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: winfsp_sys::FILE_ACCESS_RIGHTS,
        file_info: &mut OpenFileInfo,
    ) -> FspResult<Self::FileContext> {
        let path = file_name.to_string_lossy();
        let tree = self.archive.tree();
        let node = tree
            .resolve(&path)
            .ok_or(FspError::NTSTATUS(STATUS_OBJECT_NAME_NOT_FOUND))?;
        let is_dir = tree.node(node).is_dir;

        self.fill_info(node, file_info.as_mut());

        // The normalized name gives File Explorer the real case from the
        // archive, even when the path was opened in a different case.
        let normalized: Vec<u16> = tree.path_of(node).encode_utf16().collect();
        file_info.set_normalized_name(&normalized, Some(b'\\' as u16));

        // The file reader is created right away: that way errors such as an
        // encrypted entry surface at open time rather than mid-read.
        let reader = if is_dir {
            None
        } else {
            Some(
                FileHandle::open(&self.archive, node)
                    .map_err(|_| FspError::NTSTATUS(STATUS_IO_DEVICE_ERROR))?,
            )
        };

        Ok(ZipFileContext {
            node,
            is_dir,
            dir_buffer: DirBuffer::new(),
            reader: Mutex::new(reader),
            filled_pattern: Mutex::new(None),
        })
    }

    fn close(&self, _context: Self::FileContext) {}

    fn get_file_info(
        &self,
        context: &Self::FileContext,
        file_info: &mut FileInfo,
    ) -> FspResult<()> {
        self.fill_info(context.node, file_info);
        Ok(())
    }

    fn get_security(
        &self,
        _context: &Self::FileContext,
        security_descriptor: Option<&mut [c_void]>,
    ) -> FspResult<u64> {
        self.copy_security(security_descriptor);
        Ok(self.security.len() as u64)
    }

    fn read(&self, context: &Self::FileContext, buffer: &mut [u8], offset: u64) -> FspResult<u32> {
        if context.is_dir {
            return Err(FspError::NTSTATUS(STATUS_FILE_IS_A_DIRECTORY));
        }

        let mut guard = context.reader.lock().expect("reader poisoned by a panic");
        let handle = guard
            .as_mut()
            .ok_or(FspError::NTSTATUS(STATUS_IO_DEVICE_ERROR))?;

        if offset >= handle.size() {
            return Err(FspError::NTSTATUS(STATUS_END_OF_FILE));
        }

        let read = handle
            .read_at(&self.archive, &self.cache, offset, buffer)
            .map_err(|_| FspError::NTSTATUS(STATUS_IO_DEVICE_ERROR))?;

        if read == 0 {
            return Err(FspError::NTSTATUS(STATUS_END_OF_FILE));
        }
        Ok(read as u32)
    }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        pattern: Option<&U16CStr>,
        marker: DirMarker,
        buffer: &mut [u8],
    ) -> FspResult<u32> {
        if !context.is_dir {
            return Err(FspError::NTSTATUS(STATUS_NOT_A_DIRECTORY));
        }

        let tree = self.archive.tree();
        let children = &tree.node(context.node).children;
        let pattern = pattern.map(|p| p.to_string_lossy());

        // Filtering by pattern is done here: WinFsp does not do it for us,
        // and without it `dir *.log` and File Explorer's search find nothing.
        let pattern_changed = {
            let mut filled = context
                .filled_pattern
                .lock()
                .expect("directory state poisoned by a panic");
            let changed = *filled != pattern;
            if changed {
                filled.clone_from(&pattern);
            }
            changed
        };

        let mut written = 0usize;
        let reset = marker.is_none() || pattern_changed;
        let acquired = context
            .dir_buffer
            .acquire(reset, Some(children.len() as u32));
        let acquired_ok = acquired.is_ok();

        if let Ok(lock) = acquired {
            let mut entry = DirInfo::<255>::new();
            let mut wide: Vec<u16> = Vec::with_capacity(64);
            for &child in children {
                entry.reset();

                // Deliberately not set_name: it appends a terminating NUL and
                // counts it in the name length, so the kernel sees the name as
                // "file.log\0". Prefix patterns still match, but anything
                // anchored at the end (*.log) no longer does.
                wide.clear();
                wide.extend(tree.node(child).name.encode_utf16());
                entry.set_name_raw(wide.as_slice())?;

                self.fill_info(child, entry.file_info_mut());
                lock.write(&mut entry)?;
                written += 1;
            }
        }

        // Look at the marker before reading: read consumes it.
        let marker_dbg = marker.inner_as_cstr().map(|m| m.to_string_lossy());
        let buffer_len = buffer.len();
        let produced = context.dir_buffer.read(marker, buffer);

        if debug_enabled() {
            eprintln!(
                "[read_directory] path={:?} children={} pattern={:?} reset={} acquire_ok={} marker={:?} buffer={} produced={} written={}",
                tree.path_of(context.node),
                children.len(),
                pattern.as_deref(),
                reset,
                acquired_ok,
                marker_dbg,
                buffer_len,
                produced,
                written,
            );
        }

        Ok(produced)
    }

    fn get_volume_info(&self, out_volume_info: &mut VolumeInfo) -> FspResult<()> {
        out_volume_info.total_size = self.total_bytes;
        // There is no free space and there cannot be: the volume is read-only.
        out_volume_info.free_size = 0;
        out_volume_info.set_volume_label(&self.label);
        Ok(())
    }
}

/// Diagnostic output is turned on by the environment variable
/// `ZIPMOUNT_DEBUG=1`. Useful for working out what exactly Windows asks the
/// filesystem.
fn debug_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("ZIPMOUNT_DEBUG").is_ok_and(|v| v != "0"))
}
