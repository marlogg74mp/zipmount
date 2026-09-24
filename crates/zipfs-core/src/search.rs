//! Searching an archive.
//!
//! Content search deliberately bypasses the filesystem, but the walk order
//! depends on the format, because formats differ in their unit of access:
//!
//! * **zip** — entries are compressed independently, so work is spread over
//!   files;
//! * **7z** — entries are glued into solid blocks, so work is spread over
//!   blocks, streaming within each. That is also the only cheap order:
//!   pulling files out one by one would expand a block again for each;
//! * **tar** — contents are available as slices; there is nothing to copy.
//!
//! The same search through a mounted volume would be an order of magnitude
//! slower — the kernel would drive callbacks with small reads, one file at a
//! time.
//!
//! **The pattern is a substring by default, not a regular expression.** A
//! deliberate choice: strings like `[ERROR]` or `app.config` come up in logs
//! all the time, and as regexes they mean something else entirely. On a real
//! log archive, `[ERROR]` in regex mode matched 29,748 files out of 30,778 —
//! silently, without a single error. That failure mode is worse than breaking
//! with grep habits.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use anyhow::{Context, Result};
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkContext, SinkMatch};
use rayon::prelude::*;
use zipmount_i18n::t;

use crate::archive::{Archive, Backend};

/// Entries above this size are skipped by search: in zip every thread holds
/// its own decompressed file, and without a ceiling peak memory would be
/// "number of cores × size of the largest file".
pub const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct GrepOptions {
    pub pattern: String,
    /// Treat the pattern as a plain substring. That is the default.
    pub literal: bool,
    pub case_insensitive: bool,
    /// File name pattern, for example `*.log`.
    pub glob: Option<String>,
    /// Search only inside this branch of the tree.
    pub subpath: Option<String>,
    /// Stop at the first match in a file ("paths only" mode).
    pub files_only: bool,
    /// Count matches rather than show lines.
    pub count_only: bool,
    /// How many lines to show before and after a match.
    pub context_before: usize,
    pub context_after: usize,
    /// Ceilings on matches per file and for the whole search.
    pub max_per_file: Option<usize>,
    pub max_total: Option<usize>,
    pub max_file_bytes: u64,
    /// A prefix for printed paths: the mounted drive's letter, so that what
    /// was found can be opened right away.
    pub path_prefix: Option<String>,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            literal: true,
            case_insensitive: false,
            glob: None,
            subpath: None,
            files_only: false,
            count_only: false,
            context_before: 0,
            context_after: 0,
            max_per_file: None,
            max_total: None,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            path_prefix: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub path: String,
    pub line_number: u64,
    pub line: String,
    /// A context line rather than the match itself.
    pub is_context: bool,
}

#[derive(Debug, Clone)]
pub struct FileCount {
    pub path: String,
    pub matches: u64,
}

#[derive(Debug, Default)]
pub struct GrepOutcome {
    pub matches: Vec<Match>,
    /// Filled in count mode.
    pub counts: Vec<FileCount>,
    pub files_matched: u64,
    pub files_scanned: u64,
    pub files_skipped: u64,
    pub bytes_scanned: u64,
    /// Output was cut because a ceiling was reached.
    pub truncated: bool,
}

/// Collects the matches and context lines of one file.
struct Collector<'a> {
    path: &'a str,
    opts: &'a GrepOptions,
    lines: Vec<Match>,
    match_count: u64,
}

impl<'a> Collector<'a> {
    fn new(path: &'a str, opts: &'a GrepOptions) -> Self {
        Self {
            path,
            opts,
            lines: Vec::new(),
            match_count: 0,
        }
    }

    fn push(&mut self, line_number: u64, bytes: &[u8], is_context: bool) {
        let text = String::from_utf8_lossy(bytes);
        self.lines.push(Match {
            path: self.path.to_string(),
            line_number,
            line: text.trim_end_matches(['\r', '\n']).to_string(),
            is_context,
        });
    }
}

