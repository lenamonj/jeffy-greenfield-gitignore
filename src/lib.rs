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
    // dir.c add_patterns_from_buffer skips a UTF-8 BOM at the head of
    // every ignore source; a BOM anywhere else stays literal bytes.
    let content = content.strip_prefix(b"\xef\xbb\xbf".as_slice()).unwrap_or(content);
    // Ignore sources are hand-authored text; treat undecodable bytes as
    // lines that match nothing rather than erroring, which is git's
    // effective posture toward malformed patterns.
    let text = String::from_utf8_lossy(content);
    text.split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .filter_map(parse_line)
        .collect()
}

/// ASCII classification for a POSIX bracket class name, per git
/// wildmatch.c; None for an unknown name, which makes the whole pattern
/// match nothing. `c` arrives already case-folded; under `icase` the
/// upper class also accepts lowercase, exactly as WM_CASEFOLD does.
fn posix_class_matches(name: &[u8], c: u8, icase: bool) -> Option<bool> {
    let m = match name {
        b"alnum" => c.is_ascii_alphanumeric(),
        b"alpha" => c.is_ascii_alphabetic(),
        b"blank" => c == b' ' || c == b'\t',
        b"cntrl" => c.is_ascii_control(),
        b"digit" => c.is_ascii_digit(),
        b"graph" => c.is_ascii_graphic(),
        b"lower" => c.is_ascii_lowercase(),
        b"print" => c == b' ' || c.is_ascii_graphic(),
        b"punct" => c.is_ascii_punctuation(),
        // git classifies via its own sane-ctype, not libc: GIT_SPACE is
        // exactly space, tab, LF, CR - VT and FF are not isspace there.
        b"space" => matches!(c, b' ' | b'\t' | b'\n' | b'\r'),
        b"upper" => c.is_ascii_uppercase() || (icase && c.is_ascii_lowercase()),
        b"xdigit" => c.is_ascii_hexdigit(),
        _ => return None,
    };
    Some(m)
}

