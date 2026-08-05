# Plan

## Mode
Scoped. Build a `.gitignore` pattern matcher in Rust from an empty repository, judged differentially against the real `git check-ignore` binary.

## Goal
Produce a Rust library and a thin command-line driver that, given a repository tree and its `.gitignore` files, decides for any path whether git would ignore it, and agrees with `git check-ignore` on every case in a frozen corpus.

The oracle is git itself. This project never decides what the right answer is; it only decides what to ask. Where this implementation and git disagree, git is right by definition.

## The oracle contract
Verified against git 2.50.1.windows.1 during pre-registration. These are facts about the judge, not design choices, and getting any of them wrong silently corrupts every verdict the harness produces.

Invocation:

```
git check-ignore -z -v --non-matching --no-index --stdin < <NUL-delimited paths>
```

`-z` requires `--stdin`; the two are used together or not at all. NUL delimiting is mandatory rather than optional, because the corpus contains paths with characters that a line-based or tab-based parse would split wrongly.

Output: four NUL-delimited fields per queried path, in order `source`, `lineno`, `pattern`, `path`. An unmatched path yields three empty fields followed by the path.

Verdict rule, and this is the trap:
- pattern field empty: no pattern matched, the path is **not ignored**.
- pattern field begins with `!`: a negation pattern matched, the path is **not ignored**.
- otherwise: the path is **ignored**.

A harness that treats "appears in `-v` output" as "ignored" is wrong, because `-v` reports negation matches too. Verified: with `*.log` and `!keep.log`, the command reports lines for both `a.log` and `keep.log`, and only `a.log` is ignored.

Exit status: 0 when at least one queried path is ignored, 1 when none are, 128 on error. Exit 128 is a **harness failure, never a verdict**. A run that reads 128 as "not ignored" has a broken gate.

Isolation, verified during pre-registration:
- The oracle consults `core.excludesFile` from the machine's global and system git config, and a configured excludes file silently changes verdicts: with one simulated, a path this corpus never wrote a pattern for was reported ignored. Every oracle invocation therefore runs with `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null GIT_CONFIG_NOSYSTEM=1`. The nested-and-layered row exercises excludesFile layering by setting it in the case repository's own local config, never by inheriting the machine's.
- `git check-ignore` outside a repository exits 128. Each corpus case therefore materializes as its own temporary repository (`git init`), and the oracle runs inside that repository, never inside this project's, whose own `.gitignore` (`target/`, `.claude/`) would layer into every verdict.

## Operating envelope
Filled in advance as part of pre-registration. Do not reclassify.

Surfaces:
- `.gitignore` file contents: user-error - a repository owner hand-authors these. Malformed patterns deserve the same treatment git gives them, which is usually to match nothing rather than to error.
- The path being queried: user-error - supplied by the caller.
- The generated corpus: machine-generated - this project's own harness produces it, and the harness's real output is the contract.

Binding rules:
- A finding exercised only by out-of-envelope input is Low at most.
- Only the user widens the envelope. An audit that believes this table is wrong files one Proposed item and moves on.

## Surface inventory
The rows below are the pattern features enumerated from `gitignore(5)` and are **frozen before iteration 1**. No iteration may add, remove, or reword a row.

A row flips to `- [x] swept at <commit> - <cases run, all agreeing with the oracle>` only when the differential harness replays that row's corpus slice and every case matches. Each row's scope line names the corpus slice that defines it.

