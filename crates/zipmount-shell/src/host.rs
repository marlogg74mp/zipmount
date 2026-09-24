//! Everything this handler can do with the system: find `zipmount.exe` next
//! to itself, get the path of the selected object, and start a process.
//!
//! This code runs inside File Explorer, so it obeys two rules.
//!
//! **No panics.** The build profile sets `panic = "abort"`, and a panic in
//! our DLL would bring all of File Explorer down. So not a single `unwrap`,
//! not a single slice index: anything that can fail returns `Option`, and the
//! caller simply does nothing.
//!
//! **Hands off the archive.** The handler parses and reads nothing — it only
//! starts `zipmount.exe`. Opening an archive takes time and memory, and the
//! File Explorer process would be paying for it.

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

use windows::core::{Ref, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HMODULE};
use windows::Win32::Storage::FileSystem::{GetLogicalDrives, GetVolumeInformationW};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows::Win32::System::Threading::{
    CreateProcessW, CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::UI::Shell::{IShellItem, IShellItemArray, SIGDN_FILESYSPATH};

/// Path to the DLL itself.
///
/// The handle comes not from `DllMain` but from the address of one of our own
/// functions: that works whether or not the runtime called our entry point,
/// and needs no global state that would have to be filled in time.
fn module_path() -> Option<PathBuf> {
    // The anchor: any address inside our DLL. This very function will do.
    let anchor = module_path as *const u16;

    let mut module = HMODULE::default();
    // SAFETY: the FROM_ADDRESS flag means the second argument is an address,
    // not a string; UNCHANGED_REFCOUNT does not bump the count, so the handle
    // need not be released.
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(anchor),
            &mut module,
        )
    }
    .ok()?;

    let mut buf = vec![0u16; 4096];
    // SAFETY: the handle comes from above, the buffer lives to the end of the
    // function.
    let len = unsafe { GetModuleFileNameW(Some(module), &mut buf) } as usize;
    // len == buf.len() means truncation — such a path cannot be trusted.
    if len == 0 || len >= buf.len() {
        return None;
    }
    buf.truncate(len);
    Some(PathBuf::from(String::from_utf16(&buf).ok()?))
}

/// Path to the `zipmount.exe` lying next to the DLL.
///
/// Both come from the same external content directory of the package, so
/// there is no need to search PATH or the registry. The result is remembered:
/// the shell asks for title and icon every time it draws the menu.
pub fn zipmount_exe() -> Option<PathBuf> {
    static EXE: OnceLock<Option<PathBuf>> = OnceLock::new();
    EXE.get_or_init(|| {
        let exe = module_path()?.parent()?.join("zipmount.exe");
        exe.is_file().then_some(exe)
    })
    .clone()
}

/// The icon for the menu items.
///
/// A separate file rather than a resource inside `zipmount.exe`: the program
/// is built as a plain console binary without resources, so there is no icon
/// to take from it.
pub fn icon_path() -> Option<PathBuf> {
    static ICON: OnceLock<Option<PathBuf>> = OnceLock::new();
    ICON.get_or_init(|| {
        let icon = module_path()?.parent()?.join("ZipMount.ico");
        icon.is_file().then_some(icon)
    })
    .clone()
}

/// Path of a shell item, if it exists in the filesystem at all.
pub fn item_path(item: &IShellItem) -> Option<String> {
    // SAFETY: GetDisplayName allocates the string with CoTaskMemAlloc; it is
    // freed right after reading.
    let raw = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.ok()?;
    let text = unsafe { raw.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(raw.0 as *const c_void)) };
    text
}

/// Path of the first selected object.
///
/// There is nothing sensible to do with a multiple selection, so the first
/// one is taken and that is it. For the folder background item the array
/// comes empty — the path is found another way there, see `ZipCommand`.
pub fn selected_path(items: Ref<'_, IShellItemArray>) -> Option<String> {
    let items = items.as_ref()?;
    // SAFETY: the pointer is checked for null above; the interface outlives
    // the call.
    let item = unsafe { items.GetItemAt(0) }.ok()?;
    item_path(&item)
}

