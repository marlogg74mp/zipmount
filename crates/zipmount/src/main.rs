//! ZipMount — an archive as a Windows drive.
//!
//! Browsing and search commands need neither WinFsp nor administrator
//! rights: they work with the archive directly. Mounting is a separate
//! command on top.

mod modern;
mod mounts;
mod shell;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use zipfs_core::{
    find_by_name, grep, Archive, GrepOptions, NameEncoding, OpenOptions, Secret, ROOT,
};
use zipmount_i18n::{decimal, t, Language, Source};

#[derive(Parser)]
#[command(name = "zipmount", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Archive-opening options shared by every command.
#[derive(Args, Clone)]
struct ArchiveArgs {
    #[arg(long, default_value = "auto")]
    encoding: String,

    /// The value is deliberately not accepted as an option: a password on
    /// the command line settles into shell history and is visible to other
    /// users of the system in the process list. Scripts have
    /// --password-stdin.
    #[arg(short = 'p', long)]
    password: bool,

    #[arg(long, conflicts_with = "password")]
    password_stdin: bool,

    /// Read the password from a file and delete the file at once.
    ///
    /// An internal option: this is how a parent hands the password to a
    /// background process, which can be given neither a pipe nor a visible
    /// command line.
    #[arg(long, hide = true)]
    password_file: Option<PathBuf>,
}

// Help texts come from the translations at run time (see `HELP`), so the
// variants below carry no doc comments except for hidden commands, which
// only the installer calls.
#[derive(Subcommand)]
enum Command {
    Mount {
        archive: PathBuf,
        mountpoint: Option<String>,
        #[arg(long)]
        detach: bool,
        #[arg(long)]
        open: bool,
        #[arg(long)]
        label: Option<String>,
        /// For 7z this is the solid block cache budget, and it matters more:
        /// a block can run to gigabytes and is expanded whole.
        #[arg(long, default_value_t = 512)]
        cache_mb: usize,
        #[command(flatten)]
        common: ArchiveArgs,
    },
    Ls {
        archive: PathBuf,
        path: Option<String>,
        #[arg(short, long)]
        recursive: bool,
        #[command(flatten)]
        common: ArchiveArgs,
    },
    Find {
        archive: PathBuf,
        pattern: String,
        #[arg(long)]
        prefix: Option<String>,
        #[command(flatten)]
        common: ArchiveArgs,
    },
    Grep {
        archive: PathBuf,
        pattern: String,
        /// By default the pattern is matched literally. That guards against
        /// a silent change of meaning: [ERROR] as a regex means "any of the
        /// letters E, R, O" and matches nearly every log.
        #[arg(long)]
        regex: bool,
        #[arg(short = 'l', long)]
        files_only: bool,
        #[arg(short = 'c', long, conflicts_with = "files_only")]
        count: bool,
        #[arg(short = 'i', long)]
        ignore_case: bool,
        #[arg(short = 'A', long, default_value_t = 0)]
        after_context: usize,
        #[arg(short = 'B', long, default_value_t = 0)]
        before_context: usize,
        #[arg(short = 'C', long)]
        context: Option<usize>,
        #[arg(long)]
        max_count: Option<usize>,
        #[arg(long)]
        max_total: Option<usize>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        glob: Option<String>,
        #[arg(long)]
        copy_to: Option<PathBuf>,
        #[arg(long)]
        prefix: Option<String>,
        #[command(flatten)]
        common: ArchiveArgs,
    },
    Info {
        archive: PathBuf,
        #[command(flatten)]
        common: ArchiveArgs,
    },
    Verify {
        archive: PathBuf,
        #[arg(long)]
        random: bool,
        #[command(flatten)]
        common: ArchiveArgs,
    },
    Unmount {
        target: String,
    },
    Mounts,
    Search {
        archive: PathBuf,
    },
    ShellInstall {
        #[arg(long)]
        modern: bool,
    },
    ShellUninstall,
    Language {
        code: Option<String>,
    },

    /// Build and sign the menu package (needs the Windows SDK).
    ///
    /// An internal command for building the installer: it carries a ready
    /// package, because the user's machine has no SDK.
    #[command(hide = true)]
    ShellPackage {
        /// Where to put ZipMount.msix and ZipMount.cer
        #[arg(long)]
        out: PathBuf,
        /// Name of the library the manifest will refer to.
        ///
        /// Without it the name gets a fingerprint of the contents, and the
        /// library itself is put next to the package.
        #[arg(long)]
        library: Option<String>,
    },

    /// Install or remove an already built menu package.
    #[command(hide = true)]
    ShellRegister {
        /// Path to ZipMount.msix
        #[arg(long, required_unless_present = "remove")]
        package: Option<PathBuf>,
        /// Directory with zipmount.exe and the library
        #[arg(long, required_unless_present = "remove")]
        external: Option<PathBuf>,
        /// Remove instead of installing
        #[arg(long)]
        remove: bool,
    },

    /// Add the certificate to the trusted ones, or remove it (needs
    /// administrator rights).
    #[command(hide = true)]
    ShellTrust {
        /// Path to ZipMount.cer
        #[arg(long, required_unless_present = "remove")]
        certificate: Option<PathBuf>,
        /// Remove instead of adding
        #[arg(long)]
        remove: bool,
    },
    Doctor,
}

