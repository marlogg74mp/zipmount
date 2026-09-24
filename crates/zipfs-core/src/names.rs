//! Decoding entry names.
//!
//! By the specification a name is either UTF-8 (bit 11 of the general
//! purpose flag) or CP437. In practice Russian archivers write CP866 or
//! CP1251 with no flag at all, so a naive "decode as UTF-8" turns the tree
//! into mojibake. Order of trust: the Unicode Path extra field, then the
//! UTF-8 flag, then a heuristic.

use std::str::FromStr;

use encoding_rs::{IBM866, WINDOWS_1251};
use zipmount_i18n::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NameEncoding {
    #[default]
    Auto,
    Utf8,
    Cp866,
    Cp1251,
}

impl FromStr for NameEncoding {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
            "auto" => Ok(Self::Auto),
            "utf8" | "utf 8" => Ok(Self::Utf8),
            "cp866" | "ibm866" | "866" | "oem" => Ok(Self::Cp866),
            "cp1251" | "windows1251" | "win1251" | "1251" | "ansi" => Ok(Self::Cp1251),
            other => Err(t!("core-unknown-encoding", value = other.to_string())),
        }
    }
}

/// Decodes a raw entry name into a string.
///
/// `unicode_path` is the name from extra field 0x7075, already checked
/// against its CRC: if there is one, there is nothing to argue about — the
/// archiver has stated the right answer itself.
pub fn decode_name(
    raw: &[u8],
    unicode_path: Option<&str>,
    utf8_flag: bool,
    encoding: NameEncoding,
) -> String {
    if let Some(name) = unicode_path {
        return name.to_string();
    }

    match encoding {
        NameEncoding::Utf8 => String::from_utf8_lossy(raw).into_owned(),
        NameEncoding::Cp866 => decode_with(IBM866, raw),
        NameEncoding::Cp1251 => decode_with(WINDOWS_1251, raw),
        NameEncoding::Auto => decode_auto(raw, utf8_flag),
    }
}

fn decode_with(enc: &'static encoding_rs::Encoding, raw: &[u8]) -> String {
    let (cow, _, _) = enc.decode(raw);
    cow.into_owned()
}

fn decode_auto(raw: &[u8], utf8_flag: bool) -> String {
    // Pure ASCII decodes the same in every variant — nothing to guess.
    if raw.is_ascii() {
        return String::from_utf8_lossy(raw).into_owned();
    }

    if utf8_flag {
        if let Ok(s) = std::str::from_utf8(raw) {
            return s.to_string();
        }
        // The flag lied: carry on with the heuristic rather than hand out
        // garbage full of U+FFFD.
    }

    // Non-ASCII bytes that happen to form valid UTF-8 are almost certainly
    // UTF-8: random CP866 text passes UTF-8 validation with vanishingly
    // small probability.
    let mut best: Option<(i32, String)> = None;
    if let Ok(s) = std::str::from_utf8(raw) {
        best = Some((score(s) + UTF8_VALIDITY_BONUS, s.to_string()));
    }

    for candidate in [decode_with(IBM866, raw), decode_with(WINDOWS_1251, raw)] {
        let sc = score(&candidate);
        if best.as_ref().is_none_or(|(best_score, _)| sc > *best_score) {
            best = Some((sc, candidate));
        }
    }

    best.map(|(_, s)| s)
        .unwrap_or_else(|| String::from_utf8_lossy(raw).into_owned())
}

/// The head start valid UTF-8 gets simply for being valid.
const UTF8_VALIDITY_BONUS: i32 = 40;

