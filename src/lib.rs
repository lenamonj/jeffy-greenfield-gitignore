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

/// Parse a bracket expression starting at `p[0] == b'['` and test `c`
/// against it. Returns (bytes consumed, matched-after-negation), or None
/// for an unclosed class, which matches nothing, matching git's posture
/// toward malformed patterns. `!` and `^` both negate; a `]` first in the
/// member list is literal; `a-z` ranges and backslash escapes apply.
fn class_match(p: &[u8], c: u8) -> Option<(usize, bool)> {
    let mut i = 1;
    let mut negate = false;
    if i < p.len() && (p[i] == b'!' || p[i] == b'^') {
        negate = true;
        i += 1;
    }
    let mut matched = false;
    let mut prev: Option<u8> = None;
    let mut first = true;
    loop {
        if i >= p.len() {
            return None;
        }
        let raw = p[i];
        if raw == b']' && !first {
            i += 1;
            break;
        }
        first = false;
        if raw == b'-' && prev.is_some() && i + 1 < p.len() && p[i + 1] != b']' {
            i += 1;
            let hi = if p[i] == b'\\' {
                i += 1;
                if i >= p.len() {
                    return None;
                }
                p[i]
            } else {
                p[i]
            };
            if prev.unwrap() <= c && c <= hi {
                matched = true;
            }
            prev = None;
            i += 1;
            continue;
        }
        let lit = if raw == b'\\' {
            i += 1;
            if i >= p.len() {
                return None;
            }
            p[i]
        } else {
            raw
        };
        if lit == c {
            matched = true;
        }
        prev = Some(lit);
        i += 1;
    }
    Some((i, matched != negate))
}

