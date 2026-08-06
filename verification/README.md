# Post-convergence verification

Nothing in this directory is part of the Verify gate. The gate is the frozen
corpus at `corpus/`, unchanged since convergence at `ab3f17e`, and the command
recorded in `PLAN.md` still runs exactly that and nothing else. These are
independent checks run **after** the run ended, to answer the fair objection
that this target's corpus was authored by the run itself.

## 1. The frozen-at-start slice

The corpus was created and frozen at iteration 3 of run 1 (commit `3d1a0d3`),
before the matcher core existed - at that point every case failed. Those
original 53 cases are byte-identical at the converged tree, and replayed alone
they give:

```
total: 53 cases, 123 queries, 0 disagreements, 0 harness failures
```

This matters because the full corpus grew to 106 cases across the run, with
growth recorded in `PLAN.md`. A reader is entitled to ask how much of the final
score comes from cases added after the code that passes them. The answer is
that the slice fixed before any matching code existed passes on its own.

Reproduce:

```sh
git ls-tree -r --name-only 3d1a0d3 -- corpus/   # the 53 frozen paths
# copy those paths to a scratch dir, then:
cargo run --release --bin differential -- --corpus <scratch>/corpus/ --strict
```

## 2. The cold corpus (`cold-corpus/`)

40 cases and 119 queries authored **blind** by a fresh context that never read
`src/`, `JOURNAL.md`, `BACKLOG.md`, or any existing corpus case - only
`PLAN.md` for the oracle contract and two case files to learn the byte format.
The brief was to target the trickiest parts of `gitignore(5)` adversarially.
No case was edited after seeing a verdict.

Result against the converged matcher at `ab3f17e`:

```
total: 40 cases, 119 queries, 0 disagreements, 0 harness failures
```

Topics covered: negation traps (dead negations, `\!` escapes, re-exclusion
order), anchoring, globstar shapes including the attached `lib/x**`
prefix-strip quirk, character-class edges (`]` first, hyphen members, escaped
hyphens, POSIX classes), directory-only versus existence, nested layering with
`info/exclude` and `core.excludesFile`, escapes and trailing whitespace
including CRLF-authored carriage returns, and deliberately weird-but-legal
shapes (`a//b.txt`, `..` segments, `st***ar`).

Reproduce:

```sh
cargo run --release --bin differential -- --corpus verification/cold-corpus/ --strict
```

## What these do and do not settle

They settle that the matcher agrees with git on questions it was not built
against: 119 adversarial queries it had never seen, and 123 queries fixed
before it could match anything.

They do not make this target's oracle equal to the TOML receipt's, where the
corpus and the inventory rows are both externally authored. The judge here is
external - git renders every verdict - but the questions are still ours. The
white paper states that asymmetry rather than leaving it to be found.
