# Resume: launcher preflight and actionable failure reporting

Design: [README.md](README.md). Packets: [work-packets.md](work-packets.md), [work-packets-display.md](work-packets-display.md), [work-packets-mac-extras.md](work-packets-mac-extras.md), [work-packets-wave3.md](work-packets-wave3.md). Owner rulings: [OWNER-DECISIONS.md](OWNER-DECISIONS.md) (D1-D5). Live state: [ledger.md](ledger.md).

## State (2026-10-24)

- Autonomous mode (owner order). darwin host: role agents run as `general-purpose` stand-ins (see ledger.md).
- Baseline `main` `2019922fe`. Owner questions all resolved (D5).
- Wave 1 and D in flight: PD-22 (PR #2004, UAT-PD7 gate), PD-01 (PR #2006, auto-merge), PD-02 (review-2), PD-23 (blocked, ruling-1). PD-38 done (`none`).
- Campaign docs: PR #2005.

## Waves

1 -> 2 -> 3; D beside 1-2; M (PD-28, PD-31..35, PD-38) after D, beside 2-3; 4 (PD-27, PD-29, PD-30, PD-36, PD-37, PD-41); 5 gated (PD-19, PD-20, PD-39, PD-40); close-out PD-21.

## Next actions (orchestrator)

1. PD-23: apply ruling-1 (Sonnet-class implementer), then a fresh reviewer.
2. PD-02: act on the review-2 verdict.
3. Wave 1 rest (PD-03..PD-06) once its dependencies land; PD-41 after wave 1.
4. Batch the live-Mac UATs (UAT-PD7 first) into one owner session.
5. After #2005 merges: sync the main checkout and keep the ledger on main.
