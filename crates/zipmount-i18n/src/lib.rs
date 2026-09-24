//! Localized user-facing text.
//!
//! Every string a user reads — command output, errors, help, context menu
//! items — goes through [`t!`] and lives in `locales/<language>.ftl`, in
//! [Fluent](https://projectfluent.org/) syntax. Fluent rather than a plain
//! key-value table because of plurals: Russian needs three forms ("1 файл",
//! "2 файла", "5 файлов") and picks them by CLDR rules, which Fluent applies
//! and simpler formats do not.
//!
//! English is the default and the reference: a message missing from another
//! language falls back to it, and the tests insist that every language carries
//! exactly the same set of messages as English.
//!
//! The language is decided in this order ([`Source`]):
//! 1. `ZIPMOUNT_LANG` — for one command or one session;
//! 2. the choice saved by `zipmount language <code>`, in
//!    `HKCU\Software\ZipMount\Language`;
//! 3. the user's Windows display languages, in their order of preference;
//! 4. English.
//!
//! Developer-facing text — panics, broken invariants, test messages — stays
//! plain English and is not translated: nobody but a developer sees it, and
//! a translated invariant message is harder to search for.

use std::sync::{Arc, RwLock};

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::FluentResource;
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::{FluentArgs, FluentValue};

/// A language the program speaks.
pub struct Language {
    /// BCP 47 tag, as accepted by `ZIPMOUNT_LANG` and `zipmount language`.
    pub code: &'static str,
    /// The language's name in itself, for listings.
    pub native_name: &'static str,
    source: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        native_name: "English",
        source: include_str!("../locales/en.ftl"),
    },
    Language {
        code: "ru",
        native_name: "Русский",
        source: include_str!("../locales/ru.ftl"),
    },
    Language {
        code: "zh-CN",
        native_name: "简体中文",
        source: include_str!("../locales/zh-CN.ftl"),
    },
    Language {
        code: "ja",
        native_name: "日本語",
        source: include_str!("../locales/ja.ftl"),
    },
    Language {
        code: "ko",
        native_name: "한국어",
        source: include_str!("../locales/ko.ftl"),
    },
    Language {
        code: "pt-BR",
        native_name: "Português (Brasil)",
        source: include_str!("../locales/pt-BR.ftl"),
    },
    Language {
        code: "es",
        native_name: "Español",
        source: include_str!("../locales/es.ftl"),
    },
    Language {
        code: "de",
        native_name: "Deutsch",
        source: include_str!("../locales/de.ftl"),
    },
];

/// The environment variable that overrides everything else.
pub const LANG_VAR: &str = "ZIPMOUNT_LANG";

/// Where the saved choice lives, under `HKEY_CURRENT_USER`.
pub const SETTINGS_KEY: &str = "Software\\ZipMount";
const LANGUAGE_VALUE: &str = "Language";

/// What decided the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `ZIPMOUNT_LANG`.
    Environment,
    /// `zipmount language <code>`.
    Saved,
    /// The Windows display language.
    Windows,
    /// Nothing matched; English.
    Default,
}

type Bundle = FluentBundle<FluentResource>;

struct State {
    language: &'static Language,
    source: Source,
    bundle: Bundle,
    /// English, when the chosen language is something else.
    fallback: Option<Bundle>,
}

/// Behind a lock rather than a `OnceLock`: the context menu handler lives in
/// File Explorer for hours, and must pick up a language changed meanwhile.
static STATE: RwLock<Option<Arc<State>>> = RwLock::new(None);

/// Formats a message by id, with optional arguments.
///
/// Prefer [`t!`]; this is for ids that are chosen at run time, such as a
/// table of menu items.
pub fn message(id: &str, args: Option<&FluentArgs>) -> String {
    let state = state();
    format(&state.bundle, id, args)
        .or_else(|| state.fallback.as_ref().and_then(|b| format(b, id, args)))
        // A missing id is a bug the tests catch; showing the id beats
        // showing nothing.
        .unwrap_or_else(|| id.to_string())
}

/// A number with a fixed count of decimals and this language's decimal
/// separator: "1.5" in English, "1,5" in Russian or German.
pub fn decimal(value: f64, places: usize) -> String {
    let text = format!("{value:.places$}");
    let separator = message("decimal-separator", None);
    if separator == "." {
        text
    } else {
        text.replacen('.', &separator, 1)
    }
}

/// The language this process speaks.
pub fn language() -> &'static Language {
    state().language
}

