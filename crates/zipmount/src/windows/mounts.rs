//! Bookkeeping of mounted archives.
//!
//! From the context menu a drive is mounted in the background, and unmounting
//! it later takes another command — so something has to remember which letter
//! belongs to which archive and which process holds it.
//!
//! The state lives in `%LOCALAPPDATA%\ZipMount\mounts.tsv` and is always
//! re-checked: a process may have been killed from outside, and its record is
//! a hint, not the truth. So every read cleans out records whose processes
//! are already dead.

use std::ffi::c_void;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
use zipmount_i18n::t;

/// Brings a path to a form fit both for comparing and for showing.
///
/// On Windows `canonicalize` returns a path with the extended `\\?\`
/// prefix, which looks like garbage in a listing. Comparing does not need it:
/// it is enough that both sides go through the same normalization.
pub fn normalize(path: &Path) -> PathBuf {
    let full = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = full.to_string_lossy();
    match text.strip_prefix("\\\\?\\") {
        Some(rest) => PathBuf::from(rest),
        None => full,
    }
}

#[derive(Debug, Clone)]
pub struct MountRecord {
    pub letter: String,
    pub archive: PathBuf,
    pub pid: u32,
}

/// The user's local data directory.
///
/// Asked of the system rather than taken from the environment. A process
/// started from the context menu arrives **without** `LOCALAPPDATA`: the list
/// of mounts came back empty for it, and a drive mounted from the same menu
/// could not be unmounted afterwards — "not listed as mounted" with the drive
/// right there. The variable remains a fallback.
fn local_app_data() -> Result<PathBuf> {
    // SAFETY: the folder id is a constant, the token the default one, and
    // the returned string is freed right here.
    if let Ok(raw) = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None) }
    {
        let text = unsafe { raw.to_string() };
        unsafe { CoTaskMemFree(Some(raw.0 as *const c_void)) };
        if let Ok(text) = text {
            return Ok(PathBuf::from(text));
        }
    }

    std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .context("cannot determine the local application data directory")
}

/// The state file. The format is deliberately the simplest — a line per
/// mount, fields separated by tabs: people read it, and so do we when
/// debugging.
fn state_path() -> Result<PathBuf> {
    Ok(local_app_data()?.join("ZipMount").join("mounts.tsv"))
}

fn read_raw() -> Vec<MountRecord> {
    let Ok(path) = state_path() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };

    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let letter = parts.next()?.to_string();
            let pid = parts.next()?.parse().ok()?;
            let archive = PathBuf::from(parts.next()?);
            Some(MountRecord {
                letter,
                archive,
                pid,
            })
        })
        .collect()
}

fn write_raw(records: &[MountRecord]) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| t!("err-create-dir", path = parent.display().to_string()))?;
    }
    let text: String = records
        .iter()
        .map(|r| format!("{}\t{}\t{}\n", r.letter, r.pid, r.archive.display()))
        .collect();
    std::fs::write(&path, text).with_context(|| t!("err-write", path = path.display().to_string()))
}

/// Live mounts. Dead records are dropped and cleaned out of the file.
///
/// A record is alive when its process is alive **and** the drive exists. A
/// process id alone is not enough: the system reuses them, and the record of
/// a killed mount can "come back to life" on an unrelated process — that has
/// been seen.
pub fn list() -> Vec<MountRecord> {
    let all = read_raw();
    let alive: Vec<MountRecord> = all
        .into_iter()
        .filter(|r| process_alive(r.pid) && Path::new(&format!("{}\\", r.letter)).exists())
        .collect();
    let _ = write_raw(&alive);
    alive
}

pub fn register(letter: &str, archive: &Path, pid: u32) -> Result<()> {
    let mut records = list();
    records.retain(|r| !r.letter.eq_ignore_ascii_case(letter));
    records.push(MountRecord {
        letter: letter.to_string(),
        archive: archive.to_path_buf(),
        pid,
    });
    write_raw(&records)
}

pub fn unregister(letter: &str) -> Result<()> {
    let mut records = list();
    records.retain(|r| !r.letter.eq_ignore_ascii_case(letter));
    write_raw(&records)
}

