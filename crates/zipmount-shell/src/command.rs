//! The menu items themselves.
//!
//! One type, `ZipCommand`, serves every item and tells them apart by its
//! `kind` field — the items differ only in title and arguments, and a type
//! per item would mean writing the same interface four times.
//!
//! Unlike the registry version, the letter submenu is built **at display
//! time**, so it lists only free letters. The registry could not do that: its
//! list is fixed at install time and soon starts lying.
//!
//! Titles come from the text catalogue in the language chosen with
//! `zipmount language`, asked again before every menu is built: File Explorer
//! keeps this library loaded for hours, and a language changed meanwhile has
//! to show up without restarting it.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows::core::{
    implement, IUnknown, Interface, Ref, Result as WinResult, GUID, HRESULT, PCWSTR, PWSTR,
};
use windows::Win32::Foundation::{E_FAIL, E_NOTIMPL, E_POINTER, S_FALSE, S_OK};
use windows::Win32::System::Com::{IBindCtx, IServiceProvider};
use windows::Win32::System::Ole::{IObjectWithSite, IObjectWithSite_Impl};
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl,
    IFolderView, IShellItem, IShellItemArray, SHStrDupW, ECF_DEFAULT, ECF_HASSUBCOMMANDS,
    ECS_ENABLED, ECS_HIDDEN,
};
use zipmount_i18n::t;

use crate::host;

/// The filesystem name WinFsp reports for our volumes.
const OUR_FILESYSTEM: &str = "ZipFS";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Mount on the first free letter and open.
    Mount,
    /// Parent of the letter submenu.
    MountAs,
    /// Mount on a particular letter.
    Letter(char),
    /// Search the archive's contents.
    Search,
    /// Unmount the volume we are inside.
    Unmount,
}

#[implement(IExplorerCommand, IObjectWithSite)]
pub struct ZipCommand {
    kind: Kind,
    /// What the shell offers to reach the open folder through.
    site: RefCell<Option<IUnknown>>,
}

impl ZipCommand {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            site: RefCell::new(None),
        }
    }

    /// The folder open in the window the menu belongs to.
    ///
    /// Needed for the item on a folder's background: there the shell does not
    /// put the folder into the array of selected items — nothing is selected
    /// — but hands the window over separately, through `IObjectWithSite`.
    /// Without it the background item never shows: the volume check finds no
    /// path and hides it.
    fn folder_from_site(&self) -> Option<String> {
        let site = self.site.borrow().clone()?;
        let provider: IServiceProvider = site.cast().ok()?;

        // The service id is the same as the interface id — that is
        // SID_SFolderView from the shell headers.
        // SAFETY: both the service and the interface belong to the shell; we
        // only ask.
        let view: IFolderView = unsafe { provider.QueryService(&IFolderView::IID) }.ok()?;
        let folder: IShellItem = unsafe { view.GetFolder() }.ok()?;
        host::item_path(&folder)
    }

    /// The path the item applies to: the selected object or, with nothing
    /// selected, the open folder.
    fn target_path(&self, items: Ref<'_, IShellItemArray>) -> Option<String> {
        host::selected_path(items).or_else(|| self.folder_from_site())
    }

    fn title(&self) -> String {
        match self.kind {
            Kind::Mount => t!("menu-mount"),
            Kind::MountAs => t!("menu-mount-as"),
            Kind::Letter(letter) => format!("{letter}:"),
            Kind::Search => t!("menu-search"),
            Kind::Unmount => t!("menu-unmount"),
        }
    }

    /// Arguments for `zipmount.exe` for the selected object.
    fn arguments(&self, path: &str) -> Option<(Vec<String>, bool)> {
        let args = match self.kind {
            Kind::Mount => vec![
                "mount".into(),
                path.into(),
                "--detach".into(),
                "--open".into(),
            ],
            Kind::Letter(letter) => vec![
                "mount".into(),
                path.into(),
                format!("{letter}:"),
                "--detach".into(),
                "--open".into(),
            ],
            Kind::Search => return Some((vec!["search".into(), path.into()], true)),
            Kind::Unmount => {
                // The item lives on a folder's background, so the path can be
                // anywhere inside the volume — and the whole volume is what
                // gets unmounted.
                let letter = drive_letter(path)?;
                vec!["unmount".into(), format!("{letter}:")]
            }
            // The submenu parent has no action of its own.
            Kind::MountAs => return None,
        };
        Some((args, false))
    }
}

