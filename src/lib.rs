//! Gitignore pattern matcher, built greenfield and judged differentially
//! against `git check-ignore`. PLAN.md carries the oracle contract; where
//! this implementation and git disagree, git is right by definition.

/// Matches nothing yet. The twelve inventory-row tasks implement the real
/// semantics; the differential harness must first observe this failing.
#[derive(Default)]
pub struct Matcher;

impl Matcher {
    pub fn new() -> Self {
        Matcher
    }

    /// Whether git would ignore `rel_path` (repo-relative, `/`-separated).
    pub fn is_ignored(&self, _rel_path: &str, _is_dir: bool) -> bool {
        false
    }
}