/// Waits for the drive to disappear.
///
/// Needed where the mounting process is unknown: there is no one to watch,
/// but the unmount itself shows as the letter going away.
pub fn wait_for_drive_gone(letter: &str, millis: u64) -> bool {
    let root = format!("{letter}\\");
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(millis);
    while std::time::Instant::now() < deadline {
        if !Path::new(&root).exists() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    !Path::new(&root).exists()
}

/// Whether this looks like a drive letter such as `D:`.
pub fn is_drive_letter(target: &str) -> bool {
    let mut chars = target.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.next() == Some(':')
        && chars.next().is_none()
}

pub fn find(letter: &str) -> Option<MountRecord> {
    list()
        .into_iter()
        .find(|r| r.letter.eq_ignore_ascii_case(letter))
}

/// The mount of this archive, if there already is one.
pub fn find_by_archive(archive: &Path) -> Option<MountRecord> {
    let target = normalize(archive);
    list().into_iter().find(|r| normalize(&r.archive) == target)
}

/// The first free drive letter.
///
/// A and B historically belong to floppies and some programs still treat
/// them specially, so we start at D — C is almost always the system drive.
pub fn first_free_letter() -> Option<String> {
    ('D'..='Z')
        .map(|c| format!("{c}:"))
        .find(|d| !Path::new(&format!("{d}\\")).exists())
}

#[allow(dead_code)] // for when the registry letter submenu becomes dynamic
pub fn free_letters() -> Vec<String> {
    ('D'..='Z')
        .map(|c| format!("{c}:"))
        .filter(|d| !Path::new(&format!("{d}\\")).exists())
        .collect()
}

#[allow(dead_code)]
pub fn letter_is_free(letter: &str) -> bool {
    !Path::new(&format!("{letter}\\")).exists()
}

/// Name of the event that tells a mounting process to finish.
///
/// The event lives in the session namespace (`Local\`), not the global one:
/// a mount is visible to its own session only anyway, and a global name would
/// need extra rights.
pub fn stop_event_name(letter: &str) -> String {
    let clean: String = letter
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    format!("Local\\ZipMount-Stop-{}", clean.to_uppercase())
}

/// Starts a background process without handing it a single handle of ours.
///
/// A plain `std::process::Command` inherits handles, and with them the child
/// gets the parent's output pipe. The parent then exits, but the pipe stays
/// open — whoever called us through a pipeline waits for the end of the
/// stream forever. So we start the process ourselves, with
/// `bInheritHandles = FALSE`.
#[cfg(windows)]
pub fn spawn_detached(exe: &Path, args: &[String]) -> Result<u32> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, CREATE_NO_WINDOW, PROCESS_INFORMATION, STARTUPINFOW,
    };

    let mut command_line = quote_arg(&exe.display().to_string());
    for a in args {
        command_line.push(' ');
        command_line.push_str(&quote_arg(a));
    }
    let mut command_line: Vec<u16> = command_line
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    // SAFETY: the command line is mutable and NUL-terminated, as
    // CreateProcessW requires; the structures are zeroed and correctly sized.
    let ok = unsafe {
        CreateProcessW(
            std::ptr::null(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0, // no handles are inherited — the whole point of this function
            CREATE_NO_WINDOW,
            std::ptr::null(),
            std::ptr::null(),
            &startup,
            &mut info,
        )
    };
    if ok == 0 {
        bail!(t!("err-spawn-background"));
    }

    let pid = info.dwProcessId;
    // SAFETY: the handles come from CreateProcessW and are closed once.
    unsafe {
        CloseHandle(info.hThread);
        CloseHandle(info.hProcess);
    }
    Ok(pid)
}

