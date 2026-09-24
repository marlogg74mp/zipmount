//! The Windows side of the command line: mounting through WinFsp, drive
//! letters, the File Explorer context menu, and `doctor`.

pub(crate) mod modern;
pub(crate) mod mounts;
pub(crate) mod shell;

use std::path::Path;
use std::process::Command as ProcCommand;
use std::time::Instant;

use anyhow::{Context, Result};
use zipmount_i18n::{decimal, t};

use crate::{
    human, open_archive, print_table, read_password, report, source_text, ArchiveArgs, MountArgs,
};

/// Registry menu items carry static texts; `zipmount language` rewrites them.
pub(crate) fn refresh_menu_texts(language: &zipmount_i18n::Language) -> Result<()> {
    let rewritten = shell::refresh_texts(language)?;
    println!("{}", t!("language-menu-updated", count = rewritten));
    Ok(())
}

pub(crate) fn cmd_mount(archive: &Path, args: MountArgs, common: &ArchiveArgs) -> Result<()> {
    // Initialize WinFsp before opening the archive: if the driver is missing,
    // better say so at once than after parsing a gigabyte-sized directory.
    let winfsp = zipfs_mount::init();
    if let Err(e) = &winfsp {
        return report(Err(anyhow::anyhow!("{e:#}")));
    }
    let _winfsp = winfsp?;

    // The same archive is already mounted — no need for a second drive on it.
    // Clicking the same archive twice in the context menu is easy.
    if let Some(existing) = mounts::find_by_archive(archive) {
        if args.open {
            open_in_explorer(&existing.letter);
        }
        println!("{}", t!("mount-already", letter = existing.letter.clone()));
        return Ok(());
    }

    let letter = match &args.mountpoint {
        Some(m) => m.clone(),
        None => match mounts::first_free_letter() {
            Some(l) => l,
            None => return report(Err(anyhow::anyhow!(t!("err-no-free-letters")))),
        },
    };

    if let Err(e) = check_mountpoint_free(&letter) {
        return report(Err(e));
    }

    if args.detach {
        return spawn_detached(archive, &letter, &args, common);
    }

    run_mount(archive, &letter, &args, common)
}

/// Starts a copy of itself in the background and returns.
///
/// The password, if needed, goes through a temporary file that the child
/// deletes at once. The command line is no place for it — it is visible in
/// the process list; a pipe will not do either, because the background
/// process starts without inheriting handles (see mounts::spawn_detached).
fn spawn_detached(
    archive: &Path,
    letter: &str,
    args: &MountArgs,
    common: &ArchiveArgs,
) -> Result<()> {
    let password = read_password(common)?;

    let exe = std::env::current_exe().context("cannot determine the program's path")?;
    let mut argv: Vec<String> = vec![
        "mount".into(),
        archive.display().to_string(),
        letter.to_string(),
        "--cache-mb".into(),
        args.cache_mb.to_string(),
        "--encoding".into(),
        common.encoding.clone(),
    ];
    if let Some(label) = &args.label {
        argv.push("--label".into());
        argv.push(label.clone());
    }

    let password_file = match &password {
        Some(pw) => {
            let path = std::env::temp_dir().join(format!(
                "zipmount-pw-{}-{}.tmp",
                std::process::id(),
                letter.trim_end_matches(':')
            ));
            std::fs::write(&path, pw.bytes()).with_context(|| t!("err-spawn-password"))?;
            argv.push("--password-file".into());
            argv.push(path.display().to_string());
            Some(path)
        }
        None => None,
    };

    let pid = mounts::spawn_detached(&exe, &argv)?;

    // Wait for the drive to appear: if the archive did not open, the process
    // dies silently, and it is up to us to say so.
    let appeared = wait_for_drive(letter, std::time::Duration::from_secs(20));

    // The child deletes the password file itself; clean up in case it did
    // not get that far.
    if let Some(path) = password_file {
        let _ = std::fs::remove_file(path);
    }

    if !appeared {
        return report(Err(anyhow::anyhow!(t!(
            "err-mount-failed-detached",
            path = archive.display().to_string()
        ))));
    }

    mounts::register(letter, &mounts::normalize(archive), pid)?;
    println!("{}", t!("mount-done", letter = letter.to_string()));

    if args.open {
        open_in_explorer(letter);
    }
    Ok(())
}

