//! Linux: FUSE through `zipfs-fuse`, and the kernel's own list of mounts.

use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;

use anyhow::{Context, Result};
use zipfs_core::Archive;
use zipmount_i18n::t;

use super::MountRecord;

/// The filesystem type our mounts show in /proc/self/mountinfo: "fuse." and
/// the subtype zipfs-fuse sets.
const FSTYPE: &str = "fuse.zipmount";

pub(crate) fn check_available() -> Result<()> {
    zipfs_fuse::available().map_err(|missing| anyhow::anyhow!(t!("err-no-fuse", missing = missing)))
}

/// What is mounted, straight from the kernel: no list of our own to go stale
/// when a mount process is killed.
pub(crate) fn mounts() -> Vec<MountRecord> {
    std::fs::read_to_string("/proc/self/mountinfo")
        .map(|text| parse_mountinfo(&text))
        .unwrap_or_default()
}

/// Lines look like
/// `36 25 0:32 / /home/u/ZipMount/logs ro,nosuid - fuse.zipmount /data/logs.7z ro,...`:
/// the mount point is the fifth field, and after the " - " separator come
/// the filesystem type and the source.
fn parse_mountinfo(text: &str) -> Vec<MountRecord> {
    text.lines()
        .filter_map(|line| {
            let (left, right) = line.split_once(" - ")?;
            let mountpoint = left.split(' ').nth(4)?;
            let mut right = right.split(' ');
            if right.next()? != FSTYPE {
                return None;
            }
            let source = right.next()?;
            Some(MountRecord {
                mountpoint: PathBuf::from(unescape(mountpoint)),
                archive: PathBuf::from(unescape(source)),
            })
        })
        .collect()
}

/// The kernel writes a space as `\040`, a tab as `\011`, a newline as `\012`
/// and a backslash as `\134`.
fn unescape(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let octal = bytes
            .get(i + 1..i + 4)
            .filter(|d| bytes[i] == b'\\' && d.iter().all(|c| (b'0'..=b'7').contains(c)));
        match octal {
            Some(d) => {
                out.push((d[0] - b'0') * 64 + (d[1] - b'0') * 8 + (d[2] - b'0'));
                i += 4;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Mounts, calls `on_mounted`, and serves until the directory is unmounted.
pub(crate) fn serve(
    archive: Archive,
    mountpoint: &Path,
    source: String,
    cache_budget: usize,
    on_mounted: impl FnOnce(),
) -> Result<()> {
    let mount = zipfs_fuse::Mount::new(
        archive,
        zipfs_fuse::MountOptions {
            mountpoint: mountpoint.to_path_buf(),
            source,
            cache_budget,
        },
    )?;

    // Ctrl+C, `kill` and a closed terminal all unmount; the serving loop
    // then returns by itself, as it does after `fusermount3 -u`.
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
    let output = ProcCommand::new("fusermount3")
        .arg("-u")
        .arg(mountpoint)
        .output()
        .context("cannot run fusermount3")?;
    if !output.status.success() {
        // fusermount3 explains itself well enough ("Device or resource busy"
        // when a file is still open), after its own name.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.trim().trim_start_matches("fusermount3: ");
        anyhow::bail!(t!(
            "err-unmount-failed",
            target = mountpoint.display().to_string(),
            error = reason.to_string()
        ));
    }
    Ok(())
}

pub(crate) fn doctor_label() -> String {
    t!("doctor-fuse")
}

pub(crate) fn doctor_ok() -> String {
    t!("doctor-fuse-ok")
}

pub(crate) fn doctor_note() -> String {
    t!("doctor-fuse-note")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountinfo_lists_only_our_mounts_and_unescapes_paths() {
        let text = "\
22 1 8:1 / / rw,relatime shared:1 - ext4 /dev/sda1 rw
36 25 0:32 / /home/u/ZipMount/my\\040logs ro,nosuid,nodev,noatime - fuse.zipmount /data/my\\040logs.7z ro,user_id=1000
37 25 0:33 / /mnt/other ro - fuse.sshfs host:/ ro
";
        let mounts = parse_mountinfo(text);
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].mountpoint, Path::new("/home/u/ZipMount/my logs"));
        assert_eq!(mounts[0].archive, Path::new("/data/my logs.7z"));
    }

    #[test]
    fn unescape_handles_every_kernel_escape() {
        assert_eq!(unescape("a\\040b\\011c\\012d\\134e"), "a b\tc\nd\\e");
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape("trailing\\04"), "trailing\\04");
    }
}
