# Lab roadmap

> Type: index. Audience: the coordinator of any lab-roadmap campaign.
> Opened 2026-10-10 against `main` @ `6996c9403`. The brief the ledgers were
> planned from: [handoff.md](handoff.md).
>
> **Status (2026-10-10): seven campaigns planned, nothing built.** Each
> campaign has its own ledger and packets. This page says how they fit
> together: the order they run in, the files more than one of them edits,
> and the owner decisions that gate them.

The goal behind all of them is inferenceless lab work: a model writes or
reviews a spec once, and every run after that is deterministic and needs no
inference.

## The campaigns

| Effort | Campaign | Prefix | Packets | Can start now | Ledger |
|---|---|---|---|---|---|
| 1 | Spec vocabulary and self-calibration | `SV-` | 16 | SV-01, SV-02, SV-03, SV-05 | [lab-spec-vocab](../lab-spec-vocab/README.md) |
| 2 | Row fixtures and rewind | `FX-` | 13 | After D-FX1 to D-FX3 | [lab-fixtures](../lab-fixtures/README.md) |
| 3 | Golden runs and run diffing | `GD-` | 11 | GD-01 to GD-05 (pure code) | [lab-golden](../lab-golden/README.md) |
| 4 | `lab record` and `.bug spec` | `LR-` | 17 | After SV- and GD- contracts land | [lab-record](../lab-record/README.md) |
| 5A | Client limits and bounds | `CL-` | 16 | CL-01 to CL-06, CL-11 | [client-limits](../client-limits/README.md) |
| 5B | Chaos mode | `CH-` | 12 | CH-01, CH-03 | [lab-chaos](../lab-chaos/README.md) |
| 6 | `lab watch` | `LW-` | 6 | LW-01, LW-02 | [lab-watch](../lab-watch/README.md) |

The wire-client CI campaign ([wireclient-ci](../wireclient-ci/README.md)) was
planned in the same session and is not part of this roadmap, but lab-chaos
depends on its #1341 outcome (D-WC8).

## Order

Effort 1 goes first, because everything else uses its step vocabulary. Then
2, 3 and 6 run in parallel; 4 starts after 1 and 3; 5A can run at any time;
5B starts after 3 and 5A.

```text
SV (1) ──┬── FX (2) ─────────────┐
         ├── GD (3) ──┬── LR (4) │
         └── LW (6)   │          │
CL (5A) ──────────────┴── CH (5B)┘
```

"After" means after the packets the later campaign names, not after the
whole earlier campaign. Each ledger's packet table lists the exact
cross-campaign dependencies, for example CH-09 on GD-04 and GD-09.

## Files more than one campaign edits

These are the collision points. The second packet to merge rebases onto the
first; none of them needs a design change.

| File | Campaigns | Rule |
|---|---|---|
| `crates/lab/src/uat/spec.rs` | SV-03, FX-01, GD-01 | Each adds its fields in one marked block (`// lab-spec-vocab (SV-03)`, `// ── Row fixtures (lab-fixtures FX-01) ──`, the GD-01 `GoldenSpec` block). The structs use `deny_unknown_fields`, so a spec that uses a new field fails to load until that block has merged. |
| `crates/lab/src/uat/runner/mod.rs` | SV-, FX-06, GD-06, LW-01, CH-08 | Order inside a row: fixture establish (FX-06), then the tap starts (GD-06), then the steps. LW-01 only adds progress fields to `Runner`. |
| `crates/lab/src/supervisor/mod.rs` (the `Supervisor` struct) | LW-01, LR-06 | Each adds one field. |
| `docs/guides/uat-specs/first-session.toml` | SV-15, FX-08, GD-10 | SV- recalibrates first; FX-08 then adds fixtures and drops its own camera prelude if SV-'s absolute camera has landed; GD-10 records the golden last. |
| `uat-runs\batch-*\batch.jsonl` and `plan.json` | LW-02 (owner), GD-, CH-, LR- | LW-02 owns the record shape. Others add fields; nobody rewrites it. |
| `tools/lab/cli/uat-lib.ps1` | LW-02, GD-08, SV-, CH- | Shared helpers move here once (GD-08 moves `Get-LabMcpUrl`, `Get-DefaultSpecsDir`, `Get-UatRoot`); later packets reuse them. |

## Cross-campaign decisions made while planning

| Decision | Reason |
|---|---|
| **Teleports.** A fixture's `position` is set server-side with `.gotoxyz` through `server_console_exec`, straight after the fixture relog (FX-06). Mid-row stand-offs use `runner::targets::stand_at` (SV-07). | `.gotoxyz` moves the GM's selected target if there is one (lab-fixtures F15). Straight after a relog the fresh entity has no selection, so the server-side path is safe there and needs no client typing; mid-row it is not. |
| **Fingerprint names** come from the packet tap's own `cimmeria-wire-log` names, not the wireclient decoders (D-GD2). | A run-to-run diff compares two runs through the same decoder, so it does not have D-WC4's shared-oracle problem. |
| **Fixture actions are tagged** `Role::Setup`, `kind = "fixture"`, and lab-golden strips setup actions from fingerprints. | A fixture must not make two otherwise-identical runs disagree. |
| **Chaos never records or re-blesses a golden** (D-CH7); a chaos-only divergence is `chaos_divergence`, not a row failure. | Keeps goldens clean. |
| **Recorded drafts are unconfirmed** until `lab golden record` agrees five times; a recorded pose carries `calibrated = "recorded <date>"`. | The owner's five-run rule applies to drafts as well. |
| **The mission bag is container 2** (`INV_MISSION`, `crates/entity/src/inventory.rs:14`), in FX-01 and every snapshot. | The first draft of FX-01 said 0. |

## Decisions waiting on the owner or the user

None of these is needed for the packets marked "can start now" above.

| Campaign | Owner decisions | Live lab use (needs the user's OK) |
|---|---|---|
| lab-spec-vocab | D-SV7 (calibration on the colo), D-SV8 (what `-Heal` does) | D-SV9: SV-15 |
| lab-fixtures | D-FX1 (`.fixture` console commands or a DB write tool), D-FX2 (the lab-only gate), D-FX3 (which character a fixture row plays) | FX-12 |
| lab-golden | D-GD6 (what may become golden) | GD-10 |
| lab-record | D-LR12 (drafts need five agreeing runs), D-LR13 (`.bug spec` on lab instances only) | D-LR15: LR-13, LR-16 |
| client-limits | D-CL4 (Large Address Aware, a maintainer call after CL-05), D-CL5 (lab ini overrides), D-CL7 (more than five clients; recommend defer), D-CL8 (the stock-client residue probe) | CL-08, CL-09, CL-10 |
| lab-chaos | D-CH1 (the `net` shim), D-CH2 (the triple gate; `network-security-auth` signs off), D-CH3 (the control tools) | CH-10, CH-11 |
| lab-watch | none | D-LW7: LW-05 |

## Rules every campaign inherits

These are in [handoff.md § Rules every campaign inherits](handoff.md#rules-every-campaign-inherits):
Haiku-sized packets with an adversarial Sonnet review, PowerShell and the
build lane only, one worktree and one test database per worker, never
`git worktree prune`, ask the user before any lab use, and compact output.
