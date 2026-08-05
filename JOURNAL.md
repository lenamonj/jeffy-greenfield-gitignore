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

## iter 11/12 | cd1c2080-020939 | 2026-08-04 | row-char-classes | done

Task: row-char-classes (High, runtime) - sweep inventory row 7. Slice was green from the glob engine, so grew it 7 to 12 cases first: [a\-c] escaped hyphen as literal member (not a range - bx must not match), [a-a] single-char range, [a/b] class containing a slash member, [!\!] negated class with escaped bang, [z-a] inverted range matching nothing.
Changed: corpus/char-classes/08-12.case hand-authored; PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - class_match survived all new cases including the inverted range.
Checkpoint: 7912a65e559a1c456ed84cb4807e9cf2bf27ac37; PLAN.md row char-classes ticked with this hash in the bookkeeping edit. Stall check: 5 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4's recorded 9 disagreements on this slice against the stub; grown slice replays 12 cases, 30 queries, 0 disagreements, exit 0. Full Verify: exit 1, 74 cases, 178 queries, 8 disagreements, all in globstar-leading and globstar-infix; red at last checkpoint too, not a regression. Corpus grew 69 to 74, never shrank. cargo test: 4 suites ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: iteration 12 is the final iteration - WRAPUP: tidy the ledger and write the handoff; the remaining 5 rows (globstar x3 needing ** implementation, precedence, nested-and-layered) carry to the next run.

## iter 12/12 | cd1c2080-020939 | 2026-08-04 | WRAPUP | done

Task: WRAPUP - final iteration; the next row task (row-globstar-leading) requires implementing ** and cannot both land and be swept safely in the last slot, and 4 more rows follow it, so the run closes with a tidy ledger and this handoff instead.
Changed: JOURNAL.md only (this entry); BACKLOG.md already tidy - 5 one-line row tasks in Next, in frozen inventory order, nothing in Now or Later, nothing Proposed, nothing blocked.
Checkpoint: d6a79c65ad3a563a3d1993f3b9c1b5335bc0085f. Stall check: only state files changed this iteration and no backlog item changed state - this is the designed WRAPUP shape on the final iteration, not a stall; the previous primary entry closed a task, so no consecutive no-progress pair exists.
Verification: Final Verify this iteration: exit 1, 74 cases, 178 queries, 8 disagreements, 0 harness failures - the 8 sit entirely in globstar-leading (4) and globstar-infix (4). Slices green but rows unswept: globstar-trailing, precedence, nested-and-layered. Inventory position 7 of 12 rows swept, each ticked with its sweep commit. Corpus grew 53 to 74 cases across the run, never shrank.
Learnings: Handoff for the next run, in ledger order: (1) row-globstar-leading and the ** work - extend glob_match in src/lib.rs; per gitignore(5), ** is special only as a whole path segment (leading **/, trailing /**, infix /**/), and ** adjacent to other characters behaves as a plain *; note **/foo is anchored by its slash, so the fix lives in path-relative matching, and expect the current engine's 8 disagreements to be the failing-first evidence. (2) globstar-trailing, precedence, nested-and-layered are green side-effect slices - grow each with oracle-blind cases before ticking, per the Lessons rule. (3) Invariants that bind every future run: corpus only grows; corpus_gen freeze guard stays; oracle contract in PLAN.md is settled fact; verify command never narrows.
Next: fresh session, relaunch /jeffy in this directory; the state files carry everything forward.

## iter 1/12 | ca25dfca-024109 | 2026-08-04 | row-globstar-leading | done

