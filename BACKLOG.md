# Backlog

Ledger, not narrative. Top unblocked item is next. Markers: [ ] open, [b] blocked.

Rules:
- One line per item: `- [ ] <ID> (<Severity>, <class>, <dimension>): <finding>. Acceptance: <runnable command or observable fact>.`
- Class is one of runtime, test, build-ci, docs, dev-tooling, chosen by the files the fix will touch; a line without one is read as runtime. Within a section, order by severity first, then runtime before the other classes.
- A finished task is deleted from its section and recorded as one line in the JOURNAL entry that closed it. No done markers accumulate here.
- Run context, audit scores, and DONE annotations live in JOURNAL.md only. No prose sections and no headings beyond the ones below, ever.

## Now

- [ ] fix-dir-applies-prefix-panic (High, runtime, matcher): FS-resolved dir_applies admits query prefixes whose byte length differs from the stored source dir (8.3 short names: LONGDI~1 resolves to longdirectoryname), and judge then strips dir.len()+1 bytes from the query - reproduced: F longdirectoryname/.gitignore y.txt, Q LONGDI~1/y.txt panics the differential binary (byte index 18 out of bounds of LONGDI~1/y.txt, exit 101) while the oracle answers ignored (pattern y.txt at LONGDI~1/.gitignore:1). Fix by deriving the source-relative path from the query's own segments at the source's depth, never from dir.len(). Acceptance: an 8.3 short-name query case observed crashing first, then replaying 0 disagreements.
- [ ] fix-trailing-dot-prefix-overreach (Medium, runtime, matcher): dir_applies canonicalize admits Win32-normalization-only aliases that git's own source open does not honor - reproduced: F sub/.gitignore plain.txt, Q sub./plain.txt and Q sub /plain.txt both matcher=ignored vs oracle=not-ignored (no pattern matched), control sub/plain.txt agreeing. NTFS-level aliases (case fold, 8.3 names) must stay honored. Acceptance: trailing-dot and trailing-space prefix queries observed failing first, then replaying 0 disagreements.




## Next


## Later

## Proposed

Items needing a user decision before any work, one plain line each, never a checkbox task: envelope changes, audit escalations, challenges to a settled class. Never worked without explicit user approval and never counted against convergence.

## Settled classes

One line per class: the idiom or defect class, the surface it applies to, and how it was settled - fixed class-complete with its enumerating check, or declined with the reason. Audits must not file findings inside a settled class unless its implementing code changed after settlement.

- icase-reach (matcher and harness): every byte comparison on the verdict path folds under core.ignoreCase - fixed class-complete; sites are wildmatch text matching (WM_CASEFOLD port in glob_match/class_match/prefix strip), nested-.gitignore dir-prefix applicability (dir_applies), ignore-source name discovery in materialize, and the filesystem's fold on write during materialization (colliding F records fold to one source, last content under the first name, keyed by the canonical on-disk path so the volume answers its own fold and directory-component collisions fold too); each pinned by corpus cases the Verify command replays (wildcards/10, char-classes/13-15, nested-and-layered/11-13) plus unit tests; the oracle echo check is byte-exact by design since git echoes queries verbatim. Run-3 evaluator reopened the class at the Unicode boundary; run 09db09fe-045619 iters 1-2 closed it: the fold's breadth was probed live (the volume's $UpCase table, matching no portable Unicode fold), collision modeling and source discovery and applicability now ask the filesystem itself (canonical-path source keying and FS-resolved is_source lookup in materialize, dir_applies through set_repo_root), and wildmatch text matching keeps the ASCII-only WM_CASEFOLD port - probed: patterns match the query's own spelling, never the disk's; pinned by corpus nested-and-layered/14-15 and unit tests exercising both folding and non-folding volumes.

## Declined

Findings judged not worth fixing, one line each with the reason. Audits must not re-file these.

## Converged

One line per convergence, appended, never rewritten: Converged: <full commit hash> - <date>. The ratchet reads the latest line here.
