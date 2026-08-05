//! Differential driver: replays the frozen corpus, asks `git check-ignore`
//! for its verdict per the oracle contract in PLAN.md, and compares against
//! the matcher. Exits non-zero on any disagreement or harness failure.

use std::path::PathBuf;
use std::process::ExitCode;

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

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("differential: {e}");
            eprintln!("usage: differential --corpus <dir> [--strict]");
            return ExitCode::from(2);
        }
    };
    eprintln!(
        "differential: harness not implemented yet (corpus: {}, strict: {})",
        args.corpus.display(),
        args.strict
    );
    ExitCode::from(2)
}
