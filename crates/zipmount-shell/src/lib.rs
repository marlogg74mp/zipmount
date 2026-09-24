//! A COM handler for the File Explorer context menu.
//!
//! Why it exists when registry items already work: in Windows 11 registry
//! commands do not reach the main context menu; they live under "Show more
//! options". The main menu admits only `IExplorerCommand` handlers registered
//! by a package with application identity. Hence this library, and the sparse
//! package around it.
//!
//! The library is small on purpose: it does not depend on `zipfs-core` and
//! knows nothing about archives. Its job is to show an item and start the
//! `zipmount.exe` lying next to it. Everything heavy happens in a separate
//! process, not inside File Explorer.
#![cfg(windows)]

mod command;
mod host;

use std::ffi::c_void;

use windows::core::{implement, IUnknown, Interface, Ref, Result as WinResult, GUID, HRESULT};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_POINTER, S_FALSE,
};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::Shell::IExplorerCommand;

use command::{Kind, ZipCommand};

/// Classes declared in the package manifest.
///
/// Every visible menu item is a class of its own: the shell creates an object
/// by the id from the manifest and only then asks it for a title. Letters in
/// the submenu have no classes; the enumerator hands them out.
const CLSID_MOUNT: GUID = GUID::from_u128(0xf4dea0b3_b14e_4cbf_91f4_3fd0bb248c2c);
const CLSID_MOUNT_AS: GUID = GUID::from_u128(0x131a8c81_e972_4504_8b7b_60b629139398);
const CLSID_SEARCH: GUID = GUID::from_u128(0x657508c5_2bde_45b6_90e3_cac87d4465fa);
const CLSID_UNMOUNT: GUID = GUID::from_u128(0xb4eb55eb_2ef5_4bd4_9fd7_13fb7d12e51b);

/// Where a menu item shows.
///
/// The package manifest allows only a few targets: a file extension, `*`,
/// `Directory` and `Directory\Background`. A drive is not among them, and
/// that is the main limitation: the package cannot reach the volume itself,
/// so unmounting is declared on a folder — both a selected one and the
/// background of an open one.
pub enum Target {
    /// On files with a supported extension.
    Archive,
    /// On a folder, and on empty space inside an open folder.
    Folder,
}

/// A menu item the way the package manifest expects it.
pub struct Verb {
    /// Verb id; unique within one file type.
    pub id: &'static str,
    /// The class the shell creates when it gets to this item.
    /// Without braces: that is the only form the manifest schema accepts.
    pub clsid: &'static str,
    pub target: Target,
}

impl Verb {
    /// The same id in a form fit for COM.
    ///
    /// Returns `None` only if the string is damaged — a bug in the code,
    /// caught by a test below rather than by a panic at the user's.
    pub fn guid(&self) -> Option<GUID> {
        let digits: String = self
            .clsid
            .trim_start_matches('{')
            .trim_end_matches('}')
            .chars()
            .filter(|c| *c != '-')
            .collect();
        if digits.len() != 32 {
            return None;
        }
        u128::from_str_radix(&digits, 16).ok().map(GUID::from_u128)
    }
}

/// The table for the manifest generator.
///
/// The manifest and this library must agree on class ids: a mismatch breaks
/// no build and logs nothing — the item simply does not appear. So the
/// strings live next to the classes themselves, and a test checks they match.
pub const VERBS: &[Verb] = &[
    Verb {
        id: "Mount",
        clsid: "F4DEA0B3-B14E-4CBF-91F4-3FD0BB248C2C",
        target: Target::Archive,
    },
    Verb {
        id: "MountAs",
        clsid: "131A8C81-E972-4504-8B7B-60B629139398",
        target: Target::Archive,
    },
    Verb {
        id: "Search",
        clsid: "657508C5-2BDE-45B6-90E3-CAC87D4465FA",
        target: Target::Archive,
    },
    Verb {
        id: "Unmount",
        clsid: "B4EB55EB-2EF5-4BD4-9FD7-13FB7D12E51B",
        target: Target::Folder,
    },
];

