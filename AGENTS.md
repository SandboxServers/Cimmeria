# Repo Instructions

## Frontend UAT Rule

For every new frontend feature, milestone, or meaningful behavior change:

- perform a JS REPL-style logic UAT in addition to normal tests/builds
- verify the underlying state/persistence behavior, not just compile success
- summarize what was exercised and what passed
- explicitly call out what was **not** covered by the REPL pass

This applies especially to:

- card create/update/delete flows
- sequence/thread edits
- card movement and layout state
- input/output port changes
- persistence and dirty-state transitions
- validation-state changes
- content-engine serialization/deserialization changes

## Notes

- REPL-style logic UAT supplements browser/manual UAT; it does not replace visual verification.
- If a feature cannot be meaningfully exercised in the JS REPL, state that clearly and explain why.

## PR process — tests and docs

Every PR that changes runtime behaviour must add or update a test, and every PR that changes user-visible behaviour, public surface, file layout, build steps, or test policy must update the corresponding documentation in the same PR.

- **Tests**: read [TESTING.md](TESTING.md) before authoring. It covers the eleven test types (unit / wire-format / live-DB / smoke / concurrency / chain-replay / legacy reference / fan-out byte / Mercury session / network chaos / wire-level replay), the picker for which type fits which bug shape, and the review gotchas mined from PRs #131 onwards. A PR that adds or removes ≥5% of the workspace test count (~100 tests at the current 2,012 baseline) updates the catalogue at [docs/testing/inventory/](docs/testing/inventory/) in the same PR; smaller drifts get folded in by periodic sweeps.
- **Docs**: see the "Required documentation for every PR" mapping in [CLAUDE.md](CLAUDE.md). For non-trivial doc work, use the **Documentation Writer** agent — Diátaxis-aware (tutorials / how-to / reference / explanation) and keeps voice consistent with the rest of `docs/`. Index entries in `docs/readme.md` and per-section README files stay in sync with the documents they list.
