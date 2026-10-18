# ADO work item generator resume card

Spec only; no code, no worktree, no PR. The spec is [spec.md](spec.md): a standalone `adogen` tool that seeds about 10,000 work items, with a ground-truth manifest for stress-testing the work item migration.

## State, 2026-10-16

- The spec draft is written. There were no earlier ADO migration notes in `docs/`, `.claude/` or the auto-memory (spec Section 0, A1).
- Nothing was checked against live Microsoft docs or a live org. Claims are tagged K, R or U; M0 (`adogen probe`) measures every R and U item.
- The brief's limits "1,000 revisions" and "batch size 100" conflict with what I recall (10,000 and 200). Both are flagged in spec Section 10.

## Owner decisions (BlockedDecision, spec Section 13)

- D-AG1 language (Rust proposed; C# if the migration code is .NET).
- D-AG2 location (standalone repo recommended).
- D-AG3 Services only, or also Server 2022.
- D-AG4 sandbox org and identity pool.
- D-AG5 diff profile: what the migration must preserve.
- D-AG6 `extreme` tier on by default?
- D-AG7 process churn in scope?
- D-AG8 migration target kind.

## Next actions

1. Owner answers D-AG1, D-AG2, D-AG4 and D-AG8. These block packet slicing.
2. Architect pass: turn milestones M0 to M7 (spec Section 11.3) into a README.md and work-packets.md ledger here (packets of about 40k tokens, waves of about 6).
3. M0 probe spike first; update spec Sections 6.4 and 10 with measured values before M1.
