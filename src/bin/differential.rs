//! Differential driver: replays the frozen corpus, asks `git check-ignore`
//! for its verdict per the oracle contract in PLAN.md, and compares against
//! the matcher. Exits 0 only when every query of every case agrees; 1 on any
//! disagreement; 3 on a harness failure (oracle exit 128, malformed corpus,
//! unparseable oracle output, io errors). A harness failure is never a
//! verdict. `--strict` aborts on the first harness failure instead of
//! continuing to the summary.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use gitignore_matcher::Matcher;

/// Isolation env for every git invocation, per the pre-registered oracle
/// contract: the machine's global and system config must never leak into a
/// verdict.
const ORACLE_ENV: [(&str, &str); 3] = [
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ("GIT_CONFIG_SYSTEM", "/dev/null"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
];

struct Args {
    corpus: PathBuf,
    strict: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut corpus = None;
    let mut strict = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--corpus" => {
                let v = it.next().ok_or("--corpus requires a directory argument")?;
                corpus = Some(PathBuf::from(v));
            }
            "--strict" => strict = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let corpus = corpus.ok_or("--corpus <dir> is required")?;
    Ok(Args { corpus, strict })
}

enum Rec {
    File(String, Vec<u8>),
    Dir(String),
    InfoExclude(Vec<u8>),
    ExcludesFile(Vec<u8>),
    Query(String),
}

/// Parse one NUL-terminated token-stream case file (format documented in
/// corpus_gen.rs).
fn parse_case(bytes: &[u8], path: &Path) -> Result<Vec<Rec>, String> {
    let ctx = path.display();
    if bytes.is_empty() {
        return Err(format!("{ctx}: empty case file"));
    }
    if *bytes.last().unwrap() != 0 {
        return Err(format!("{ctx}: does not end with NUL"));
    }
    let mut toks: Vec<&[u8]> = bytes.split(|b| *b == 0).collect();
    toks.pop(); // artifact of the final terminator, not a token
    let utf8 = |t: &[u8]| -> Result<String, String> {
        String::from_utf8(t.to_vec()).map_err(|_| format!("{ctx}: non-UTF-8 token"))
    };
    let mut recs = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let op = toks[i];
        let need = match op {
            b"F" => 2,
            b"D" | b"E" | b"X" | b"Q" => 1,
            _ => return Err(format!("{ctx}: unknown op at token {i}")),
        };
        if i + need >= toks.len() {
            return Err(format!("{ctx}: truncated record at token {i}"));
        }
        let rec = match op {
            b"F" => Rec::File(utf8(toks[i + 1])?, toks[i + 2].to_vec()),
            b"D" => Rec::Dir(utf8(toks[i + 1])?),
            b"E" => Rec::InfoExclude(toks[i + 1].to_vec()),
            b"X" => Rec::ExcludesFile(toks[i + 1].to_vec()),
            b"Q" => Rec::Query(utf8(toks[i + 1])?),
            _ => unreachable!(),
        };
        recs.push(rec);
        i += 1 + need;
    }
    Ok(recs)
}

#[derive(Debug)]
struct OracleRow {
    ignored: bool,
    /// git's own reasoning: "pattern P at S:L", or "no pattern matched".
    evidence: String,
}