/// Help texts: subcommand, argument id (empty for the subcommand itself), and
/// the message. `*` stands for an argument shared by every command.
///
/// Clap reads help from doc comments at compile time, in one language; this
/// table is how it gets the language chosen at run time. A test makes sure
/// every visible command and argument is listed.
const HELP: &[(&str, &str, &str)] = &[
    ("*", "encoding", "help-encoding"),
    ("*", "password", "help-password"),
    ("*", "password_stdin", "help-password-stdin"),
    ("*", "archive", "help-archive"),
    ("*", "prefix", "help-prefix"),
    ("mount", "", "help-mount"),
    ("mount", "mountpoint", "help-mount-mountpoint"),
    ("mount", "detach", "help-mount-detach"),
    ("mount", "open", "help-mount-open"),
    ("mount", "label", "help-mount-label"),
    ("mount", "cache_mb", "help-mount-cache-mb"),
    ("ls", "", "help-ls"),
    ("ls", "path", "help-ls-path"),
    ("ls", "recursive", "help-ls-recursive"),
    ("find", "", "help-find"),
    ("find", "pattern", "help-find-pattern"),
    ("grep", "", "help-grep"),
    ("grep", "pattern", "help-grep-pattern"),
    ("grep", "regex", "help-grep-regex"),
    ("grep", "files_only", "help-grep-files-only"),
    ("grep", "count", "help-grep-count"),
    ("grep", "ignore_case", "help-grep-ignore-case"),
    ("grep", "after_context", "help-grep-after-context"),
    ("grep", "before_context", "help-grep-before-context"),
    ("grep", "context", "help-grep-context"),
    ("grep", "max_count", "help-grep-max-count"),
    ("grep", "max_total", "help-grep-max-total"),
    ("grep", "path", "help-grep-path"),
    ("grep", "glob", "help-grep-glob"),
    ("grep", "copy_to", "help-grep-copy-to"),
    ("info", "", "help-info"),
    ("verify", "", "help-verify"),
    ("verify", "random", "help-verify-random"),
    ("unmount", "", "help-unmount"),
    ("unmount", "target", "help-unmount-target"),
    ("mounts", "", "help-mounts"),
    ("search", "", "help-search"),
    ("shell-install", "", "help-shell-install"),
    ("shell-install", "modern", "help-shell-install-modern"),
    ("shell-uninstall", "", "help-shell-uninstall"),
    ("language", "", "help-language"),
    ("language", "code", "help-language-code"),
    ("doctor", "", "help-doctor"),
];

fn help_for(command: &str, arg: &str) -> Option<String> {
    HELP.iter()
        .find(|(c, a, _)| *c == command && *a == arg)
        .or_else(|| HELP.iter().find(|(c, a, _)| *c == "*" && *a == arg))
        .map(|(_, _, id)| zipmount_i18n::message(id, None))
}

/// The same headings and built-in flags on every level, in the chosen
/// language. Clap's own error messages stay in English: it has no way to
/// translate them.
fn localize_level(cmd: clap::Command, name: &str) -> clap::Command {
    let template = format!(
        "{{about-with-newline}}\n{} {{usage}}\n\n{{all-args}}{{after-help}}",
        t!("help-heading-usage")
    );
    let mut cmd = cmd
        .help_template(template)
        .subcommand_help_heading(t!("help-heading-commands"))
        .disable_help_flag(true)
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .help(t!("help-flag-help")),
        );
    let (arguments, options) = (t!("help-heading-arguments"), t!("help-heading-options"));
    cmd = cmd.mut_args(|arg| {
        let heading = if arg.is_positional() {
            &arguments
        } else {
            &options
        };
        let help = help_for(name, arg.get_id().as_str());
        let arg = arg.help_heading(heading.clone());
        match help {
            Some(help) => arg.help(help),
            None => arg,
        }
    });
    if let Some(about) = help_for(name, "") {
        cmd = cmd.about(about);
    }
    cmd
}

fn localized_command() -> clap::Command {
    let mut cmd = localize_level(Cli::command(), "")
        .about(t!("help-about"))
        .after_help(t!("help-notice"))
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help(t!("help-flag-version"))
                .help_heading(t!("help-heading-options")),
        );
    let names: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for name in names {
        cmd = cmd.mut_subcommand(&name, |sub| localize_level(sub, &name));
    }
    cmd
}