impl Sink for Collector<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, m: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        self.match_count += 1;

        // "Paths only" and "count only" need no lines.
        if self.opts.files_only {
            return Ok(false);
        }
        if !self.opts.count_only {
            self.push(m.line_number().unwrap_or(0), m.bytes(), false);
            if let Some(max) = self.opts.max_per_file {
                if self.match_count as usize >= max {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn context(&mut self, _searcher: &Searcher, c: &SinkContext<'_>) -> Result<bool, Self::Error> {
        if self.opts.files_only || self.opts.count_only {
            return Ok(true);
        }
        self.push(c.line_number().unwrap_or(0), c.bytes(), true);
        Ok(true)
    }
}

/// Shared state of one search pass.
struct GrepRun<'a> {
    archive: &'a Archive,
    opts: &'a GrepOptions,
    matcher: RegexMatcher,
    /// The node to search below; `None` means the whole archive.
    root: Option<u32>,
    files_matched: AtomicU64,
    files_scanned: AtomicU64,
    files_skipped: AtomicU64,
    bytes_scanned: AtomicU64,
    emitted: AtomicU64,
    truncated: AtomicBool,
    results: Mutex<Vec<Match>>,
    counts: Mutex<Vec<FileCount>>,
}

impl GrepRun<'_> {
    fn new_searcher(&self) -> Searcher {
        SearcherBuilder::new()
            // Stop reading binary files at the first zero byte: an archive of
            // texts has few, but there is no reason to spend time on them.
            .binary_detection(BinaryDetection::quit(0))
            .line_number(true)
            .before_context(self.opts.context_before)
            .after_context(self.opts.context_after)
            .build()
    }

    fn accepts(&self, node: u32) -> bool {
        match &self.opts.glob {
            Some(g) => glob_match(g, &self.archive.tree().node(node).name),
            None => true,
        }
    }

    fn in_scope(&self, node: u32) -> bool {
        match self.root {
            Some(root) => self.archive.tree().is_under(node, root),
            None => true,
        }
    }

    fn path_of(&self, node: u32) -> String {
        let path = self.archive.tree().path_of(node);
        match &self.opts.path_prefix {
            Some(prefix) => format!("{prefix}{path}"),
            None => path,
        }
    }

    /// Runs the search over one entry's contents.
    fn scan(&self, searcher: &mut Searcher, node: u32, data: &[u8]) {
        self.files_scanned.fetch_add(1, Ordering::Relaxed);
        self.bytes_scanned
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        let path = self.path_of(node);
        let mut sink = Collector::new(&path, self.opts);
        let _ = searcher.search_slice(&self.matcher, data, &mut sink);

        // Take the result before the borrow ends: `path` then moves into the
        // output structures by value.
        let match_count = sink.match_count;
        let lines = std::mem::take(&mut sink.lines);
        drop(sink);

        if match_count == 0 {
            return;
        }
        self.files_matched.fetch_add(1, Ordering::Relaxed);

        if self.opts.count_only {
            self.counts
                .lock()
                .expect("results poisoned by a panic")
                .push(FileCount {
                    path,
                    matches: match_count,
                });
            return;
        }

        if self.opts.files_only {
            self.emit(vec![Match {
                path,
                line_number: 0,
                line: String::new(),
                is_context: false,
            }]);
            return;
        }

        self.emit(lines);
    }

    /// Adds lines to the shared result, honouring the overall output ceiling.
    fn emit(&self, lines: Vec<Match>) {
        if let Some(max) = self.opts.max_total {
            let already = self.emitted.load(Ordering::Relaxed) as usize;
            if already >= max {
                self.truncated.store(true, Ordering::Relaxed);
                return;
            }
        }
        self.emitted
            .fetch_add(lines.len() as u64, Ordering::Relaxed);
        self.results
            .lock()
            .expect("results poisoned by a panic")
            .extend(lines);
    }

    fn finish(self) -> GrepOutcome {
        let mut matches = self
            .results
            .into_inner()
            .expect("results poisoned by a panic");
        // A parallel walk yields arbitrary order — make it stable.
        matches.sort_by(|a, b| a.path.cmp(&b.path).then(a.line_number.cmp(&b.line_number)));

        let mut counts = self
            .counts
            .into_inner()
            .expect("results poisoned by a panic");
        counts.sort_by(|a, b| a.path.cmp(&b.path));

        // The overall ceiling applies to the ordered result, or the cut would
        // depend on which thread came first.
        let mut truncated = self.truncated.into_inner();
        if let Some(max) = self.opts.max_total {
            if matches.len() > max {
                matches.truncate(max);
                truncated = true;
            }
            if counts.len() > max {
                counts.truncate(max);
                truncated = true;
            }
        }

        GrepOutcome {
            matches,
            counts,
            files_matched: self.files_matched.into_inner(),
            files_scanned: self.files_scanned.into_inner(),
            files_skipped: self.files_skipped.into_inner(),
            bytes_scanned: self.bytes_scanned.into_inner(),
            truncated,
        }
    }
}

