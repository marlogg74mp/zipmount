//! macOS: a local NFS server through `zipfs-nfs`, mounted by the system's
//! own NFS client.

use std::path::Path;
use std::process::Command as ProcCommand;

use anyhow::{Context, Result};
use zipfs_core::Archive;
use zipmount_i18n::t;

use super::MountRecord;

const MOUNT_NFS: &str = "/sbin/mount_nfs";

pub(crate) fn check_available() -> Result<()> {
    // Part of every macOS; its absence means something unusual.
    if !Path::new(MOUNT_NFS).exists() {
        anyhow::bail!(t!("err-mount-unsupported"));
    }
    Ok(())
}

/// What is mounted, from the system's list of mounts.
pub(crate) fn mounts() -> Vec<MountRecord> {
    zipfs_nfs::mounts()
        .into_iter()
        .map(|m| MountRecord {
            mountpoint: m.mountpoint,
            archive: m.archive,
        })
        .collect()
}

/// Mounts, calls `on_mounted`, and serves until the directory is unmounted.
pub(crate) fn serve(
    archive: Archive,
    mountpoint: &Path,
    source: String,
    cache_budget: usize,
    on_mounted: impl FnOnce(),
) -> Result<()> {
    let mount = zipfs_nfs::Mount::new(
        archive,
        zipfs_nfs::MountOptions {
            mountpoint: mountpoint.to_path_buf(),
            archive_path: source,
            cache_budget,
        },
    )?;

    // Ctrl+C, `kill` and a closed terminal all unmount; the serving loop
    // then sees the mount gone and returns, as it does after Eject in Finder.
    let unmounter = mount.unmounter();
    ctrlc::set_handler(move || {
        println!("{}", t!("mount-unmounting"));
        unmounter.unmount();
    })
    .context("cannot install the Ctrl+C handler")?;

    on_mounted();
    mount.run()
}

pub(crate) fn unmount(mountpoint: &Path) -> Result<()> {
    let umount = |force: bool| {
        let mut command = ProcCommand::new("/sbin/umount");
        if force {
            command.arg("-f");
        }
        command
            .arg(mountpoint)
            .output()
            .context("cannot run umount")
    };

    // Right after a program finishes reading, the NFS client holds on to
    // the file for a moment, and umount answers "Resource busy"; give that
    // moment five seconds to pass.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut output = loop {
        let output = umount(false)?;
        if output.status.success() || std::time::Instant::now() > deadline {
            break output;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };

    // Still busy. If one of the user's programs has a file open there, say
    // which and leave it be. If none has, what holds the volume is a system
    // service looking it over (Spotlight and the like, which lsof does not
    // show to a user): forcing is safe then — the volume is read-only, so
    // nothing unwritten can be lost.
    if !output.status.success() {
        let users = processes_using(mountpoint);
        if users.is_empty() {
            output = umount(true)?;
        } else {
            anyhow::bail!(t!(
                "err-unmount-failed",
                target = mountpoint.display().to_string(),
                error = users.join(", ")
            ));
        }
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.trim().trim_start_matches("umount: ");
        anyhow::bail!(t!(
            "err-unmount-failed",
            target = mountpoint.display().to_string(),
            error = reason.to_string()
        ));
    }
    // The serving process sees the mount gone within half a second and exits
    // by itself.
    Ok(())
}

/// This user's processes with a file open on the volume, as "Preview (812)".
fn processes_using(mountpoint: &Path) -> Vec<String> {
    // +f: everything open on the filesystem mounted there. -F pc: one field
    // per line, "p<pid>" then "c<command>".
    let Ok(output) = ProcCommand::new("/usr/sbin/lsof")
        .args(["-F", "pc", "+f", "--"])
        .arg(mountpoint)
        .output()
    else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut pid = "";
    for line in std::str::from_utf8(&output.stdout)
        .unwrap_or_default()
        .lines()
    {
        if let Some(p) = line.strip_prefix('p') {
            pid = p;
        } else if let Some(command) = line.strip_prefix('c') {
            found.push(format!("{command} ({pid})"));
        }
    }
    found
}

pub(crate) fn doctor_label() -> String {
    t!("doctor-mount")
}

pub(crate) fn doctor_ok() -> String {
    t!("doctor-nfs-ok")
}

pub(crate) fn doctor_note() -> String {
    t!("doctor-mount-unsupported-note")
}