fn main() {
    // Printed here rather than by returning the error from main: Rust would
    // prefix it with an English "Error:" and list causes under "Caused by:".
    if let Err(e) = run() {
        eprintln!("{}: {e:#}", t!("error-prefix"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let matches = localized_command().get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };
    match cli.command {
        Command::Mount {
            archive,
            mountpoint,
            detach,
            open,
            label,
            cache_mb,
            common,
        } => cmd_mount(
            &archive,
            MountArgs {
                mountpoint,
                detach,
                open,
                label,
                cache_mb,
            },
            &common,
        ),
        Command::Unmount { target } => cmd_unmount(&target),
        Command::Mounts => cmd_mounts(),
        Command::Ls {
            archive,
            path,
            recursive,
            common,
        } => cmd_ls(&archive, path.as_deref(), recursive, &common),
        Command::Find {
            archive,
            pattern,
            prefix,
            common,
        } => cmd_find(&archive, &pattern, prefix.as_deref(), &common),
        Command::Grep {
            archive,
            pattern,
            regex,
            files_only,
            count,
            ignore_case,
            after_context,
            before_context,
            context,
            max_count,
            max_total,
            path,
            glob,
            copy_to,
            prefix,
            common,
        } => cmd_grep(
            &archive,
            GrepArgs {
                pattern,
                regex,
                files_only,
                count,
                ignore_case,
                after_context: context.unwrap_or(after_context),
                before_context: context.unwrap_or(before_context),
                max_count,
                max_total,
                path,
                glob,
                copy_to,
                prefix,
            },
            &common,
        ),
        Command::Info { archive, common } => cmd_info(&archive, &common),
        Command::Verify {
            archive,
            random,
            common,
        } => cmd_verify(&archive, random, &common),
        Command::Search { archive } => cmd_search(&archive),
        Command::ShellInstall { modern } => cmd_shell_install(modern),
        Command::ShellUninstall => cmd_shell_uninstall(),
        Command::Language { code } => cmd_language(code.as_deref()),
        Command::ShellPackage { out, library } => cmd_shell_package(&out, library.as_deref()),
        Command::ShellRegister {
            package,
            external,
            remove,
        } => cmd_shell_register(package.as_deref(), external.as_deref(), remove),
        Command::ShellTrust {
            certificate,
            remove,
        } => cmd_shell_trust(certificate.as_deref(), remove),
        Command::Doctor => cmd_doctor(),
    }
}

/// Gets the password so that it ends up neither in command history nor in
/// the process list: either hidden input from the terminal, or standard
/// input.
fn read_password(args: &ArchiveArgs) -> Result<Option<Secret>> {
    if let Some(path) = &args.password_file {
        let bytes = std::fs::read(path)
            .with_context(|| t!("err-read", path = path.display().to_string()))?;
        // The file is needed exactly once.
        let _ = std::fs::remove_file(path);
        return Ok(Some(Secret::new(bytes)));
    }

    if args.password_stdin {
        // Bytes, not a string: a password need not be valid UTF-8, and the
        // Windows console may well hand it over in a single-byte code page.
        // Parsing it as UTF-8 would simply lose the password.
        let mut raw = Vec::new();
        std::io::stdin()
            .read_to_end(&mut raw)
            .with_context(|| t!("err-read-password-stdin"))?;

        // Take the first line and strip the line ending in any of its forms.
        let line_end = raw.iter().position(|&b| b == 0x0A).unwrap_or(raw.len());
        let mut line = &raw[..line_end];
        if line.last() == Some(&0x0D) {
            line = &line[..line.len() - 1];
        }
        return Ok(Some(Secret::new(line.to_vec())));
    }

    if args.password {
        return prompt_password().map(Some);
    }

    Ok(None)
}

fn prompt_password() -> Result<Secret> {
    // Fluent trims trailing spaces from messages, so the one after the
    // prompt is added here.
    let entered = rpassword::prompt_password(format!("{} ", t!("password-prompt")))
        .with_context(|| t!("err-read-password"))?;
    Ok(Secret::from_str_secret(&entered))
}

fn open_archive(path: &Path, args: &ArchiveArgs, cache_mb: Option<usize>) -> Result<Archive> {
    let encoding: NameEncoding = args.encoding.parse().map_err(anyhow::Error::msg)?;
    let mut opts = OpenOptions {
        encoding,
        password: read_password(args)?,
        ..Default::default()
    };
    if let Some(mb) = cache_mb {
        opts.block_budget = mb * 1024 * 1024;
    }
    Archive::open_with(path, opts)
}

fn cmd_ls(archive: &Path, path: Option<&str>, recursive: bool, args: &ArchiveArgs) -> Result<()> {
    let a = open_archive(archive, args, None)?;
    let tree = a.tree();
    let start = tree.resolve(path.unwrap_or("")).with_context(|| {
        t!(
            "err-path-not-found",
            path = path.unwrap_or("\\").to_string()
        )
    })?;

    if recursive {
        let mut rows: Vec<(String, bool, u64)> = Vec::new();
        tree.walk(|id, node| {
            if id != ROOT {
                rows.push((tree.path_of(id), node.is_dir, node.size));
            }
        });
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, is_dir, size) in rows {
            print_row(&path, is_dir, size);
        }
        return Ok(());
    }

    if !tree.node(start).is_dir {
        let node = tree.node(start);
        print_row(&node.name, false, node.size);
        return Ok(());
    }

    for &child in &tree.node(start).children {
        let node = tree.node(child);
        print_row(&node.name, node.is_dir, node.size);
    }
    Ok(())
}

fn print_row(name: &str, is_dir: bool, size: u64) {
    // `<DIR>` as in `dir`: a marker, not a word, so it is not translated.
    if is_dir {
        println!("{:>12}  {}\\", "<DIR>", name);
    } else {
        println!("{:>12}  {}", human(size), name);
    }
}

fn cmd_find(archive: &Path, pattern: &str, prefix: Option<&str>, args: &ArchiveArgs) -> Result<()> {
    let a = open_archive(archive, args, None)?;
    let hits = find_by_name(&a, pattern, prefix);
    for h in &hits {
        println!("{h}");
    }
    eprintln!("{}", t!("find-summary", count = hits.len()));
    Ok(())
}

struct GrepArgs {
    pattern: String,
    regex: bool,
    files_only: bool,
    count: bool,
    ignore_case: bool,
    after_context: usize,
    before_context: usize,
    max_count: Option<usize>,
    max_total: Option<usize>,
    path: Option<String>,
    glob: Option<String>,
    copy_to: Option<PathBuf>,
    prefix: Option<String>,
}

fn cmd_grep(archive: &Path, args: GrepArgs, common: &ArchiveArgs) -> Result<()> {
    let a = open_archive(archive, common, None)?;

    let opts = GrepOptions {
        pattern: args.pattern,
        literal: !args.regex,
        case_insensitive: args.ignore_case,
        glob: args.glob,
        subpath: args.path,
        files_only: args.files_only || args.copy_to.is_some(),
        count_only: args.count,
        context_before: args.before_context,
        context_after: args.after_context,
        max_per_file: args.max_count,
        max_total: args.max_total,
        path_prefix: args.prefix,
        ..Default::default()
    };

    let started = Instant::now();
    let outcome = grep(&a, &opts)?;
    let elapsed = started.elapsed();

    if opts.count_only {
        for c in &outcome.counts {
            println!("{}: {}", c.path, c.matches);
        }
    } else if opts.files_only {
        let mut seen: Vec<&str> = outcome.matches.iter().map(|m| m.path.as_str()).collect();
        seen.dedup();
        for path in &seen {
            println!("{path}");
        }
    } else {
        // As in grep: a colon after a match's line number, a hyphen after a
        // context line's.
        for m in &outcome.matches {
            let sep = if m.is_context { '-' } else { ':' };
            println!("{}:{}{} {}", m.path, m.line_number, sep, m.line);
        }
    }

    if let Some(dest) = &args.copy_to {
        let copied = extract_matches(&a, &outcome, dest)?;
        eprintln!(
            "{}",
            t!(
                "grep-copied",
                count = copied,
                path = dest.display().to_string()
            )
        );
    }

    let secs = elapsed.as_secs_f64();
    eprintln!(
        "{}",
        t!(
            "grep-summary",
            matched = outcome.files_matched,
            scanned = outcome.files_scanned,
            skipped = outcome.files_skipped,
            size = human(outcome.bytes_scanned),
            seconds = decimal(secs, 2),
            speed = throughput(outcome.bytes_scanned, secs)
        )
    );
    if outcome.truncated {
        eprintln!("{}", t!("grep-truncated"));
    }
    if !opts.literal {
        eprintln!("{}", t!("grep-regex-note"));
    }
    Ok(())
}

/// Extracts the files that had matches, keeping the directory structure.
fn extract_matches(
    archive: &Archive,
    outcome: &zipfs_core::GrepOutcome,
    dest: &Path,
) -> Result<usize> {
    let tree = archive.tree();
    let mut unique: Vec<&str> = outcome.matches.iter().map(|m| m.path.as_str()).collect();
    unique.sort();
    unique.dedup();

    let mut copied = 0usize;
    for path in unique {
        // The path may have been printed with a drive prefix — the tree does
        // not need it.
        let inner = path.split_once(':').map_or(path, |(_, rest)| rest);
        let Some(id) = tree.resolve(inner) else {
            continue;
        };
        let data = archive.read_full(id)?;
        let target = dest.join(inner.trim_start_matches(['\\', '/']));
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| t!("err-create-dir", path = parent.display().to_string()))?;
        }
        std::fs::write(&target, &data)
            .with_context(|| t!("err-write", path = target.display().to_string()))?;
        copied += 1;
    }
    Ok(copied)
}