/// Searches the contents of every matching entry.
pub fn grep(archive: &Archive, opts: &GrepOptions) -> Result<GrepOutcome> {
    // In substring mode the whole pattern is escaped, so that exactly what
    // the user typed matches.
    let pattern = if opts.literal {
        regex::escape(&opts.pattern)
    } else {
        opts.pattern.clone()
    };

    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(opts.case_insensitive)
        .line_terminator(Some(b'\n'))
        .build(&pattern)
        .with_context(|| {
            if opts.literal {
                t!("core-grep-substring", pattern = opts.pattern.clone())
            } else {
                t!("core-grep-regex", pattern = opts.pattern.clone())
            }
        })?;

    let root = match &opts.subpath {
        Some(p) => Some(
            archive
                .tree()
                .resolve(p)
                .with_context(|| t!("err-path-not-found", path = p.clone()))?,
        ),
        None => None,
    };

    let run = GrepRun {
        archive,
        opts,
        matcher,
        root,
        files_matched: AtomicU64::new(0),
        files_scanned: AtomicU64::new(0),
        files_skipped: AtomicU64::new(0),
        bytes_scanned: AtomicU64::new(0),
        emitted: AtomicU64::new(0),
        truncated: AtomicBool::new(false),
        results: Mutex::new(Vec::new()),
        counts: Mutex::new(Vec::new()),
    };

    match archive.backend() {
        // 7z glues entries into blocks, so walk by blocks; in other formats an
        // entry stands alone and work is spread over files.
        Backend::SevenZ(_) => grep_per_block(&run)?,
        // RAR cannot pull an entry out cheaply either, but has no blocks as
        // such — walk the archive in one pass.
        #[cfg(feature = "rar")]
        Backend::Rar(_) => grep_streaming(&run)?,
        _ => grep_per_file(&run),
    }

    Ok(run.finish())
}

/// zip and tar: entries are independent, so work goes over files in parallel.
fn grep_per_file(run: &GrepRun) {
    let tree = run.archive.tree();
    let ids = match run.root {
        Some(root) => tree.file_ids_under(root),
        None => tree.file_ids(),
    };
    let candidates: Vec<u32> = ids.into_iter().filter(|id| run.accepts(*id)).collect();

    candidates.par_iter().for_each_init(
        || run.new_searcher(),
        |searcher, &id| {
            if tree.node(id).size > run.opts.max_file_bytes {
                run.files_skipped.fetch_add(1, Ordering::Relaxed);
                return;
            }
            // tar offers the contents as a slice without copying — use it.
            if let Some(slice) = run.archive.entry_slice(id) {
                run.scan(searcher, id, slice);
                return;
            }
            let Ok(data) = run.archive.read_full(id) else {
                run.files_skipped.fetch_add(1, Ordering::Relaxed);
                return;
            };
            run.scan(searcher, id, &data);
        },
    );
}