/// The filesystem of the volume a path lies on — or `None` if it is not a
/// drive-letter path.
///
/// WinFsp reports the filesystem name we set, `ZipFS`, to the system; that
/// is how our volumes are told from others.
pub fn volume_filesystem(path: &str) -> Option<String> {
    let mut chars = path.chars();
    let letter = chars.next()?;
    if !letter.is_ascii_alphabetic() || chars.next() != Some(':') {
        return None;
    }

    let root: Vec<u16> = format!("{letter}:\\")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut name = [0u16; 32];
    // SAFETY: the root is NUL-terminated, the name buffer is ours.
    unsafe {
        GetVolumeInformationW(
            PCWSTR(root.as_ptr()),
            None,
            None,
            None,
            None,
            Some(&mut name),
        )
    }
    .ok()?;

    let len = name.iter().position(|c| *c == 0).unwrap_or(name.len());
    String::from_utf16(name.get(..len)?).ok()
}

/// Letters no volume holds, from D to Z.
///
/// A–C are not offered: the first two historically belong to floppies, the
/// third is almost always the system drive.
pub fn free_letters() -> Vec<char> {
    // SAFETY: no arguments; returns a bit mask of the letters in use.
    let busy = unsafe { GetLogicalDrives() };
    ('D'..='Z')
        .filter(|letter| {
            let bit = *letter as u32 - 'A' as u32;
            busy & (1 << bit) == 0
        })
        .collect()
}

/// Starts `zipmount.exe` and returns at once.
///
/// Handles are not inherited: File Explorer keeps plenty open, and there is
/// no reason to hand it to a background process.
pub fn launch(args: &[String], console: bool) -> Option<()> {
    let exe = zipmount_exe()?;

    let mut line = quote_arg(&exe.to_string_lossy());
    for arg in args {
        line.push(' ');
        line.push_str(&quote_arg(arg));
    }
    let mut line: Vec<u16> = line.encode_utf16().chain(std::iter::once(0)).collect();

    // Search needs a window — it shows results in a console; mounting does
    // not, it reports a failure in a message box.
    let flags = if console {
        CREATE_NEW_CONSOLE
    } else {
        CREATE_NO_WINDOW
    };

    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut info = PROCESS_INFORMATION::default();

    // SAFETY: the command line is mutable and NUL-terminated, as
    // CreateProcessW requires; the structures are zeroed and correctly sized.
    let started = unsafe {
        CreateProcessW(
            PCWSTR::null(),
            Some(PWSTR(line.as_mut_ptr())),
            None,
            None,
            false,
            flags,
            None,
            PCWSTR::null(),
            &startup,
            &mut info,
        )
    };
    if started.is_err() {
        return None;
    }

    // SAFETY: the handles come from CreateProcessW and are closed once.
    unsafe {
        let _ = CloseHandle(info.hThread);
        let _ = CloseHandle(info.hProcess);
    }
    Some(())
}

/// Quotes an argument by the rules of Windows command-line parsing.
///
/// A copy of the same rule in `zipmount::mounts`: linking the crates for
/// twenty lines would drag all the archive parsing into File Explorer.
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
                // Before a quote, every preceding backslash is doubled and
                // the quote itself is escaped.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_leaves_simple_arguments_alone() {
        assert_eq!(quote_arg("mount"), "mount");
        assert_eq!(quote_arg("C:\\logs\\a.zip"), "C:\\logs\\a.zip");
    }

    #[test]
    fn quoting_doubles_backslashes_before_closing_quote() {
        // Without doubling, the trailing backslash escapes the closing quote
        // and the path runs into the next argument.
        assert_eq!(quote_arg("C:\\my logs\\"), "\"C:\\my logs\\\\\"");
    }

    #[test]
    fn volume_filesystem_rejects_non_drive_paths() {
        // Network paths and anything not starting with a drive letter are
        // none of our business: the unmount item must not show there.
        assert_eq!(volume_filesystem("\\\\server\\share"), None);
        assert_eq!(volume_filesystem(""), None);
    }
}