Task: row-globstar-leading (High, runtime) - sweep inventory row 8 by implementing ** in the glob engine, per the previous run's handoff.
Changed: src/lib.rs - glob_match star arm now consumes the whole star run and crosses / only when a ** run forms a whole path segment, with the **/ zero-directory branch; pattern_matches anchored branch now ports dir.c match_pathname's literal-prefix strip via no_wildcard_len. Contract preserved: is_ignored signature and source precedence untouched; all 7 swept rows re-verified green under the changed engine in the same full replay, so no flip-back needed. corpus/globstar-leading/04-06.case hand-authored (od-verified); PLAN.md growth line added; BACKLOG.md row line deleted. 2 new unit tests pin whole-segment globstar semantics and the prefix-strip quirk.
Checkpoint: 7365453e92a6ea249698ba58a04fedc2abd58e42; PLAN.md row globstar-leading ticked with this hash in the bookkeeping edit. Stall check: src/lib.rs and 3 corpus case files changed outside state files, row task closed, no stall.
Verification: Acceptance observed failing first this iteration: slice replay 4 disagreements against the old engine (foo, a/b/foo, foo/bar, dir-only d - the zero-depth and multi-depth shapes), then 0 after implementing. Oracle-blind growth caught a real defect live: pure wildmatch semantics call x**/foo not-special, but git strips the literal prefix in dir.c match_pathname before wildmatch, so x**/foo ignores x/a/foo and xfoo (2 disagreements, fix attempt 1 landed the strip, then 0). Grown slice: 6 cases, 19 queries, 0 disagreements, exit 0. Full Verify: exit 0, 77 cases, 188 queries, 0 disagreements, 0 harness failures - first green full replay; globstar-infix went green as a side effect, its row stays unticked pending grow-before-tick. Corpus grew 74 to 77, never shrank. cargo test: 9 tests across 4 suites, all ok. cargo tree: gitignore-matcher alone.
Learnings: git's gitignore semantics live in dir.c call sites as much as in wildmatch: the match_pathname literal-prefix strip is semantic, making the first wildcard run segment-initial (x**/foo is x + globstar, not x*/foo). Port the composition, never the manpage alone; the oracle judges the composition.
Next: row-globstar-trailing - slice green from the trailing-/** arm; grow with oracle-blind cases before ticking, per the Lessons rule.

## iter 2/12 | ca25dfca-024109 | 2026-08-04 | row-globstar-trailing | done

Task: row-globstar-trailing (High, runtime) - sweep inventory row 9. Slice green as a side effect of iter 1's ** engine, so grew it 3 to 6 cases first per the Lessons rule, oracle-blind: build/** plus !build/keep (re-inclusion works because dir/** excludes contents, never the dir itself), attached a/x** (the prefix-strip quirk's trailing twin - a/xfoo/bar is ignored because the stripped ** crosses depth, and a/x itself matches the bare ** remainder), and dir-only d/**/ (ignores the subdirectory, spares the file).
Changed: corpus/globstar-trailing/04-06.case hand-authored per the NUL token format (od-verified); PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - the iter 1 engine survived all new cases.
Checkpoint: c68865bae65b0f0beea474fb81bb7e18146c3e60; PLAN.md row globstar-trailing ticked with this hash in the bookkeeping edit. Stall check: 3 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4's recorded full-replay failure against the stub, every slice showing disagreements (69 total); grown slice replays 6 cases, 19 queries, 0 disagreements, exit 0 - all three oracle-blind predictions confirmed live by the oracle. Full Verify: exit 0, 80 cases, 198 queries, 0 disagreements, 0 harness failures. Corpus grew 77 to 80, never shrank. cargo test: 9 tests across 4 suites, all ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-globstar-infix - slice green from the ** engine; grow with oracle-blind cases before ticking.

## iter 3/12 | ca25dfca-024109 | 2026-08-04 | row-globstar-infix | done

Task: row-globstar-infix (High, runtime) - sweep inventory row 10. Slice green as a side effect of iter 1's ** engine, so grew it 3 to 6 cases first per the Lessons rule, oracle-blind: chained a/**/b/**/c (both runs collapse to zero dirs in a/b/c, expand independently in a/x/b/y/c), dir-only infix a/**/tmp/ at zero and one depth plus an out-of-anchor miss, infix inside a nested .gitignore (anchored to sub, root x/a/y unmatched), and attached a/b**/c (prefix strip makes the run segment-initial, so a/b/x/c is ignored, not just a/bx/c).
Changed: corpus/globstar-infix/04-06.case hand-authored per the NUL token format (od-verified); PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - the iter 1 engine survived all new cases.
Checkpoint: 149b1b6985016d9b8dbc857efbd921e9a3b81e62; PLAN.md row globstar-infix ticked with this hash in the bookkeeping edit. Stall check: 3 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is this run's iter 1 pre-fix full replay showing globstar-infix 4 disagreements against the pre-** engine (recorded in the iter 12/12 WRAPUP of the previous run and re-observed at iter 1 start); grown slice replays 6 cases, 22 queries, 0 disagreements, exit 0. Full Verify: exit 0, 83 cases, 210 queries, 0 disagreements, 0 harness failures. Corpus grew 80 to 83, never shrank. cargo test: 9 tests across 4 suites, all ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-precedence - slice green from the core's last-match-wins; grow with oracle-blind precedence cases before ticking.

## iter 4/12 | ca25dfca-024109 | 2026-08-04 | row-precedence | done

