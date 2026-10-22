## Summary

Wave 1 of the launcher-uat-fixes campaign (U1): unbricks light in-place adoption found in the 2026-10-22 laptop UAT (F1, F2).

- LF-01/LF-04/LF-05: per-file Light final check (D-LF1), Recover finishes old adoptions, non-ASCII stock names accepted (D-LF3).
- LF-02: engine-side adoption state carried by the C-LF3 stage files (no `Plan` field added; the plan digest is unchanged).
- LF-03: Abandon works after `Promoted` for Light adoptions (D-LF2).

Reviewed per packet (LF-01 ship p1; LF-04, LF-05, LF-02 ship p2; LF-03 ship p2) and as a wave (pass 2 ship).

## Owner note

D-LF1 replaces the dry-run option (a) of launcher-telemetry D2 with a per-file final check (F1b). D2 (abandon wording) stays open.

## Tested / untested

Engine unit and integration tests run through the build lane. Never tested: a real half-adopted folder on the laptop (owner UAT U1, Recover with a U1 build; ask before any laptop use).

## Docs

Campaign ledger: docs/analysis/launcher-uat-fixes/ (committed with the close-out packet LF-20). No status docs change per packet.

Merge order: U1 first.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NdhQoumCZtZKGCMdPDbWJF
