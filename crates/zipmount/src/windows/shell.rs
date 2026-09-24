//! Context menu items in File Explorer, written to the registry.
//!
//! Everything goes to `HKEY_CURRENT_USER` only, so no administrator rights are
//! needed and removing it is as simple as adding it.
//!
//! Two decisions worth explaining:
//!
//! **Items go into `SystemFileAssociations`, not into the extension's ProgID.**
//! An archive's ProgID is usually taken by an archiver (WinRAR, say), and
//! writing our commands there would mean meddling with someone else's
//! registration. `SystemFileAssociations` is the designated place for adding
//! verbs to a file type without touching its association.
//!
//! **The letter submenu lists every letter, not just the free ones.** The
//! registry is static while the set of free letters changes; a submenu fixed
//! at install time would soon start lying. Picking a busy letter produces a
//! clear error — more honest than a stale list.
//!
//! An important limitation: in Windows 11 registry items do not reach the main
//! context menu; they live under "Show more options" (or straight away with
//! Shift+right-click). The main menu needs a COM handler with application
//! identity — see `modern.rs`.
//!
//! Item texts are static registry strings, so they are written in one
//! language — the one File Explorer speaks, see
//! [`zipmount_i18n::decide_without_environment`] — and rewritten by
//! [`refresh_texts`] when that language changes.

use std::path::Path;

use anyhow::{bail, Result};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE,
    REG_SZ, RRF_RT_REG_SZ,
};
use zipmount_i18n::{message_in, Language};

/// Extensions that get the items. `.rar` only in a build with the `rar`
/// feature: without it, an item on a rar file would promise what the program
/// cannot do.
#[cfg(feature = "rar")]
const EXTENSIONS: &[&str] = &[".zip", ".7z", ".rar", ".tar", ".gz", ".tgz"];
#[cfg(not(feature = "rar"))]
const EXTENSIONS: &[&str] = &[".zip", ".7z", ".tar", ".gz", ".tgz"];

/// The key with the letter submenu. It sits next to the ProgIDs, as
/// `ExtendedSubCommandsKey` requires.
const LETTERS_KEY: &str = "Software\\Classes\\ZipMount.Letters";

/// The unmount item on a drive. The installer writes the same key to
/// `HKEY_LOCAL_MACHINE`.
const DRIVE_VERB_KEY: &str = "Software\\Classes\\Drive\\shell\\ZipMount.Unmount";

/// Archive items: the verb key, the value holding the text, and the message.
///
/// The text value differs: a submenu parent shows `MUIVerb`, a plain verb its
/// default value.
const ARCHIVE_TEXTS: &[(&str, &str, &str)] = &[
    ("ZipMount.Mount", "", "menu-mount"),
    ("ZipMount.MountAs", "MUIVerb", "menu-mount-as"),
    ("ZipMount.Search", "", "menu-search"),
];

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Creates a key and writes string values into it.
/// An empty value name means the default value.
fn write_key(path: &str, values: &[(&str, &str)]) -> Result<()> {
    let path_w = wide(path);
    let mut key: HKEY = std::ptr::null_mut();

    // SAFETY: the path is NUL-terminated; the handle is checked and closed
    // below.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path_w.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        bail!("cannot create registry key {path} (code {status})");
    }

    for (name, value) in values {
        let name_w = wide(name);
        let value_w = wide(value);
        let bytes = std::mem::size_of_val(&value_w[..]) as u32;
        // SAFETY: the key is open for writing, the buffers are alive, the
        // length in bytes is right.
        let status = unsafe {
            RegSetValueExW(
                key,
                if name.is_empty() {
                    std::ptr::null()
                } else {
                    name_w.as_ptr()
                },
                0,
                REG_SZ,
                value_w.as_ptr() as *const u8,
                bytes,
            )
        };
        if status != ERROR_SUCCESS {
            // SAFETY: the handle came from RegCreateKeyExW.
            unsafe { RegCloseKey(key) };
            bail!("cannot write value {name:?} to {path} (code {status})");
        }
    }

    // SAFETY: the handle came from RegCreateKeyExW and is closed once.
    unsafe { RegCloseKey(key) };
    Ok(())
}

