//! Gitignore pattern matcher, built greenfield and judged differentially
//! against `git check-ignore`. PLAN.md carries the oracle contract; where
//! this implementation and git disagree, git is right by definition.
//!
//! The harness feeds a [`Matcher`] every ignore source of a materialized
//! case repository, then asks it to judge repo-relative paths. Precedence
//! (deeper .gitignore over shallower, tree over info/exclude over
//! core.excludesFile) is the matcher's job, not the caller's.

/// Accumulates the ignore sources of one repository, then judges paths.
///
/// Sources rank, strongest first: tree `.gitignore` files (deeper directory
/// wins over shallower), then `.git/info/exclude`, then `core.excludesFile`.
/// The twelve inventory-row tasks implement the real matching semantics;
/// until they land, [`Matcher::is_ignored`] answers false for every path and
/// the differential harness must observe exactly that failing.
#[derive(Default)]
pub struct Matcher {
    /// (repo-relative directory of the .gitignore, "" for root; raw bytes).
    gitignores: Vec<(String, Vec<u8>)>,
    info_exclude: Option<Vec<u8>>,
    excludes_file: Option<Vec<u8>>,
}

impl Matcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a `.gitignore` found in `dir` (repo-relative, `/`-separated,
    /// "" for the repository root).
    pub fn add_gitignore(&mut self, dir: &str, content: &[u8]) {
        self.gitignores.push((dir.to_string(), content.to_vec()));
    }

    /// Register the repository's `.git/info/exclude` content.
    pub fn set_info_exclude(&mut self, content: &[u8]) {
        self.info_exclude = Some(content.to_vec());
    }

    /// Register the content of the file named by `core.excludesFile`.
    pub fn set_excludes_file(&mut self, content: &[u8]) {
        self.excludes_file = Some(content.to_vec());
    }

    /// Whether git would ignore `rel_path` (repo-relative, `/`-separated).
    /// `is_dir` states whether the path exists as a directory on disk.
    pub fn is_ignored(&self, _rel_path: &str, _is_dir: bool) -> bool {
        // Row tasks implement the semantics; sources are held but unread.
        let _ = (&self.gitignores, &self.info_exclude, &self.excludes_file);
        false
    }
}