- [x] blank-and-comment: blank lines match nothing; `#` starts a comment; `\#` escapes a literal hash. Slice: `corpus/blank-and-comment` - swept at 7a43e72e3210b23e1bc1065daee0244e91dd8cd2 - 5 cases, 11 queries, all agreeing with the oracle
- [x] trailing-whitespace: trailing spaces are stripped unless backslash-quoted. Slice: `corpus/trailing-whitespace` - swept at 18bf16df780d562bc42b37be586d47534f5beff8 - 8 cases, 18 queries, all agreeing with the oracle
- [x] negation: `!` re-includes a previously excluded path; `\!` escapes a literal bang. Slice: `corpus/negation` - swept at 18bf16df780d562bc42b37be586d47534f5beff8 - 4 cases, 7 queries, all agreeing with the oracle
- [x] directory-only: a trailing `/` restricts the match to directories. Slice: `corpus/directory-only` - swept at 144b9a0b8eb17375293b2e0dad8d72dde4fe7ebb - 10 cases, 22 queries, all agreeing with the oracle
- [x] anchoring: a pattern with a non-trailing `/` is relative to the `.gitignore` location; otherwise it matches at any depth. Slice: `corpus/anchoring` - swept at d705e3fac5d73cd7d7cdc62bb4d39d4c1d870654 - 10 cases, 29 queries, all agreeing with the oracle
- [x] wildcards: `*` and `?` never cross a `/`. Slice: `corpus/wildcards` - swept at 4e39d2500b95b8b8c7777cfc13aa27846c73eeba - 10 cases, 37 queries, all agreeing with the oracle
- [x] char-classes: bracket expressions and ranges. Slice: `corpus/char-classes` - swept at a9724c236a660b859fb90111f1220bf6707eb3f4 - 16 cases, 47 queries, all agreeing with the oracle
- [x] globstar-leading: a leading `**/` matches in all directories. Slice: `corpus/globstar-leading` - swept at 7365453e92a6ea249698ba58a04fedc2abd58e42 - 6 cases, 19 queries, all agreeing with the oracle
- [x] globstar-trailing: a trailing `/**` matches everything inside. Slice: `corpus/globstar-trailing` - swept at c68865bae65b0f0beea474fb81bb7e18146c3e60 - 6 cases, 19 queries, all agreeing with the oracle
- [x] globstar-infix: `/**/` matches zero or more directories. Slice: `corpus/globstar-infix` - swept at eee469021ee289f237b0299b1e2fdd853e33be4b - 7 cases, 26 queries, all agreeing with the oracle
- [x] precedence: the last matching pattern in a file wins. Slice: `corpus/precedence` - swept at 1b32ddc656d40b60dc8c08942b18995450bd58b3 - 7 cases, 14 queries, all agreeing with the oracle
- [x] nested-and-layered: a `.gitignore` in a subdirectory overrides its parent, and `.git/info/exclude` and `core.excludesFile` layer beneath both. An excluded directory cannot be re-included by negating a file inside it. Slice: `corpus/nested-and-layered` - swept at e32a1fe91287c58a6165ff0af3434992f5683f91 - 14 cases, 37 queries, all agreeing with the oracle

Twelve rows. Each slice's case count is recorded here by the harness once the corpus is frozen, and is never reduced afterwards.

Frozen counts, recorded at corpus-freeze: blank-and-comment 4, trailing-whitespace 4, negation 4, directory-only 5, anchoring 4, wildcards 5, char-classes 7, globstar-leading 3, globstar-trailing 3, globstar-infix 3, precedence 4, nested-and-layered 7. Total 53 cases, 123 queries.

Grown at row-trailing-whitespace sweep: trailing-whitespace 4 to 8 (multiple escaped spaces, lone trailing backslash, interior spaces, escaped-space-only pattern). Total 57 cases, 132 queries.

Grown at row-directory-only sweep: directory-only 5 to 9 (negated dir pattern, unanchored dir pattern at depth plus nonexistent same-name at root, wildcard dir pattern vs same-named file, escaped trailing slash). Total 61 cases, 142 queries.

Grown at row-anchoring sweep: anchoring 4 to 8 (anchored pattern inside nested .gitignore, anchored root wildcard, middle-slash single-star segment, anchored dir-only vs same name at depth). Total 65 cases, 152 queries.

Grown at row-wildcards sweep: wildcards 5 to 9 (bare star incl dotfile, multi-star backtracking, repeated question marks, star plus class). Total 69 cases, 166 queries.

Grown at row-char-classes sweep: char-classes 7 to 12 (escaped hyphen member, single-char range, class containing slash, negated class with escaped bang, inverted range). Total 74 cases, 178 queries.

Grown at row-globstar-leading sweep: globstar-leading 3 to 6 (leading globstar inside a nested .gitignore, negated leading globstar, attached x**/ exercising the dir.c prefix-strip quirk). Total 77 cases, 188 queries.

Grown at row-globstar-trailing sweep: globstar-trailing 3 to 6 (negation re-inclusion under dir/**, attached a/x** crossing depth via the prefix strip, dir-only trailing d/**/). Total 80 cases, 198 queries.

Grown at row-globstar-infix sweep: globstar-infix 3 to 6 (chained a/**/b/**/c, dir-only infix a/**/tmp/, infix in a nested .gitignore, attached a/b**/c via the prefix strip). Total 83 cases, 210 queries.

Grown at row-precedence sweep: precedence 4 to 7 (dir-only exclude vs plain negation both directions, anchored exclude vs unanchored negation and the reverse, dir-only negation matching the dir but never the file). Total 86 cases, 217 queries.

Grown at row-nested-and-layered sweep: nested-and-layered 7 to 10 (negation inside an excluded directory stays dead, tree negation overriding core.excludesFile, nested .gitignore with own-directory-anchored pattern plus dir exclusion killing a descendant via the ancestor walk). Total 89 cases, 224 queries.

