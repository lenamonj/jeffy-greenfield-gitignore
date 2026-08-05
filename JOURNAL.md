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
