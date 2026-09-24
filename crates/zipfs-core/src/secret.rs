//! A password in memory.
//!
//! The wrapper is there for three things: the password is wiped on drop, is
//! never printed in debug output, and never ends up in an error message. It
//! is no complete protection (the unpacking crate makes its own copy anyway,
//! and pages can go to the swap file), but it removes the most embarrassing
//! leaks — a password in a log, in a panic, or in a dump after it was freed.

use std::fmt;

use zeroize::{Zeroize, Zeroizing};
use zipmount_i18n::t;

/// The archive needs a password: none was given, or the one given is wrong.
///
/// A type rather than just a message, so that callers can recognise the case
/// whatever language the message is in — the context menu's search asks for
/// a password when it sees one.
#[derive(Debug)]
pub struct PasswordError {
    message: String,
}

impl PasswordError {
    pub fn missing() -> Self {
        Self::with_message(t!("core-password-missing"))
    }

    pub fn wrong() -> Self {
        Self::with_message(t!("core-password-wrong"))
    }

    /// For a format that can only say "this may be a password problem".
    pub fn with_message(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for PasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PasswordError {}

/// Whether an error is, somewhere down its chain, about a password.
pub fn needs_password(error: &anyhow::Error) -> bool {
    // `downcast_ref` also finds it when it was attached as context, and through
    // any layers of context added on top; the chain covers a password error
    // wrapped inside a foreign error type.
    error.downcast_ref::<PasswordError>().is_some()
        || error
            .chain()
            .any(|e| e.downcast_ref::<PasswordError>().is_some())
}

#[derive(Clone)]
pub struct Secret(Zeroizing<Vec<u8>>);

impl Secret {
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(Zeroizing::new(value.into()))
    }

    pub fn from_str_secret(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }

    /// The password's bytes.
    ///
    /// Both WinZip AES and 7z treat the password as UTF-8/UTF-16 of the
    /// original string, so the bytes of the typed string are what is kept
    /// and handed out.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// As a string — only for handing to a foreign API that takes `&str`
    /// (7z, for one).
    pub fn as_str(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }
}

/// A password must never surface in debug output or in an error text.
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<hidden>)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_exact_bytes() {
        // Non-ASCII on purpose: the bytes must be the UTF-8 of the string.
        let s = Secret::from_str_secret("пароль");
        assert_eq!(s.bytes(), "пароль".as_bytes());
        assert_eq!(s.as_str(), "пароль");
    }

    #[test]
    fn empty_is_detected() {
        assert!(Secret::new(Vec::new()).is_empty());
        assert!(!Secret::from_str_secret("x").is_empty());
    }

    #[test]
    fn password_errors_are_recognised_under_context() {
        let root: anyhow::Error = PasswordError::wrong().into();
        assert!(needs_password(
            &root.context("reading entry 3").context("grep")
        ));
        let attached = anyhow::anyhow!("crate failure").context(PasswordError::missing());
        assert!(needs_password(&attached.context("opening")));
        assert!(!needs_password(&anyhow::anyhow!("disk full")));
    }

    #[test]
    fn debug_output_does_not_leak_password() {
        let s = Secret::from_str_secret("top-secret");
        let printed = format!("{s:?}");
        assert!(
            !printed.contains("top-secret"),
            "password leaked into Debug: {printed}"
        );
        assert!(printed.contains("hidden"));
    }
}