/// The drive letter a path lies on.
fn drive_letter(path: &str) -> Option<char> {
    let mut chars = path.chars();
    let letter = chars.next()?;
    (letter.is_ascii_alphabetic() && chars.next() == Some(':')).then_some(letter)
}

/// Allocates a string the way the shell expects: with `CoTaskMemAlloc`.
fn to_shell_string(text: &str) -> WinResult<PWSTR> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: the string is NUL-terminated and lives until the end of the
    // call; the shell owns the copy from then on.
    unsafe { SHStrDupW(PCWSTR(wide.as_ptr())) }
}

impl IExplorerCommand_Impl for ZipCommand_Impl {
    fn GetTitle(&self, items: Ref<'_, IShellItemArray>) -> WinResult<PWSTR> {
        // A registry read and a list of display languages: cheap next to
        // drawing a menu, and the only way to follow `zipmount language`
        // without restarting File Explorer.
        zipmount_i18n::refresh();

        // Unmounting is declared both on a folder and on an open folder's
        // background, so the click can come from anywhere inside the volume.
        // Naming the letter in the item costs less than making people guess.
        if self.kind == Kind::Unmount {
            if let Some(letter) = self.target_path(items).as_deref().and_then(drive_letter) {
                return to_shell_string(&t!("menu-unmount-letter", letter = letter.to_string()));
            }
        }
        to_shell_string(&self.title())
    }

    fn GetIcon(&self, _items: Ref<'_, IShellItemArray>) -> WinResult<PWSTR> {
        // Letters in the submenu get no icon: a row of identical pictures
        // next to "D:", "E:", "F:" tells nothing.
        if matches!(self.kind, Kind::Letter(_)) {
            return Err(E_NOTIMPL.into());
        }
        let icon = host::icon_path().ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        to_shell_string(&format!("{},0", icon.display()))
    }

    fn GetToolTip(&self, _items: Ref<'_, IShellItemArray>) -> WinResult<PWSTR> {
        // The new menu does not show tooltips anyway.
        Err(E_NOTIMPL.into())
    }

    fn GetCanonicalName(&self) -> WinResult<GUID> {
        Ok(GUID::zeroed())
    }

    fn GetState(
        &self,
        items: Ref<'_, IShellItemArray>,
        _ok_to_be_slow: windows::core::BOOL,
    ) -> WinResult<u32> {
        // Archive items are already filtered by extension in the manifest;
        // no need to check them again.
        if self.kind != Kind::Unmount {
            return Ok(ECS_ENABLED.0 as u32);
        }

        // Unmounting, though, is declared on any folder and must show only
        // inside our volume.
        let ours = self
            .target_path(items)
            .and_then(|path| host::volume_filesystem(&path))
            .is_some_and(|fs| fs == OUR_FILESYSTEM);

        Ok(if ours {
            ECS_ENABLED.0 as u32
        } else {
            ECS_HIDDEN.0 as u32
        })
    }

    fn Invoke(&self, items: Ref<'_, IShellItemArray>, _bind: Ref<'_, IBindCtx>) -> WinResult<()> {
        let path = self
            .target_path(items)
            .ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        let (args, console) = self
            .arguments(&path)
            .ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        host::launch(&args, console).ok_or_else(|| windows::core::Error::from(E_FAIL))
    }

    fn GetFlags(&self) -> WinResult<u32> {
        Ok(if self.kind == Kind::MountAs {
            ECF_HASSUBCOMMANDS.0 as u32
        } else {
            ECF_DEFAULT.0 as u32
        })
    }

    fn EnumSubCommands(&self) -> WinResult<IEnumExplorerCommand> {
        if self.kind != Kind::MountAs {
            return Err(E_NOTIMPL.into());
        }

        let letters = host::free_letters()
            .into_iter()
            .map(|letter| ZipCommand::new(Kind::Letter(letter)).into())
            .collect();

        Ok(LetterEnum {
            letters,
            position: AtomicUsize::new(0),
        }
        .into())
    }
}