fn key_exists(root: HKEY, path: &str) -> bool {
    let path_w = wide(path);
    let mut key: HKEY = std::ptr::null_mut();
    // SAFETY: the path is NUL-terminated; a handle, if any, is closed at once.
    unsafe {
        if RegOpenKeyExW(root, path_w.as_ptr(), 0, KEY_READ, &mut key) != ERROR_SUCCESS {
            return false;
        }
        RegCloseKey(key);
    }
    true
}

/// A string value; an empty name reads the default value.
fn read_value(root: HKEY, path: &str, name: &str) -> Option<String> {
    let path_w = wide(path);
    let name_w = wide(name);
    let mut buffer = vec![0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: the buffer and its size in bytes describe the same memory;
    // RRF_RT_REG_SZ guarantees a NUL-terminated string on success.
    let status = unsafe {
        RegGetValueW(
            root,
            path_w.as_ptr(),
            if name.is_empty() {
                std::ptr::null()
            } else {
                name_w.as_ptr()
            },
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

fn delete_tree(path: &str) {
    let path_w = wide(path);
    // SAFETY: the path is NUL-terminated; a missing key is not an error for us.
    unsafe {
        RegDeleteTreeW(HKEY_CURRENT_USER, path_w.as_ptr());
    }
}

/// The language File Explorer speaks — the one registry texts are written in.
fn explorer_language() -> &'static Language {
    zipmount_i18n::decide_without_environment().0
}

/// The whole menu: items on archives and unmounting on the drive.
pub fn install(exe: &Path) -> Result<()> {
    archive_verbs(exe)?;
    drive_verb(exe)
}

/// Only unmounting on the drive.
///
/// That is the one thing the modern menu package cannot do: a drive is not
/// among the targets its manifest allows. The archive items come from the
/// package in that case, and duplicating them in the registry would make them
/// show up a second time under "Show more options".
pub fn install_drive_only(exe: &Path) -> Result<()> {
    remove_archive_verbs();
    drive_verb(exe)
}

fn archive_verbs(exe: &Path) -> Result<()> {
    let exe = exe.display().to_string();
    let language = explorer_language();
    let text = |id: &str| message_in(id, language, None);

    for ext in EXTENSIONS {
        let base = format!("Software\\Classes\\SystemFileAssociations\\{ext}\\shell");

        // Mount and open — the most common case, so it comes first.
        write_key(
            &format!("{base}\\ZipMount.Mount"),
            &[("", &text("menu-mount")), ("Icon", &format!("{exe},0"))],
        )?;
        write_key(
            &format!("{base}\\ZipMount.Mount\\command"),
            &[("", &format!("\"{exe}\" mount \"%1\" --detach --open"))],
        )?;

        // Picking a letter — a submenu.
        write_key(
            &format!("{base}\\ZipMount.MountAs"),
            &[
                ("MUIVerb", &text("menu-mount-as")),
                ("Icon", &format!("{exe},0")),
                ("ExtendedSubCommandsKey", "ZipMount.Letters"),
            ],
        )?;

        write_key(
            &format!("{base}\\ZipMount.Search"),
            &[("", &text("menu-search")), ("Icon", &format!("{exe},0"))],
        )?;
        write_key(
            &format!("{base}\\ZipMount.Search\\command"),
            &[("", &format!("\"{exe}\" search \"%1\""))],
        )?;
    }

    // The letter submenu. Every letter is listed: see the module comment.
    for letter in 'D'..='Z' {
        let key = format!("{LETTERS_KEY}\\shell\\{letter}");
        write_key(&key, &[("MUIVerb", &format!("{letter}:"))])?;
        write_key(
            &format!("{key}\\command"),
            &[(
                "",
                &format!("\"{exe}\" mount \"%1\" {letter}: --detach --open"),
            )],
        )?;
    }

    Ok(())
}

/// Unmounting with a click on the drive itself.
///
/// AppliesTo restricts the item to our volumes: WinFsp tells the system the
/// filesystem name, which we set to ZipFS. Without the condition the item
/// would hang on every drive in the system.
fn drive_verb(exe: &Path) -> Result<()> {
    let exe = exe.display().to_string();
    write_drive_verb(
        &message_in("menu-unmount", explorer_language(), None),
        &format!("{exe},0"),
        &format!("\"{exe}\" unmount \"%1\""),
    )
}

fn write_drive_verb(text: &str, icon: &str, command: &str) -> Result<()> {
    write_key(
        DRIVE_VERB_KEY,
        &[
            ("", text),
            ("Icon", icon),
            ("AppliesTo", "System.Volume.FileSystem:ZipFS"),
        ],
    )?;
    write_key(&format!("{DRIVE_VERB_KEY}\\command"), &[("", command)])
}

/// Rewrites the texts of the registry items that exist, in `language`.
/// Returns how many kinds of item were rewritten.
///
/// Only texts change; commands and icons stay as installed. Nothing is added
/// that was not there — with one exception: the installer writes the drive
/// item to `HKEY_LOCAL_MACHINE`, where a user cannot write without elevation.
/// For that one a full copy with the new text goes to `HKEY_CURRENT_USER`,
/// which File Explorer prefers. Should ZipMount be uninstalled later, the copy
/// stays harmless: its AppliesTo shows it only on ZipMount volumes, and there
/// are none without the program.
pub fn refresh_texts(language: &Language) -> Result<usize> {
    let mut rewritten = 0;

    for (verb, value, id) in ARCHIVE_TEXTS {
        let text = message_in(id, language, None);
        let mut found = false;
        for ext in EXTENSIONS {
            let key = format!("Software\\Classes\\SystemFileAssociations\\{ext}\\shell\\{verb}");
            if key_exists(HKEY_CURRENT_USER, &key) {
                write_key(&key, &[(value, &text)])?;
                found = true;
            }
        }
        rewritten += usize::from(found);
    }

    let text = message_in("menu-unmount", language, None);
    if key_exists(HKEY_CURRENT_USER, DRIVE_VERB_KEY) {
        write_key(DRIVE_VERB_KEY, &[("", &text)])?;
        rewritten += 1;
    } else if key_exists(HKEY_LOCAL_MACHINE, DRIVE_VERB_KEY) {
        let command = read_value(
            HKEY_LOCAL_MACHINE,
            &format!("{DRIVE_VERB_KEY}\\command"),
            "",
        );
        if let Some(command) = command {
            let icon = read_value(HKEY_LOCAL_MACHINE, DRIVE_VERB_KEY, "Icon").unwrap_or_default();
            write_drive_verb(&text, &icon, &command)?;
            rewritten += 1;
        }
    }

    Ok(rewritten)
}

fn remove_archive_verbs() {
    for ext in EXTENSIONS {
        let base = format!("Software\\Classes\\SystemFileAssociations\\{ext}\\shell");
        for (verb, _, _) in ARCHIVE_TEXTS {
            delete_tree(&format!("{base}\\{verb}"));
        }
    }
    delete_tree(LETTERS_KEY);
}

pub fn uninstall() -> Result<()> {
    remove_archive_verbs();
    delete_tree(DRIVE_VERB_KEY);
    Ok(())
}

/// Extensions served by the menu items.
pub fn extensions() -> &'static [&'static str] {
    EXTENSIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_every_supported_format() {
        // The extension list must cover every readable format, or the item
        // simply will not appear where it is needed.
        for needed in [".zip", ".7z", ".tar", ".gz"] {
            assert!(
                EXTENSIONS.contains(&needed),
                "extension {needed} has no menu items"
            );
        }
        // And the reverse: an item on a format the build cannot read is worse
        // than no item.
        assert_eq!(EXTENSIONS.contains(&".rar"), cfg!(feature = "rar"));
    }

    #[test]
    fn letters_key_is_relative_to_classes() {
        // ExtendedSubCommandsKey expects a path relative to Software\\Classes,
        // so the value must carry no prefix.
        assert!(LETTERS_KEY.starts_with("Software\\Classes\\"));
        assert_eq!(
            LETTERS_KEY.trim_start_matches("Software\\Classes\\"),
            "ZipMount.Letters"
        );
    }

    #[test]
    fn menu_texts_exist_in_every_language() {
        // A registry text that falls back to its id would show "menu-mount"
        // in File Explorer.
        for language in zipmount_i18n::LANGUAGES {
            for (_, _, id) in ARCHIVE_TEXTS.iter().chain([&("", "", "menu-unmount")]) {
                assert_ne!(
                    message_in(id, language, None),
                    *id,
                    "{} lacks {id}",
                    language.code
                );
            }
        }
    }
}
