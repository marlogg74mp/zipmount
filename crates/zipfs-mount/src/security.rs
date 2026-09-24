//! The volume's security descriptor.
//!
//! Without a valid descriptor File Explorer shows "Access denied" before the
//! first read. The volume is read-only and shared by all of the user's
//! processes, so one static descriptor serves everything.

use anyhow::{bail, Result};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};

/// Owner and group are administrators; full access to the system and
/// administrators, read and execute to everyone else. No one gets write: the
/// volume is read-only.
///
/// The rights are deliberately `FRFX`, not `GRGX`: the kernel does **not**
/// expand generic bits (GENERIC_READ, GENERIC_EXECUTE) in an ACE into specific
/// rights during an access check, and a descriptor with them silently denies
/// everyone. Generic rights are expanded only when an ACL is created, not
/// when it is checked.
const SDDL: &str = "O:BAG:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;FRFX;;;WD)";

/// Builds a self-relative security descriptor as bytes.
pub fn default_descriptor() -> Result<Vec<u8>> {
    let sddl: Vec<u16> = SDDL.encode_utf16().chain(std::iter::once(0)).collect();
    let mut psd = std::ptr::null_mut();
    let mut size: u32 = 0;

    // SAFETY: sddl is a valid NUL-terminated string; psd and size are filled
    // in by the call, and psd is freed with LocalFree below.
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut psd,
            &mut size,
        )
    };

    if ok == 0 || psd.is_null() {
        bail!("cannot build a security descriptor from SDDL");
    }

    // SAFETY: the call succeeded, so psd points to size valid bytes.
    let bytes = unsafe { std::slice::from_raw_parts(psd as *const u8, size as usize).to_vec() };
    // SAFETY: psd came from ConvertStringSecurityDescriptor... and is freed once.
    unsafe { LocalFree(psd as _) };

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_nonempty_descriptor() {
        let sd = default_descriptor().expect("the descriptor must build");
        assert!(
            sd.len() > 20,
            "the descriptor is suspiciously short: {}",
            sd.len()
        );
        // The first byte of a self-relative descriptor is its revision.
        assert_eq!(sd[0], 1);
    }
}