Task: row-precedence (High, runtime) - sweep inventory row 11. Slice green from the core's last-match-wins, so grew it 4 to 7 cases first per the Lessons rule, oracle-blind. The frozen cases already covered re-exclusion ping-pong and last-matching-vs-last-line, so growth targeted last-wins interacting with the other pattern flags: temp/ then !temp (plain negation last spares the dir at root and at depth), anchored sub/a.txt overridden by a later unanchored !a.txt and the reverse order re-excluding sub/d.txt, and n then !n/ vs f then !f/ (dir-only negation matches the dir, never the file, so n survives and f stays ignored).
Changed: corpus/precedence/05-07.case hand-authored per the NUL token format (od-verified); PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - last_match, the dir_only gate, and the anchoring split survived all new cases.
Checkpoint: 1b32ddc656d40b60dc8c08942b18995450bd58b3; PLAN.md row precedence ticked with this hash in the bookkeeping edit. Stall check: 3 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4 of the previous run - full replay against the stub showed every slice failing (69 disagreements); grown slice replays 7 cases, 14 queries, 0 disagreements, exit 0. Full Verify: exit 0, 86 cases, 217 queries, 0 disagreements, 0 harness failures. Corpus grew 83 to 86, never shrank. cargo test: 9 tests across 4 suites, all ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: row-nested-and-layered - the last unswept row; grow with oracle-blind cases exercising deeper-overrides-shallower and the source layering beneath the tree.

## iter 5/12 | ca25dfca-024109 | 2026-08-04 | row-nested-and-layered | done

Task: row-nested-and-layered (High, runtime) - sweep inventory row 12, the last unswept row. Slice green from the core's source-precedence stack, so grew it 7 to 10 cases first per the Lessons rule, oracle-blind: !keep.txt inside build/.gitignore while build/ is excluded by the root (the ancestor exclusion is absolute - a negation inside an excluded directory is never honored), tree !keep.dat overriding a core.excludesFile *.dat (the X-to-F pairing; only E-to-F existed), and sub/.gitignore carrying both an own-directory-anchored /local.txt (matches sub/local.txt, not sub/deep/local.txt or root local.txt) and cache/ whose exclusion kills sub/cache/data.txt through the ancestor walk.
Changed: corpus/nested-and-layered/08-10.case hand-authored per the NUL token format (od-verified); PLAN.md growth line added; BACKLOG.md row line deleted. No source changes - the precedence stack, ancestor walk, and per-source rel-path anchoring survived all new cases.
Checkpoint: 449703383cd28c20561e2816f3b6d3bf95779109; PLAN.md row nested-and-layered ticked with this hash in the bookkeeping edit. Stall check: 3 corpus case files created outside state files, row task closed, no stall.
Verification: Acceptance: failing-first evidence is iter 4 of the previous run - full replay against the stub showed every slice failing (69 disagreements); grown slice replays 10 cases, 20 queries, 0 disagreements, exit 0. Full Verify: exit 0, 89 cases, 224 queries, 0 disagreements, 0 harness failures. Corpus grew 86 to 89, never shrank. cargo test: 9 tests across 4 suites, all ok. cargo tree: gitignore-matcher alone.
Learnings: none.
Next: ledger empties with all 12 rows swept - Scoped-mode audit against the Definition of done to derive any remaining tasks or start closeout.

## iter 6/12 | ca25dfca-024109 | 2026-08-04 | AUDIT | audit

Task: Scoped-mode audit against the Definition of done, ledger empty and all 12 rows ticked. Fresh evidence this iteration: cargo build --release exit 0; full Verify exit 0 - 89 cases, 224 queries, 0 disagreements, 0 harness failures, every slice individually 0; cargo test 9 tests across 4 suites ok; cargo tree shows gitignore-matcher alone; 12 rows [x] and 0 unswept in PLAN.md; corpus on disk 89 case files matching the recorded total, monotone growth 53-57-61-65-69-74-77-80-83-86-89 never shrinking; corpus_gen freeze guard re-probed live - refuses the existing corpus with exit 1 and leaves the tree untouched. Staleness handled: the 7 rows ticked in the previous run (blank-and-comment, trailing-whitespace, negation, directory-only, anchoring, wildcards, char-classes) recorded sweep commits predating this run's iter 1 engine change to glob_match and pattern_matches, so they were formally stale; this audit's full replay re-swept all 7 slices at the current commit with 0 disagreements, and the bookkeeping edit re-ticks them with this audit's checkpoint hash. Doc consistency re-checked on the changed module: glob_match, pattern_matches, Pattern, and Matcher doc comments state the implemented semantics including the prefix-strip quirk.
Changed: no source or corpus changes; PLAN.md row hashes refreshed in the bookkeeping edit; BACKLOG.md untouched (nothing filed).
Checkpoint: 18bf16df780d562bc42b37be586d47534f5beff8; the 7 stale rows re-ticked with this hash in the bookkeeping edit (grep confirms 7 rows carry it, 12 rows [x] total). Stall check: audit iteration, no non-state file changed and no backlog item changed state - this is the designed audit shape entering closeout, not a stall; the previous primary entry closed a row task, so no consecutive no-progress pair exists.
Verification: Scores: High 0, Medium 0, Low 0 in-envelope, all 12 inventory rows swept fresh this iteration by the full replay - no unexamined remainder. DoD items 1 through 6 verified true with the evidence above; items 7 and 8 (evaluator PASS and the Converged line) are the convergence sequence itself. Closeout begins: no further audits or replenishment this run; the remaining work is the evaluator gate and, on PASS, the declaration.
Learnings: none.
Next: evaluator gate next iteration - ledger empty, clean full audit recorded, 6 iterations remain, so the gate runs now rather than deferring to a declaration it could reject too late.