/// What decided the language this process speaks.
pub fn source() -> Source {
    state().source
}

/// Decides the language again and switches to it if it changed. Returns
/// whether it did.
///
/// A command-line process has no use for this: it lives for one command.
/// The context menu handler calls it before building its items.
pub fn refresh() -> bool {
    let (language, source) = choose();
    let current = state();
    if current.language.code == language.code && current.source == source {
        return false;
    }
    install(language, source);
    true
}

/// Switches this process to a language right away, whatever decides it
/// otherwise — so that `zipmount language de` can confirm in German.
pub fn switch_to(language: &'static Language, source: Source) {
    install(language, source);
}

/// Formats a message in a given language, leaving this process's own
/// language alone.
///
/// For text written somewhere other than this console — context menu items in
/// the registry are read by File Explorer, which speaks the language decided
/// by [`decide_without_environment`].
pub fn message_in(id: &str, language: &Language, args: Option<&FluentArgs>) -> String {
    format(&bundle_for(language), id, args)
        .or_else(|| format(&bundle_for(&LANGUAGES[0]), id, args))
        .unwrap_or_else(|| id.to_string())
}

/// The language as decided for a process that does not see `ZIPMOUNT_LANG` —
/// File Explorer, above all.
pub fn decide_without_environment() -> (&'static Language, Source) {
    if let Some(language) = saved() {
        return (language, Source::Saved);
    }
    if let Some(language) = from_windows() {
        return (language, Source::Windows);
    }
    (&LANGUAGES[0], Source::Default)
}

/// Looks up a message by id, with named arguments:
///
/// ```ignore
/// t!("core-open-failed", path = "C:\\logs.7z")
/// ```
///
/// Arguments are anything convertible into a [`FluentValue`]: strings and
/// numbers. Numbers stay numbers, so that plural selection works on them.
#[macro_export]
macro_rules! t {
    ($id:literal) => {
        $crate::message($id, None)
    };
    ($id:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = $crate::FluentArgs::new();
        $( args.set(stringify!($name), $crate::FluentValue::from($value)); )+
        $crate::message($id, Some(&args))
    }};
}

/// Finds the supported language for a BCP 47 tag such as `ru-RU` or
/// `zh-Hans-CN`.
pub fn match_tag(tag: &str) -> Option<&'static Language> {
    let lower = tag.trim().to_ascii_lowercase().replace('_', "-");
    let mut parts = lower.split('-');
    let primary = parts.next().filter(|p| !p.is_empty())?;
    if primary == "zh" {
        // Only Simplified Chinese is translated. Traditional-script locales
        // fall through to the user's next preference rather than getting a
        // script they may read with difficulty.
        let rest: Vec<&str> = parts.collect();
        let simplified = rest.contains(&"hans");
        let traditional = rest
            .iter()
            .any(|p| matches!(*p, "hant" | "tw" | "hk" | "mo"));
        return if traditional && !simplified {
            None
        } else {
            find("zh-CN")
        };
    }
    if primary == "pt" {
        // One Portuguese for both sides of the Atlantic: the Brazilian one,
        // which a reader in Portugal follows without trouble.
        return find("pt-BR");
    }
    LANGUAGES.iter().find(|l| l.code == primary)
}

/// The language saved by `zipmount language`, if any.
pub fn saved() -> Option<&'static Language> {
    read_setting(SETTINGS_KEY, LANGUAGE_VALUE).and_then(|tag| match_tag(&tag))
}

/// Saves the language choice for this user; `None` removes it, handing the
/// decision back to Windows.
pub fn save(language: Option<&Language>) -> std::io::Result<()> {
    match language {
        Some(language) => write_setting(SETTINGS_KEY, LANGUAGE_VALUE, language.code),
        None => delete_setting(SETTINGS_KEY, LANGUAGE_VALUE),
    }
}

/// The language `ZIPMOUNT_LANG` asks for, if it is set to one we speak.
pub fn from_environment() -> Option<&'static Language> {
    std::env::var(LANG_VAR).ok().and_then(|tag| match_tag(&tag))
}

/// The language Windows asks for, if any of the user's display languages is
/// one we speak.
pub fn from_windows() -> Option<&'static Language> {
    preferred_ui_languages()
        .iter()
        .find_map(|tag| match_tag(tag))
}

fn find(code: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|l| l.code == code)
}

fn choose() -> (&'static Language, Source) {
    match from_environment() {
        Some(language) => (language, Source::Environment),
        None => decide_without_environment(),
    }
}

