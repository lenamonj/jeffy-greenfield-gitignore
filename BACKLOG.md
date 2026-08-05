# Backlog

Ledger, not narrative. Top unblocked item is next. Markers: [ ] open, [b] blocked.

Rules:
- One line per item: `- [ ] <ID> (<Severity>, <class>, <dimension>): <finding>. Acceptance: <runnable command or observable fact>.`
- Class is one of runtime, test, build-ci, docs, dev-tooling, chosen by the files the fix will touch; a line without one is read as runtime. Within a section, order by severity first, then runtime before the other classes.
- A finished task is deleted from its section and recorded as one line in the JOURNAL entry that closed it. No done markers accumulate here.
- Run context, audit scores, and DONE annotations live in JOURNAL.md only. No prose sections and no headings beyond the ones below, ever.

## Now


## Next

- [ ] row-globstar-leading (High, runtime, matcher): Sweep inventory row 8. Acceptance: `corpus/globstar-leading` replays with zero disagreements, observed failing first; row ticked with sweep commit.
- [ ] row-globstar-trailing (High, runtime, matcher): Sweep inventory row 9. Acceptance: `corpus/globstar-trailing` replays with zero disagreements, observed failing first; row ticked with sweep commit.
- [ ] row-globstar-infix (High, runtime, matcher): Sweep inventory row 10. Acceptance: `corpus/globstar-infix` replays with zero disagreements, observed failing first; row ticked with sweep commit.
- [ ] row-precedence (High, runtime, matcher): Sweep inventory row 11. Acceptance: `corpus/precedence` replays with zero disagreements, observed failing first; row ticked with sweep commit.
- [ ] row-nested-and-layered (High, runtime, matcher): Sweep inventory row 12. Acceptance: `corpus/nested-and-layered` replays with zero disagreements, observed failing first; row ticked with sweep commit.

## Later

## Proposed

Items needing a user decision before any work, one plain line each, never a checkbox task: envelope changes, audit escalations, challenges to a settled class. Never worked without explicit user approval and never counted against convergence.

## Settled classes

One line per class: the idiom or defect class, the surface it applies to, and how it was settled - fixed class-complete with its enumerating check, or declined with the reason. Audits must not file findings inside a settled class unless its implementing code changed after settlement.

## Declined

Findings judged not worth fixing, one line each with the reason. Audits must not re-file these.

## Converged

One line per convergence, appended, never rewritten: Converged: <full commit hash> - <date>. The ratchet reads the latest line here.