/// Glob match of a pattern body against a candidate, with fnmatch
/// FNM_PATHNAME semantics as gitignore uses them: `*` and `?` and bracket
/// expressions never match `/`; `\x` is literal x; a lone trailing
/// backslash matches nothing. A `**` run crosses `/` only when it forms a
/// whole path segment (preceded by start or `/`, followed by end or `/`),
/// and `**/` also matches zero directories; any other star run behaves as
/// one plain `*`, per git wildmatch under WM_PATHNAME.
/// Recursive backtracking; corpus-scale patterns keep it cheap.
fn glob_match(body: &[u8], text: &[u8]) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    while pi < body.len() {
        match body[pi] {
            b'*' => {
                let mut run_end = pi;
                while run_end < body.len() && body[run_end] == b'*' {
                    run_end += 1;
                }
                let at_seg_start = pi == 0 || body[pi - 1] == b'/';
                let at_seg_end = run_end == body.len() || body[run_end] == b'/';
                let match_slash = run_end - pi >= 2 && at_seg_start && at_seg_end;
                let rest = &body[run_end..];
                if match_slash && !rest.is_empty() {
                    // `**/`: the zero-directory reading skips the slash too.
                    if glob_match(&rest[1..], &text[ti..]) {
                        return true;
                    }
                }
                if rest.is_empty() {
                    return match_slash || !text[ti..].contains(&b'/');
                }
                let mut k = ti;
                loop {
                    if glob_match(rest, &text[k..]) {
                        return true;
                    }
                    if k >= text.len() || (!match_slash && text[k] == b'/') {
                        return false;
                    }
                    k += 1;
                }
            }
            b'?' => {
                if ti >= text.len() || text[ti] == b'/' {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
            b'[' => {
                if ti >= text.len() || text[ti] == b'/' {
                    return false;
                }
                match class_match(&body[pi..], text[ti]) {
                    Some((consumed, m)) => {
                        if !m {
                            return false;
                        }
                        pi += consumed;
                        ti += 1;
                    }
                    None => return false,
                }
            }
            b'\\' => {
                if pi + 1 >= body.len() {
                    return false;
                }
                if ti >= text.len() || text[ti] != body[pi + 1] {
                    return false;
                }
                pi += 2;
                ti += 1;
            }
            ch => {
                if ti >= text.len() || text[ti] != ch {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }
    }
    ti == text.len()
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Length of the wildcard-free prefix: bytes before the first `*`, `?`,
/// `[`, or `\`, per git dir.c simple_length.
fn no_wildcard_len(body: &[u8]) -> usize {
    body.iter()
        .position(|b| matches!(b, b'*' | b'?' | b'[' | b'\\'))
        .unwrap_or(body.len())
}

fn pattern_matches(p: &Pattern, rel: &str, is_dir: bool) -> bool {
    if p.dir_only && !is_dir {
        return false;
    }
    if !p.anchored {
        return glob_match(p.body.as_bytes(), basename(rel).as_bytes());
    }
    // Port of git dir.c match_pathname: the literal prefix is compared
    // outright and stripped before globbing. The strip is semantic, not
    // just a fast path: a `**` first wildcard then sits at the start of
    // what wildmatch sees and crosses slashes even when the prefix ends
    // mid-segment, so `x**/foo` matches `x/a/foo` and `xfoo`.
    let body = p.body.as_bytes();
    let rel = rel.as_bytes();
    let prefix = no_wildcard_len(body);
    if prefix > rel.len() || body[..prefix] != rel[..prefix] {
        return false;
    }
    if prefix == body.len() {
        return prefix == rel.len();
    }
    glob_match(&body[prefix..], &rel[prefix..])
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
        assert!(glob_match(p.body.as_bytes(), b"#hash"));
        assert!(!glob_match(p.body.as_bytes(), b"hash"));
    }

    #[test]
    fn glob_star_and_question_never_cross_slash() {
        assert!(glob_match(b"*.log", b"a.log"));
        assert!(!glob_match(b"*.log", b"sub/a.log"), "* must not cross /");
        assert!(glob_match(b"a?c", b"abc"));
        assert!(!glob_match(b"a?c", b"a/c"), "? must not cross /");
        assert!(glob_match(b"sub/*.txt", b"sub/a.txt"));
        assert!(!glob_match(b"sub/*.txt", b"sub/d/b.txt"));
    }

    #[test]
    fn glob_classes_and_escapes() {
        assert!(glob_match(b"[abc].txt", b"a.txt"));
        assert!(!glob_match(b"[abc].txt", b"d.txt"));
        assert!(glob_match(b"[!a-c].txt", b"d.txt"));
        assert!(!glob_match(b"[!a-c].txt", b"a.txt"));
        assert!(glob_match(b"[]-]x", b"]x"), "leading ] is a literal member");
        assert!(glob_match(b"[]-]x", b"-x"));
        assert!(!glob_match(b"[]-]x", b"ax"));
        assert!(glob_match(b"\\*.lit", b"*.lit"), "escaped star is literal");
        assert!(!glob_match(b"\\*.lit", b"a.lit"));
        assert!(!glob_match(b"[abc", b"a"), "unclosed class matches nothing");
    }

    #[test]
    fn globstar_is_special_only_as_a_whole_segment() {
        assert!(glob_match(b"**/foo", b"foo"), "leading **/ matches zero dirs");
        assert!(glob_match(b"**/foo", b"a/b/foo"));
        assert!(!glob_match(b"**/foo", b"xfoo"));
        assert!(glob_match(b"a/**/b", b"a/b"), "infix /**/ matches zero dirs");
        assert!(glob_match(b"a/**/b", b"a/x/y/b"));
        assert!(glob_match(b"abc/**", b"abc/d/e"), "trailing /** swallows depth");
        assert!(!glob_match(b"abc/**", b"abc"), "trailing /** needs content");
        assert!(glob_match(b"**", b"a"), "bare ** matches anything");
        assert!(glob_match(b"a**b", b"axyb"), "adjacent ** is a plain *");
        assert!(!glob_match(b"a**b", b"a/b"), "adjacent ** must not cross /");
        assert!(!glob_match(b"x**/foo", b"x/a/foo"), "wildmatch alone: not special");
        assert!(glob_match(b"x**/foo", b"xa/foo"));
    }

    #[test]
    fn prefix_strip_makes_first_globstar_special() {
        // dir.c match_pathname strips the literal prefix before wildmatch,
        // so at the pattern level x**/foo is x + **/foo, not x*/foo.
        let p = parse_line("x**/foo").expect("pattern parses");
        assert!(pattern_matches(&p, "x/a/foo", false), "strip exposes leading **");
        assert!(pattern_matches(&p, "xfoo", false), "**/ then matches zero dirs");
        assert!(pattern_matches(&p, "xa/foo", false));
        assert!(!pattern_matches(&p, "y/a/foo", false), "literal prefix must match");
    }

    #[test]
    fn trailing_space_trim_respects_backslash() {
        assert_eq!(trim_trailing_spaces("a.txt   "), "a.txt");
        assert_eq!(trim_trailing_spaces("b.txt\\ "), "b.txt\\ ");
        assert_eq!(trim_trailing_spaces("c.txt \\ "), "c.txt \\ ");
        assert_eq!(trim_trailing_spaces("d.txt\t"), "d.txt\t", "tabs are not trimmed");
    }
}
