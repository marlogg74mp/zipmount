//! Systems without a mounting backend yet (macOS, until its NFS one lands):
//! every archive command works, mounting says plainly that it does not.

use std::path::Path;

use anyhow::Result;
use zipfs_core::Archive;
use zipmount_i18n::t;

use super::MountRecord;

pub(crate) fn check_available() -> Result<()> {
    anyhow::bail!(t!("err-mount-unsupported"))
}

pub(crate) fn mounts() -> Vec<MountRecord> {
    Vec::new()
}

pub(crate) fn serve(
    _archive: Archive,
    _mountpoint: &Path,
    _source: String,
    _cache_budget: usize,
    _on_mounted: impl FnOnce(),
) -> Result<()> {
    check_available()
}

pub(crate) fn unmount(_mountpoint: &Path) -> Result<()> {
    check_available()
}

pub(crate) fn doctor_label() -> String {
    t!("doctor-mount")
}

pub(crate) fn doctor_ok() -> String {
    String::new()
}

pub(crate) fn doctor_note() -> String {
    t!("doctor-mount-unsupported-note")
}
