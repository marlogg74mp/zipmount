//! Turning names from an archive into something Windows can digest.
//!
//! Zip allows names Win32 simply cannot open: with `*?<>|:"` in them, with a
//! trailing dot or space, or matching device names (`CON`, `COM1`). Our
//! filesystem itself would serve them, but File Explorer and every other
//! program trip over parsing the path before they even reach us — so they
//! are fixed here.

/// DOS device names. Reserved with any extension too: to Win32, `CON.txt`
/// is still the CON device.
const RESERVED_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

const REPLACEMENT: char = '_';

/// Makes one path component safe for Windows.
pub fn sanitize_component(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let safe = match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => REPLACEMENT,
            c if (c as u32) < 0x20 => REPLACEMENT,
            c => c,
        };
        out.push(safe);
    }

    // Win32 silently strips trailing dots and spaces, leaving the file
    // unreachable by its own name. Strip them ourselves so the name stays
    // honest.
    let trimmed = out.trim_end_matches([' ', '.']);
    let mut result = if trimmed.is_empty() {
        REPLACEMENT.to_string()
    } else {
        trimmed.to_string()
    };

    if is_reserved(&result) {
        result.insert(0, REPLACEMENT);
    }

    result
}

fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    RESERVED_STEMS
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// Separates colliding names with a suffix before the extension:
/// `log.txt` -> `log~1.txt`.
pub fn disambiguate(name: &str, n: usize) -> String {
    match name.rfind('.') {
        // A leading dot is not an extension but a hidden file like
        // `.gitignore`.
        Some(pos) if pos > 0 => format!("{}~{}{}", &name[..pos], n, &name[pos..]),
        _ => format!("{name}~{n}"),
    }
}

/// Splits a path from the archive into components, defusing `..` on the way.
///
/// An entry like `../../windows/system32/evil.dll` is the classic zip-slip.
/// It cannot make us write to disk (the volume is read-only), but it could
/// lead a node outside the volume root, so such components are just dropped.
pub fn split_path(path: &str) -> Vec<String> {
    path.split(['/', '\\'])
        .filter(|c| !c.is_empty() && *c != "." && *c != "..")
        .map(sanitize_component)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_characters_windows_rejects() {
        assert_eq!(sanitize_component("a:b*c?d"), "a_b_c_d");
        assert_eq!(sanitize_component("quote\"here"), "quote_here");
    }

    #[test]
    fn strips_trailing_dots_and_spaces() {
        assert_eq!(sanitize_component("report."), "report");
        assert_eq!(sanitize_component("report   "), "report");
        assert_eq!(sanitize_component("report. . "), "report");
    }

    #[test]
    fn escapes_device_names_with_and_without_extension() {
        assert_eq!(sanitize_component("CON"), "_CON");
        assert_eq!(sanitize_component("con.txt"), "_con.txt");
        assert_eq!(sanitize_component("COM1.log"), "_COM1.log");
        // Not a device: similar, but no match.
        assert_eq!(sanitize_component("CONSOLE.txt"), "CONSOLE.txt");
        assert_eq!(sanitize_component("COM10"), "COM10");
    }

    #[test]
    fn keeps_cyrillic_and_spaces_inside_name() {
        assert_eq!(sanitize_component("отчёт за год.txt"), "отчёт за год.txt");
    }

    #[test]
    fn empty_after_cleanup_becomes_placeholder() {
        assert_eq!(sanitize_component("..."), "_");
        assert_eq!(sanitize_component("   "), "_");
    }

    #[test]
    fn disambiguate_inserts_before_extension() {
        assert_eq!(disambiguate("log.txt", 1), "log~1.txt");
        assert_eq!(disambiguate("noext", 2), "noext~2");
        assert_eq!(disambiguate(".gitignore", 1), ".gitignore~1");
        assert_eq!(disambiguate("a.tar.gz", 1), "a.tar~1.gz");
    }

    #[test]
    fn split_path_drops_traversal_and_empty_segments() {
        assert_eq!(split_path("a//b/./c.txt"), vec!["a", "b", "c.txt"]);
        assert_eq!(
            split_path("../../windows/system32/evil.dll"),
            vec!["windows", "system32", "evil.dll"]
        );
        assert_eq!(
            split_path("mixed\\sep/here.txt"),
            vec!["mixed", "sep", "here.txt"]
        );
    }

    #[test]
    fn split_path_of_directory_entry_ignores_trailing_slash() {
        assert_eq!(split_path("docs/sub/"), vec!["docs", "sub"]);
    }
}
