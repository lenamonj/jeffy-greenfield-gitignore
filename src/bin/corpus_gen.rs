//! One-shot corpus generator. Writes the frozen corpus slices for the twelve
//! inventory rows in PLAN.md, then never runs again: it refuses to touch an
//! existing corpus/ directory, because the corpus is frozen once written and
//! may only be grown by adding new case files, never regenerated.
//!
//! Case file format (NUL-safe, byte-exact; corpus/** is -text in
//! .gitattributes so nothing rewrites line endings):
//! a case file is a flat sequence of NUL-terminated tokens forming records.
//! The first token of a record is the op; its arity is fixed per op:
//!   F <path> <content>  write a file inside the case worktree (parents made)
//!   D <path>            create a directory inside the case worktree
//!   E <content>         write .git/info/exclude for the case repository
//!   X <content>         write an excludes file outside the worktree and set
//!                       core.excludesFile to it in the case repo local config
//!   Q <path>            query this path against the oracle and the matcher
//! The corpus stores inputs only. Verdicts come from the live oracle at
//! replay time; expected answers are never data.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

type Rec = Vec<&'static str>;
type Case = Vec<Rec>;

fn arity(op: &str) -> Option<usize> {
    match op {
        "F" => Some(2),
        "D" | "E" | "X" | "Q" => Some(1),
        _ => None,
    }
}

