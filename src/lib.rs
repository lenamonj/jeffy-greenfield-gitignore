//! Gitignore pattern matcher, built greenfield and judged differentially
//! against `git check-ignore`. PLAN.md carries the oracle contract; where
//! this implementation and git disagree, git is right by definition.
//!
//! The harness feeds a [`Matcher`] every ignore source of a materialized
//! case repository, then asks it to judge repo-relative paths. Precedence
//! (deeper .gitignore over shallower, tree over info/exclude over
//! core.excludesFile) is the matcher's job, not the caller's.

/// One parsed, non-comment, non-blank gitignore line.
struct Pattern {
    negated: bool,
    dir_only: bool,
    /// A separator at the beginning or middle anchors the pattern to the
    /// directory of its .gitignore; otherwise it matches basenames at any
    /// depth below it.
    anchored: bool,
    /// Pattern text after stripping `!`, a trailing `/`, and a leading `/`.
    /// Backslash escapes are kept and interpreted at match time.
    body: String,
}

/// Port of git dir.c trim_trailing_spaces: strip trailing spaces, except
/// spaces (or anything) preceded by a backslash; a lone trailing backslash
/// leaves the line untouched.
fn trim_trailing_spaces(line: &str) -> &str {
    let b = line.as_bytes();
    let mut last_space: Option<usize> = None;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b' ' => {
                if last_space.is_none() {
                    last_space = Some(i);
                }
            }
            b'\\' => {
                i += 1;
                if i >= b.len() {
                    return line;
                }
                last_space = None;
            }
            _ => last_space = None,
        }
        i += 1;
    }
    match last_space {
        Some(p) => &line[..p],
        None => line,
    }
}

/// Parse one raw gitignore line. Blank lines and `#` comments yield no
/// pattern; `\#` and `\!` escapes survive into the body.
fn parse_line(raw: &str) -> Option<Pattern> {
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    let line = trim_trailing_spaces(raw);
    if line.is_empty() {
        return None;
    }
    let (negated, rest) = match line.strip_prefix('!') {
        Some(r) => (true, r),
        None => (false, line),
    };
    let (dir_only, rest) = match rest.strip_suffix('/') {
        Some(r) => (true, r),
        None => (false, rest),
    };
    let anchored = rest.contains('/');
    let body = rest.strip_prefix('/').unwrap_or(rest);
    Some(Pattern {
        negated,
        dir_only,
        anchored,
        body: body.to_string(),
    })
}

fn parse_source(content: &[u8]) -> Vec<Pattern> {
    // Ignore sources are hand-authored text; treat undecodable bytes as
    // lines that match nothing rather than erroring, which is git's
    // effective posture toward malformed patterns.
    let text = String::from_utf8_lossy(content);
    text.split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .filter_map(parse_line)
        .collect()
}