fn kind_for(clsid: &GUID) -> Option<Kind> {
    match *clsid {
        CLSID_MOUNT => Some(Kind::Mount),
        CLSID_MOUNT_AS => Some(Kind::MountAs),
        CLSID_SEARCH => Some(Kind::Search),
        CLSID_UNMOUNT => Some(Kind::Unmount),
        _ => None,
    }
}

#[implement(IClassFactory)]
struct ClassFactory {
    kind: Kind,
}

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> WinResult<()> {
        if object.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: the pointer is checked for null; zero it before any
        // possible error, as the CreateInstance contract requires.
        unsafe { *object = std::ptr::null_mut() };

        // No aggregation — the shell does not ask for it either.
        if outer.as_ref().is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        let command: IExplorerCommand = ZipCommand::new(self.kind).into();
        // SAFETY: iid comes from the shell, object is checked above.
        unsafe { command.query(iid, object).ok() }
    }

    fn LockServer(&self, _lock: windows::core::BOOL) -> WinResult<()> {
        Ok(())
    }
}

/// # Safety
///
/// Called only by COM, by the rules of `DllGetClassObject`: `clsid` and `iid`
/// point to valid ids, `object` to room for one pointer.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    iid: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if clsid.is_null() || object.is_null() {
        return E_POINTER;
    }
    // SAFETY: the pointer is checked for null.
    let Some(kind) = kind_for(unsafe { &*clsid }) else {
        return CLASS_E_CLASSNOTAVAILABLE;
    };

    let factory: IClassFactory = ClassFactory { kind }.into();
    // SAFETY: iid and object come from COM and are checked above.
    unsafe { factory.query(iid, object) }
}

/// The library is not unloaded for the lifetime of the process.
///
/// That is safer: live objects would have to be counted by hand, and a
/// counting mistake brings File Explorer down. It gets in nobody's way — the
/// files live in their own install directory, not the build directory, so
/// the copy held does not block rebuilding.
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_class_id_maps_to_a_command() {
        // Should the manifest declare a class missing from this table, the
        // shell gets CLASS_E_CLASSNOTAVAILABLE and the item silently never
        // appears. Let a test catch that.
        for clsid in [CLSID_MOUNT, CLSID_MOUNT_AS, CLSID_SEARCH, CLSID_UNMOUNT] {
            assert!(kind_for(&clsid).is_some());
        }
    }

    #[test]
    fn class_ids_are_distinct() {
        let all = [CLSID_MOUNT, CLSID_MOUNT_AS, CLSID_SEARCH, CLSID_UNMOUNT];
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "two items share one class id");
            }
        }
    }

    #[test]
    fn unknown_class_is_rejected() {
        assert!(kind_for(&GUID::zeroed()).is_none());
    }

    #[test]
    fn manifest_class_ids_match_the_real_classes() {
        // The main check of this file: the string in the manifest and the
        // constant DllGetClassObject hands out objects by must match. If they
        // drift apart, the menu item vanishes and nothing reports it.
        for verb in VERBS {
            let clsid = verb.guid().expect("the class id parses");
            assert!(
                kind_for(&clsid).is_some(),
                "the class of item {} is not served",
                verb.id
            );
        }
    }

    #[test]
    fn every_class_appears_in_the_manifest_table() {
        // And the reverse: a class we can create but did not declare in the
        // manifest will never be asked for by the shell.
        for clsid in [CLSID_MOUNT, CLSID_MOUNT_AS, CLSID_SEARCH, CLSID_UNMOUNT] {
            assert!(
                VERBS.iter().any(|verb| verb.guid() == Some(clsid)),
                "class {clsid:?} is missing from the manifest table"
            );
        }
    }

    #[test]
    fn manifest_form_has_no_braces() {
        // The manifest schema accepts GUIDs without braces and rejects the
        // whole package if they are there.
        for verb in VERBS {
            assert!(
                !verb.clsid.contains('{') && !verb.clsid.contains('}'),
                "braces left in the class id of {}",
                verb.id
            );
        }
    }
}
