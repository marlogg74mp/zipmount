//! The translations are data the compiler does not check, so these tests do:
//! every file parses, every language says the same set of things, no
//! translation invents a variable English does not pass, and every id the
//! code asks for exists.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use fluent_syntax::ast;

use super::*;

/// Message id → the variables its pattern refers to.
fn messages(language: &Language) -> BTreeMap<String, BTreeSet<String>> {
    let resource = match fluent_syntax::parser::parse(language.source) {
        Ok(resource) => resource,
        Err((_, errors)) => panic!("{}.ftl does not parse: {errors:?}", language.code),
    };
    let mut out = BTreeMap::new();
    for entry in resource.body {
        if let ast::Entry::Message(message) = entry {
            let mut vars = BTreeSet::new();
            if let Some(pattern) = &message.value {
                collect_pattern(pattern, &mut vars);
            }
            let id = message.id.name.to_string();
            assert!(
                out.insert(id.clone(), vars).is_none(),
                "{}.ftl defines {id} twice",
                language.code
            );
        }
    }
    out
}

fn collect_pattern(pattern: &ast::Pattern<&str>, vars: &mut BTreeSet<String>) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            collect_expression(expression, vars);
        }
    }
}

fn collect_expression(expression: &ast::Expression<&str>, vars: &mut BTreeSet<String>) {
    match expression {
        ast::Expression::Inline(inline) => collect_inline(inline, vars),
        ast::Expression::Select { selector, variants } => {
            collect_inline(selector, vars);
            for variant in variants {
                collect_pattern(&variant.value, vars);
            }
        }
    }
}

fn collect_inline(inline: &ast::InlineExpression<&str>, vars: &mut BTreeSet<String>) {
    match inline {
        ast::InlineExpression::VariableReference { id } => {
            vars.insert(id.name.to_string());
        }
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for argument in &arguments.positional {
                collect_inline(argument, vars);
            }
            for argument in &arguments.named {
                collect_inline(&argument.value, vars);
            }
        }
        ast::InlineExpression::Placeable { expression } => collect_expression(expression, vars),
        _ => {}
    }
}

#[test]
fn every_language_parses_and_matches_english() {
    let english = messages(&LANGUAGES[0]);
    assert!(!english.is_empty());
    for language in &LANGUAGES[1..] {
        let translated = messages(language);
        let missing: Vec<_> = english
            .keys()
            .filter(|id| !translated.contains_key(*id))
            .collect();
        let extra: Vec<_> = translated
            .keys()
            .filter(|id| !english.contains_key(*id))
            .collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "{}.ftl: missing {missing:?}, not in English {extra:?}",
            language.code
        );
        for (id, vars) in &translated {
            let unknown: Vec<_> = vars.difference(&english[id]).collect();
            assert!(
                unknown.is_empty(),
                "{}.ftl: {id} uses {unknown:?}, which English does not pass",
                language.code
            );
        }
    }
}

fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).expect("readable source directory") {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).expect("UTF-8 source file");
                out.push((path.display().to_string(), text));
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory");
    let mut out = Vec::new();
    walk(crates, &mut out);
    out
}

/// Every `t!("id"`, `message("id"` and `message_in("id"` in the workspace,
/// including calls that put the id on the next line.
fn requested_ids() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (file, text) in sources() {
        // This file quotes the openers in its own comments and strings.
        if file.ends_with(file!().rsplit(['/', '\\']).next().unwrap_or_default())
            && file.contains("zipmount-i18n")
        {
            continue;
        }
        for opener in ["t!(", "message(", "message_in("] {
            let mut rest = text.as_str();
            while let Some(at) = rest.find(opener) {
                // `assert!(` ends in `t!(` too; a real call has no identifier
                // character right before it.
                let glued = rest[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
                rest = &rest[at + opener.len()..];
                if glued {
                    continue;
                }
                let Some(literal) = rest.trim_start().strip_prefix('"') else {
                    continue;
                };
                let Some(end) = literal.find('"') else {
                    continue;
                };
                let id = &literal[..end];
                // Message ids are kebab-case; anything else is some other
                // string that happens to follow an opener.
                let kebab = !id.is_empty()
                    && id
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if kebab {
                    out.push((file.clone(), id.to_string()));
                }
            }
        }
    }
    out
}

#[test]
fn every_requested_id_exists_in_english() {
    let english = messages(&LANGUAGES[0]);
    let missing: Vec<_> = requested_ids()
        .into_iter()
        .filter(|(_, id)| !english.contains_key(id))
        .collect();
    assert!(
        missing.is_empty(),
        "ids asked for but missing from en.ftl: {missing:?}"
    );
}