/// Prints label–value pairs with the values in one column, whatever the
/// language makes of the labels' widths.
fn print_table(rows: &[(String, String)]) {
    let width = rows
        .iter()
        .map(|(label, _)| display_width(label))
        .max()
        .unwrap_or(0);
    for (label, value) in rows {
        let pad = width - display_width(label) + 2;
        println!("{label}{:pad$}{value}", "");
    }
}

/// Width in console columns: CJK characters take two.
fn display_width(text: &str) -> usize {
    text.chars()
        .map(|c| match c as u32 {
            0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6 => 2,
            _ => 1,
        })
        .sum()
}

fn cmd_info(archive: &Path, args: &ArchiveArgs) -> Result<()> {
    let started = Instant::now();
    let a = open_archive(archive, args, None)?;
    let elapsed = started.elapsed();
    let stats = a.tree().stats();

    let mut rows: Vec<(String, String)> = vec![
        (t!("info-archive"), a.path().display().to_string()),
        (t!("info-format"), a.format().name().to_string()),
        (t!("info-size"), human(a.size())),
        (t!("info-files"), stats.files.to_string()),
        (t!("info-dirs"), stats.dirs.to_string()),
        (t!("info-uncompressed"), human(stats.total_uncompressed)),
    ];
    if a.size() > 0 && stats.total_uncompressed > 0 {
        let ratio = stats.total_uncompressed as f64 / a.size() as f64;
        rows.push((t!("info-ratio"), format!("{}x", decimal(ratio, 1))));
    }
    rows.push((t!("info-nodes"), a.tree().node_count().to_string()));

    if let Ok(tar) = a.as_tar() {
        if tar.is_compressed() {
            rows.push((
                t!("info-in-memory"),
                t!("info-in-memory-value", size = human(tar.stream_len())),
            ));
        }
    }

    #[cfg(feature = "rar")]
    if let Some(rar) = a.as_rar() {
        let solid = if rar.is_solid() {
            t!("info-rar-solid-yes")
        } else {
            t!("info-solid-no")
        };
        rows.push((t!("info-solid"), solid));
        if rar.has_encrypted_headers() {
            rows.push((t!("info-headers"), t!("info-headers-encrypted")));
        }
    }

    if let Some(sz) = a.as_sevenz() {
        rows.push((t!("info-blocks"), sz.block_count().to_string()));
        let solid = if sz.is_solid() {
            t!("info-7z-solid-yes")
        } else {
            t!("info-solid-no")
        };
        rows.push((t!("info-solid"), solid));
    }

    if let Some(n) = a.encrypted_entry_count() {
        if n > 0 {
            rows.push((t!("info-encrypted"), t!("info-encrypted-value", count = n)));
        }
    }

    rows.push((
        t!("info-parse-time"),
        t!("seconds", value = decimal(elapsed.as_secs_f64(), 3)),
    ));
    print_table(&rows);
    Ok(())
}