fn slices() -> Vec<(&'static str, Vec<Case>)> {
    vec![
        (
            "blank-and-comment",
            vec![
                vec![vec!["F", ".gitignore", "\n\na.txt\n\n"], vec!["Q", "a.txt"], vec!["Q", "b.txt"]],
                vec![vec!["F", ".gitignore", "#note\nnote\n"], vec!["Q", "#note"], vec!["Q", "note"]],
                vec![vec!["F", ".gitignore", "\\#hash\n"], vec!["Q", "#hash"], vec!["Q", "hash"]],
                vec![
                    vec!["F", ".gitignore", "# only comments\n\n#another\n   \n"],
                    vec!["Q", "anything.txt"],
                    vec!["Q", "#another"],
                ],
            ],
        ),
        (
            "trailing-whitespace",
            vec![
                vec![vec!["F", ".gitignore", "a.txt   \n"], vec!["Q", "a.txt"], vec!["Q", "a.txt   "]],
                vec![vec!["F", ".gitignore", "b.txt\\ \n"], vec!["Q", "b.txt "], vec!["Q", "b.txt"]],
                vec![
                    vec!["F", ".gitignore", "c.txt \\ \n"],
                    vec!["Q", "c.txt"],
                    vec!["Q", "c.txt "],
                    vec!["Q", "c.txt  "],
                ],
                vec![vec!["F", ".gitignore", "d.txt\t\n"], vec!["Q", "d.txt"], vec!["Q", "d.txt\t"]],
            ],
        ),
        (
            "negation",
            vec![
                vec![
                    vec!["F", ".gitignore", "*.log\n!keep.log\n"],
                    vec!["Q", "a.log"],
                    vec!["Q", "keep.log"],
                    vec!["Q", "b.log"],
                ],
                vec![vec!["F", ".gitignore", "!never.txt\n"], vec!["Q", "never.txt"]],
                vec![vec!["F", ".gitignore", "\\!bang\n"], vec!["Q", "!bang"], vec!["Q", "bang"]],
                vec![vec!["F", ".gitignore", "keep.log\n!keep.log\nkeep.log\n"], vec!["Q", "keep.log"]],
            ],
        ),
        (
            "directory-only",
            vec![
                vec![vec!["F", ".gitignore", "target/\n"], vec!["D", "target"], vec!["Q", "target"]],
                vec![vec!["F", ".gitignore", "target/\n"], vec!["F", "target", ""], vec!["Q", "target"]],
                vec![vec!["F", ".gitignore", "target/\n"], vec!["Q", "target"]],
                vec![
                    vec!["F", ".gitignore", "build/\n"],
                    vec!["D", "build"],
                    vec!["F", "build/inner.txt", ""],
                    vec!["Q", "build/inner.txt"],
                    vec!["Q", "build"],
                ],
                vec![
                    vec!["F", ".gitignore", "sub/dir/\n"],
                    vec!["D", "sub/dir"],
                    vec!["Q", "sub/dir"],
                    vec!["Q", "x/sub/dir"],
                ],
            ],
        ),
        (
            "anchoring",
            vec![
                vec![vec!["F", ".gitignore", "/top.txt\n"], vec!["Q", "top.txt"], vec!["Q", "sub/top.txt"]],
                vec![vec!["F", ".gitignore", "any.txt\n"], vec!["Q", "any.txt"], vec!["Q", "a/b/any.txt"]],
                vec![vec!["F", ".gitignore", "doc/frotz\n"], vec!["Q", "doc/frotz"], vec!["Q", "a/doc/frotz"]],
                vec![
                    vec!["F", "sub/.gitignore", "/x.txt\n"],
                    vec!["Q", "sub/x.txt"],
                    vec!["Q", "sub/deep/x.txt"],
                    vec!["Q", "x.txt"],
                ],
            ],
        ),
        (
            "wildcards",
            vec![
                vec![
                    vec!["F", ".gitignore", "*.txt\n"],
                    vec!["Q", "a.txt"],
                    vec!["Q", "sub/b.txt"],
                    vec!["Q", "a.txtx"],
                    vec!["Q", "a.tx"],
                ],
                vec![
                    vec!["F", ".gitignore", "a*b\n"],
                    vec!["Q", "ab"],
                    vec!["Q", "axxb"],
                    vec!["Q", "a/b"],
                    vec!["Q", "x/ayb"],
                ],
                vec![
                    vec!["F", ".gitignore", "?.txt\n"],
                    vec!["Q", "a.txt"],
                    vec!["Q", "ab.txt"],
                    vec!["Q", ".txt"],
                ],
                vec![
                    vec!["F", ".gitignore", "sub/*.txt\n"],
                    vec!["Q", "sub/a.txt"],
                    vec!["Q", "sub/dir/b.txt"],
                    vec!["Q", "other/sub/a.txt"],
                ],
                vec![
                    vec!["F", ".gitignore", "a?c\n"],
                    vec!["Q", "abc"],
                    vec!["Q", "a/c"],
                    vec!["Q", "ac"],
                ],
            ],
        ),
        (
            "char-classes",
            vec![
                vec![
                    vec!["F", ".gitignore", "[abc].txt\n"],
                    vec!["Q", "a.txt"],
                    vec!["Q", "b.txt"],
                    vec!["Q", "d.txt"],
                ],
                vec![
                    vec!["F", ".gitignore", "[a-c].txt\n"],
                    vec!["Q", "b.txt"],
                    vec!["Q", "d.txt"],
                    vec!["Q", "-.txt"],
                ],
                vec![vec!["F", ".gitignore", "[!a-c].txt\n"], vec!["Q", "d.txt"], vec!["Q", "a.txt"]],
                vec![vec!["F", ".gitignore", "[^a-c].txt\n"], vec!["Q", "d.txt"], vec!["Q", "b.txt"]],
                vec![
                    vec!["F", ".gitignore", "x[0-9][0-9]\n"],
                    vec!["Q", "x12"],
                    vec!["Q", "x1"],
                    vec!["Q", "x123"],
                ],
                vec![vec!["F", ".gitignore", "[[]a\n"], vec!["Q", "[a"], vec!["Q", "a"]],
                vec![
                    vec!["F", ".gitignore", "[]-]x\n"],
                    vec!["Q", "]x"],
                    vec!["Q", "-x"],
                    vec!["Q", "ax"],
                ],
            ],
        ),
        (
            "globstar-leading",
            vec![
                vec![
                    vec!["F", ".gitignore", "**/foo\n"],
                    vec!["Q", "foo"],
                    vec!["Q", "a/foo"],
                    vec!["Q", "a/b/foo"],
                    vec!["Q", "xfoo"],
                ],
                vec![
                    vec!["F", ".gitignore", "**/foo/bar\n"],
                    vec!["Q", "foo/bar"],
                    vec!["Q", "a/foo/bar"],
                    vec!["Q", "bar"],
                ],
                vec![
                    vec!["F", ".gitignore", "**/d/\n"],
                    vec!["D", "d"],
                    vec!["D", "a/d"],
                    vec!["Q", "d"],
                    vec!["Q", "a/d"],
                ],
            ],
        ),
        (
            "globstar-trailing",
            vec![
                vec![
                    vec!["F", ".gitignore", "abc/**\n"],
                    vec!["Q", "abc"],
                    vec!["Q", "abc/x"],
                    vec!["Q", "abc/d/e"],
                    vec!["Q", "xabc/y"],
                ],
                vec![
                    vec!["F", ".gitignore", "sub/inner/**\n"],
                    vec!["Q", "sub/inner/f"],
                    vec!["Q", "sub/inner"],
                    vec!["Q", "sub/f"],
                ],
                vec![
                    vec!["F", "sub/.gitignore", "gen/**\n"],
                    vec!["Q", "sub/gen/a"],
                    vec!["Q", "gen/a"],
                ],
            ],
        ),
        (
            "globstar-infix",
            vec![
                vec![
                    vec!["F", ".gitignore", "a/**/b\n"],
                    vec!["Q", "a/b"],
                    vec!["Q", "a/x/b"],
                    vec!["Q", "a/x/y/b"],
                    vec!["Q", "z/a/b"],
                ],
                vec![
                    vec!["F", ".gitignore", "src/**/test/**\n"],
                    vec!["Q", "src/test/x"],
                    vec!["Q", "src/a/test/x/y"],
                    vec!["Q", "src/test"],
                ],
                vec![
                    vec!["F", ".gitignore", "doc/**/*.md\n"],
                    vec!["Q", "doc/a.md"],
                    vec!["Q", "doc/x/b.md"],
                    vec!["Q", "doc/x/b.txt"],
                ],
            ],
        ),
        (
            "precedence",
            vec![
                vec![
                    vec!["F", ".gitignore", "*.log\n!important.log\n*.log\n"],
                    vec!["Q", "important.log"],
                    vec!["Q", "other.log"],
                ],
                vec![vec!["F", ".gitignore", "x\n!x\n"], vec!["Q", "x"]],
                vec![vec!["F", ".gitignore", "!y\ny\n"], vec!["Q", "y"]],
                vec![
                    vec!["F", ".gitignore", "a*\n!ab\nab*\n"],
                    vec!["Q", "ab"],
                    vec!["Q", "ac"],
                    vec!["Q", "abc"],
                ],
            ],
        ),
        (
            "nested-and-layered",
            vec![
                vec![
                    vec!["F", ".gitignore", "*.log\n"],
                    vec!["F", "sub/.gitignore", "!keep.log\n"],
                    vec!["Q", "sub/keep.log"],
                    vec!["Q", "sub/other.log"],
                    vec!["Q", "keep.log"],
                ],
                vec![vec!["E", "secret.txt\n"], vec!["Q", "secret.txt"], vec!["Q", "other.txt"]],
                vec![
                    vec!["E", "secret.txt\n"],
                    vec!["F", ".gitignore", "!secret.txt\n"],
                    vec!["Q", "secret.txt"],
                ],
                vec![vec!["X", "global.txt\n"], vec!["Q", "global.txt"]],
                vec![
                    vec!["X", "global.txt\n"],
                    vec!["E", "!global.txt\n"],
                    vec!["Q", "global.txt"],
                ],
                vec![
                    vec!["F", ".gitignore", "build/\n!build/keep.txt\n"],
                    vec!["D", "build"],
                    vec!["F", "build/keep.txt", ""],
                    vec!["Q", "build/keep.txt"],
                    vec!["Q", "build/other.txt"],
                ],
                vec![
                    vec!["F", ".gitignore", "*.tmp\n"],
                    vec!["F", "a/.gitignore", "!special.tmp\n"],
                    vec!["F", "a/b/.gitignore", "special.tmp\n"],
                    vec!["Q", "a/b/special.tmp"],
                    vec!["Q", "a/special.tmp"],
                    vec!["Q", "top.tmp"],
                ],
            ],
        ),
    ]
}