/// 7z: parallel over solid blocks, streaming within a block.
///
/// The `max_file_bytes` ceiling is not needed for memory here — the stream
/// holds one entry at a time — but large entries are skipped anyway, so as
/// not to spend time on files that are surely not text.
fn grep_per_block(run: &GrepRun) -> Result<()> {
    let backend = run.archive.as_sevenz().context("expected the 7z backend")?;
    let node_of = run.archive.tree().entry_node_map(backend.entry_count());

    // Work out in advance which blocks hold any entries of interest at all.
    // Without that, restricting by directory or pattern would save only the
    // scanning, not the decompression — that is, almost nothing.
    let mut needed = vec![false; backend.block_count()];
    for (entry_index, node) in node_of.iter().enumerate() {
        let Some(node) = *node else { continue };
        if run.in_scope(node) && run.accepts(node) {
            if let Some(block) = backend.block_of(entry_index as u32) {
                needed[block] = true;
            }
        }
    }
    let blocks: Vec<usize> = (0..backend.block_count()).filter(|b| needed[*b]).collect();

    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());

    blocks.into_par_iter().for_each_init(
        || run.new_searcher(),
        |searcher, block| {
            let outcome = backend.for_each_in_block(block, |entry_index, data| {
                let Some(node) = node_of.get(entry_index as usize).copied().flatten() else {
                    return Ok(());
                };
                if data.is_empty() || !run.accepts(node) || !run.in_scope(node) {
                    return Ok(());
                }
                if data.len() as u64 > run.opts.max_file_bytes {
                    run.files_skipped.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }
                run.scan(searcher, node, &data);
                Ok(())
            });

            if let Err(e) = outcome {
                errors
                    .lock()
                    .expect("error list poisoned by a panic")
                    .push(format!("{e:#}"));
            }
        },
    );

    let errors = errors.into_inner().expect("error list poisoned by a panic");
    if !errors.is_empty() {
        anyhow::bail!(t!("core-grep-block-errors", errors = errors.join("; ")));
    }
    Ok(())
}

