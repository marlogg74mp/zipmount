//! ZipMount — an archive as a drive.
//!
//! Browsing and search commands need neither a filesystem driver nor
//! administrator rights: they work with the archive directly, the same way on
//! every system. Mounting is a separate command on top, and the part that
//! differs: WinFsp and drive letters on Windows (`windows`), FUSE and
//! directories on Linux (`unix`).

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix as platform;
#[cfg(windows)]
use windows as platform;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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
    #[cfg(windows)]
    ShellInstall {
        #[arg(long)]
        modern: bool,
    },
    #[cfg(windows)]
    ShellUninstall,
    Language {
        code: Option<String>,
    },

    /// Build and sign the menu package (needs the Windows SDK).
    ///
    /// An internal command for building the installer: it carries a ready
    /// package, because the user's machine has no SDK.
    #[cfg(windows)]
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
    #[cfg(windows)]
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
    #[cfg(windows)]
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

/// Where the wording differs on Linux and macOS: no drive letters there, and
/// no File Explorer. Looked up before `HELP`.
#[cfg(unix)]
const PLATFORM_HELP: &[(&str, &str, &str)] = &[
    ("*", "prefix", "help-prefix-unix"),
    ("mount", "mountpoint", "help-mount-mountpoint-unix"),
    ("mount", "open", "help-mount-open-unix"),
    ("unmount", "", "help-unmount-unix"),
    ("unmount", "target", "help-unmount-target-unix"),
    ("language", "code", "help-language-code-unix"),
];
#[cfg(windows)]
const PLATFORM_HELP: &[(&str, &str, &str)] = &[];

/// Commands that exist only on Windows, though `HELP` lists them everywhere.
#[cfg(test)]
const WINDOWS_ONLY: &[&str] = &["shell-install", "shell-uninstall"];

fn help_for(command: &str, arg: &str) -> Option<String> {
    fn find(
        table: &[(&str, &str, &'static str)],
        command: &str,
        arg: &str,
    ) -> Option<&'static str> {
        table
            .iter()
            .find(|(c, a, _)| *c == command && *a == arg)
            .map(|(_, _, id)| *id)
    }
    // The command's own entry beats a shared one; either way, this system's
    // wording beats the general one.
    [command, "*"]
        .iter()
        .find_map(|c| find(PLATFORM_HELP, c, arg).or_else(|| find(HELP, c, arg)))
        .map(|id| zipmount_i18n::message(id, None))
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
    #[cfg(windows)]
    let (about, notice) = (t!("help-about"), t!("help-notice"));
    #[cfg(unix)]
    let (about, notice) = (t!("help-about-unix"), t!("help-notice-unix"));
    let mut cmd = localize_level(Cli::command(), "")
        .about(about)
        .after_help(notice)
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
    // `zipmount ls big.zip | head` closes the pipe early. Rust ignores
    // SIGPIPE, so the next write fails and println! panics with "Broken pipe";
    // a command-line tool should just stop quietly, as the signal makes it.
    #[cfg(unix)]
    // SAFETY: restores the default disposition before any thread starts.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

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
        } => platform::cmd_mount(
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
        Command::Unmount { target } => platform::cmd_unmount(&target),
        Command::Mounts => platform::cmd_mounts(),
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
        #[cfg(windows)]
        Command::ShellInstall { modern } => windows::cmd_shell_install(modern),
        #[cfg(windows)]
        Command::ShellUninstall => windows::cmd_shell_uninstall(),
        Command::Language { code } => cmd_language(code.as_deref()),
        #[cfg(windows)]
        Command::ShellPackage { out, library } => {
            windows::cmd_shell_package(&out, library.as_deref())
        }
        #[cfg(windows)]
        Command::ShellRegister {
            package,
            external,
            remove,
        } => windows::cmd_shell_register(package.as_deref(), external.as_deref(), remove),
        #[cfg(windows)]
        Command::ShellTrust {
            certificate,
            remove,
        } => windows::cmd_shell_trust(certificate.as_deref(), remove),
        Command::Doctor => platform::cmd_doctor(),
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
    /// The volume label File Explorer shows; a FUSE mount has none.
    #[cfg_attr(unix, allow(dead_code))]
    label: Option<String>,
    cache_mb: usize,
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

    if has_console() {
        return Err(e);
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let text = format!("{e:#}");
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

fn source_text(source: Source) -> String {
    match source {
        Source::Environment => t!("language-source-environment"),
        Source::Saved => t!("language-source-saved"),
        #[cfg(windows)]
        Source::Windows => t!("language-source-windows"),
        #[cfg(unix)]
        Source::Windows => t!("language-source-system"),
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
        #[cfg(windows)]
        println!("{}", t!("language-hint"));
        #[cfg(unix)]
        println!("{}", t!("language-hint-unix"));
        return Ok(());
    };

    let chosen = if code.eq_ignore_ascii_case("auto") {
        None
    } else {
        let codes: Vec<&str> = zipmount_i18n::LANGUAGES.iter().map(|l| l.code).collect();
        let language = zipmount_i18n::match_tag(code).with_context(|| {
            #[cfg(windows)]
            let text = t!(
                "err-language-unknown",
                code = code.to_string(),
                available = codes.join(", ")
            );
            #[cfg(unix)]
            let text = t!(
                "err-language-unknown-unix",
                code = code.to_string(),
                available = codes.join(", ")
            );
            text
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
        #[cfg(windows)]
        None => t!(
            "language-follows-windows",
            name = language.native_name,
            code = language.code
        ),
        #[cfg(unix)]
        None => t!(
            "language-follows-system",
            name = language.native_name,
            code = language.code
        ),
    };
    println!("{message}");

    platform::refresh_menu_texts(language)?;

    if let Ok(value) = std::env::var(zipmount_i18n::LANG_VAR) {
        println!();
        println!("{}", t!("language-env-overrides", value = value));
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
        for (command, arg, id) in HELP.iter().chain(PLATFORM_HELP) {
            if *command == "*" || (cfg!(unix) && WINDOWS_ONLY.contains(command)) {
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
