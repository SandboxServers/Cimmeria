# Union SGC continuation: resume card

> **Last updated:** 2026-10-18 (US-00)
> **Ledger:** [README.md](README.md) · **Crossings:** [crossings.md](crossings.md) · **Index:** [sgc-work-packets.md](../union-zone-planning/sgc-work-packets.md)

## Current wave

**S0** (specialist seals). US-00 is the ledger and check packet and is committed on `union-sgc/us-00-sgc-start`. It is not yet merged.

## In flight

- US-00: this commit (docs only; no seed or code changed).
- Nothing else is dispatched by this ledger.

## Next action

1. Coordinator merges the US-00 commit and the D-US1 amendment request into `docs/analysis/four-zone-planning/id-allocation.md` (request rows are in README § Amendment request).
2. Dispatch US-01, US-02, US-10 and US-14 (each depends only on US-00).
3. Coordinator aligns the C4 arrival pin with Omega (crossings.md § C4 mismatch) before US-32 is dispatched.

## Open decisions

| Decision | Blocks | Default in the packet |
|---|---|---|
| D-US2, D-US3 (label policy, authored scale) | US-40 (HELD) | none |
| D-US4 (dial before crossings) | nothing hard | play both sequences |
| D-US5 (return state) | US-34 | D-US5 default |
| D-US6 (gate walk) | nothing | cross on the Harriman interact |
| D-US7 (virus minigame) | nothing | plain interact |
| D-US8 (1565 flank objective) | nothing | untracked |
| D-US10 (static Jaffa rows) | US-11 step 5, US-20, US-21 | option B |
| D-US11 (letter 6786) | US-32 | 6786 to Humans only |

## Findings to carry

- ID re-check: zero collisions in the five blocks (README § ID re-check).
- C4 arrival pin differs between SGC K5 and Omega's plan (coordinator).
- Engine: an all-optional step completes the mission on the first completion (progression.rs:469-505); chain order is priority then load order (chain/mod.rs:152-168), not chain id.
- Send-side dialog rule (-1 on close) is not in `dialog.rs`; lab check before a packet relies on it.