Grown at fix-globstar-escaped-slash: globstar-infix 6 to 7 (a/**\/b - escaped slash ends the globstar segment and crosses depth, with no zero-directory reading). Total 90 cases, 228 queries.

Grown at fix-posix-bracket-classes: char-classes 12 to 15 ([[:digit:]] both directions, multiple classes in one bracket, negated POSIX class, unknown class matching nothing, missing :] falling back to a literal set) and wildcards 9 to 10 (case-insensitive literal and range matching under the oracle's core.ignoreCase=true, uppercase bracket member dead under icase). Total 94 cases, 247 queries.

Grown at fix-bom-skip: blank-and-comment 4 to 5 (head BOM skipped in .gitignore and info/exclude, mid-file BOM stays literal). Total 95 cases, 250 queries.

Grown at fix-icase-dir-filter: nested-and-layered 10 to 11 (Sub/.gitignore applying to sub/foo.txt under core.ignoreCase, exact-case and unmatched controls). Total 96 cases, 253 queries.

Grown at fix-posix-space-set: char-classes 15 to 16 (a[[:space:]]b dead at VT and FF per git sane-ctype, space and tab member controls matching). Total 97 cases, 257 queries.

Grown at fix-gitignore-name-case: nested-and-layered 11 to 12 (Sub/.GitIgnore live as a source under core.ignoreCase, hit by cross-case and exact-case queries, unmatched control). Total 98 cases, 260 queries.

Grown at fix-source-collision-model: nested-and-layered 12 to 13 (colliding case-variant .gitignore records at root and in a nested dir - the filesystem folds each pair to one file, last content under the first name; queries on both records' patterns plus an unmatched control). Total 99 cases, 265 queries.

Grown at fix-dotslash-query: anchoring 8 to 9 (leading ./ and ././ queries hitting an anchored pattern - check-ignore matches the normalized pathspec while echoing it verbatim; plain-query and unmatched controls). Total 100 cases, 269 queries.

Grown at fix-pathspec-normalization-port: anchoring 9 to 10 (interior ./ segment, .. through an existing and a nonexistent segment, duplicate slash - all textually normalized by check-ignore before matching anchored patterns; matched and unmatched controls). Total 101 cases, 275 queries.

Grown at fix-trailing-dot-normalization: directory-only 9 to 10 (final-. and final-.. queries normalizing to dir/ against a dir-only pattern on a nonexistent directory; bare-name, raw trailing-slash, and unmatched controls). Total 102 cases, 280 queries.

Grown at fix-unicode-collision-fold: nested-and-layered 13 to 14 (Unicode case-variant collision - a folding u-diaeresis pair leaving one file, last content under the first name, beside a non-folding sharp-s pair keeping two sources as an anti-overfold control; queries in both directions plus controls). Total 103 cases, 286 queries.

## Verify command
Command: `cargo build --release && cargo run --release --bin differential -- --corpus corpus/ --strict`

The `differential` driver replays every frozen corpus case, asks the oracle for its verdict using the contract above, compares, and exits non-zero on any disagreement. `--strict` makes an oracle exit status of 128 a harness failure rather than a verdict.

Never narrow it to a subset. Never regenerate the corpus smaller. Never pipe it through `head` or `tail`.

## Method
Build the harness first, then the matcher. The harness is the instrument, and it is worthless if written to fit the implementation.

Corpus discipline, and this is the rule the whole target rests on:
- The corpus is generated once, early, from the twelve rows above, then **frozen**: written to disk, committed, and never regenerated smaller.
- Growing the corpus is allowed and encouraged. Shrinking it, or regenerating it in a way that drops cases, is a violation and voids the run.
- The corpus's size and shape are reported alongside the pass count in every run report. A pass rate over a corpus the run chose is worth less than a pass rate over a corpus the run cannot shrink, and the receipt must let a reader see which one this is.

A task is one inventory row. Its acceptance check is that row's corpus slice replaying with zero disagreements, observed failing first.

Evidence rule: every claim is a command that was run with its real output recorded. The oracle prints the pattern it matched and the line it came from, so a disagreement is always reported with git's own reasoning attached.

Severity rubric, judged against the envelope:
- High: a disagreement with the oracle on any frozen corpus case, a crash, a hang, or a build failure.
- Medium: a plausible in-envelope case outside the corpus that disagrees, a silently swallowed error, or documentation contradicting behaviour.
- Low: polish - naming, style, minor docs.

Class rules: when a disagreement is one instance of a repeatable idiom, fix the class and enumerate every site. The third disagreement sharing one root cause ends instance work and becomes one structural task.

Regressions: a row that was ticked and later disagrees is a High. Flip its row back to unswept and fix it before taking new work.

## Constraints

- **No gitignore-matching dependency.** Not `ignore`, not `gitignore`, not `globset` used as a gitignore engine, not a vendored copy, not a crate that wraps one. Verify with `cargo tree` before every checkpoint. A general-purpose regex crate is permitted for the matcher's internals; a crate that implements gitignore semantics is not.
- **Never shrink the corpus.** Regenerating it smaller, or filtering out cases that fail, voids the run.
- **Never treat oracle exit 128 as a verdict.** It is a harness failure.
- **No special-casing the corpus.** No branch that exists to satisfy a specific named case. The matcher must not be able to tell it is being tested.
- **No editing the inventory rows.** They are frozen. A disagreement with the row list goes to Proposed.
- Full implementations only. A stub written to satisfy an acceptance check is a violation, not progress.
- Keep each change atomic. Never push, never create branches, never rewrite checkpoint history.

## Lessons

- TMPDIR is unset in this Git Bash; write temp logs to the session scratchpad path, never $TMPDIR.
- Probe harness failure paths with malformed scratch inputs the corpus cannot contain; the happy path hides bounds bugs.
- A slice that went green as a side effect gets grown with new oracle-blind cases before its row is ticked; never tick on a rubber-stamp replay.
- Git's semantics live in dir.c call sites as much as in wildmatch (e.g. the match_pathname literal-prefix strip makes the first wildcard run segment-initial); port the composition, never the manpage alone.
- git init on this machine sets core.ignoreCase=true (NTFS), so every oracle repo matches case-insensitively; the engine ports WM_CASEFOLD (text and plain literals fold, bracket members and escaped literals never fold, ranges try the upcased byte) and the harness mirrors the repo's config into the matcher.
- WM_CASEFOLD reaches source applicability too: the nested-.gitignore directory-prefix test folds case (dir.c fspathncmp), not only wildmatch - and source-name discovery folds as well, since the filesystem lookup of .gitignore is case-insensitive on an icase filesystem.
- A queried path starting with ':' is reinterpreted by check-ignore as pathspec magic before matching; never author corpus queries with a leading colon.
- The filesystem folds colliding names on write: case-variant F records land in one file (last content, first name), so a hand-authored case with two same-folded paths tests the fold, never two coexisting files; the harness models the collision as one source.
- check-ignore matches the normalized pathspec but echoes it verbatim: ./-shaped corpus queries are legal, the echo check stays byte-exact, and normalization belongs on the matcher side of the comparison only.
- Pathspec normalization is textual: .. consumes the previous segment with no existence check, duplicate slashes and . segments drop, one trailing slash survives. Never author corpus queries git cannot normalize (leading /, .. above the root, a query normalizing to nothing): those are exit-128 harness failures, not verdicts.
- A pathspec's dir-ness survives normalization through a final . or .. segment, not only a literal trailing slash: git leaves dir/ behind, and a literal dir-only pattern matches it with no existence check; a wildcard-tail dir-only pattern (ghost/**/, pit/*/) does consult existence at the path-itself match, so the two families diverge on nonexistent directories.
- The volume's case fold is the NTFS $UpCase table baked in at format time, matching no portable Unicode fold (probed live: u-diaeresis, fullwidth, and sigma pairs fold; sharp-s, dotless-i, Kelvin, final-sigma, supplementary-plane, and NFC/NFD pairs stay distinct); never predict the fold - read the disk back, as materialize does by keying sources on canonical on-disk paths.

## Definition of done
Convergence requires all of the following simultaneously:

1. `cargo build --release` succeeds with no errors.
2. The Verify command exits 0 with zero disagreements across the whole frozen corpus, its real output recorded in the declaring iteration's JOURNAL entry.
3. All 12 inventory rows carry `- [x]` with the commit hash they were swept at.
4. `cargo tree` shows no gitignore-implementing dependency.
5. The corpus size is recorded in the JOURNAL and is greater than or equal to every size recorded earlier in the run.
6. `BACKLOG.md` has zero open tasks in Now, Next, and Later. Every filed finding, Low included, is completed, moved to Declined with a genuine reason, or marked `[b]` with its reason on record.
7. The adversarial evaluator returned PASS in the declaring iteration, spawned fresh-context, having re-run the Verify command and the closed tasks' acceptance checks.
8. A line `Converged: <full commit hash> - <date>` is appended under `## Converged` in `BACKLOG.md`.

A run that ends short of this ends out of budget or blocked, and its journal is kept and published exactly as it stands. A partial result is a receipt.