/// Run the oracle once for a case: `git check-ignore -z -v --non-matching
/// --no-index --stdin` inside `repo`, all queries NUL-delimited on stdin.
/// Exit 0 and 1 are the only verdict-bearing statuses; anything else
/// (notably 128) is a harness failure reported as Err.
fn run_oracle(repo: &Path, queries: &[String]) -> Result<Vec<OracleRow>, String> {
    let mut child = Command::new("git")
        .args(["check-ignore", "-z", "-v", "--non-matching", "--no-index", "--stdin"])
        .current_dir(repo)
        .envs(ORACLE_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawning git check-ignore: {e}"))?;
    let mut input = Vec::new();
    for q in queries {
        input.extend_from_slice(q.as_bytes());
        input.push(0);
    }
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(&input)
        .map_err(|e| format!("writing oracle stdin: {e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("waiting for oracle: {e}"))?;
    match out.status.code() {
        Some(0) | Some(1) => {}
        other => {
            return Err(format!(
                "oracle exit {other:?} (harness failure, never a verdict): {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }
    let mut toks: Vec<&[u8]> = out.stdout.split(|b| *b == 0).collect();
    match toks.pop() {
        Some(last) if last.is_empty() => {}
        _ => return Err("oracle output does not end with NUL".to_string()),
    }
    if toks.len() != queries.len() * 4 {
        return Err(format!(
            "oracle output has {} fields for {} queries (want 4 per query)",
            toks.len(),
            queries.len()
        ));
    }
    let mut rows = Vec::new();
    for (i, q) in queries.iter().enumerate() {
        let source = String::from_utf8_lossy(toks[i * 4]).into_owned();
        let lineno = String::from_utf8_lossy(toks[i * 4 + 1]).into_owned();
        let pattern = String::from_utf8_lossy(toks[i * 4 + 2]).into_owned();
        let echoed = toks[i * 4 + 3];
        if echoed != q.as_bytes() {
            return Err(format!(
                "oracle echoed path {:?} for query {q:?} (order broken)",
                String::from_utf8_lossy(echoed)
            ));
        }
        // The verdict rule from the oracle contract: empty pattern means not
        // ignored; a negation pattern match also means not ignored.
        let (ignored, evidence) = if pattern.is_empty() {
            (false, "no pattern matched".to_string())
        } else {
            (
                !pattern.starts_with('!'),
                format!("pattern {pattern:?} at {source}:{lineno}"),
            )
        };
        rows.push(OracleRow { ignored, evidence });
    }
    Ok(rows)
}

/// check-ignore normalizes each pathspec before matching while the -z
/// output echoes it verbatim, so the matcher must receive the normalized
/// path even though the oracle is fed, and echoes, the original. The
/// normalization is git's textual pass over a repo-relative pathspec:
/// "." segments and duplicate slashes drop, ".." consumes the previous
/// segment with no existence check, and one trailing slash survives.
/// Shapes this port cannot resolve are returned unchanged and left to
/// the oracle, which rejects them with exit 128, turning the case into
/// a harness failure: a ".." climbing above the repo root, a leading
/// "/" (outside the repository), and a query normalizing to nothing.
fn normalize_query(q: &str) -> String {
    if q.starts_with('/') {
        return q.to_string();
    }
    let mut segs: Vec<&str> = Vec::new();
    for seg in q.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if segs.pop().is_none() {
                    return q.to_string();
                }
            }
            s => segs.push(s),
        }
    }
    if segs.is_empty() {
        return q.to_string();
    }
    let mut out = segs.join("/");
    if q.ends_with('/') {
        out.push('/');
    }
    out
}

fn git_in(dir: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .envs(ORACLE_ENV)
        .output()
        .map_err(|e| format!("spawning git {args:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed ({:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// Materialize a case as its own temporary repository (never inside this
/// project's tree) and build the matcher from the same sources. The
/// matcher mirrors the repository's core.ignoreCase, exactly as dir.c
/// reads it: git init sets it true on case-insensitive filesystems, and
/// the oracle's matching folds case there - including which files count
/// as ignore sources, since the filesystem name lookup folds too, and
/// which files exist at all, since the filesystem folds colliding names
/// on write: case-variant F records land in one file (last content,
/// first name) and the source model folds with them.
fn materialize(case_dir: &Path, recs: &[Rec]) -> Result<(PathBuf, Matcher, Vec<String>), String> {
    let repo = case_dir.join("repo");
    std::fs::create_dir_all(&repo).map_err(|e| format!("mkdir {}: {e}", repo.display()))?;
    git_in(case_dir, &["init", "-q", "repo"])?;
    let ignore_case = {
        let out = Command::new("git")
            .args(["config", "--get", "core.ignorecase"])
            .current_dir(&repo)
            .envs(ORACLE_ENV)
            .output()
            .map_err(|e| format!("spawning git config: {e}"))?;
        // Exit 1 means unset, which is git's case-sensitive default.
        String::from_utf8_lossy(&out.stdout).trim() == "true"
    };
    let mut matcher = Matcher::new();
    matcher.set_ignore_case(ignore_case);
    // Ignore sources are collected, not registered per record: the write
    // below folds colliding paths - case-variants on an ignore_case
    // filesystem, or the same path recorded twice anywhere - into one file
    // holding the last content under the first name. The model must hold
    // exactly one source per surviving file, so a colliding record replaces
    // the earlier source's content instead of layering a second source the
    // disk cannot hold.
    let mut sources: Vec<(String, Vec<u8>)> = Vec::new();
    let mut source_index: BTreeMap<String, usize> = BTreeMap::new();
    let mut queries = Vec::new();
    for rec in recs {
        match rec {
            Rec::File(path, content) => {
                let full = repo.join(path);
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
                }
                std::fs::write(&full, content).map_err(|e| format!("write {path}: {e}"))?;
                // git discovers ignore files by asking the filesystem for
                // "<dir>/.gitignore"; on an ignore_case repo that lookup is
                // case-insensitive, so a file written as ".GitIgnore" is a
                // live source there and must reach the matcher too.
                let (dir, name) = match path.rsplit_once('/') {
                    Some((d, n)) => (d, n),
                    None => ("", path.as_str()),
                };
                let is_source = if ignore_case {
                    name.eq_ignore_ascii_case(".gitignore")
                } else {
                    name == ".gitignore"
                };
                if is_source {
                    let key = if ignore_case {
                        path.to_ascii_lowercase()
                    } else {
                        path.clone()
                    };
                    match source_index.get(&key) {
                        Some(&i) => sources[i].1 = content.clone(),
                        None => {
                            source_index.insert(key, sources.len());
                            sources.push((dir.to_string(), content.clone()));
                        }
                    }
                }
            }
            Rec::Dir(path) => {
                std::fs::create_dir_all(repo.join(path))
                    .map_err(|e| format!("mkdir {path}: {e}"))?;
            }
            Rec::InfoExclude(content) => {
                let info = repo.join(".git").join("info");
                std::fs::create_dir_all(&info)
                    .map_err(|e| format!("mkdir {}: {e}", info.display()))?;
                std::fs::write(info.join("exclude"), content)
                    .map_err(|e| format!("write info/exclude: {e}"))?;
                matcher.set_info_exclude(content);
            }
            Rec::ExcludesFile(content) => {
                let excl = case_dir.join("excludes.txt");
                std::fs::write(&excl, content).map_err(|e| format!("write excludes: {e}"))?;
                let fwd = excl.to_string_lossy().replace('\\', "/");
                git_in(&repo, &["config", "core.excludesFile", &fwd])?;
                matcher.set_excludes_file(content);
            }
            Rec::Query(q) => queries.push(q.clone()),
        }
    }
    for (dir, content) in &sources {
        matcher.add_gitignore(dir, content);
    }
    Ok((repo, matcher, queries))
}

fn collect_cases(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("reading {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("reading {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_cases(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "case") {
            out.push(path);
        }
    }
    Ok(())
}

#[derive(Default)]
struct SliceStats {
    cases: usize,
    queries: usize,
    disagreements: usize,
}

fn run(args: &Args) -> Result<ExitCode, String> {
    let mut case_files = Vec::new();
    collect_cases(&args.corpus, &mut case_files)?;
    case_files.sort();
    if case_files.is_empty() {
        return Err(format!(
            "no .case files under {} (an empty corpus must never read as green)",
            args.corpus.display()
        ));
    }
    let tmp_base = std::env::temp_dir().join(format!("gitignore-diff-{}", std::process::id()));
    if tmp_base.exists() {
        let _ = std::fs::remove_dir_all(&tmp_base);
    }
    let mut stats: BTreeMap<String, SliceStats> = BTreeMap::new();
    let mut harness_failures = 0usize;
    let mut total_disagreements = 0usize;
    for (idx, case_path) in case_files.iter().enumerate() {
        let slice = case_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "?".to_string());
        let case_res = (|| -> Result<usize, String> {
            let bytes = std::fs::read(case_path)
                .map_err(|e| format!("{}: {e}", case_path.display()))?;
            let recs = parse_case(&bytes, case_path)?;
            let case_dir = tmp_base.join(format!("case-{idx}"));
            std::fs::create_dir_all(&case_dir)
                .map_err(|e| format!("mkdir {}: {e}", case_dir.display()))?;
            let result = (|| {
                let (repo, matcher, queries) = materialize(&case_dir, &recs)?;
                let oracle = run_oracle(&repo, &queries)?;
                let mut disagreements = 0;
                for (q, row) in queries.iter().zip(oracle.iter()) {
                    let norm = normalize_query(q);
                    let is_dir = std::fs::metadata(repo.join(&norm))
                        .map(|m| m.is_dir())
                        .unwrap_or(false);
                    let mine = matcher.is_ignored(&norm, is_dir);
                    if mine != row.ignored {
                        disagreements += 1;
                        println!(
                            "DISAGREE {} q={q:?}: matcher={} oracle={} ({})",
                            case_path.display(),
                            if mine { "ignored" } else { "not-ignored" },
                            if row.ignored { "ignored" } else { "not-ignored" },
                            row.evidence
                        );
                    }
                }
                let st = stats.entry(slice.clone()).or_default();
                st.cases += 1;
                st.queries += queries.len();
                st.disagreements += disagreements;
                Ok(disagreements)
            })();
            let _ = std::fs::remove_dir_all(&case_dir);
            result
        })();
        match case_res {
            Ok(d) => total_disagreements += d,
            Err(e) => {
                eprintln!("HARNESS FAILURE {}: {e}", case_path.display());
                harness_failures += 1;
                if args.strict {
                    let _ = std::fs::remove_dir_all(&tmp_base);
                    return Err(format!("aborting under --strict after: {e}"));
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp_base);
    println!("---");
    for (slice, st) in &stats {
        println!(
            "{slice}: {} cases, {} queries, {} disagreements",
            st.cases, st.queries, st.disagreements
        );
    }
    let total_cases: usize = stats.values().map(|s| s.cases).sum();
    let total_queries: usize = stats.values().map(|s| s.queries).sum();
    println!(
        "total: {total_cases} cases, {total_queries} queries, \
         {total_disagreements} disagreements, {harness_failures} harness failures"
    );
    if harness_failures > 0 {
        Ok(ExitCode::from(3))
    } else if total_disagreements > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("differential: {e}");
            eprintln!("usage: differential --corpus <dir> [--strict]");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("differential: {e}");
            ExitCode::from(3)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The oracle contract's trap: exit 128 (here provoked by running the
    /// oracle outside any repository, the pre-registered 128 case) must
    /// surface as Err - a harness failure - never as a verdict.
    #[test]
    fn oracle_exit_128_is_error_not_verdict() {
        let dir = std::env::temp_dir().join(format!("gitignore-diff-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let res = run_oracle(&dir, &["a.txt".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
        let err = res.expect_err("oracle outside a repository must be a harness failure");
        assert!(err.contains("128"), "error should name exit 128, got: {err}");
    }

    /// The pathspec port is git's full textual pass: "." segments and
    /// duplicate slashes drop anywhere, ".." consumes the previous segment
    /// without an existence check, one trailing slash survives, and the
    /// unresolvable shapes (root escape, leading "/", nothing left) pass
    /// through unchanged for the oracle to reject as exit 128.
    #[test]
    fn normalize_query_ports_pathspec_normalization() {
        assert_eq!(normalize_query("./a.log"), "a.log");
        assert_eq!(normalize_query("././a.log"), "a.log");
        assert_eq!(normalize_query("a.log"), "a.log");
        assert_eq!(normalize_query("sub/./x.txt"), "sub/x.txt");
        assert_eq!(normalize_query("sub/../a.log"), "a.log");
        assert_eq!(normalize_query("nonexist/../a.log"), "a.log");
        assert_eq!(normalize_query("sub//x.txt"), "sub/x.txt");
        assert_eq!(normalize_query("keep/"), "keep/");
        assert_eq!(normalize_query("sub/.//./x.txt"), "sub/x.txt");
        assert_eq!(normalize_query("../a.log"), "../a.log");
        assert_eq!(normalize_query("/abs.log"), "/abs.log");
        assert_eq!(normalize_query("./"), "./");
    }

    /// Colliding F records - the same path written twice, or case-variants
    /// of .gitignore on an ignore_case repo - leave one file on disk: last
    /// content under the first name. The matcher model must fold the same
    /// way, one source per surviving file, never two layered sources the
    /// disk cannot hold.
    #[test]
    fn colliding_source_records_fold_to_last_content() {
        let base = std::env::temp_dir().join(format!("gitignore-collide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let recs = vec![
            Rec::File(".gitignore".to_string(), b"alpha.txt\n".to_vec()),
            Rec::File(".gitignore".to_string(), b"beta.txt\n".to_vec()),
        ];
        let res = materialize(&base, &recs);
        let _ = std::fs::remove_dir_all(&base);
        let (_repo, matcher, _queries) = res.unwrap();
        assert!(
            !matcher.is_ignored("alpha.txt", false),
            "first record's pattern must be dead: its content was overwritten on disk"
        );
        assert!(
            matcher.is_ignored("beta.txt", false),
            "last record's pattern is the file's real content and must match"
        );
    }

    /// Empty pattern field and a negation pattern both mean not ignored;
    /// only a plain pattern match means ignored.
    #[test]
    fn verdict_rule_from_fields() {
        // Exercised end-to-end in a real temp repo: *.log ignores a.log,
        // !keep.log re-includes keep.log, b.txt matches nothing.
        let base = std::env::temp_dir().join(format!("gitignore-verdict-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        git_in(&base, &["init", "-q", "repo"]).unwrap();
        let repo = base.join("repo");
        std::fs::write(repo.join(".gitignore"), "*.log\n!keep.log\n").unwrap();
        let queries = vec!["a.log".to_string(), "keep.log".to_string(), "b.txt".to_string()];
        let rows = run_oracle(&repo, &queries).unwrap();
        let _ = std::fs::remove_dir_all(&base);
        assert!(rows[0].ignored, "a.log must be ignored: {}", rows[0].evidence);
        assert!(!rows[1].ignored, "keep.log negated, not ignored: {}", rows[1].evidence);
        assert!(!rows[2].ignored, "b.txt unmatched, not ignored: {}", rows[2].evidence);
        assert!(rows[1].evidence.contains("!keep.log"), "negation evidence: {}", rows[1].evidence);
    }
}