fn cmd_verify(archive: &Path, random: bool, args: &ArchiveArgs) -> Result<()> {
    let a = open_archive(archive, args, None)?;
    let opts = zipfs_core::VerifyOptions {
        random_access: random,
        ..Default::default()
    };

    let started = Instant::now();
    let outcome = zipfs_core::verify(&a, opts)?;
    let secs = started.elapsed().as_secs_f64();

    for f in &outcome.failures {
        println!(
            "{}",
            t!(
                "verify-failure",
                path = f.path.clone(),
                reason = f.reason.clone()
            )
        );
    }

    let mut summary = t!(
        "verify-summary",
        ok = outcome.files_ok,
        errors = outcome.failures.len(),
        size = human(outcome.bytes_read),
        seconds = decimal(secs, 2),
        speed = throughput(outcome.bytes_read, secs)
    );
    if random {
        summary.push_str(&format!(" | {}", t!("verify-random")));
    }
    println!("{summary}");

    if outcome.files_without_checksum > 0 {
        println!(
            "{}",
            t!("verify-no-checksum", count = outcome.files_without_checksum)
        );
        let why = match a.format() {
            zipfs_core::Format::Tar => Some(t!("verify-no-checksum-tar")),
            zipfs_core::Format::TarGz => Some(t!("verify-no-checksum-targz")),
            zipfs_core::Format::Rar => Some(t!("verify-no-checksum-rar")),
            zipfs_core::Format::Zip => Some(t!("verify-no-checksum-zip")),
            _ => None,
        };
        for line in why.iter().flat_map(|text| text.lines()) {
            println!("  {line}");
        }
    }

    if outcome.failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(t!("err-verify-failed", count = outcome.failures.len()))
    }
}

struct MountArgs {
    mountpoint: Option<String>,
    detach: bool,
    open: bool,
    label: Option<String>,
    cache_mb: usize,
}

fn cmd_mount(archive: &Path, args: MountArgs, common: &ArchiveArgs) -> Result<()> {
    // Initialize WinFsp before opening the archive: if the driver is missing,
    // better say so at once than after parsing a gigabyte-sized directory.
    let winfsp = zipfs_mount::init();
    if let Err(e) = &winfsp {
        return report(Err(anyhow::anyhow!("{e:#}")));
    }
    let _winfsp = winfsp?;

    // The same archive is already mounted — no need for a second drive on it.
    // Clicking the same archive twice in the context menu is easy.
    if let Some(existing) = mounts::find_by_archive(archive) {
        if args.open {
            open_in_explorer(&existing.letter);
        }
        println!("{}", t!("mount-already", letter = existing.letter.clone()));
        return Ok(());
    }

    let letter = match &args.mountpoint {
        Some(m) => m.clone(),
        None => match mounts::first_free_letter() {
            Some(l) => l,
            None => return report(Err(anyhow::anyhow!(t!("err-no-free-letters")))),
        },
    };

    if let Err(e) = check_mountpoint_free(&letter) {
        return report(Err(e));
    }

    if args.detach {
        return spawn_detached(archive, &letter, &args, common);
    }

    run_mount(archive, &letter, &args, common)
}

/// Starts a copy of itself in the background and returns.
///
/// The password, if needed, goes through a temporary file that the child
/// deletes at once. The command line is no place for it — it is visible in
/// the process list; a pipe will not do either, because the background
/// process starts without inheriting handles (see mounts::spawn_detached).
fn spawn_detached(
    archive: &Path,
    letter: &str,
    args: &MountArgs,
    common: &ArchiveArgs,
) -> Result<()> {
    let password = read_password(common)?;

    let exe = std::env::current_exe().context("cannot determine the program's path")?;
    let mut argv: Vec<String> = vec![
        "mount".into(),
        archive.display().to_string(),
        letter.to_string(),
        "--cache-mb".into(),
        args.cache_mb.to_string(),
        "--encoding".into(),
        common.encoding.clone(),
    ];
    if let Some(label) = &args.label {
        argv.push("--label".into());
        argv.push(label.clone());
    }

    let password_file = match &password {
        Some(pw) => {
            let path = std::env::temp_dir().join(format!(
                "zipmount-pw-{}-{}.tmp",
                std::process::id(),
                letter.trim_end_matches(':')
            ));
            std::fs::write(&path, pw.bytes()).with_context(|| t!("err-spawn-password"))?;
            argv.push("--password-file".into());
            argv.push(path.display().to_string());
            Some(path)
        }
        None => None,
    };

    let pid = mounts::spawn_detached(&exe, &argv)?;

    // Wait for the drive to appear: if the archive did not open, the process
    // dies silently, and it is up to us to say so.
    let appeared = wait_for_drive(letter, std::time::Duration::from_secs(20));

    // The child deletes the password file itself; clean up in case it did
    // not get that far.
    if let Some(path) = password_file {
        let _ = std::fs::remove_file(path);
    }

    if !appeared {
        return report(Err(anyhow::anyhow!(t!(
            "err-mount-failed-detached",
            path = archive.display().to_string()
        ))));
    }

    mounts::register(letter, &mounts::normalize(archive), pid)?;
    println!("{}", t!("mount-done", letter = letter.to_string()));

    if args.open {
        open_in_explorer(letter);
    }
    Ok(())
}

