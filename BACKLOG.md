# Backlog

Ledger, not narrative. Top unblocked item is next. Markers: [ ] open, [b] blocked.

Rules:
- One line per item: `- [ ] <ID> (<Severity>, <class>, <dimension>): <finding>. Acceptance: <runnable command or observable fact>.`
- Class is one of runtime, test, build-ci, docs, dev-tooling, chosen by the files the fix will touch; a line without one is read as runtime. Within a section, order by severity first, then runtime before the other classes.
- A finished task is deleted from its section and recorded as one line in the JOURNAL entry that closed it. No done markers accumulate here.
- Run context, audit scores, and DONE annotations live in JOURNAL.md only. No prose sections and no headings beyond the ones below, ever.

## Now

- [ ] fix-unicode-source-applicability (Medium, runtime, matcher): dir_applies and the harness source model fold ASCII-only, but git reaches a nested .gitignore through the filesystem's Unicode fold along the query path - reproduced: single source authored at sÜb, q=süb/a.txt matcher=not-ignored vs oracle=ignored (pattern a.txt at süb/.gitignore:1, git naming the source in the query's own spelling). Fold breadth probed at iter 1/12 of run 09db09fe-045619: the fold is the volume's $UpCase table, matching no portable Unicode fold, so applicability must follow the filesystem, never a static fold function. Acceptance: a cross-case Unicode query against a non-ASCII-cased nested source observed failing first, then replaying 0 disagreements.



## Next


## Later

## Proposed

Items needing a user decision before any work, one plain line each, never a checkbox task: envelope changes, audit escalations, challenges to a settled class. Never worked without explicit user approval and never counted against convergence.

## Settled classes

One line per class: the idiom or defect class, the surface it applies to, and how it was settled - fixed class-complete with its enumerating check, or declined with the reason. Audits must not file findings inside a settled class unless its implementing code changed after settlement.

- icase-reach (matcher and harness): every byte comparison on the verdict path folds under core.ignoreCase - fixed class-complete; sites are wildmatch text matching (WM_CASEFOLD port in glob_match/class_match/prefix strip), nested-.gitignore dir-prefix applicability (dir_applies), ignore-source name discovery in materialize, and the filesystem's fold on write during materialization (colliding F records fold to one source, last content under the first name, keyed by the canonical on-disk path so the volume answers its own fold and directory-component collisions fold too); each pinned by corpus cases the Verify command replays (wildcards/10, char-classes/13-15, nested-and-layered/11-13) plus unit tests; the oracle echo check is byte-exact by design since git echoes queries verbatim. Run-3 evaluator reopened the class at the Unicode boundary: NTFS folds Unicode while every ported fold is ASCII-only, desynchronizing collision modeling (fix-unicode-collision-fold) and source applicability (fix-unicode-source-applicability); the class regains class-complete standing when both are fixed, with the fold's true breadth decided by direct oracle probes.

## Declined

Findings judged not worth fixing, one line each with the reason. Audits must not re-file these.

## Converged

One line per convergence, appended, never rewritten: Converged: <full commit hash> - <date>. The ratchet reads the latest line here.