/// RAR: one sequential pass over the archive.
#[cfg(feature = "rar")]
fn grep_streaming(run: &GrepRun) -> Result<()> {
    let backend = run.archive.as_rar().context("expected the RAR backend")?;
    let node_of = run.archive.tree().entry_node_map(backend.entry_count());

    let wanted = |entry_index: u32| {
        node_of
            .get(entry_index as usize)
            .copied()
            .flatten()
            .is_some_and(|node| run.accepts(node) && run.in_scope(node))
    };

    let mut searcher = run.new_searcher();
    backend.for_each_entry_filtered(wanted, |entry_index, data| {
        let Some(node) = node_of.get(entry_index as usize).copied().flatten() else {
            return Ok(());
        };
        if data.is_empty() {
            return Ok(());
        }
        if data.len() as u64 > run.opts.max_file_bytes {
            run.files_skipped.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        run.scan(&mut searcher, node, &data);
        Ok(())
    })
}

/// Search by name. Walks the tree in memory without touching file bodies.
pub fn find_by_name(archive: &Archive, pattern: &str, prefix: Option<&str>) -> Vec<String> {
    let tree = archive.tree();
    let mut out = Vec::new();
    tree.walk(|id, node| {
        if id != crate::tree::ROOT && glob_match(pattern, &node.name) {
            let path = tree.path_of(id);
            out.push(match prefix {
                Some(p) => format!("{p}{path}"),
                None => path,
            });
        }
    });
    out.sort();
    out
}

/// Pattern matching the way Windows passes patterns when enumerating a
/// directory (`QueryDirectory`).
///
/// Win32 substitutes three special characters for ordinary wildcards in
/// patterns: `<` (DOS_STAR), `>` (DOS_QM) and `"` (DOS_DOT). They come, for
/// instance, from `*.*` and from patterns ending in a dot.
pub fn dos_pattern_match(pattern: &str, name: &str) -> bool {
    // `*` and `*.*` mean "everything" in Win32, names without a dot included.
    if pattern.is_empty() || pattern == "*" || pattern == "*.*" {
        return true;
    }

    let translated: String = pattern
        .chars()
        .map(|c| match c {
            '<' => '*',
            '>' => '?',
            '"' => '.',
            other => other,
        })
        .collect();

    glob_match(&translated, name)
}

/// Command-line style pattern matching: `*` is any sequence, `?` one
/// character. Case is ignored.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().flat_map(char::to_lowercase).collect();
    let n: Vec<char> = name.chars().flat_map(char::to_lowercase).collect();

    // The classic two-pointer algorithm that backtracks to the last star: no
    // recursion and no risk of exponential time on patterns like `*a*a*a*`.
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut backtrack) = (usize::MAX, 0usize);

    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = pi;
            backtrack = ni;
            pi += 1;
        } else if star != usize::MAX {
            pi = star + 1;
            backtrack += 1;
            ni = backtrack;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_basic_wildcards() {
        assert!(glob_match("*.log", "server.log"));
        assert!(glob_match("*.log", "a.b.log"));
        assert!(!glob_match("*.log", "server.txt"));
        assert!(glob_match("report?.txt", "report1.txt"));
        assert!(!glob_match("report?.txt", "report12.txt"));
    }

    #[test]
    fn glob_is_case_insensitive_including_cyrillic() {
        assert!(glob_match("*.LOG", "server.log"));
        assert!(glob_match("отчёт*", "ОТЧЁТ за 2024.txt"));
    }

    #[test]
    fn glob_handles_exact_and_star_only() {
        assert!(glob_match("*", "anything at all"));
        assert!(glob_match("exact.txt", "exact.txt"));
        assert!(!glob_match("exact.txt", "exact.txt.bak"));
    }

    #[test]
    fn glob_does_not_blow_up_on_many_stars() {
        // A pattern that would send naive recursion exponential.
        let name = "a".repeat(64);
        assert!(!glob_match("*a*a*a*a*a*b", &name));
    }

    #[test]
    fn glob_matches_empty_segments() {
        assert!(glob_match("a*b", "ab"));
        assert!(glob_match("*", ""));
        assert!(!glob_match("?", ""));
    }

    #[test]
    fn dos_star_matches_everything_including_extensionless() {
        // `*.*` in Win32 means "any name", even one without a dot.
        assert!(dos_pattern_match("*.*", "README"));
        assert!(dos_pattern_match("*", "README"));
        assert!(dos_pattern_match("", "README"));
    }

    #[test]
    fn dos_special_wildcards_are_translated() {
        // `<` = DOS_STAR, `>` = DOS_QM, `"` = DOS_DOT.
        assert!(dos_pattern_match("file<", "file00001.log"));
        assert!(dos_pattern_match("file0000>.log", "file00001.log"));
        assert!(dos_pattern_match("file00001\"log", "file00001.log"));
    }

    #[test]
    fn dos_pattern_still_handles_ordinary_masks() {
        assert!(dos_pattern_match("file0000?.log", "file00001.log"));
        assert!(!dos_pattern_match("file0000?.log", "file00012.log"));
        assert!(dos_pattern_match("*.LOG", "server.log"));
    }

    /// Exactly the case that made substring the default: as a regex, `[ERROR]`
    /// means "any of the letters E, R, O".
    #[test]
    fn literal_pattern_escapes_regex_metacharacters() {
        let escaped = regex::escape("[ERROR]");
        // The same settings as the real code.
        let build = |p: &str| {
            RegexMatcherBuilder::new()
                // 10 is a newline; as a number, to avoid tripping over escaping
                .line_terminator(Some(10))
                .build(p)
                .unwrap()
        };
        let matcher = build(&escaped);
        let re_matcher = build("[ERROR]");

        // The line has no [ERROR] substring but has a capital O — exactly what
        // the pattern read as a character class will catch.
        let haystack = b"Operation failed without brackets\n";
        let mut searcher = SearcherBuilder::new().build();

        let mut literal_hits = 0;
        searcher
            .search_slice(
                &matcher,
                haystack,
                grep_searcher::sinks::UTF8(|_, _| {
                    literal_hits += 1;
                    Ok(true)
                }),
            )
            .unwrap();

        let mut regex_hits = 0;
        searcher
            .search_slice(
                &re_matcher,
                haystack,
                grep_searcher::sinks::UTF8(|_, _| {
                    regex_hits += 1;
                    Ok(true)
                }),
            )
            .unwrap();

        assert_eq!(literal_hits, 0, "the substring [ERROR] is not in the line");
        assert_eq!(
            regex_hits, 1,
            "but as a regex it matches the line because of the letter O"
        );
    }

    #[test]
    fn literal_escape_covers_common_log_patterns() {
        for pattern in ["[ERROR]", "app.config", "GET /api/v1", "a+b", "x(y)z", "*"] {
            let escaped = regex::escape(pattern);
            let matcher = RegexMatcherBuilder::new().build(&escaped).unwrap();
            let haystack = format!("prefix {pattern} suffix\n");

            let mut hits = 0;
            let mut searcher = SearcherBuilder::new().build();
            searcher
                .search_slice(
                    &matcher,
                    haystack.as_bytes(),
                    grep_searcher::sinks::UTF8(|_, _| {
                        hits += 1;
                        Ok(true)
                    }),
                )
                .unwrap();
            assert_eq!(hits, 1, "the substring {pattern:?} must be found literally");
        }
    }
}