fn main() -> ExitCode {
    let root = Path::new("corpus");
    if root.exists() {
        eprintln!(
            "corpus-gen: corpus/ already exists; the corpus is frozen. \
             Grow it by adding case files; never regenerate."
        );
        return ExitCode::from(1);
    }
    let mut total_cases = 0usize;
    let mut total_queries = 0usize;
    for (name, cases) in slices() {
        let dir = root.join(name);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("corpus-gen: creating {}: {e}", dir.display());
            return ExitCode::from(1);
        }
        for (i, case) in cases.iter().enumerate() {
            let mut buf = Vec::new();
            for rec in case {
                let op = rec[0];
                let Some(n) = arity(op) else {
                    eprintln!("corpus-gen: bad op {op:?} in {name} case {}", i + 1);
                    return ExitCode::from(1);
                };
                assert_eq!(rec.len(), n + 1, "arity mismatch for {op} in {name} case {}", i + 1);
                if op == "Q" {
                    total_queries += 1;
                }
                for tok in rec {
                    buf.extend_from_slice(tok.as_bytes());
                    buf.push(0);
                }
            }
            let path = dir.join(format!("{:02}.case", i + 1));
            if let Err(e) = std::fs::File::create(&path).and_then(|mut f| f.write_all(&buf)) {
                eprintln!("corpus-gen: writing {}: {e}", path.display());
                return ExitCode::from(1);
            }
            total_cases += 1;
        }
        println!("{name}: {} cases", cases.len());
    }
    println!("total: {total_cases} cases, {total_queries} queries");
    ExitCode::SUCCESS
}