// No panics anywhere below: this crate is loaded into File Explorer by the
// context menu handler, and with `panic = "abort"` a panic there takes the
// whole Explorer down. A poisoned lock still holds a usable state.

fn state() -> Arc<State> {
    let current = STATE.read().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(state) = current {
        return state;
    }
    let (language, source) = choose();
    install(language, source)
}

fn install(language: &'static Language, source: Source) -> Arc<State> {
    let state = Arc::new(State {
        language,
        source,
        bundle: bundle_for(language),
        fallback: (language.code != "en").then(|| bundle_for(&LANGUAGES[0])),
    });
    *STATE.write().unwrap_or_else(|e| e.into_inner()) = Some(state.clone());
    state
}

fn bundle_for(language: &Language) -> Bundle {
    // The codes are constants and a test parses every one of them.
    let id: LanguageIdentifier = language.code.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // Fluent wraps every substituted value in Unicode isolation marks by
    // default. They keep right-to-left text in order, but a console prints
    // them as visible junk around paths and numbers, and none of our
    // languages is written right to left.
    bundle.set_use_isolating(false);
    let resource = match FluentResource::try_new(language.source.to_string()) {
        Ok(resource) => resource,
        // A broken entry must not take the whole language down; the tests
        // make sure there are none.
        Err((resource, _)) => resource,
    };
    let _ = bundle.add_resource(resource);
    bundle
}

fn format(bundle: &Bundle, id: &str, args: Option<&FluentArgs>) -> Option<String> {
    let message = bundle.get_message(id)?;
    let pattern = message.value()?;
    let mut errors = Vec::new();
    Some(
        bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned(),
    )
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteKeyValueW, RegGetValueW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The user's Windows display languages, most preferred first.
    pub fn preferred_ui_languages() -> Vec<String> {
        let mut count = 0u32;
        let mut len = 0u32;
        // SAFETY: the first call only reports the buffer size; the second
        // writes at most `len` UTF-16 units into a buffer of exactly that size.
        unsafe {
            if GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                std::ptr::null_mut(),
                &mut len,
            ) == 0
                || len == 0
            {
                return Vec::new();
            }
            let mut buffer = vec![0u16; len as usize];
            if GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                buffer.as_mut_ptr(),
                &mut len,
            ) == 0
            {
                return Vec::new();
            }
            // A double-NUL-terminated list: "ru-RU\0en-US\0\0".
            buffer
                .split(|&c| c == 0)
                .filter(|s| !s.is_empty())
                .map(String::from_utf16_lossy)
                .collect()
        }
    }

    pub fn read_setting(key: &str, value: &str) -> Option<String> {
        let (key, value) = (wide(key), wide(value));
        let mut buffer = [0u16; 64];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        // SAFETY: the buffer and its size in bytes describe the same memory;
        // RRF_RT_REG_SZ guarantees a NUL-terminated string on success.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
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

    pub fn write_setting(key: &str, value: &str, data: &str) -> std::io::Result<()> {
        let (key, value, data) = (wide(key), wide(value), wide(data));
        let mut hkey: HKEY = std::ptr::null_mut();
        // SAFETY: plain registry calls with NUL-terminated wide strings; the
        // key is closed on every path after it is opened.
        unsafe {
            let status = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            );
            if status != ERROR_SUCCESS {
                return Err(std::io::Error::from_raw_os_error(status as i32));
            }
            let status = RegSetValueExW(
                hkey,
                value.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            );
            RegCloseKey(hkey);
            if status != ERROR_SUCCESS {
                return Err(std::io::Error::from_raw_os_error(status as i32));
            }
        }
        Ok(())
    }

    pub fn delete_setting(key: &str, value: &str) -> std::io::Result<()> {
        let (key, value) = (wide(key), wide(value));
        // SAFETY: NUL-terminated wide strings.
        let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr()) };
        match status {
            // Nothing saved is exactly the state asked for.
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            other => Err(std::io::Error::from_raw_os_error(other as i32)),
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn preferred_ui_languages() -> Vec<String> {
        std::env::var("LANG").into_iter().collect()
    }
    pub fn read_setting(_: &str, _: &str) -> Option<String> {
        None
    }
    pub fn write_setting(_: &str, _: &str, _: &str) -> std::io::Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    pub fn delete_setting(_: &str, _: &str) -> std::io::Result<()> {
        Ok(())
    }
}

use platform::{delete_setting, preferred_ui_languages, read_setting, write_setting};

#[cfg(test)]
mod tests;