#[test]
fn every_english_message_is_used() {
    // A message is used if its id appears in the sources as a string
    // literal — through t!, or in a table of ids chosen at run time.
    let all: String = sources().into_iter().map(|(_, text)| text).collect();
    let unused: Vec<_> = messages(&LANGUAGES[0])
        .into_keys()
        .filter(|id| !all.contains(&format!("\"{id}\"")))
        .collect();
    assert!(unused.is_empty(), "messages nobody asks for: {unused:?}");
}

#[test]
fn every_language_code_is_a_valid_tag() {
    for language in LANGUAGES {
        assert!(
            language.code.parse::<LanguageIdentifier>().is_ok(),
            "{} is not a BCP 47 tag",
            language.code
        );
        assert_eq!(
            match_tag(language.code).map(|l| l.code),
            Some(language.code)
        );
    }
}

#[test]
fn matches_windows_language_tags() {
    let code = |tag: &str| match_tag(tag).map(|l| l.code);
    assert_eq!(code("ru-RU"), Some("ru"));
    assert_eq!(code("en-GB"), Some("en"));
    assert_eq!(code("de-AT"), Some("de"));
    assert_eq!(code("es-419"), Some("es"));
    assert_eq!(code("zh-CN"), Some("zh-CN"));
    assert_eq!(code("zh-Hans-HK"), Some("zh-CN"));
    assert_eq!(code("zh-TW"), None);
    assert_eq!(code("zh-Hant"), None);
    assert_eq!(code("ja-JP"), Some("ja"));
    assert_eq!(code("ko-KR"), Some("ko"));
    assert_eq!(code("pt-BR"), Some("pt-BR"));
    assert_eq!(code("pt-PT"), Some("pt-BR"));
    assert_eq!(code("fr-FR"), None);
    assert_eq!(code("he-IL"), None);
    assert_eq!(code(""), None);
    assert_eq!(code("RU"), Some("ru"));
}

#[cfg(windows)]
#[test]
fn setting_round_trips_through_the_registry() {
    // A key of its own, so the test never touches the real choice.
    let key = format!("Software\\ZipMount-test-{}", std::process::id());
    assert_eq!(read_setting(&key, "Language"), None);
    write_setting(&key, "Language", "zh-CN").expect("write");
    assert_eq!(read_setting(&key, "Language").as_deref(), Some("zh-CN"));
    delete_setting(&key, "Language").expect("delete");
    assert_eq!(read_setting(&key, "Language"), None);
    // Deleting what is not there is not an error: "auto" twice in a row.
    delete_setting(&key, "Language").expect("second delete");
    let wide: Vec<u16> = key.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: NUL-terminated key name under HKCU.
    unsafe {
        windows_sys::Win32::System::Registry::RegDeleteKeyW(
            windows_sys::Win32::System::Registry::HKEY_CURRENT_USER,
            wide.as_ptr(),
        );
    }
}

#[cfg(not(windows))]
#[test]
fn setting_round_trips_through_a_file() {
    // A directory of its own, so the test never touches the real choice.
    let name = format!("ZipMount-test-{}", std::process::id());
    let key = format!("Software\\{name}");
    assert_eq!(read_setting(&key, "Language"), None);
    write_setting(&key, "Language", "zh-CN").expect("write");
    assert_eq!(read_setting(&key, "Language").as_deref(), Some("zh-CN"));
    delete_setting(&key, "Language").expect("delete");
    assert_eq!(read_setting(&key, "Language"), None);
    // Deleting what is not there is not an error: "auto" twice in a row.
    delete_setting(&key, "Language").expect("second delete");

    let home = std::path::PathBuf::from(std::env::var_os("HOME").expect("HOME"));
    let dir = if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(&name)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"))
            .join(name.to_lowercase())
    };
    std::fs::remove_dir(&dir).expect("the test directory, now empty");
}

#[test]
fn switching_changes_the_language_at_once() {
    let before = language();
    let other = LANGUAGES
        .iter()
        .find(|l| l.code != before.code)
        .expect("a second language");
    switch_to(other, Source::Saved);
    assert_eq!(language().code, other.code);
    assert_eq!(source(), Source::Saved);
    switch_to(before, Source::Default);
}

#[test]
fn substituted_values_carry_no_isolation_marks() {
    let mut args = FluentArgs::new();
    args.set("path", FluentValue::from("C:\\a.zip"));
    for language in LANGUAGES {
        let bundle = bundle_for(language);
        // Any message with a variable will do; the first one found is used.
        let text = messages(language)
            .into_iter()
            .find(|(_, vars)| vars.contains("path"))
            .and_then(|(id, _)| format(&bundle, &id, Some(&args)));
        if let Some(text) = text {
            assert!(
                !text.contains('\u{2068}') && !text.contains('\u{2069}'),
                "{}: {text:?}",
                language.code
            );
        }
    }
}