fn wait_for_drive(letter: &str, timeout: std::time::Duration) -> bool {
    let root = format!("{letter}\\");
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if Path::new(&root).exists() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

fn open_in_explorer(letter: &str) {
    let _ = ProcCommand::new("explorer.exe")
        .arg(format!("{letter}\\"))
        .spawn();
}

/// Mounts and holds the drive until asked to stop.
fn run_mount(archive: &Path, letter: &str, args: &MountArgs, common: &ArchiveArgs) -> Result<()> {
    let started = Instant::now();
    let a = open_archive(archive, common, Some(args.cache_mb))?;
    let stats = a.tree().stats();
    let parse_time = started.elapsed();

    let label = args.label.clone().unwrap_or_else(|| {
        archive
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ZipMount".to_string())
    });

    let mount = zipfs_mount::Mount::new(
        a,
        zipfs_mount::MountOptions {
            mountpoint: letter.to_string(),
            label,
            cache_budget: args.cache_mb * 1024 * 1024,
        },
    )?;

    mounts::register(letter, &mounts::normalize(archive), std::process::id())?;

    println!(
        "{}",
        t!(
            "mount-done-stats",
            letter = letter.to_string(),
            files = stats.files,
            dirs = stats.dirs,
            size = human(stats.total_uncompressed)
        )
    );
    println!(
        "{}",
        t!(
            "mount-parse-time",
            seconds = decimal(parse_time.as_secs_f64(), 3)
        )
    );
    println!();
    println!("{}", t!("mount-stop-hint", letter = letter.to_string()));

    wait_for_stop(letter)?;

    println!("{}", t!("mount-unmounting"));
    drop(mount);
    let _ = mounts::unregister(letter);
    println!("{}", t!("mount-finished"));
    Ok(())
}

/// Waits for either Ctrl+C or a request to stop from the unmount command.
fn wait_for_stop(letter: &str) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let ctrl_tx = tx.clone();
    ctrlc::set_handler(move || {
        let _ = ctrl_tx.send(());
    })
    .context("cannot install the Ctrl+C handler")?;

    let name = mounts::stop_event_name(letter);
    std::thread::spawn(move || {
        if let Some(event) = mounts::StopEvent::create(&name) {
            event.wait();
            let _ = tx.send(());
        }
    });

    let _ = rx.recv();
    Ok(())
}

fn cmd_unmount(target: &str) -> Result<()> {
    // File Explorer passes a drive with a trailing backslash, a person types
    // just the letter. Bring both to one form, or the menu item would not
    // find its own mount.
    let trimmed = target.trim_end_matches(['\\', '/']);
    let normalized = if trimmed.len() == 1
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
    {
        format!("{}:", trimmed.to_uppercase())
    } else {
        trimmed.to_string()
    };

    // The target can be named by letter or by archive path — whichever is
    // handier in the menu.
    let record = mounts::find(&normalized).or_else(|| mounts::find_by_archive(Path::new(target)));

    if let Some(record) = record {
        if !mounts::signal_stop(&mounts::stop_event_name(&record.letter)) {
            // The process may not have created the event yet, or be dead.
            let _ = mounts::unregister(&record.letter);
            return report(Err(anyhow::anyhow!(t!(
                "err-unmount-not-responding",
                letter = record.letter.clone()
            ))));
        }

        return if mounts::wait_for_exit(record.pid, 15_000) {
            let _ = mounts::unregister(&record.letter);
            println!("{}", t!("unmount-done", letter = record.letter.clone()));
            Ok(())
        } else {
            report(Err(anyhow::anyhow!(t!(
                "err-unmount-timeout",
                letter = record.letter.clone()
            ))))
        };
    }

    // No record — but the event's name follows from the letter alone, so a
    // mount can be stopped without the list. This is not a just-in-case
    // fallback: the list lives in the user's profile, which may be out of
    // reach — a mount made by another user of this machine, for one.
    if mounts::is_drive_letter(&normalized)
        && mounts::signal_stop(&mounts::stop_event_name(&normalized))
    {
        return if mounts::wait_for_drive_gone(&normalized, 15_000) {
            let _ = mounts::unregister(&normalized);
            println!("{}", t!("unmount-done", letter = normalized.clone()));
            Ok(())
        } else {
            report(Err(anyhow::anyhow!(t!(
                "err-unmount-timeout",
                letter = normalized.clone()
            ))))
        };
    }

    report(Err(anyhow::anyhow!(t!(
        "err-not-mounted",
        target = target.to_string()
    ))))
}

fn cmd_mounts() -> Result<()> {
    let records = mounts::list();
    if records.is_empty() {
        println!("{}", t!("mounts-none"));
        return Ok(());
    }
    for r in &records {
        println!("{}  {}", r.letter, r.archive.display());
    }
    Ok(())
}

/// Reports an error so that it is seen both from a console and from File
/// Explorer.
///
/// A context menu item runs without a console, and ordinary output to stderr
/// would simply vanish — so a window is shown there.
fn report(result: Result<()>) -> Result<()> {
    let Err(e) = result else {
        return Ok(());
    };
    let text = format!("{e:#}");

    if has_console() {
        return Err(e);
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let title: Vec<u16> = "ZipMount"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: both strings live until the end of the call and are
        // NUL-terminated.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                wide.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    Err(e)
}

fn has_console() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::GetConsoleWindow;
        // SAFETY: no arguments; returns a window handle or null.
        unsafe { !GetConsoleWindow().is_null() }
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn check_mountpoint_free(mountpoint: &str) -> Result<()> {
    let looks_like_letter = mountpoint.len() == 2
        && mountpoint.ends_with(':')
        && mountpoint
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic());
    if !looks_like_letter {
        return Ok(());
    }

    let root = format!("{mountpoint}\\");
    if !Path::new(&root).exists() {
        return Ok(());
    }

    let free: Vec<String> = ('D'..='Z')
        .map(|c| format!("{c}:"))
        .filter(|d| !Path::new(&format!("{d}\\")).exists())
        .collect();

    anyhow::bail!(t!(
        "err-letter-busy",
        letter = mountpoint.to_string(),
        free = if free.is_empty() {
            t!("letters-none")
        } else {
            free.join(" ")
        }
    ))
}

