# Journal

Append-only. One primary entry per iteration; SALVAGE and ROTATION entries are additional. Never rewrite past entries (filling the current entry's Checkpoint field is completion, not a rewrite).

Heading grammar, exactly (fenced and indented here so this example is never mistaken for an entry by anything that counts or rotates them):

```
  ## iter <i>/<N> | <run-id> | <YYYY-MM-DD> | <task-id or AUDIT or EVALUATOR or RATCHET or WRAPUP or SALVAGE or ROTATION> | <done|blocked|audit|converged|salvage|rotation>
```

Write a real heading at column zero, never indented: the indentation above belongs to the example alone, and an indented heading is invisible to the rotation anchor and to the archive counter, so the entry under it is not counted and not rotated.

SALVAGE entries take status salvage; ROTATION entries take status rotation. An EVALUATOR entry records an evaluator-gate iteration: status audit when the run continues after the verdict, blocked on a terminal second REJECT, converged when that same iteration declares.

run-id is the first 8 characters of the session id, a hyphen, then the HHMMSS of started_at from the loop state frontmatter, so two runs in one session are told apart. Body fields, in order: Task, Changed, Checkpoint (the jeffy checkpoint commit hash, or none with the reason), Verification, Learnings, Next.

The closing entry that declares convergence carries the evaluator verdict in its Verification field: `Evaluator: PASS - <one-line summary>`, or `Evaluator: unavailable (<reason>)`. An earlier EVALUATOR entry records its own verdict the same way and never stands in for the closing one: the Stop hook reads the closing entry alone, so a run that gates early and keeps working re-invokes the gate at the declaration.

Closed tasks are recorded here as one line each (ID, title, closing evidence), because BACKLOG.md deletes them. Rotation: when this file exceeds 500 lines, move all but the last 10 entries to the end of JOURNAL-archive.md, appending to whatever that file already holds and never overwriting it, because the archive accumulates across every rotation and every run; create it only when it does not already exist, and record the rotation as a ROTATION entry.

## iter 1/12 | cd1c2080-020939 | 2026-08-04 | AUDIT | audit

Task: Scoped-mode derivation audit. Greenfield tree (tracked: .gitattributes, .gitignore, PLAN.md, BACKLOG.md only); derived the full task set from the Definition of done per the Method (harness before matcher).
Changed: BACKLOG.md seeded with 15 open tasks: Now = scaffold, corpus-freeze, harness; Next = the 12 frozen inventory rows in frozen order. JOURNAL.md bootstrapped from template this iteration.
Checkpoint: 3033c98fcf470ae6a0134a7a3a0e371c98e484d5. Stall check: audit iteration, 15 backlog items changed state (created open), no stall.
Verification: Toolchain verified: cargo 1.90.0, git 2.50.1.windows.1 - exactly the version the oracle contract was pre-registered against. Verify command payload passes bash -n. Scores: High 15 (DoD items 1-5 wholly unmet: no build, no corpus, no harness, 12 unswept rows), Medium 0, Low 0. All findings filed as tasks; nothing in-envelope left unfiled.
Learnings: Oracle git version on this machine matches the pre-registration version string byte for byte, so no version-drift caveat is needed in the harness.
Next: scaffold - create the crate skeleton so the Verify command can run at all.

## iter 2/12 | cd1c2080-020939 | 2026-08-04 | scaffold | done

Task: scaffold (High, build-ci) - crate skeleton so the Verify command can run. Closed: created package gitignore-matcher (edition 2024, zero dependencies) with lib Matcher stub (matches nothing, to be observed failing by the harness) and differential bin parsing --corpus <dir> --strict, exiting 2 with a not-implemented message.
Changed: Cargo.toml, Cargo.lock, src/lib.rs, src/bin/differential.rs created. BACKLOG.md: scaffold line deleted.
Checkpoint: c32d3d7d4df61aa5a883528f0fa85fdd4c1df00f. Stall check: 4 source files created outside state files, scaffold changed state open -> closed, no stall.
Verification: Acceptance: cargo build --release exit 0, observed. Verify command: exit 2 (harness stub not implemented) - red at last checkpoint too (no project existed), so not a regression; expected red until harness and corpus land. cargo tree: gitignore-matcher v0.1.0 alone, no gitignore-implementing dependency.
Learnings: TMPDIR is unset in this Git Bash; redirect logs to the session scratchpad path, never $TMPDIR.
Next: corpus-freeze - generate and freeze the 12 slices, record counts in PLAN.md.

## iter 3/12 | cd1c2080-020939 | 2026-08-04 | corpus-freeze | done

Task: corpus-freeze (High, test) - generate and freeze the corpus. Closed: corpus_gen bin wrote 12 slices named exactly per the frozen inventory rows, 53 cases, 123 queries, NUL-terminated token records (F/D/E/X/Q), inputs only - verdicts come from the live oracle at replay. Generator refuses to run when corpus/ exists (freeze guard, observed exit 1 on rerun).
Changed: src/bin/corpus_gen.rs created; corpus/ (53 case files in 12 dirs) created; PLAN.md frozen counts line added under the inventory; BACKLOG.md corpus-freeze line deleted.
Checkpoint: 3d1a0d395c65a5de58ac1944b71a708cc35227c3. Stall check: corpus/ (53 files) and corpus_gen.rs created outside state files, corpus-freeze changed state open -> closed, no stall.
Verification: Acceptance: ls corpus shows the 12 inventory slice names, all non-empty (4/4/4/5/4/5/7/3/3/3/4/7 cases); od -c confirms NUL delimiting with tab and trailing-space bytes intact; counts recorded in PLAN.md. Freeze guard observed: second corpus_gen run exits 1 without touching corpus/. Verify command: still exit 2 (harness stub), red at last checkpoint too, not a regression. cargo tree: gitignore-matcher alone, no gitignore-implementing dependency.
Learnings: corpus/** -text in .gitattributes was pre-registered and od confirms it: NULs, tabs, and trailing spaces survive byte-exact through git.
Next: harness - implement the oracle contract in the differential bin; first run must show the stub matcher failing.

## iter 4/12 | cd1c2080-020939 | 2026-08-04 | harness | done

Task: harness (High, test) - differential driver per the oracle contract. Closed: one temp repo per case under the system temp dir (never this tree), config-clean env on every git call, -z -v --non-matching --no-index --stdin invocation, four-field NUL parse validated against query order, negation-aware verdict rule, oracle exit 128 = Err at the run_oracle choke point every case flows through; exit codes 0 agree / 1 disagree / 3 harness failure, empty corpus refuses to read as green.
Changed: src/bin/differential.rs rewritten from stub to full driver plus two unit tests; src/lib.rs Matcher API changed from bare stub to source-accumulating builder (add_gitignore, set_info_exclude, set_excludes_file, is_ignored(rel_path, is_dir)); contract preserved: is_ignored still answers false for every path until row tasks land, callers = differential bin only, no swept inventory rows exist to flip. BACKLOG.md harness line deleted.
Checkpoint: af75a5ba7ea0fda1169fe40ae9d71a3a0d1640b8. Stall check: differential.rs and lib.rs changed outside state files, harness changed state open -> closed, no stall.
Verification: Acceptance observed: full replay = 53 cases, 123 queries, 69 disagreements, 0 harness failures, exit 1 - the stub failing first, every slice showing disagreements, each DISAGREE line carrying git's pattern, source, and lineno. cargo test --release --bin differential: 2 passed (oracle-outside-repo exit 128 -> Err naming 128; verdict rule end-to-end with real git, negation evidence carries !keep.log). Strict abort probed with a malformed scratch case (never the frozen corpus): exit 3 with HARNESS FAILURE line; non-strict continues and still exits 3. The probe caught a real off-by-one: truncated record panicked (exit 101) before the fix, clean harness failure after; fix attempt 2 of 3. cargo tree: gitignore-matcher alone.
Learnings: The strict-abort probe against a deliberately malformed scratch case caught a bounds-check panic the happy path never would; keep probing harness failure paths with inputs the corpus cannot contain.
Next: row-blank-and-comment - first matcher semantics; its slice currently shows 3 disagreements to eliminate.

## iter 5/12 | cd1c2080-020939 | 2026-08-04 | row-blank-and-comment | done

Task: row-blank-and-comment (High, runtime) - first matcher semantics plus the matcher core they require. Closed: lib.rs implements gitignore line parsing (blank skip, # comment skip, dir.c trim_trailing_spaces port, ! negation flag, trailing-/ dir_only flag, separator-anchoring flag), escape-aware literal matching (wildcard metacharacters deliberately literal until their rows land), last-match-wins per source, source precedence deepest .gitignore -> info/exclude -> excludesFile, and the excluded-ancestor walk.
Changed: src/lib.rs rewritten from stub builder to working core. Contract: is_ignored(rel_path, is_dir) signature unchanged; caller (differential bin) untouched. BACKLOG.md row-blank-and-comment line deleted.
Checkpoint: c7c8c158b5e6b224d3247a438973038572ab62ef; PLAN.md row blank-and-comment ticked with this hash in the bookkeeping edit. Stall check: lib.rs changed outside state files, row task closed, no stall.
Verification: Acceptance observed failing first this iteration: slice replay 3 disagreements against the stub (a.txt, note, escaped \#hash), then 0 disagreements after, exit 0, 4 cases 8 queries. cargo test: 5 passed across lib and harness (parse edges, trim port incl tab-not-trimmed, oracle contract pins). Full Verify: exit 1 with 45 disagreements (was 69) - red at last checkpoint too, not a regression; anchoring, directory-only, trailing-whitespace slices went green as core side effects, their rows stay unticked pending their own sweep iterations. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-trailing-whitespace - slice already shows 0 disagreements from the trim port; sweep and tick it.

## iter 6/12 | cd1c2080-020939 | 2026-08-04 | row-trailing-whitespace | done

Task: row-trailing-whitespace (High, runtime) - sweep inventory row 2. The slice went green as a side effect of iter 5's trim port, so a bare replay would rubber-stamp; grew the slice from 4 to 8 cases first (two escaped trailing spaces, lone trailing backslash, interior spaces with trailing trim, escaped-space-only pattern vs a single-space path), then swept.
Changed: corpus/trailing-whitespace/05-08.case hand-authored per the NUL token format (od-verified); PLAN.md growth line added under frozen counts; BACKLOG.md row line deleted. No source changes - the existing trim port survived all new cases.
Checkpoint: 2f0a52af22656cdedad5af1704c6334e6891c5a9; PLAN.md row trailing-whitespace ticked with this hash in the bookkeeping edit. Stall check: 4 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4's recorded 4 disagreements on this slice against the stub; grown slice replays 8 cases, 18 queries, 0 disagreements, exit 0 - including the genuinely new cases the matcher had never seen. Full Verify: exit 1, 57 cases, 132 queries, 45 disagreements (unchanged set, all in glob-engine slices), red at last checkpoint too, not a regression. Corpus grew 53 to 57 cases, never shrank. cargo test: 4 suites ok. cargo tree: gitignore-matcher alone.
Learnings: Growing a green slice before ticking its row turns a rubber-stamp sweep into a real one; the growth cases are chosen blind to the implementation (oracle decides), so agreement is evidence, not tautology.
Next: row-negation - slice shows 2 disagreements; needs * wildcard support in the glob engine.

## iter 7/12 | cd1c2080-020939 | 2026-08-04 | row-negation | done

Task: row-negation (High, runtime) - sweep inventory row 3. The negation mechanics (last-match-wins, ! flag, \! escape) already agreed; the slice's two disagreements both needed *, so this iteration landed the segment glob engine: * and ? and bracket expressions that never match /, ranges, ] -literal-first, ! and ^ class negation, backslash escapes, lone trailing backslash matches nothing, unclosed class matches nothing. ** stays deliberately non-special until the globstar rows.
Changed: src/lib.rs - lit_match replaced by glob_match plus class_match; pattern_matches now globs. Contract preserved: is_ignored signature and source precedence untouched; swept rows blank-and-comment and trailing-whitespace re-verified green under the new engine in the same full replay, so no flip-back needed. BACKLOG.md row line deleted. 3 new unit tests pin slash-crossing, classes, escapes.
Checkpoint: f949a7904bced6f69ce640ff7c16154f1346f9f9; PLAN.md row negation ticked with this hash in the bookkeeping edit. Stall check: lib.rs changed outside state files, row task closed, no stall.
Verification: Acceptance: observed failing first this iteration - slice replay 2 disagreements (a.log, b.log vs pattern *.log), then 0 after, 4 cases 7 queries, exit 0. Full Verify: 45 disagreements fell to 8, all in globstar-leading and globstar-infix (the un-implemented ** rows); globstar-trailing went green via the excluded-ancestor walk judging abc/d ignored under abc/**, the same mechanism git uses. Red at last checkpoint too, not a regression. cargo test: 4 suites ok (9 tests total). cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-directory-only - slice already green from the core; grow it with oracle-blind cases before ticking, per the Lessons rule.

## iter 8/12 | cd1c2080-020939 | 2026-08-04 | row-directory-only | done

Task: row-directory-only (High, runtime) - sweep inventory row 4. Slice was green from the core, so grew it 5 to 9 cases first per the Lessons rule: negated dir pattern (build/ then !build/), unanchored cache/ matching a nested dir plus a nonexistent same-name at root, wildcard *.d/ against a real dir and a same-named file, and foo\/ whose body degenerates to a lone backslash.
Changed: corpus/directory-only/06-09.case hand-authored; PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - existing dir_only handling, is_dir stat plumbing, and the ancestor walk survived all new cases.
Checkpoint: 67109f71d6ed2e1fabe443b2b00cdb4ff12dca96; PLAN.md row directory-only ticked with this hash in the bookkeeping edit. Stall check: 4 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4's recorded 4 disagreements on this slice against the stub; grown slice replays 9 cases, 17 queries, 0 disagreements, exit 0. The foo\/ prediction (dir_only strip leaves lone-backslash body, matches nothing) confirmed live by the oracle. Full Verify: exit 1, 61 cases, 142 queries, 8 disagreements, all in globstar-leading and globstar-infix; red at last checkpoint too, not a regression. Corpus grew 57 to 61, never shrank. cargo test: 4 suites ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-anchoring - also green from the core; grow with oracle-blind anchoring cases before ticking.

## iter 9/12 | cd1c2080-020939 | 2026-08-04 | row-anchoring | done

Task: row-anchoring (High, runtime) - sweep inventory row 5. Slice was green from the core, so grew it 4 to 8 cases first: doc/frotz inside sub/.gitignore (anchored to the nested file's directory, not the root and not deeper), anchored /*.txt at root only, a/*/c with the star bound to exactly one segment, and anchored dir-only /build/ not matching sub/build.
Changed: corpus/anchoring/05-08.case hand-authored; PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - the anchoring flag, per-source rel-path computation, and glob engine survived all new cases.
Checkpoint: f94168a6335a112962770b6dcf53e96adcb2a854; PLAN.md row anchoring ticked with this hash in the bookkeeping edit. Stall check: 4 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4's recorded 5 disagreements on this slice against the stub; grown slice replays 8 cases, 19 queries, 0 disagreements, exit 0. Full Verify: exit 1, 65 cases, 152 queries, 8 disagreements, all in globstar-leading and globstar-infix; red at last checkpoint too, not a regression. Corpus grew 61 to 65, never shrank. cargo test: 4 suites ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-wildcards - green from the glob engine; grow with oracle-blind wildcard cases before ticking.

## iter 10/12 | cd1c2080-020939 | 2026-08-04 | row-wildcards | done

Task: row-wildcards (High, runtime) - sweep inventory row 6. Slice was green from the glob engine, so grew it 5 to 9 cases first: bare * (matching dotfiles too - gitignore globs carry no shell dotfile exemption, confirmed by the oracle), a*b*c multi-star backtracking including the abcbc absorb case, ??.txt repeated single-char, and *.[ch] star-plus-class with a basename match at depth.
Changed: corpus/wildcards/06-09.case hand-authored; PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - the glob engine survived all new cases.
Checkpoint: 1db77f8e8b793425caa376e2809d55e0bf0879b3; PLAN.md row wildcards ticked with this hash in the bookkeeping edit. Stall check: 4 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4's recorded 8 disagreements on this slice against the stub; grown slice replays 9 cases, 31 queries, 0 disagreements, exit 0. Full Verify: exit 1, 69 cases, 166 queries, 8 disagreements, all in globstar-leading and globstar-infix; red at last checkpoint too, not a regression. Corpus grew 65 to 69, never shrank. cargo test: 4 suites ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-char-classes - green from the engine; grow with oracle-blind class cases before ticking.