/// Parse a bracket expression starting at `p[0] == b'['` and test `c`
/// against it. Returns (bytes consumed, matched-after-negation), or None
/// for an unclosed class or unknown POSIX class name, which matches
/// nothing, matching git's posture toward malformed patterns. `!` and `^`
/// both negate; a `]` first in the member list is literal; `a-z` ranges,
/// `[:name:]` POSIX classes, and backslash escapes apply. `c` arrives
/// already case-folded; per wildmatch, literal members are never folded
/// (an uppercase member is dead under icase) while ranges also try the
/// upcased text byte.
fn class_match(p: &[u8], c: u8, icase: bool) -> Option<(usize, bool)> {
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
            let lo = prev.unwrap();
            if lo <= c && c <= hi {
                matched = true;
            } else if icase && c.is_ascii_lowercase() {
                let cu = c.to_ascii_uppercase();
                if lo <= cu && cu <= hi {
                    matched = true;
                }
            }
            prev = None;
            i += 1;
            continue;
        }
        if raw == b'[' && p.get(i + 1) == Some(&b':') {
            // POSIX named class, per wildmatch.c: scan to the first ]
            // after [:; without a ":]" ending there, [ stays an ordinary
            // member and the name characters parse as members too.
            match p[i + 2..].iter().position(|b| *b == b']') {
                Some(off) => {
                    let j = i + 2 + off;
                    if j > i + 2 && p[j - 1] == b':' {
                        match posix_class_matches(&p[i + 2..j - 1], c, icase) {
                            Some(m) => {
                                if m {
                                    matched = true;
                                }
                                prev = None;
                                i = j + 1;
                                continue;
                            }
                            None => return None,
                        }
                    }
                }
                None => return None,
            }
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
/// whole path segment (preceded by start or `/`, followed by end, `/`, or
/// `\/`), and `**/` also matches zero directories - the escaped form `**\/`
/// crosses slashes but has no zero-directory reading; any other star run
/// behaves as one plain `*`, per git wildmatch under WM_PATHNAME.
/// Under `icase` (WM_CASEFOLD, git's core.ignoreCase), text bytes and
/// plain pattern literals fold to lowercase; escaped literals and bracket
/// members are read outside wildmatch's fold and stay exact.
/// Recursive backtracking; corpus-scale patterns keep it cheap.
fn glob_match(body: &[u8], text: &[u8], icase: bool) -> bool {
    let fold = |b: u8| if icase { b.to_ascii_lowercase() } else { b };
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
                let at_seg_end = run_end == body.len()
                    || body[run_end] == b'/'
                    || (body[run_end] == b'\\' && body.get(run_end + 1) == Some(&b'/'));
                let match_slash = run_end - pi >= 2 && at_seg_start && at_seg_end;
                let rest = &body[run_end..];
                if match_slash && rest.first() == Some(&b'/') {
                    // `**/`: the zero-directory reading skips the slash too.
                    // wildmatch grants it only to the literal `/`, never `\/`.
                    if glob_match(&rest[1..], &text[ti..], icase) {
                        return true;
                    }
                }
                if rest.is_empty() {
                    return match_slash || !text[ti..].contains(&b'/');
                }
                let mut k = ti;
                loop {
                    if glob_match(rest, &text[k..], icase) {
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
                match class_match(&body[pi..], fold(text[ti]), icase) {
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
                if ti >= text.len() || fold(text[ti]) != body[pi + 1] {
                    return false;
                }
                pi += 2;
                ti += 1;
            }
            ch => {
                if ti >= text.len() || fold(text[ti]) != fold(ch) {
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

fn pattern_matches(p: &Pattern, rel: &str, is_dir: bool, icase: bool) -> bool {
    if p.dir_only && !is_dir {
        return false;
    }
    if !p.anchored {
        return glob_match(p.body.as_bytes(), basename(rel).as_bytes(), icase);
    }
    // Port of git dir.c match_pathname: the literal prefix is compared
    // outright (case-insensitively under icase, per strncmp_icase) and
    // stripped before globbing. The strip is semantic, not just a fast
    // path: a `**` first wildcard then sits at the start of what
    // wildmatch sees and crosses slashes even when the prefix ends
    // mid-segment, so `x**/foo` matches `x/a/foo` and `xfoo`.
    let body = p.body.as_bytes();
    let rel = rel.as_bytes();
    let prefix = no_wildcard_len(body);
    let eq = |a: &[u8], b: &[u8]| {
        if icase {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    };
    if prefix > rel.len() || !eq(&body[..prefix], &rel[..prefix]) {
        return false;
    }
    if prefix == body.len() {
        return prefix == rel.len();
    }
    glob_match(&body[prefix..], &rel[prefix..], icase)
}

/// Last matching pattern in one source decides for that source; None when
/// nothing in the source matches.
fn last_match(patterns: &[Pattern], rel: &str, is_dir: bool, icase: bool) -> Option<bool> {
    let mut verdict = None;
    for p in patterns {
        if pattern_matches(p, rel, is_dir, icase) {
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
    /// Mirror of the repository's core.ignoreCase: when true, matching
    /// case-folds per wildmatch WM_CASEFOLD, exactly as git dir.c does,
    /// including the directory-prefix test that scopes each nested
    /// `.gitignore` (fspathncmp). git init sets it true on
    /// case-insensitive filesystems (Windows).
    ignore_case: bool,
    /// Root of the materialized repository on disk, when the caller has
    /// one. git reaches a nested `.gitignore` by opening
    /// `<query-prefix>/.gitignore` through the filesystem, so which
    /// sources apply to a query is decided by the volume's own name
    /// resolution - a fold (NTFS $UpCase) no portable function
    /// reproduces. With a root set, applicability asks the filesystem;
    /// without one, it falls back to string comparison (ASCII fold
    /// under `ignore_case`). Pattern text matching is unaffected either
    /// way: git matches the query's own spelling, never the disk's.
    repo_root: Option<std::path::PathBuf>,
}

impl Matcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether matching folds case, mirroring the repository's
    /// core.ignoreCase. Defaults to false, git's own default.
    pub fn set_ignore_case(&mut self, ignore_case: bool) {
        self.ignore_case = ignore_case;
    }

    /// Point the matcher at the materialized repository root so nested
    /// `.gitignore` applicability follows the filesystem's own name
    /// resolution, exactly as git finds sources by opening
    /// `<dir>/.gitignore` through the volume's fold. Without a root,
    /// applicability falls back to string comparison.
    pub fn set_repo_root(&mut self, root: impl Into<std::path::PathBuf>) {
        self.repo_root = Some(root.into());
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

    /// Whether a `.gitignore` in `dir` applies to `path`: `dir` must be a
    /// proper directory prefix. With a repo root set, the test is the
    /// filesystem's: the query prefix at the source's depth must resolve
    /// to the same directory as the source, under whatever fold the
    /// volume implements - the same lookup git performs when it opens
    /// `<prefix>/.gitignore`. Without a root, string comparison folding
    /// case under core.ignoreCase, as dir.c fspathncmp does for ASCII.
    fn dir_applies(&self, dir: &str, path: &str) -> bool {
        if dir.is_empty() {
            return true;
        }
        if let Some(root) = &self.repo_root {
            let depth = dir.split('/').count();
            let segs: Vec<&str> = path.split('/').collect();
            if segs.len() <= depth {
                return false;
            }
            // A prefix segment with a trailing dot or space reaches a
            // directory only through Win32 normalization stripping those
            // characters before the filesystem sees the name. git opens
            // paths in extended-length form where no such stripping
            // happens, so these spellings never reach a source for git
            // and must not here - unlike NTFS-level aliases (the case
            // fold, 8.3 short names), which are real directory entries
            // the resolution below honors.
            if segs[..depth].iter().any(|s| s.ends_with('.') || s.ends_with(' ')) {
                return false;
            }
            let prefix = root.join(segs[..depth].join("/"));
            let src = root.join(dir);
            return match (prefix.canonicalize(), src.canonicalize()) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            };
        }
        let (d, p) = (dir.as_bytes(), path.as_bytes());
        p.len() > d.len()
            && p[d.len()] == b'/'
            && if self.ignore_case {
                p[..d.len()].eq_ignore_ascii_case(d)
            } else {
                &p[..d.len()] == d
            }
    }

    /// Verdict for one path from the full source stack, or None when no
    /// pattern anywhere matches. `path` must not be judged through an
    /// excluded ancestor here; [`Matcher::is_ignored`] owns that walk.
    fn judge(&self, path: &str, is_dir: bool) -> Option<bool> {
        // Applicable tree .gitignore files, deepest directory first.
        let mut applicable: Vec<&(String, Vec<Pattern>)> = self
            .gitignores
            .iter()
            .filter(|(dir, _)| self.dir_applies(dir, path))
            .collect();
        applicable.sort_by_key(|(dir, _)| std::cmp::Reverse(dir.len()));
        for (dir, patterns) in applicable {
            // The query may reach this source through an alias spelling of
            // the prefix (case variant, 8.3 short name) whose byte length
            // differs from the stored canonical dir, so the source-relative
            // remainder is carved by segment count in the query's own
            // spelling, never by the stored dir's byte length.
            let rel = if dir.is_empty() {
                path
            } else {
                let depth = dir.split('/').count();
                match path.splitn(depth + 1, '/').nth(depth) {
                    Some(rest) => rest,
                    None => continue,
                }
            };
            if let Some(ignored) = last_match(patterns, rel, is_dir, self.ignore_case) {
                return Some(ignored);
            }
        }
        if let Some(ignored) = last_match(&self.info_exclude, path, is_dir, self.ignore_case) {
            return Some(ignored);
        }
        if let Some(ignored) = last_match(&self.excludes_file, path, is_dir, self.ignore_case) {
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
    fn head_bom_is_skipped_and_midfile_bom_stays_literal() {
        let patterns = parse_source(b"\xef\xbb\xbffoo\n\xef\xbb\xbfqux\n");
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].body, "foo", "head BOM stripped");
        assert_ne!(patterns[1].body, "qux", "mid-file BOM stays in the pattern");
    }

    #[test]
    fn escaped_hash_is_a_literal_pattern() {
        let p = parse_line("\\#hash").expect("escaped hash is a pattern");
        assert!(glob_match(p.body.as_bytes(), b"#hash", false));
        assert!(!glob_match(p.body.as_bytes(), b"hash", false));
    }

    #[test]
    fn glob_star_and_question_never_cross_slash() {
        assert!(glob_match(b"*.log", b"a.log", false));
        assert!(!glob_match(b"*.log", b"sub/a.log", false), "* must not cross /");
        assert!(glob_match(b"a?c", b"abc", false));
        assert!(!glob_match(b"a?c", b"a/c", false), "? must not cross /");
        assert!(glob_match(b"sub/*.txt", b"sub/a.txt", false));
        assert!(!glob_match(b"sub/*.txt", b"sub/d/b.txt", false));
    }

    #[test]
    fn glob_classes_and_escapes() {
        assert!(glob_match(b"[abc].txt", b"a.txt", false));
        assert!(!glob_match(b"[abc].txt", b"d.txt", false));
        assert!(glob_match(b"[!a-c].txt", b"d.txt", false));
        assert!(!glob_match(b"[!a-c].txt", b"a.txt", false));
        assert!(glob_match(b"[]-]x", b"]x", false), "leading ] is a literal member");
        assert!(glob_match(b"[]-]x", b"-x", false));
        assert!(!glob_match(b"[]-]x", b"ax", false));
        assert!(glob_match(b"\\*.lit", b"*.lit", false), "escaped star is literal");
        assert!(!glob_match(b"\\*.lit", b"a.lit", false));
        assert!(!glob_match(b"[abc", b"a", false), "unclosed class matches nothing");
    }

    #[test]
    fn globstar_is_special_only_as_a_whole_segment() {
        assert!(glob_match(b"**/foo", b"foo", false), "leading **/ matches zero dirs");
        assert!(glob_match(b"**/foo", b"a/b/foo", false));
        assert!(!glob_match(b"**/foo", b"xfoo", false));
        assert!(glob_match(b"a/**/b", b"a/b", false), "infix /**/ matches zero dirs");
        assert!(glob_match(b"a/**/b", b"a/x/y/b", false));
        assert!(glob_match(b"abc/**", b"abc/d/e", false), "trailing /** swallows depth");
        assert!(!glob_match(b"abc/**", b"abc", false), "trailing /** needs content");
        assert!(glob_match(b"**", b"a", false), "bare ** matches anything");
        assert!(glob_match(b"a**b", b"axyb", false), "adjacent ** is a plain *");
        assert!(!glob_match(b"a**b", b"a/b", false), "adjacent ** must not cross /");
        assert!(!glob_match(b"x**/foo", b"x/a/foo", false), "wildmatch alone: not special");
        assert!(glob_match(b"x**/foo", b"xa/foo", false));
        assert!(glob_match(br"a/**\/b", b"a/x/b", false), "escaped slash ends the segment");
        assert!(glob_match(br"a/**\/b", b"a/x/y/b", false), "escaped-slash ** crosses depth");
        assert!(!glob_match(br"a/**\/b", b"a/b", false), "no zero-dir reading for **\\/");
    }

    #[test]
    fn prefix_strip_makes_first_globstar_special() {
        // dir.c match_pathname strips the literal prefix before wildmatch,
        // so at the pattern level x**/foo is x + **/foo, not x*/foo.
        let p = parse_line("x**/foo").expect("pattern parses");
        assert!(pattern_matches(&p, "x/a/foo", false, false), "strip exposes leading **");
        assert!(pattern_matches(&p, "xfoo", false, false), "**/ then matches zero dirs");
        assert!(pattern_matches(&p, "xa/foo", false, false));
        assert!(!pattern_matches(&p, "y/a/foo", false, false), "literal prefix must match");
    }

    #[test]
    fn posix_classes_and_casefold() {
        assert!(glob_match(b"[[:digit:]].txt", b"5.txt", false));
        assert!(!glob_match(b"[[:digit:]].txt", b"d].txt", false));
        assert!(!glob_match(b"[[:bogus:]]z", b"5z", false), "unknown class matches nothing");
        assert!(glob_match(b"[[:digit]w", b"dw", false), "no :] ending falls back to a literal set");
        assert!(!glob_match(b"[[:digit]w", b"5w", false));
        assert!(glob_match(b"[![:space:]]y", b"ay", false));
        assert!(!glob_match(b"[![:space:]]y", b" y", false));
        // git sane-ctype GIT_SPACE members, all four, and only those.
        assert!(glob_match(b"a[[:space:]]b", b"a b", false));
        assert!(glob_match(b"a[[:space:]]b", b"a\tb", false));
        assert!(glob_match(b"a[[:space:]]b", b"a\nb", false));
        assert!(glob_match(b"a[[:space:]]b", b"a\rb", false));
        assert!(!glob_match(b"a[[:space:]]b", b"a\x0bb", false), "VT is not git isspace");
        assert!(!glob_match(b"a[[:space:]]b", b"a\x0cb", false), "FF is not git isspace");
        // Casefold semantics pinned live against the oracle (core.ignoreCase
        // repos): literals and ranges fold, bracket members do not.
        assert!(glob_match(b"K.txt", b"k.txt", true), "icase folds plain literals");
        assert!(!glob_match(b"K.txt", b"k.txt", false));
        assert!(glob_match(b"ra[m-p]q", b"raNq", true), "icase range tries the upcased byte");
        assert!(glob_match(b"u[A-C]v", b"ubv", true));
        assert!(!glob_match(b"x[K]y", b"xKy", true), "uppercase bracket member is dead under icase");
        assert!(glob_match(b"z[k]w", b"zKw", true));
        assert!(glob_match(b"[[:upper:]]x", b"ax", true), "upper accepts lower under icase");
        assert!(!glob_match(b"[[:upper:]]x", b"ax", false));
        assert!(glob_match(b"[[:lower:]]L", b"AL", true));
    }

    #[test]
    fn icase_folds_nested_gitignore_dir_prefix() {
        // Pinned live against the oracle: under core.ignoreCase, a
        // Sub/.gitignore applies to sub/foo.txt (dir.c fspathncmp).
        let mut m = Matcher::new();
        m.add_gitignore("Sub", b"foo.txt\n");
        m.set_ignore_case(true);
        assert!(m.is_ignored("sub/foo.txt", false), "prefix folds under icase");
        assert!(m.is_ignored("Sub/foo.txt", false));
        assert!(!m.is_ignored("sub/bar.txt", false));
        let mut cs = Matcher::new();
        cs.add_gitignore("Sub", b"foo.txt\n");
        assert!(!cs.is_ignored("sub/foo.txt", false), "case-sensitive default unchanged");
    }

    #[test]
    fn repo_root_applicability_asks_the_filesystem() {
        // With a root set, dir_applies resolves both the source dir and
        // the query prefix through the real filesystem, so cross-case
        // reach equals the volume's fold (ASCII variants fold on NTFS,
        // stay distinct on case-sensitive filesystems); a nonexistent
        // prefix never applies. The pure fallback keeps its own pinned
        // test above.
        let base = std::env::temp_dir().join(format!("gitignore-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("Sub")).unwrap();
        let mut m = Matcher::new();
        m.set_ignore_case(true);
        m.set_repo_root(&base);
        m.add_gitignore("Sub", b"foo.txt\n");
        let volume_folds =
            base.join("sub").canonicalize().ok() == base.join("Sub").canonicalize().ok();
        let cross_case = m.is_ignored("sub/foo.txt", false);
        let exact = m.is_ignored("Sub/foo.txt", false);
        let ghost = m.is_ignored("Ghost/foo.txt", false);
        let _ = std::fs::remove_dir_all(&base);
        assert_eq!(cross_case, volume_folds, "reach equals the volume's fold");
        assert!(exact, "exact spelling reaches the source");
        assert!(!ghost, "a nonexistent prefix resolves nowhere and never applies");
    }

    #[test]
    fn alias_prefix_with_different_byte_length_is_carved_by_segments() {
        // An 8.3 short name (LONGDI~1 for longdirectoryname) is an
        // NTFS-level alias git's own opens honor; its byte length differs
        // from the stored canonical dir, so the source-relative remainder
        // must be carved by segment count - the old byte-offset slice
        // panicked here. 8.3 generation is per-volume, so the expectation
        // branches on whether the alias actually resolves.
        let base = std::env::temp_dir().join(format!("gitignore-alias-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("longdirectoryname")).unwrap();
        let mut m = Matcher::new();
        m.set_ignore_case(true);
        m.set_repo_root(&base);
        m.add_gitignore("longdirectoryname", b"y.txt\n");
        let alias_resolves = base.join("LONGDI~1").canonicalize().ok()
            == base.join("longdirectoryname").canonicalize().ok();
        let via_alias = m.is_ignored("LONGDI~1/y.txt", false);
        let via_alias_unmatched = m.is_ignored("LONGDI~1/z.txt", false);
        let exact = m.is_ignored("longdirectoryname/y.txt", false);
        let _ = std::fs::remove_dir_all(&base);
        assert_eq!(via_alias, alias_resolves, "alias reach equals the volume's resolution");
        assert!(!via_alias_unmatched, "carved remainder must be the real basename");
        assert!(exact, "exact spelling reaches the source");
    }

    #[test]
    fn win32_normalization_aliases_never_reach_a_source() {
        // Trailing-dot and trailing-space prefix spellings resolve only
        // through Win32 normalization, which git's extended-length opens
        // never perform - they must not apply on any platform (on POSIX
        // filesystems no such directory exists, so the resolution itself
        // also answers no). The exact spelling stays live.
        let base = std::env::temp_dir().join(format!("gitignore-w32-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("sub")).unwrap();
        let mut m = Matcher::new();
        m.set_ignore_case(true);
        m.set_repo_root(&base);
        m.add_gitignore("sub", b"foo.txt\n");
        let dot = m.is_ignored("sub./foo.txt", false);
        let space = m.is_ignored("sub /foo.txt", false);
        let exact = m.is_ignored("sub/foo.txt", false);
        let _ = std::fs::remove_dir_all(&base);
        assert!(!dot, "trailing-dot prefix spelling must not reach the source");
        assert!(!space, "trailing-space prefix spelling must not reach the source");
        assert!(exact, "exact spelling stays live");
    }

    #[test]
    fn trailing_space_trim_respects_backslash() {
        assert_eq!(trim_trailing_spaces("a.txt   "), "a.txt");
        assert_eq!(trim_trailing_spaces("b.txt\\ "), "b.txt\\ ");
        assert_eq!(trim_trailing_spaces("c.txt \\ "), "c.txt \\ ");
        assert_eq!(trim_trailing_spaces("d.txt\t"), "d.txt\t", "tabs are not trimmed");
    }
}