/// Interactive search: a console window, a prompt, results.
///
/// A menu item cannot ask what to look for, so we ask ourselves once
/// started. The window stays until told to close: otherwise the results
/// would flash and vanish.
fn cmd_search(archive: &Path) -> Result<()> {
    println!(
        "{}",
        t!("search-archive", path = archive.display().to_string())
    );

    let a = match Archive::open(archive, NameEncoding::Auto) {
        Ok(a) => a,
        // The most common reason is a password-protected archive.
        Err(e) if zipfs_core::needs_password(&e) => Archive::open_with(
            archive,
            OpenOptions {
                password: Some(prompt_password()?),
                ..Default::default()
            },
        )?,
        Err(e) => {
            println!("{e:#}");
            wait_for_enter();
            return Err(e);
        }
    };

    let stats = a.tree().stats();
    println!(
        "{}",
        t!(
            "search-stats",
            files = stats.files,
            size = human(stats.total_uncompressed),
            format = a.format().name()
        )
    );
    println!("{}", t!("search-intro"));

    loop {
        println!();
        print!("{} ", t!("search-prompt"));
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let pattern = line.trim_end_matches(['\r', '\n']);
        if pattern.is_empty() {
            break;
        }

        let opts = GrepOptions {
            pattern: pattern.to_string(),
            max_total: Some(200),
            ..Default::default()
        };

        let started = Instant::now();
        match grep(&a, &opts) {
            Ok(outcome) => {
                for m in &outcome.matches {
                    println!("{}:{}: {}", m.path, m.line_number, m.line);
                }
                let mut summary = t!(
                    "search-summary",
                    matched = outcome.files_matched,
                    scanned = outcome.files_scanned,
                    seconds = decimal(started.elapsed().as_secs_f64(), 2)
                );
                if outcome.truncated {
                    summary.push_str(&format!(" {}", t!("search-truncated")));
                }
                println!("--- {summary}");
            }
            Err(e) => println!("{}", t!("search-error", error = format!("{e:#}"))),
        }
    }
    Ok(())
}

fn wait_for_enter() {
    println!();
    print!("{}", t!("press-enter"));
    let _ = std::io::stdout().flush();
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
}

fn cmd_shell_install(modern: bool) -> Result<()> {
    if modern {
        return modern::install();
    }

    let exe = std::env::current_exe().context("cannot determine the program's path")?;
    shell::install(&exe)?;

    // The menu speaks File Explorer's language, which may differ from this
    // console's (ZIPMOUNT_LANG); the listing below shows the items as they
    // will appear.
    let menu = zipmount_i18n::decide_without_environment().0;
    println!(
        "{}",
        t!(
            "shell-installed",
            extensions = shell::extensions().join(" ")
        )
    );
    println!();
    println!(
        "{}",
        zipmount_i18n::message_in("shell-installed-items", menu, None)
    );
    println!();
    println!("{}", t!("shell-installed-where"));
    println!();
    println!("{}", t!("shell-modern-hint"));
    println!("{}", t!("shell-remove-hint"));
    Ok(())
}

fn cmd_shell_package(out: &Path, library: Option<&str>) -> Result<()> {
    std::fs::create_dir_all(out)
        .with_context(|| t!("err-create-dir", path = out.display().to_string()))?;

    // Without an explicit name the library gets a fingerprint of its contents
    // in its name and is put next to the package. That way the installer
    // never has to overwrite a file loaded by File Explorer — which loads it
    // and does not let go until a reboot.
    let library = match library {
        Some(name) => name.to_string(),
        None => {
            let exe = std::env::current_exe().context("cannot determine the program's path")?;
            let source = exe
                .parent()
                .context("cannot determine the program's directory")?
                .join("zipmount_shell.dll");
            let bytes = std::fs::read(&source)
                .with_context(|| t!("err-read", path = source.display().to_string()))?;
            let name = format!("zipmount_shell_{}.dll", modern::fingerprint(&bytes));
            std::fs::write(out.join(&name), &bytes)?;
            name
        }
    };

    let package = modern::build_package(&library, out)?;
    // The installer build script reads this output, hence "key=value".
    println!("msix={}", package.msix.display());
    println!("certificate={}", package.certificate.display());
    println!("library={library}");
    Ok(())
}

fn cmd_shell_register(package: Option<&Path>, external: Option<&Path>, remove: bool) -> Result<()> {
    if remove {
        return modern::unregister();
    }
    // Both options are required unless removing — clap has checked that.
    let (Some(package), Some(external)) = (package, external) else {
        anyhow::bail!("--package and --external are required");
    };

    // The installer passes the path with a trailing dot (otherwise the
    // backslash would escape the closing quote), so bring it to its normal
    // form — it ends up in the registry and in front of the user.
    let external = external
        .canonicalize()
        .map(|path| mounts::normalize(&path))
        .unwrap_or_else(|_| external.to_path_buf());

    modern::register(package, &external)
}

fn cmd_shell_trust(certificate: Option<&Path>, remove: bool) -> Result<()> {
    if remove {
        return modern::untrust_here();
    }
    let Some(certificate) = certificate else {
        anyhow::bail!("--certificate is required");
    };
    modern::trust_here(certificate)
}

fn cmd_shell_uninstall() -> Result<()> {
    // Removes both variants at once: the registry items and the package. If
    // there was no package, nothing is said about certificates.
    modern::uninstall()
}

fn source_text(source: Source) -> String {
    match source {
        Source::Environment => t!("language-source-environment"),
        Source::Saved => t!("language-source-saved"),
        Source::Windows => t!("language-source-windows"),
        Source::Default => t!("language-source-default"),
    }
}

fn language_line(language: &Language, source: Source) -> String {
    t!(
        "language-current",
        name = language.native_name,
        code = language.code,
        source = source_text(source)
    )
}

