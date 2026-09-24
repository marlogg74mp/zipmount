//! The Linux and macOS side of the command line: mounting on a directory,
//! and `doctor`.
//!
//! There are no drive letters here. An archive is mounted on a directory,
//! by default `~/ZipMount/<archive name>`: a directory in the home folder is
//! one the file manager shows in its sidebar, and the user can write to it
//! without root. Nor is there any bookkeeping of our own: the system's list
//! of mounts already says what is mounted where, and from which archive.
//!
//! How the mount is made differs: FUSE on Linux (`linux`), a local NFS
//! server on macOS (`macos`).

mod menu;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as backend;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use unsupported as backend;

use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcCommand, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use zipmount_i18n::{decimal, t};

use crate::{
    human, open_archive, print_table, read_password, report, source_text, ArchiveArgs, MountArgs,
};

/// Set for the background copy started by `mount --detach`: its errors are
/// reported by the process that started it, which has the terminal or shows
/// the one message.
pub(crate) const DETACHED_VAR: &str = "ZIPMOUNT_DETACHED";

/// Set by the menu items: errors are shown in a dialog or notification, as
/// nobody reads the output of a program started from a file manager.
pub(crate) const NOTIFY_VAR: &str = "ZIPMOUNT_NOTIFY";

/// A mounted archive, as the system lists it.
pub(crate) struct MountRecord {
    pub mountpoint: PathBuf,
    pub archive: PathBuf,
}

/// Menu items carry their text in the language they were installed in;
/// `zipmount language` writes them again.
pub(crate) fn refresh_menu_texts(_language: &zipmount_i18n::Language) -> Result<()> {
    if menu::is_installed() {
        menu::install()?;
        println!("{}", t!("language-menu-updated-unix"));
    }
    Ok(())
}

pub(crate) fn cmd_shell_install() -> Result<()> {
    let written = menu::install()?;
    println!("{}", t!("shell-installed-unix"));
    for path in &written {
        println!("  {}", path.display());
    }
    println!();
    #[cfg(target_os = "macos")]
    println!("{}", t!("shell-installed-where-macos"));
    #[cfg(not(target_os = "macos"))]
    println!("{}", t!("shell-installed-where-linux"));
    println!("{}", t!("shell-remove-hint"));
    Ok(())
}

/// Shows an error to someone who started us from a menu, with no terminal
/// to read it in — what the message box does on Windows.
pub(crate) fn notify_error(text: &str) {
    #[cfg(target_os = "macos")]
    let spawned = {
        // An alert rather than a notification: notifications posted by
        // osascript belong to Script Editor, and are often switched off.
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        ProcCommand::new("osascript")
            .args([
                "-e",
                &format!("display alert \"ZipMount\" message \"{escaped}\" as critical"),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    };
    #[cfg(not(target_os = "macos"))]
    let spawned = ProcCommand::new("notify-send")
        .args([
            "--app-name=ZipMount",
            "--icon=dialog-error",
            "ZipMount",
            text,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    // Nothing more to be done if even that is missing.
    let _ = spawned;
}

pub(crate) fn cmd_shell_uninstall() -> Result<()> {
    menu::uninstall()?;
    println!("{}", t!("shell-removed"));
    Ok(())
}

pub(crate) fn cmd_mount(archive: &Path, args: MountArgs, common: &ArchiveArgs) -> Result<()> {
    // Before opening the archive: better say FUSE is missing at once than
    // after parsing a gigabyte-sized directory.
    backend::check_available()?;

    // An absolute path: it is what the system's list of mounts shows, and
    // how a second mount of the same archive is recognized.
    let archive = std::fs::canonicalize(archive).unwrap_or_else(|_| archive.to_path_buf());

    if let Some(existing) = backend::mounts().into_iter().find(|m| m.archive == archive) {
        if args.open {
            open_folder(&existing.mountpoint);
        }
        if args.print_path {
            println!("{}", existing.mountpoint.display());
        } else {
            println!(
                "{}",
                t!(
                    "mount-already",
                    letter = existing.mountpoint.display().to_string()
                )
            );
        }
        return Ok(());
    }

    let mountpoint = match &args.mountpoint {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            check_mountpoint(&dir)?;
            std::fs::canonicalize(&dir).unwrap_or(dir)
        }
        None => default_mountpoint(&archive)?,
    };

    if args.detach {
        return spawn_detached(&archive, &mountpoint, &args, common);
    }
    run_mount(&archive, &mountpoint, &args, common)
}

/// The directory the archive is mounted on when none is given:
/// `~/ZipMount/<name>`, or `<name> (2)` and so on if that one is taken.
fn default_mountpoint(archive: &Path) -> Result<PathBuf> {
    let base = mount_root()?;
    let name = archive_stem(archive);
    let mounted: Vec<PathBuf> = backend::mounts()
        .into_iter()
        .map(|m| m.mountpoint)
        .collect();
    for n in 1.. {
        let candidate = if n == 1 {
            base.join(&name)
        } else {
            base.join(format!("{name} ({n})"))
        };
        if mounted.contains(&candidate) {
            continue;
        }
        if candidate.exists() && !is_empty_dir(&candidate) {
            continue;
        }
        std::fs::create_dir_all(&candidate)
            .with_context(|| t!("err-create-dir", path = candidate.display().to_string()))?;
        // The form the system's list of mounts uses: with symbolic links
        // resolved, as /tmp becomes /private/tmp on macOS.
        return Ok(std::fs::canonicalize(&candidate).unwrap_or(candidate));
    }
    unreachable!("the loop above only ends by returning")
}

/// `~/ZipMount`: where default mount directories live, and the only place
/// they are removed from again after unmounting.
fn mount_root() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join("ZipMount"))
}

/// "logs.tar.gz" → "logs": the name a person would give the folder.
fn archive_stem(archive: &Path) -> String {
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "archive".into());
    let lower = name.to_ascii_lowercase();
    for ext in [".tar.gz", ".tgz", ".zip", ".7z", ".tar", ".gz", ".rar"] {
        if lower.ends_with(ext) && name.len() > ext.len() {
            return name[..name.len() - ext.len()].to_string();
        }
    }
    name
}