## iter 7/12 | ca25dfca-024109 | 2026-08-04 | EVALUATOR | audit

Task: Adversarial evaluator gate, run early per the gate-timing rule (ledger empty, clean audit at iter 6, 5 iterations of budget beyond this one). Fresh-context sub-agent reviewed the run against 6e23baf92042053c50c567d243ad2401341b2085, re-ran the Verify command (exit 0, 89 cases, 224 queries, 0 disagreements), all 5 closed tasks' slice replays (all 0 disagreements), cargo test (9 ok), cargo tree (crate alone), corpus count (89, monotone), and probed beyond the corpus with scratch cases against the live oracle.
Changed: BACKLOG.md gains 3 evaluator-filed Medium tasks in Now (fix-globstar-escaped-slash, fix-posix-bracket-classes, fix-bom-skip). No source changes this iteration.
Checkpoint: d4c0ac2c74f3e60b9562571784977aa38182c3b3. Stall check: 3 backlog items changed state (created open), evaluator iteration, no stall.
Verification: Evaluator: REJECT - three in-envelope Medium disagreements reproduced through the run's own harness on scratch cases (3 cases, 8 queries, 4 disagreements, exit 1): a/**\/b not crossing directories (glob_match segment-end omits wildmatch.c's backslash-slash case), [[:digit:]].txt disagreeing both directions (no POSIX bracket classes in class_match), and an unskipped UTF-8 BOM making the first pattern of an ignore file dead. All three are plausible hand-authored gitignore content, squarely in-envelope, Medium per the rubric (in-envelope case outside the corpus that disagrees). Each filed as an ordinary task at rubric severity; the frozen-corpus Verify remains green this iteration (exit 0 recorded above). Invocation cap: this was invocation 1 of 2 (first landed after the midpoint); the re-invocation at declaration is the last, so all three fixes must land and hold before it.
Learnings: The Lessons rule already says port the composition, not the manpage; the evaluator found the same failure mode a third way (wildmatch.c's escaped-slash clause, POSIX classes, and dir.c's BOM skip all live outside gitignore(5)). The class rule applies: the structural answer is reading wildmatch.c and the dir.c ignore-file reader as the spec, which the three filed fixes complete.
Next: fix-globstar-escaped-slash - grow corpus/globstar-infix with the a/**\/b shape, observe it failing, extend the segment-end test.

## iter 8/12 | ca25dfca-024109 | 2026-08-04 | fix-globstar-escaped-slash | done

Task: fix-globstar-escaped-slash (Medium, runtime), evaluator-filed. glob_match's segment-end test accepted only end-of-pattern or /, omitting wildmatch.c's backslash-slash clause, so a/**\/b degraded to a plain star run and never crossed directories.
Changed: src/lib.rs - at_seg_end gains the backslash-slash case, and the zero-directory branch is guarded to fire only on a literal / (wildmatch grants zero-dir to `**/`, never `**\/`); doc comment updated to state both. Contract preserved: is_ignored signature untouched; the change only widens which star runs cross slashes, and every swept slice re-verified green under the changed engine in the same full replay. corpus/globstar-infix/07.case hand-authored (od-verified); PLAN.md growth line added; BACKLOG.md task line deleted. 3 new asserts pin the escaped-slash semantics.
Checkpoint: eee469021ee289f237b0299b1e2fdd853e33be4b; PLAN.md row globstar-infix re-ticked with this hash (its slice grew and re-swept here); the other rows re-verified green in the same full replay. Stall check: src/lib.rs and 1 corpus case file changed outside state files, task closed, no stall.
Verification: Acceptance observed failing first this iteration: grown slice replayed 1 disagreement (a/x/y/b, oracle=ignored via pattern a/**\/b, matcher=not-ignored; a/x/b agreed by accident because plain-star-plus-literal covers exactly one level, and the oracle confirmed no zero-dir reading at a/b), then 0 disagreements after the fix, 7 cases, 26 queries, exit 0. Fix attempt 1 of 3. Full Verify: exit 0, 90 cases, 228 queries, 0 disagreements, 0 harness failures. Corpus grew 89 to 90, never shrank. cargo test: 9 tests across 4 suites, all ok (escaped-slash asserts added to the globstar test). cargo tree: gitignore-matcher alone.
Learnings: none.
Next: fix-posix-bracket-classes - grow corpus/char-classes with [[:digit:]] cases, observe failing both directions, implement POSIX classes in class_match.