/// How plausible a decoded name looks.
///
/// The idea: with the wrong encoding, Cyrillic turns into box drawing and
/// rare symbols; with the right one it stays letters. That is counted in
/// points.
fn score(s: &str) -> i32 {
    let mut total = 0i32;
    for ch in s.chars() {
        total += match ch {
            // Cyrillic is a strong sign of the right choice.
            '\u{0410}'..='\u{044F}' | '\u{0401}' | '\u{0451}' => 3,
            // Characters usual in file names.
            'a'..='z' | 'A'..='Z' | '0'..='9' => 1,
            '.' | '_' | '-' | '/' | ' ' | '(' | ')' | '[' | ']' | '+' | ',' | '&' | '\'' => 1,
            // Box drawing: a sure sign the encoding was chosen wrong.
            '\u{2500}'..='\u{257F}' | '\u{2580}'..='\u{259F}' => -5,
            // Control characters and outright decoding errors.
            '\u{FFFD}' => -10,
            c if c.is_control() => -10,
            // Other exotica: not a verdict, but suspicious.
            c if !c.is_alphanumeric() => -2,
            _ => 0,
        };
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Документы/отчёт.txt" ("Documents/report.txt") in CP866.
    fn cp866_sample() -> Vec<u8> {
        let (bytes, _, _) = IBM866.encode("Документы/отчёт.txt");
        bytes.into_owned()
    }

    /// The same in CP1251.
    fn cp1251_sample() -> Vec<u8> {
        let (bytes, _, _) = WINDOWS_1251.encode("Документы/отчёт.txt");
        bytes.into_owned()
    }

    #[test]
    fn ascii_is_encoding_independent() {
        let raw = b"docs/report.txt";
        for enc in [
            NameEncoding::Auto,
            NameEncoding::Utf8,
            NameEncoding::Cp866,
            NameEncoding::Cp1251,
        ] {
            assert_eq!(decode_name(raw, None, false, enc), "docs/report.txt");
        }
    }

    #[test]
    fn auto_detects_cp866() {
        let raw = cp866_sample();
        assert_eq!(
            decode_name(&raw, None, false, NameEncoding::Auto),
            "Документы/отчёт.txt"
        );
    }

    #[test]
    fn auto_detects_cp1251() {
        let raw = cp1251_sample();
        assert_eq!(
            decode_name(&raw, None, false, NameEncoding::Auto),
            "Документы/отчёт.txt"
        );
    }

    #[test]
    fn auto_detects_utf8_even_without_flag() {
        // Archivers that write UTF-8 and forget to set bit 11 do exist.
        let raw = "Документы/отчёт.txt".as_bytes();
        assert_eq!(
            decode_name(raw, None, false, NameEncoding::Auto),
            "Документы/отчёт.txt"
        );
    }

    #[test]
    fn unicode_path_extra_field_wins() {
        // The raw name is CP866, but the extra field already holds the
        // checked answer.
        let raw = cp866_sample();
        assert_eq!(
            decode_name(&raw, Some("правильное.txt"), false, NameEncoding::Auto),
            "правильное.txt"
        );
    }

    #[test]
    fn explicit_encoding_overrides_heuristic() {
        let raw = cp866_sample();
        // Forcing CP1251 gives predictable gibberish rather than CP866.
        let decoded = decode_name(&raw, None, false, NameEncoding::Cp1251);
        assert_ne!(decoded, "Документы/отчёт.txt");
    }

    #[test]
    fn lying_utf8_flag_falls_back_to_heuristic() {
        let raw = cp866_sample();
        assert_eq!(
            decode_name(&raw, None, true, NameEncoding::Auto),
            "Документы/отчёт.txt"
        );
    }

    #[test]
    fn score_prefers_cyrillic_over_pseudographics() {
        assert!(score("отчёт") > score("®в票"));
        assert!(score("Документы") > score("─│┌┐└┘"));
    }

    #[test]
    fn encoding_parses_from_common_spellings() {
        assert_eq!(
            "CP-866".parse::<NameEncoding>().unwrap(),
            NameEncoding::Cp866
        );
        assert_eq!(
            "windows_1251".parse::<NameEncoding>().unwrap(),
            NameEncoding::Cp1251
        );
        assert_eq!("UTF8".parse::<NameEncoding>().unwrap(), NameEncoding::Utf8);
        assert!("koi8r".parse::<NameEncoding>().is_err());
    }
}