/// `zipmount language [code|auto]`.
///
/// The choice lives in the registry, so it applies to every later command and
/// to the context menu — unlike `ZIPMOUNT_LANG`, which only File Explorer never
/// sees. Registry menu texts are static, so they are rewritten here; the
/// modern menu asks for its texts every time and needs nothing.
fn cmd_language(code: Option<&str>) -> Result<()> {
    let Some(code) = code else {
        println!(
            "{}",
            language_line(zipmount_i18n::language(), zipmount_i18n::source())
        );
        println!();
        println!("{}", t!("language-available"));
        for language in zipmount_i18n::LANGUAGES {
            println!("  {:<6} {}", language.code, language.native_name);
        }
        println!();
        println!("{}", t!("language-hint"));
        return Ok(());
    };

    let chosen = if code.eq_ignore_ascii_case("auto") {
        None
    } else {
        let codes: Vec<&str> = zipmount_i18n::LANGUAGES.iter().map(|l| l.code).collect();
        let language = zipmount_i18n::match_tag(code).with_context(|| {
            t!(
                "err-language-unknown",
                code = code.to_string(),
                available = codes.join(", ")
            )
        })?;
        Some(language)
    };
    zipmount_i18n::save(chosen).with_context(|| t!("err-language-save"))?;

    // What File Explorer speaks from now on, and so what this confirmation
    // is written in.
    let (language, source) = zipmount_i18n::decide_without_environment();
    zipmount_i18n::switch_to(language, source);

    let message = match chosen {
        Some(_) => t!(
            "language-set",
            name = language.native_name,
            code = language.code
        ),
        None => t!(
            "language-follows-windows",
            name = language.native_name,
            code = language.code
        ),
    };
    println!("{message}");

    let rewritten = shell::refresh_texts(language)?;
    println!("{}", t!("language-menu-updated", count = rewritten));

    if let Ok(value) = std::env::var(zipmount_i18n::LANG_VAR) {
        println!();
        println!("{}", t!("language-env-overrides", value = value));
    }
    Ok(())
}

fn cmd_doctor() -> Result<()> {
    let dll = Path::new(r"C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll");
    let mut rows: Vec<(String, String)> = vec![
        (
            t!("doctor-winfsp"),
            if dll.exists() {
                t!("doctor-winfsp-found", path = dll.display().to_string())
            } else {
                t!("doctor-winfsp-missing")
            },
        ),
        (
            t!("doctor-rar"),
            if cfg!(feature = "rar") {
                t!("doctor-rar-yes")
            } else {
                t!("doctor-rar-no")
            },
        ),
        (
            t!("doctor-language"),
            format!(
                "{} ({}), {}",
                zipmount_i18n::language().native_name,
                zipmount_i18n::language().code,
                source_text(zipmount_i18n::source())
            ),
        ),
    ];

    // A package can be registered while its item never appears — between
    // registration and the menu lies creating the COM object. So the handler
    // itself is asked, not just the list of installed packages.
    rows.push((
        t!("doctor-modern"),
        if modern::is_installed() {
            match modern::probe_handler() {
                Ok(title) => t!("doctor-modern-ok", title = title),
                Err(e) => t!("doctor-modern-silent", error = format!("{e:#}")),
            }
        } else {
            t!("doctor-modern-missing")
        },
    ));

    // The real readiness check is the same initialization mount performs.
    match zipfs_mount::init() {
        Ok(_) => {
            rows.push((t!("doctor-init"), t!("doctor-init-ok")));
            print_table(&rows);
            println!();
            println!("{}", t!("doctor-ready"));
        }
        Err(e) => {
            rows.push((t!("doctor-init"), t!("doctor-init-failed")));
            print_table(&rows);
            println!();
            println!("{}", t!("doctor-reason", error = format!("{e:#}")));
            println!();
            println!("{}", t!("doctor-no-winfsp-note"));
        }
    }
    Ok(())
}

fn throughput(bytes: u64, secs: f64) -> String {
    let mb_per_sec = if secs > 0.0 {
        bytes as f64 / secs / (1024.0 * 1024.0)
    } else {
        0.0
    };
    decimal(mb_per_sec, 0)
}

fn human(bytes: u64) -> String {
    let units = [
        t!("unit-b"),
        t!("unit-kb"),
        t!("unit-mb"),
        t!("unit-gb"),
        t!("unit-tb"),
    ];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", units[0])
    } else {
        format!("{} {}", decimal(value, 1), units[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_visible_command_and_argument_has_help() {
        // Clap would show an empty line for anything missing from HELP, and
        // nothing else would notice.
        let cli = Cli::command();
        for sub in cli.get_subcommands().filter(|s| !s.is_hide_set()) {
            let name = sub.get_name();
            assert!(help_for(name, "").is_some(), "{name} has no help");
            for arg in sub.get_arguments().filter(|a| !a.is_hide_set()) {
                let id = arg.get_id().as_str();
                assert!(help_for(name, id).is_some(), "{name} {id} has no help");
            }
        }
    }

    #[test]
    fn help_table_names_only_real_commands_and_arguments() {
        let cli = Cli::command();
        for (command, arg, id) in HELP {
            if *command == "*" {
                continue;
            }
            let sub = cli
                .find_subcommand(command)
                .unwrap_or_else(|| panic!("{id}: no command {command}"));
            if !arg.is_empty() {
                assert!(
                    sub.get_arguments().any(|a| a.get_id() == *arg),
                    "{id}: {command} has no argument {arg}"
                );
            }
        }
    }

    #[test]
    fn localized_command_is_consistent() {
        // Clap checks the whole definition in debug builds; a heading or flag
        // added at run time must not break it.
        localized_command().debug_assert();
    }

    #[test]
    fn widths_count_cjk_twice() {
        assert_eq!(display_width("files:"), 6);
        assert_eq!(display_width("文件："), 6);
        assert_eq!(display_width("파일"), 4);
    }
}