fn wait_for_drive(letter: &str, timeout: std::time::Duration) -> bool {
    let root = format!("{letter}\\");
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if Path::new(&root).exists() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

fn open_in_explorer(letter: &str) {
    let _ = ProcCommand::new("explorer.exe")
        .arg(format!("{letter}\\"))
        .spawn();
}

/// Mounts and holds the drive until asked to stop.
fn run_mount(archive: &Path, letter: &str, args: &MountArgs, common: &ArchiveArgs) -> Result<()> {
    let started = Instant::now();
    let a = open_archive(archive, common, Some(args.cache_mb))?;
    let stats = a.tree().stats();
    let parse_time = started.elapsed();

    let label = args.label.clone().unwrap_or_else(|| {
        archive
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ZipMount".to_string())
    });

    let mount = zipfs_mount::Mount::new(
        a,
        zipfs_mount::MountOptions {
            mountpoint: letter.to_string(),
            label,
            cache_budget: args.cache_mb * 1024 * 1024,
        },
    )?;

    mounts::register(letter, &mounts::normalize(archive), std::process::id())?;

    println!(
        "{}",
        t!(
            "mount-done-stats",
            letter = letter.to_string(),
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
    println!("{}", t!("mount-stop-hint", letter = letter.to_string()));

    wait_for_stop(letter)?;

    println!("{}", t!("mount-unmounting"));
    drop(mount);
    let _ = mounts::unregister(letter);
    println!("{}", t!("mount-finished"));
    Ok(())
}

/// Waits for either Ctrl+C or a request to stop from the unmount command.
fn wait_for_stop(letter: &str) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let ctrl_tx = tx.clone();
    ctrlc::set_handler(move || {
        let _ = ctrl_tx.send(());
    })
    .context("cannot install the Ctrl+C handler")?;

    let name = mounts::stop_event_name(letter);
    std::thread::spawn(move || {
        if let Some(event) = mounts::StopEvent::create(&name) {
            event.wait();
            let _ = tx.send(());
        }
    });

    let _ = rx.recv();
    Ok(())
}

pub(crate) fn cmd_unmount(target: &str) -> Result<()> {
    // File Explorer passes a drive with a trailing backslash, a person types
    // just the letter. Bring both to one form, or the menu item would not
    // find its own mount.
    let trimmed = target.trim_end_matches(['\\', '/']);
    let normalized = if trimmed.len() == 1
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
    {
        format!("{}:", trimmed.to_uppercase())
    } else {
        trimmed.to_string()
    };

    // The target can be named by letter or by archive path — whichever is
    // handier in the menu.
    let record = mounts::find(&normalized).or_else(|| mounts::find_by_archive(Path::new(target)));

    if let Some(record) = record {
        if !mounts::signal_stop(&mounts::stop_event_name(&record.letter)) {
            // The process may not have created the event yet, or be dead.
            let _ = mounts::unregister(&record.letter);
            return report(Err(anyhow::anyhow!(t!(
                "err-unmount-not-responding",
                letter = record.letter.clone()
            ))));
        }

        return if mounts::wait_for_exit(record.pid, 15_000) {
            let _ = mounts::unregister(&record.letter);
            println!("{}", t!("unmount-done", letter = record.letter.clone()));
            Ok(())
        } else {
            report(Err(anyhow::anyhow!(t!(
                "err-unmount-timeout",
                letter = record.letter.clone()
            ))))
        };
    }

    // No record — but the event's name follows from the letter alone, so a
    // mount can be stopped without the list. This is not a just-in-case
    // fallback: the list lives in the user's profile, which may be out of
    // reach — a mount made by another user of this machine, for one.
    if mounts::is_drive_letter(&normalized)
        && mounts::signal_stop(&mounts::stop_event_name(&normalized))
    {
        return if mounts::wait_for_drive_gone(&normalized, 15_000) {
            let _ = mounts::unregister(&normalized);
            println!("{}", t!("unmount-done", letter = normalized.clone()));
            Ok(())
        } else {
            report(Err(anyhow::anyhow!(t!(
                "err-unmount-timeout",
                letter = normalized.clone()
            ))))
        };
    }

    report(Err(anyhow::anyhow!(t!(
        "err-not-mounted",
        target = target.to_string()
    ))))
}

pub(crate) fn cmd_mounts() -> Result<()> {
    let records = mounts::list();
    if records.is_empty() {
        println!("{}", t!("mounts-none"));
        return Ok(());
    }
    for r in &records {
        println!("{}  {}", r.letter, r.archive.display());
    }
    Ok(())
}

fn check_mountpoint_free(mountpoint: &str) -> Result<()> {
    let looks_like_letter = mountpoint.len() == 2
        && mountpoint.ends_with(':')
        && mountpoint
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic());
    if !looks_like_letter {
        return Ok(());
    }

    let root = format!("{mountpoint}\\");
    if !Path::new(&root).exists() {
        return Ok(());
    }

    let free: Vec<String> = ('D'..='Z')
        .map(|c| format!("{c}:"))
        .filter(|d| !Path::new(&format!("{d}\\")).exists())
        .collect();

    anyhow::bail!(t!(
        "err-letter-busy",
        letter = mountpoint.to_string(),
        free = if free.is_empty() {
            t!("letters-none")
        } else {
            free.join(" ")
        }
    ))
}

pub(crate) fn cmd_shell_install(modern: bool) -> Result<()> {
    if modern {
        return modern::install();
    }

    let exe = std::env::current_exe().context("cannot determine the program's path")?;
    shell::install(&exe)?;

    // The menu speaks File Explorer's language, which may differ from this
    // console's (ZIPMOUNT_LANG); the listing below shows the items as they
    // will appear.
    let menu = zipmount_i18n::decide_without_environment().0;
    println!(
        "{}",
        t!(
            "shell-installed",
            extensions = shell::extensions().join(" ")
        )
    );
    println!();
    println!(
        "{}",
        zipmount_i18n::message_in("shell-installed-items", menu, None)
    );
    println!();
    println!("{}", t!("shell-installed-where"));
    println!();
    println!("{}", t!("shell-modern-hint"));
    println!("{}", t!("shell-remove-hint"));
    Ok(())
}

pub(crate) fn cmd_shell_package(out: &Path, library: Option<&str>) -> Result<()> {
    std::fs::create_dir_all(out)
        .with_context(|| t!("err-create-dir", path = out.display().to_string()))?;

    // Without an explicit name the library gets a fingerprint of its contents
    // in its name and is put next to the package. That way the installer
    // never has to overwrite a file loaded by File Explorer — which loads it
    // and does not let go until a reboot.
    let library = match library {
        Some(name) => name.to_string(),
        None => {
            let exe = std::env::current_exe().context("cannot determine the program's path")?;
            let source = exe
                .parent()
                .context("cannot determine the program's directory")?
                .join("zipmount_shell.dll");
            let bytes = std::fs::read(&source)
                .with_context(|| t!("err-read", path = source.display().to_string()))?;
            let name = format!("zipmount_shell_{}.dll", modern::fingerprint(&bytes));
            std::fs::write(out.join(&name), &bytes)?;
            name
        }
    };

    let package = modern::build_package(&library, out)?;
    // The installer build script reads this output, hence "key=value".
    println!("msix={}", package.msix.display());
    println!("certificate={}", package.certificate.display());
    println!("library={library}");
    Ok(())
}

pub(crate) fn cmd_shell_register(
    package: Option<&Path>,
    external: Option<&Path>,
    remove: bool,
) -> Result<()> {
    if remove {
        return modern::unregister();
    }
    // Both options are required unless removing — clap has checked that.
    let (Some(package), Some(external)) = (package, external) else {
        anyhow::bail!("--package and --external are required");
    };

    // The installer passes the path with a trailing dot (otherwise the
    // backslash would escape the closing quote), so bring it to its normal
    // form — it ends up in the registry and in front of the user.
    let external = external
        .canonicalize()
        .map(|path| mounts::normalize(&path))
        .unwrap_or_else(|_| external.to_path_buf());

    modern::register(package, &external)
}

pub(crate) fn cmd_shell_trust(certificate: Option<&Path>, remove: bool) -> Result<()> {
    if remove {
        return modern::untrust_here();
    }
    let Some(certificate) = certificate else {
        anyhow::bail!("--certificate is required");
    };
    modern::trust_here(certificate)
}

pub(crate) fn cmd_shell_uninstall() -> Result<()> {
    // Removes both variants at once: the registry items and the package. If
    // there was no package, nothing is said about certificates.
    modern::uninstall()
}

pub(crate) fn cmd_doctor() -> Result<()> {
    let dll = Path::new(r"C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll");
    let mut rows: Vec<(String, String)> = vec![
        (
            t!("doctor-winfsp"),
            if dll.exists() {
                t!("doctor-winfsp-found", path = dll.display().to_string())
            } else {
                t!("doctor-winfsp-missing")
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

    // A package can be registered while its item never appears — between
    // registration and the menu lies creating the COM object. So the handler
    // itself is asked, not just the list of installed packages.
    rows.push((
        t!("doctor-modern"),
        if modern::is_installed() {
            match modern::probe_handler() {
                Ok(title) => t!("doctor-modern-ok", title = title),
                Err(e) => t!("doctor-modern-silent", error = format!("{e:#}")),
            }
        } else {
            t!("doctor-modern-missing")
        },
    ));

    // The real readiness check is the same initialization mount performs.
    match zipfs_mount::init() {
        Ok(_) => {
            rows.push((t!("doctor-init"), t!("doctor-init-ok")));
            print_table(&rows);
            println!();
            println!("{}", t!("doctor-ready"));
        }
        Err(e) => {
            rows.push((t!("doctor-init"), t!("doctor-init-failed")));
            print_table(&rows);
            println!();
            println!("{}", t!("doctor-reason", error = format!("{e:#}")));
            println!();
            println!("{}", t!("doctor-no-winfsp-note"));
        }
    }
    Ok(())
}