/// Literal comparison of a pattern body against a candidate, interpreting
/// backslash escapes in the body. Wildcard metacharacters are compared
/// literally until the wildcard inventory rows implement them. A lone
/// trailing backslash matches nothing, as in fnmatch.
fn lit_match(body: &str, candidate: &str) -> bool {
    let p = body.as_bytes();
    let c = candidate.as_bytes();
    let mut i = 0;
    let mut j = 0;
    while i < p.len() {
        let ch = if p[i] == b'\\' {
            i += 1;
            if i >= p.len() {
                return false;
            }
            p[i]
        } else {
            p[i]
        };
        if j >= c.len() || c[j] != ch {
            return false;
        }
        i += 1;
        j += 1;
    }
    j == c.len()
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn pattern_matches(p: &Pattern, rel: &str, is_dir: bool) -> bool {
    if p.dir_only && !is_dir {
        return false;
    }
    if p.anchored {
        lit_match(&p.body, rel)
    } else {
        lit_match(&p.body, basename(rel))
    }
}

/// Last matching pattern in one source decides for that source; None when
/// nothing in the source matches.
fn last_match(patterns: &[Pattern], rel: &str, is_dir: bool) -> Option<bool> {
    let mut verdict = None;
    for p in patterns {
        if pattern_matches(p, rel, is_dir) {
            verdict = Some(!p.negated);
        }
    }
    verdict
}

/// Accumulates the ignore sources of one repository, then judges paths.
///
/// Sources rank, strongest first: tree `.gitignore` files (deeper directory
/// wins over shallower), then `.git/info/exclude`, then `core.excludesFile`.
#[derive(Default)]
pub struct Matcher {
    /// (repo-relative directory, "" for root; parsed patterns).
    gitignores: Vec<(String, Vec<Pattern>)>,
    info_exclude: Vec<Pattern>,
    excludes_file: Vec<Pattern>,
}

impl Matcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a `.gitignore` found in `dir` (repo-relative, `/`-separated,
    /// "" for the repository root).
    pub fn add_gitignore(&mut self, dir: &str, content: &[u8]) {
        self.gitignores.push((dir.to_string(), parse_source(content)));
    }

    /// Register the repository's `.git/info/exclude` content.
    pub fn set_info_exclude(&mut self, content: &[u8]) {
        self.info_exclude = parse_source(content);
    }

    /// Register the content of the file named by `core.excludesFile`.
    pub fn set_excludes_file(&mut self, content: &[u8]) {
        self.excludes_file = parse_source(content);
    }

    /// Verdict for one path from the full source stack, or None when no
    /// pattern anywhere matches. `path` must not be judged through an
    /// excluded ancestor here; [`Matcher::is_ignored`] owns that walk.
    fn judge(&self, path: &str, is_dir: bool) -> Option<bool> {
        // Applicable tree .gitignore files, deepest directory first.
        let mut applicable: Vec<&(String, Vec<Pattern>)> = self
            .gitignores
            .iter()
            .filter(|(dir, _)| {
                dir.is_empty() || path.strip_prefix(dir.as_str()).is_some_and(|r| r.starts_with('/'))
            })
            .collect();
        applicable.sort_by_key(|(dir, _)| std::cmp::Reverse(dir.len()));
        for (dir, patterns) in applicable {
            let rel = if dir.is_empty() { path } else { &path[dir.len() + 1..] };
            if let Some(ignored) = last_match(patterns, rel, is_dir) {
                return Some(ignored);
            }
        }
        if let Some(ignored) = last_match(&self.info_exclude, path, is_dir) {
            return Some(ignored);
        }
        if let Some(ignored) = last_match(&self.excludes_file, path, is_dir) {
            return Some(ignored);
        }
        None
    }

    /// Whether git would ignore `rel_path` (repo-relative, `/`-separated).
    /// `is_dir` states whether the path exists as a directory on disk.
    /// A path below an excluded directory is excluded regardless of any
    /// negation naming the path itself.
    pub fn is_ignored(&self, rel_path: &str, is_dir: bool) -> bool {
        let segs: Vec<&str> = rel_path.split('/').collect();
        for i in 1..segs.len() {
            let ancestor = segs[..i].join("/");
            if self.judge(&ancestor, true) == Some(true) {
                return true;
            }
        }
        self.judge(rel_path, is_dir) == Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_and_comment_lines_yield_no_pattern() {
        assert!(parse_line("").is_none());
        assert!(parse_line("# a comment").is_none());
        assert!(parse_line("   ").is_none(), "spaces-only trims to blank");
    }

    #[test]
    fn escaped_hash_is_a_literal_pattern() {
        let p = parse_line("\\#hash").expect("escaped hash is a pattern");
        assert!(lit_match(&p.body, "#hash"));
        assert!(!lit_match(&p.body, "hash"));
    }

    #[test]
    fn trailing_space_trim_respects_backslash() {
        assert_eq!(trim_trailing_spaces("a.txt   "), "a.txt");
        assert_eq!(trim_trailing_spaces("b.txt\\ "), "b.txt\\ ");
        assert_eq!(trim_trailing_spaces("c.txt \\ "), "c.txt \\ ");
        assert_eq!(trim_trailing_spaces("d.txt\t"), "d.txt\t", "tabs are not trimmed");
    }
}
