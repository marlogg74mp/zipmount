//! The archive's directory tree.
//!
//! There can be close to a million entries, so nodes live in an arena
//! (`Vec<Node>`) and refer to each other by `u32` indices, not pointers: the
//! tree then takes tens of megabytes instead of several times more spread
//! over `Box`es and `String`s. Each directory's children are kept sorted —
//! lookup is a binary search, with no hash table per directory.
//!
//! The tree knows nothing about the archive format: it is built from
//! `EntryMeta`, and `Node::entry` is the entry's index in the backend's list.

use std::cmp::Ordering;

use rustc_hash::FxHashMap;

use crate::entry::EntryMeta;
use crate::sanitize::{disambiguate, split_path};

/// Index of the root node in the arena.
pub const ROOT: u32 = 0;

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub parent: u32,
    pub children: Vec<u32>,
    /// The entry's index in the backend's list. `None` for synthesized
    /// directories: archives often carry no entries of their own for
    /// intermediate folders.
    pub entry: Option<u32>,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: u64,
}

pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    pub fn build(entries: &[EntryMeta]) -> Self {
        let mut nodes = vec![Node {
            name: String::new(),
            parent: ROOT,
            children: Vec::new(),
            entry: None,
            is_dir: true,
            size: 0,
            mtime: 0,
        }];

        // The "(parent, lower-case name) -> node" index is only needed while
        // building: afterwards lookup is a binary search over sorted children.
        let mut index: FxHashMap<(u32, String), u32> = FxHashMap::default();
        index.reserve(entries.len());

        for (i, meta) in entries.iter().enumerate() {
            let parts = split_path(&meta.path);
            if parts.is_empty() {
                continue;
            }

            let last_idx = parts.len() - 1;
            let mut cur = ROOT;

            for (k, part) in parts.iter().enumerate() {
                let is_last = k == last_idx;

                if is_last && !meta.is_dir {
                    let unique = unique_name(&index, cur, part);
                    let id = add_node(&mut nodes, &mut index, cur, unique, false);
                    let node = &mut nodes[id as usize];
                    node.entry = Some(i as u32);
                    node.size = meta.size;
                    node.mtime = meta.mtime;
                } else {
                    cur = get_or_create_dir(&mut nodes, &mut index, cur, part);
                    if is_last {
                        // An explicit directory entry: take its time.
                        let node = &mut nodes[cur as usize];
                        node.entry = Some(i as u32);
                        node.mtime = meta.mtime;
                    }
                }
            }
        }

        drop(index);

        // Sorting once at the end is cheaper than keeping order on insert.
        let order: Vec<u32> = (0..nodes.len() as u32).collect();
        for id in order {
            let mut children = std::mem::take(&mut nodes[id as usize].children);
            children.sort_by(|a, b| cmp_ci(&nodes[*a as usize].name, &nodes[*b as usize].name));
            children.shrink_to_fit();
            nodes[id as usize].children = children;
        }

        Self { nodes }
    }

    pub fn node(&self, id: u32) -> &Node {
        &self.nodes[id as usize]
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Finds a child by name, case-insensitively — the way Windows expects.
    pub fn lookup_child(&self, dir: u32, name: &str) -> Option<u32> {
        let children = &self.nodes[dir as usize].children;
        children
            .binary_search_by(|probe| cmp_ci(&self.nodes[*probe as usize].name, name))
            .ok()
            .map(|pos| children[pos])
    }

    /// Resolves a path like `\docs\report.txt` (or with forward slashes) to a
    /// node.
    pub fn resolve(&self, path: &str) -> Option<u32> {
        let mut cur = ROOT;
        for part in path
            .split(['/', '\\'])
            .filter(|p| !p.is_empty() && *p != ".")
        {
            if !self.nodes[cur as usize].is_dir {
                return None;
            }
            cur = self.lookup_child(cur, part)?;
        }
        Some(cur)
    }

    /// A node's full path from the root, with backslashes — fit for printing
    /// next to the mounted drive's letter.
    pub fn path_of(&self, id: u32) -> String {
        let mut parts = Vec::new();
        let mut cur = id;
        while cur != ROOT {
            parts.push(self.nodes[cur as usize].name.as_str());
            cur = self.nodes[cur as usize].parent;
        }
        parts.reverse();
        parts.join("\\")
    }

    /// Depth-first walk over all nodes. Used by search and statistics.
    pub fn walk(&self, visit: impl FnMut(u32, &Node)) {
        self.walk_from(ROOT, visit)
    }

    /// Walk over a subtree, starting at a given node.
    pub fn walk_from(&self, start: u32, mut visit: impl FnMut(u32, &Node)) {
        let mut stack = vec![start];
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id as usize];
            visit(id, node);
            stack.extend(node.children.iter().copied());
        }
    }

    /// Whether a node lies inside the subtree of `ancestor`.
    ///
    /// Walks up through parents rather than down through children: the tree is
    /// tens of levels deep, while a subtree can hold tens of thousands of
    /// nodes.
    pub fn is_under(&self, node: u32, ancestor: u32) -> bool {
        if ancestor == ROOT {
            return true;
        }
        let mut cur = node;
        loop {
            if cur == ancestor {
                return true;
            }
            if cur == ROOT {
                return false;
            }
            cur = self.nodes[cur as usize].parent;
        }
    }

    /// File nodes inside a subtree.
    pub fn file_ids_under(&self, start: u32) -> Vec<u32> {
        let mut out = Vec::new();
        self.walk_from(start, |id, node| {
            if !node.is_dir && node.entry.is_some() {
                out.push(id);
            }
        });
        out
    }

    /// Indices of all file nodes. The basis for search and verification.
    pub fn file_ids(&self) -> Vec<u32> {
        let mut out = Vec::new();
        self.walk(|id, node| {
            if !node.is_dir && node.entry.is_some() {
                out.push(id);
            }
        });
        out
    }

    /// The reverse map "backend entry index -> tree node".
    ///
    /// Needed by the block-wise 7z walk: entries arrive in block order, not
    /// tree order, and searching for the node by brute force for each of
    /// hundreds of thousands of entries will not do.
    pub fn entry_node_map(&self, entry_count: usize) -> Vec<Option<u32>> {
        let mut map = vec![None; entry_count];
        self.walk(|id, node| {
            if !node.is_dir {
                if let Some(e) = node.entry {
                    if let Some(slot) = map.get_mut(e as usize) {
                        *slot = Some(id);
                    }
                }
            }
        });
        map
    }

    /// The node matching the backend entry with the given index.
    pub fn node_of_entry(&self, entry: u32) -> Option<u32> {
        let mut found = None;
        self.walk(|id, node| {
            if node.entry == Some(entry) && !node.is_dir {
                found = Some(id);
            }
        });
        found
    }

    pub fn stats(&self) -> TreeStats {
        let mut stats = TreeStats::default();
        self.walk(|_, node| {
            if node.is_dir {
                stats.dirs += 1;
            } else {
                stats.files += 1;
                stats.total_uncompressed += node.size;
            }
        });
        // The root does not count as a directory.
        stats.dirs = stats.dirs.saturating_sub(1);
        stats
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TreeStats {
    pub files: u64,
    pub dirs: u64,
    pub total_uncompressed: u64,
}

fn add_node(
    nodes: &mut Vec<Node>,
    index: &mut FxHashMap<(u32, String), u32>,
    parent: u32,
    name: String,
    is_dir: bool,
) -> u32 {
    let lower = name.to_lowercase();
    let id = nodes.len() as u32;
    nodes.push(Node {
        name,
        parent,
        children: Vec::new(),
        entry: None,
        is_dir,
        size: 0,
        mtime: 0,
    });
    nodes[parent as usize].children.push(id);
    index.insert((parent, lower), id);
    id
}

fn get_or_create_dir(
    nodes: &mut Vec<Node>,
    index: &mut FxHashMap<(u32, String), u32>,
    parent: u32,
    name: &str,
) -> u32 {
    if let Some(&id) = index.get(&(parent, name.to_lowercase())) {
        if nodes[id as usize].is_dir {
            return id;
        }
        // The name is taken by a file — rare but legitimate: give the
        // directory a name of its own.
        let unique = unique_name(index, parent, name);
        return add_node(nodes, index, parent, unique, true);
    }
    add_node(nodes, index, parent, name.to_string(), true)
}

/// Finds a name that does not clash with its siblings, ignoring case.
fn unique_name(index: &FxHashMap<(u32, String), u32>, parent: u32, name: &str) -> String {
    if !index.contains_key(&(parent, name.to_lowercase())) {
        return name.to_string();
    }
    for n in 1.. {
        let candidate = disambiguate(name, n);
        if !index.contains_key(&(parent, candidate.to_lowercase())) {
            return candidate;
        }
    }
    unreachable!("the name counter cannot run out")
}

/// Case-insensitive comparison without allocations — it runs on every step
/// of a binary search, so a string-producing `to_lowercase()` is not an
/// option here.
fn cmp_ci(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().flat_map(char::to_lowercase);
    let mut bi = b.chars().flat_map(char::to_lowercase);
    loop {
        match (ai.next(), bi.next()) {
            (Some(x), Some(y)) => match x.cmp(&y) {
                Ordering::Equal => continue,
                other => return other,
            },
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(path: &str, is_dir: bool, size: u64) -> EntryMeta {
        EntryMeta {
            path: path.to_string(),
            is_dir,
            size,
            mtime: 0,
        }
    }

    fn tree_of(paths: &[&str]) -> Tree {
        let entries: Vec<EntryMeta> = paths.iter().map(|p| meta(p, false, 10)).collect();
        Tree::build(&entries)
    }

    #[test]
    fn synthesizes_missing_parent_directories() {
        // Not a single directory entry — just a deep file.
        let t = tree_of(&["a/b/c/d.txt"]);
        assert!(t.resolve("a").is_some());
        assert!(t.resolve("a\\b\\c").is_some());
        let file = t.resolve("a/b/c/d.txt").expect("the file must be found");
        assert!(!t.node(file).is_dir);
        assert_eq!(t.node(file).size, 10);
    }

    #[test]
    fn lookup_is_case_insensitive_including_cyrillic() {
        let t = tree_of(&["Документы/Отчёт.TXT"]);
        assert!(t.resolve("документы/отчёт.txt").is_some());
        assert!(t.resolve("ДОКУМЕНТЫ/ОТЧЁТ.txt").is_some());
    }

    #[test]
    fn duplicate_names_are_disambiguated_not_lost() {
        // Two different files differing only in case: on Windows that is one
        // name, so the second one must get a name of its own.
        let t = tree_of(&["dir/File.txt", "dir/file.txt"]);
        let dir = t.resolve("dir").unwrap();
        assert_eq!(t.node(dir).children.len(), 2, "no entry may get lost");
        assert!(t.resolve("dir/file~1.txt").is_some());
    }

    #[test]
    fn zip_slip_paths_stay_inside_root() {
        let t = tree_of(&["../../windows/system32/evil.dll"]);
        assert!(t.resolve("windows/system32/evil.dll").is_some());
        // No way out: `..` was dropped while parsing the path.
        assert_eq!(t.node(ROOT).children.len(), 1);
        assert_eq!(t.node(t.node(ROOT).children[0]).name, "windows");
    }

    #[test]
    fn directory_entries_are_directories() {
        let t = Tree::build(&[meta("docs/", true, 0)]);
        let docs = t.resolve("docs").expect("the directory must exist");
        assert!(t.node(docs).is_dir);
    }

    #[test]
    fn backslash_separator_is_understood() {
        // 7z stores paths with backslashes.
        let t = tree_of(&["dir\\sub\\file.txt"]);
        assert!(t.resolve("dir/sub/file.txt").is_some());
    }

    #[test]
    fn children_are_sorted_for_binary_search() {
        let t = tree_of(&["d/zebra.txt", "d/alpha.txt", "d/Middle.txt"]);
        let dir = t.resolve("d").unwrap();
        let names: Vec<&str> = t
            .node(dir)
            .children
            .iter()
            .map(|c| t.node(*c).name.as_str())
            .collect();
        assert_eq!(names, vec!["alpha.txt", "Middle.txt", "zebra.txt"]);
        // Sort order and lookup must agree.
        assert!(t.lookup_child(dir, "MIDDLE.TXT").is_some());
    }

    #[test]
    fn path_of_round_trips_through_resolve() {
        let t = tree_of(&["a/b/c.txt"]);
        let id = t.resolve("a/b/c.txt").unwrap();
        assert_eq!(t.path_of(id), "a\\b\\c.txt");
        assert_eq!(t.resolve(&t.path_of(id)), Some(id));
    }

    #[test]
    fn stats_count_files_and_directories() {
        let t = tree_of(&["a/b/c.txt", "a/d.txt", "e.txt"]);
        let s = t.stats();
        assert_eq!(s.files, 3);
        assert_eq!(s.dirs, 2); // a and a/b
        assert_eq!(s.total_uncompressed, 30);
    }

    #[test]
    fn resolve_root_returns_root() {
        let t = tree_of(&["a.txt"]);
        assert_eq!(t.resolve("\\"), Some(ROOT));
        assert_eq!(t.resolve(""), Some(ROOT));
    }

    #[test]
    fn file_ids_lists_only_files() {
        let t = tree_of(&["a/b/c.txt", "a/d.txt"]);
        assert_eq!(t.file_ids().len(), 2);
    }

    #[test]
    fn is_under_recognises_subtree_membership() {
        let t = tree_of(&["a/b/c.txt", "a/d.txt", "e/f.txt"]);
        let a = t.resolve("a").unwrap();
        let e = t.resolve("e").unwrap();
        let deep = t.resolve("a/b/c.txt").unwrap();

        assert!(t.is_under(deep, a));
        assert!(!t.is_under(deep, e));
        // The root contains everything.
        assert!(t.is_under(deep, ROOT));
        // A node lies inside itself.
        assert!(t.is_under(a, a));
    }

    #[test]
    fn file_ids_under_limits_to_subtree() {
        let t = tree_of(&["a/b/c.txt", "a/d.txt", "e/f.txt"]);
        let a = t.resolve("a").unwrap();
        assert_eq!(t.file_ids_under(a).len(), 2);
        assert_eq!(t.file_ids_under(ROOT).len(), 3);
    }

    #[test]
    fn node_of_entry_finds_the_file_node() {
        let t = tree_of(&["a/b.txt", "c.txt"]);
        let id = t.node_of_entry(1).expect("the second entry must be found");
        assert_eq!(t.node(id).name, "c.txt");
    }
}