/// The shell passes the window the menu belongs to through this interface.
///
/// Implemented for the folder background item alone: there is nowhere else
/// to get the open folder from.
impl IObjectWithSite_Impl for ZipCommand_Impl {
    fn SetSite(&self, site: Ref<'_, IUnknown>) -> WinResult<()> {
        *self.site.borrow_mut() = site.cloned();
        Ok(())
    }

    fn GetSite(&self, iid: *const GUID, out: *mut *mut c_void) -> WinResult<()> {
        if out.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: the pointer is checked; zero it before any possible error,
        // as the GetSite contract requires.
        unsafe { *out = std::ptr::null_mut() };

        let site = self
            .site
            .borrow()
            .clone()
            .ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        // SAFETY: iid comes from the shell, out is checked above.
        unsafe { site.query(iid, out).ok() }
    }
}

/// Enumerator of the letter submenu.
#[implement(IEnumExplorerCommand)]
struct LetterEnum {
    letters: Vec<IExplorerCommand>,
    position: AtomicUsize,
}

impl IEnumExplorerCommand_Impl for LetterEnum_Impl {
    fn Next(&self, count: u32, out: *mut Option<IExplorerCommand>, fetched: *mut u32) -> HRESULT {
        if out.is_null() {
            return E_FAIL;
        }

        let mut taken = 0u32;
        while taken < count {
            let index = self.position.fetch_add(1, Ordering::Relaxed);
            let Some(command) = self.letters.get(index) else {
                // Past the end — pull back, or the counter would creep off to
                // infinity on repeated calls.
                self.position.store(self.letters.len(), Ordering::Relaxed);
                break;
            };
            // SAFETY: the shell allocated an array of `count` elements, and
            // `taken < count`; the slot is uninitialized, so it is written
            // with `write` rather than assigned.
            unsafe { out.add(taken as usize).write(Some(command.clone())) };
            taken += 1;
        }

        if !fetched.is_null() {
            // SAFETY: the pointer is checked for null.
            unsafe { *fetched = taken };
        }

        if taken == count {
            S_OK
        } else {
            S_FALSE
        }
    }

    fn Skip(&self, count: u32) -> WinResult<()> {
        self.position.fetch_add(count as usize, Ordering::Relaxed);
        Ok(())
    }

    fn Reset(&self) -> WinResult<()> {
        self.position.store(0, Ordering::Relaxed);
        Ok(())
    }

    fn Clone(&self) -> WinResult<IEnumExplorerCommand> {
        Ok(LetterEnum {
            letters: self.letters.clone(),
            position: AtomicUsize::new(self.position.load(Ordering::Relaxed)),
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmount_takes_the_volume_not_the_folder() {
        // The item is invoked from anywhere inside the volume, and the whole
        // volume is what must be unmounted.
        let command = ZipCommand::new(Kind::Unmount);
        let (args, _) = command.arguments("Z:\\logs\\2026\\09").expect("arguments");
        assert_eq!(args, vec!["unmount".to_string(), "Z:".to_string()]);
    }

    #[test]
    fn letter_command_passes_the_chosen_letter() {
        let command = ZipCommand::new(Kind::Letter('M'));
        let (args, console) = command.arguments("C:\\logs\\a.7z").expect("arguments");
        assert_eq!(args[2], "M:");
        assert!(!console, "mounting needs no console");
    }

    #[test]
    fn search_asks_for_a_console() {
        let command = ZipCommand::new(Kind::Search);
        let (_, console) = command.arguments("C:\\logs\\a.7z").expect("arguments");
        assert!(console, "search shows its results in a console");
    }

    #[test]
    fn submenu_parent_has_no_action_of_its_own() {
        assert!(ZipCommand::new(Kind::MountAs)
            .arguments("C:\\logs\\a.zip")
            .is_none());
    }

    #[test]
    fn titles_are_translated_not_ids() {
        for kind in [Kind::Mount, Kind::MountAs, Kind::Search, Kind::Unmount] {
            let title = ZipCommand::new(kind).title();
            assert!(!title.starts_with("menu-"), "untranslated title {title}");
        }
    }
}