/// Quotes an argument by the rules of Windows command-line parsing.
fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }

    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => {
                backslashes += 1;
                out.push(ch);
            }
            '"' => {
                // Before a quote, every preceding backslash is doubled and the
                // quote itself is escaped.
                for _ in 0..=backslashes {
                    out.push('\\');
                }
                backslashes = 0;
                out.push('"');
            }
            _ => {
                backslashes = 0;
                out.push(ch);
            }
        }
    }
    // Backslashes before the closing quote are doubled too.
    for _ in 0..backslashes {
        out.push('\\');
    }
    out.push('"');
    out
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        CreateEventW, OpenEventW, OpenProcess, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
        INFINITE, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn process_alive(pid: u32) -> bool {
        // SAFETY: a plain handle request by id; the result is checked for
        // null and closed.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
            if handle.is_null() {
                return false;
            }
            CloseHandle(handle);
            true
        }
    }

    /// Creates the stop event and waits on it.
    pub struct StopEvent(HANDLE);

    impl StopEvent {
        pub fn create(name: &str) -> Option<Self> {
            let name = wide(name);
            // SAFETY: the name is a valid NUL-terminated string.
            let handle = unsafe { CreateEventW(std::ptr::null(), FALSE, FALSE, name.as_ptr()) };
            if handle.is_null() {
                None
            } else {
                Some(Self(handle))
            }
        }

        pub fn wait(&self) {
            // SAFETY: the handle came from CreateEventW and is not closed yet.
            unsafe {
                WaitForSingleObject(self.0, INFINITE);
            }
        }
    }

    impl Drop for StopEvent {
        fn drop(&mut self) {
            // SAFETY: the handle is ours and is closed exactly once.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    /// Asks a mounting process to finish. `false` means no such event.
    pub fn signal_stop(name: &str) -> bool {
        let name = wide(name);
        // SAFETY: the name is valid; the handle is checked and closed.
        unsafe {
            let handle = OpenEventW(EVENT_MODIFY_STATE, FALSE, name.as_ptr());
            if handle.is_null() {
                return false;
            }
            let ok = SetEvent(handle) != 0;
            CloseHandle(handle);
            ok
        }
    }

    /// Waits for a process to exit, no longer than the given time.
    pub fn wait_for_exit(pid: u32, millis: u32) -> bool {
        // SAFETY: the handle is checked and closed.
        unsafe {
            let handle = OpenProcess(0x0010_0000 /* SYNCHRONIZE */, FALSE, pid);
            if handle.is_null() {
                return true; // the process is already gone
            }
            let res = WaitForSingleObject(handle, millis);
            CloseHandle(handle);
            res == WAIT_OBJECT_0
        }
    }
}

pub use imp::{process_alive, signal_stop, wait_for_exit, StopEvent};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_letter_is_recognised_exactly() {
        // This check decides whether a mount can be stopped without the
        // list — that is, whether unmounting from the menu works for a process
        // that could not reach the user's profile.
        assert!(is_drive_letter("D:"));
        assert!(is_drive_letter("z:"));
        assert!(!is_drive_letter("D"));
        assert!(!is_drive_letter("D:\\"));
        assert!(!is_drive_letter("D:\\logs"));
        assert!(!is_drive_letter(""));
        assert!(!is_drive_letter("archive.zip"));
    }

    #[test]
    fn local_app_data_does_not_need_the_environment() {
        // A process from the context menu arrives without LOCALAPPDATA, and
        // the path to the list must be found anyway.
        let path = local_app_data().expect("local data directory");
        assert!(path.is_absolute(), "expected an absolute path: {path:?}");
    }

    #[test]
    fn stop_event_name_is_stable_and_safe() {
        assert_eq!(stop_event_name("Z:"), "Local\\ZipMount-Stop-Z");
        assert_eq!(stop_event_name("z:"), "Local\\ZipMount-Stop-Z");
        // Stray characters must not get into a kernel object name.
        assert_eq!(stop_event_name("Z:\\"), "Local\\ZipMount-Stop-Z");
    }

    #[test]
    fn free_letters_do_not_include_existing_drives() {
        for d in free_letters() {
            assert!(
                !Path::new(&format!("{d}\\")).exists(),
                "{d} is actually taken"
            );
        }
    }

    #[test]
    fn quoting_leaves_simple_arguments_alone() {
        // Without spaces or quotes there is no reason to quote: parsing
        // returns the argument as is, trailing backslash included.
        assert_eq!(quote_arg("simple"), "simple");
        assert_eq!(quote_arg("C:\\path\\"), "C:\\path\\");
    }

    #[test]
    fn quoting_doubles_backslashes_before_closing_quote() {
        // Here doubling is mandatory: otherwise the trailing backslash escapes
        // the closing quote and the path runs into the next argument.
        assert_eq!(quote_arg("C:\\my path\\"), "\"C:\\my path\\\\\"");
    }

    #[test]
    fn quoting_escapes_inner_quotes() {
        assert_eq!(quote_arg("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quote_arg("with space"), "\"with space\"");
        assert_eq!(quote_arg(""), "\"\"");
    }

    #[test]
    fn current_process_is_alive() {
        assert!(process_alive(std::process::id()));
    }

    #[test]
    fn absurd_pid_is_not_alive() {
        assert!(!process_alive(0xFFFF_FFF0));
    }
}