fn is_empty_dir(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
}

fn check_mountpoint(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        anyhow::bail!(t!(
            "err-mountpoint-not-dir",
            path = dir.display().to_string()
        ));
    }
    if !is_empty_dir(dir) {
        anyhow::bail!(t!(
            "err-mountpoint-not-empty",
            path = dir.display().to_string()
        ));
    }
    Ok(())
}

/// Removes a mount directory this program created in `~/ZipMount`, now
/// that nothing is mounted on it. A directory the user chose stays.
fn remove_if_ours(mountpoint: &Path) {
    let ours = mount_root()
        .map(|root| std::fs::canonicalize(&root).unwrap_or(root))
        .is_ok_and(|root| mountpoint.parent() == Some(root.as_path()));
    if ours {
        // Only if empty: remove_dir refuses otherwise, which is the point.
        let _ = std::fs::remove_dir(mountpoint);
    }
}

/// The directory's identity (device and inode), to tell it from one of the
/// same name created later.
fn dir_id(dir: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(dir)
        .ok()
        .map(|m| (m.dev(), m.ino()))
}

fn open_folder(dir: &Path) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = ProcCommand::new(opener)
        .arg(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Starts a copy of itself in the background and returns once the archive
/// is mounted.
///
/// The password, if needed, goes through a file only this user can read,
/// which the child deletes at once: the command line is visible to everyone
/// in the process list.
fn spawn_detached(
    archive: &Path,
    mountpoint: &Path,
    args: &MountArgs,
    common: &ArchiveArgs,
) -> Result<()> {
    let password = read_password(common)?;

    let exe = std::env::current_exe().context("cannot determine the program's path")?;
    let mut command = ProcCommand::new(exe);
    command
        .arg("mount")
        .arg(archive)
        .arg(mountpoint)
        .args(["--cache-mb", &args.cache_mb.to_string()])
        .args(["--encoding", &common.encoding]);

    let password_file = match &password {
        Some(pw) => {
            let path = std::env::temp_dir().join(format!("zipmount-pw-{}.tmp", std::process::id()));
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .and_then(|mut file| std::io::Write::write_all(&mut file, pw.bytes()))
                .with_context(|| t!("err-spawn-password"))?;
            command.arg("--password-file").arg(&path);
            Some(path)
        }
        None => None,
    };

    // The child's errors go to a file of its own, only this user can read:
    // if it fails, the reason is there, and this process reports it. A pipe
    // would do until the child outlives us, and then its next write would
    // kill it.
    let log_path = std::env::temp_dir().join(format!("zipmount-err-{}.txt", std::process::id()));
    let log = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&log_path)
        .context("cannot start the background process")?;

    // A process group of its own: Ctrl+C in this terminal, or closing it,
    // must not reach the background mount.
    let mut child = command
        .env(DETACHED_VAR, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .process_group(0)
        .spawn()
        .context("cannot start the background process")?;

    // Wait for the mount to show up. If the archive did not open, the child
    // exits, and it is up to us to say so.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mounted = loop {
        if backend::mounts().iter().any(|m| m.mountpoint == mountpoint) {
            break true;
        }
        if Instant::now() > deadline || matches!(child.try_wait(), Ok(Some(_))) {
            break false;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    // The child deletes the password file itself; clean up in case it did
    // not get that far.
    if let Some(path) = password_file {
        let _ = std::fs::remove_file(path);
    }

    let child_said = std::fs::read_to_string(&log_path).unwrap_or_default();
    let _ = std::fs::remove_file(&log_path);

    if !mounted {
        remove_if_ours(mountpoint);
        // The child's own words carry the reason; they start with the same
        // "Error: " this process is about to print.
        let prefix = format!("{}: ", t!("error-prefix"));
        let reason = child_said.trim();
        let reason = reason.strip_prefix(&prefix).unwrap_or(reason);
        return report(Err(if reason.is_empty() {
            anyhow::anyhow!(t!(
                "err-mount-failed-detached",
                path = archive.display().to_string()
            ))
        } else {
            anyhow::anyhow!("{reason}")
        }));
    }

    if args.print_path {
        println!("{}", mountpoint.display());
    } else {
        println!(
            "{}",
            t!("mount-done", letter = mountpoint.display().to_string())
        );
    }
    if args.open {
        open_folder(mountpoint);
    }
    Ok(())
}

/// Mounts and serves the archive until it is unmounted: by Ctrl+C, by a
/// signal, or by `zipmount unmount` from elsewhere.
fn run_mount(
    archive: &Path,
    mountpoint: &Path,
    args: &MountArgs,
    common: &ArchiveArgs,
) -> Result<()> {
    let started = Instant::now();
    let opened = open_archive(archive, common, Some(args.cache_mb));
    let a = match opened {
        Ok(a) => a,
        Err(e) => {
            remove_if_ours(mountpoint);
            return Err(e);
        }
    };
    let stats = a.tree().stats();
    let parse_time = started.elapsed();
    let where_ = mountpoint.display().to_string();
    let dir_before = dir_id(mountpoint);

    let served = backend::serve(
        a,
        mountpoint,
        archive.display().to_string(),
        args.cache_mb * 1024 * 1024,
        || {
            println!(
                "{}",
                t!(
                    "mount-done-stats",
                    letter = where_.clone(),
                    files = stats.files,
                    dirs = stats.dirs,
                    size = human(stats.total_uncompressed)
                )
            );
            println!(
                "{}",
                t!(
                    "mount-parse-time",
                    seconds = decimal(parse_time.as_secs_f64(), 3)
                )
            );
            println!();
            println!("{}", t!("mount-stop-hint", letter = where_.clone()));
            if args.open {
                open_folder(mountpoint);
            }
        },
    );

    // `zipmount unmount` removes the directory at once; this is for an
    // unmount from elsewhere (Eject in Finder, fusermount3 -u). A directory
    // of the same name created since belongs to a newer mount: leave it.
    if dir_id(mountpoint) == dir_before {
        remove_if_ours(mountpoint);
    }
    served?;
    println!("{}", t!("mount-finished"));
    Ok(())
}

pub(crate) fn cmd_unmount(target: &str) -> Result<()> {
    // Named by mount directory or by archive, whichever is handier.
    let path = std::fs::canonicalize(target).unwrap_or_else(|_| PathBuf::from(target));
    let record = backend::mounts()
        .into_iter()
        .find(|m| m.mountpoint == path || m.archive == path);

    let Some(record) = record else {
        return report(Err(anyhow::anyhow!(t!(
            "err-not-mounted",
            target = target.to_string()
        ))));
    };

    backend::unmount(&record.mountpoint)?;
    // Right away, rather than leaving it to the serving process: that one
    // notices the unmount a moment later, by which time a new mount may be
    // reusing the directory.
    remove_if_ours(&record.mountpoint);
    println!(
        "{}",
        t!(
            "unmount-done",
            letter = record.mountpoint.display().to_string()
        )
    );
    Ok(())
}

pub(crate) fn cmd_mounts() -> Result<()> {
    let records = backend::mounts();
    if records.is_empty() {
        println!("{}", t!("mounts-none"));
        return Ok(());
    }
    for r in &records {
        println!("{}  {}", r.mountpoint.display(), r.archive.display());
    }
    Ok(())
}

pub(crate) fn cmd_doctor() -> Result<()> {
    let available = backend::check_available();
    let rows: Vec<(String, String)> = vec![
        (
            backend::doctor_label(),
            match &available {
                Ok(()) => backend::doctor_ok(),
                Err(e) => format!("{e:#}"),
            },
        ),
        (
            t!("doctor-rar"),
            if cfg!(feature = "rar") {
                t!("doctor-rar-yes")
            } else {
                t!("doctor-rar-no")
            },
        ),
        (
            t!("doctor-language"),
            format!(
                "{} ({}), {}",
                zipmount_i18n::language().native_name,
                zipmount_i18n::language().code,
                source_text(zipmount_i18n::source())
            ),
        ),
    ];
    print_table(&rows);
    println!();
    match available {
        Ok(()) => println!("{}", t!("doctor-ready-unix")),
        Err(_) => println!("{}", backend::doctor_note()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_drop_archive_extensions() {
        assert_eq!(archive_stem(Path::new("/data/logs.tar.gz")), "logs");
        assert_eq!(archive_stem(Path::new("/data/Photos.ZIP")), "Photos");
        assert_eq!(archive_stem(Path::new("/data/a.b.7z")), "a.b");
        assert_eq!(archive_stem(Path::new("/data/.zip")), ".zip");
        assert_eq!(archive_stem(Path::new("/data/notes")), "notes");
    }
}
