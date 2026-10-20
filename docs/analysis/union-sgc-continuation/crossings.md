# SGC_W1 crossings (frozen C1-C4)

> **Type:** Reference (frozen contract)
> **Audience:** SGC packets US-24, US-30, US-31, US-32 and US-14; Dakara DK-10b, DK-30, DK-33 and UD-04; Omega UO-00 and UO-02
> **Last updated:** 2026-10-19 (US-99; C4 pin resolved, see below)
> **Source:** index [sgc-work-packets.md](../union-zone-planning/sgc-work-packets.md) § K5, copied row for row. Partner checks are against the Dakara and Omega plans.

## Frozen rows

| Row | Mission and step | Direction | Departs | Departure rows, in order | State on arrival | Receiver | Coordinate |
|---|---|---|---|---|---|---|---|
| C1 | 1570 4641 → 4642 → 4906 | 58 → Dakara_E1 61 | SGC US-24 | `advance_step 1570 '4642'` @0, `play_sequence 10047` @0, `play_sequence 10060` @2500, `advance_step 1570 '4906'` @3000, `cross_world_teleport 'Dakara_E1'` @3000 | 1570/4906 active | DK-10b | gate 25 pin (96.87, -16.75, 243.23) |
| C2 | 1651 4930 | 61 → 58 | DK-30 (D-UD2) | `advance_step 1651 '4928'`, `cross_world_teleport 'SGC_W1'` | 1651/4928 active | SGC US-30 | H3 = P13 (167.00, -10.80, 18.00) |
| C2b | 1651 4931 → 4929 → 5171 | 58 → 61 | SGC US-30 | Carter's 5845 grants item 6784 and completes O5956 (safe: O6037 stays open, F13); Harriman's press on 4931 with O5956 done: `advance_step '4929'` @0 (force-completes O6037), sequences @0/@2500, `advance_step '5171'` @3000, teleport @3000; restore and recovery chains on 4929 | 1651/5171 active, item 6784 held | DK-30 | gate 25 pin |
| C3 | 1654 4939 | 61 → 58 | DK-33 | `complete_objective 1654 '5961'` (never `advance_step`), teleport | 1654/4939 active, O5962 open | SGC US-31 | H3 |
| C4 | 1655 4944 → 4941 → 4940 | 58 → Omega_Site 18 | SGC US-32 | `advance_step 1655 '4941'` @0, sequences @0/@2500, `advance_step 1655 '4940'` @3000, `cross_world_teleport 'Omega_Site'` @3000 | 1655/4940 active; Humans hold 6786 (D-US11) | Omega | Omega-published (UO-00); published pin x 34.70, y -51.28, z -142.00, heading 3.1409 (UO-01 seal; the engine carries no heading on cross_world_teleport) |

- **H3, the SGC arrival point (CIMMERIA_RECONSTRUCTION):** the Gate Room floor in front of gate 27. The gate row (172.79, -8.47, 6.014) sits 2.4 m above the floor (y -10.9). SGC_W1 is instanced and `validate_gate_arrival` skips instanced worlds, so nothing at runtime catches a bad y. US-01 sealed P13 = (167.00, -10.80, 18.00) in `placement.md`, and Dakara's real-travel returns land on the gate 27 pin that DK-SGC-01 sets to it (D-US9 revised, option C). The pin for gate 27 itself is Dakara's DK-SGC-01 (D-run7).
- **Telemetry:** INFO `Content: cross-world teleporting entity` (entity, account, player, world, position, chain_id, chain_name) and ERROR `CrossWorldTeleport: cell→base GateTravel send failed` with `reason = "base_channel_closed"`. US-14 adds the tests that pin both rows.

## Partner checks (US-00 step 3)

| Row | Partner file and line | What must match | Result |
|---|---|---|---|
| C1 | `docs/analysis/union-zone-planning/dakara-work-packets.md:113` (C1 row, DK-10b receiver); `:46` (DK-10a to DK-10e cover 1570 steps 4906, 4904, 4902, 4901, 4905, 4903) | Receiver expects 1570 active on 4906; gate 25 pin (96.87, -16.75, 243.23) | **Matches** |
| C2 | `dakara-work-packets.md:114` (C2 row); `:78` (D-UD2: `advance_step 1651 → 4928`, then `cross_world_teleport 'SGC_W1'`); `:49` (DK-30 owns 1651 4932, 5171) | DK-30 departs from 4930 with `advance_step 1651 '4928'` and teleports to SGC_W1; arrival H3 = P13 | **Matches**. Dakara's H3 row (`:159`) reads P13 from K5 |
| C2b | `dakara-work-packets.md:115` | Receiver 1651/5171 active with item 6784 held, at the gate 25 pin | **Matches** |
| C3 | `dakara-work-packets.md:116` (names D-UD7); `:51` (DK-33 is 1654 4939) | DK-33 uses `complete_objective 1654 '5961'`, never `advance_step`; receiver SGC US-31 (H3) | **Matches** |
| C4 | `docs/analysis/union-zone-planning/omega-work-packets.md:119-121` (Departure, state, arrival point); `:124` (Omega's first event); `omega-packets/UO-00.md:29` | Omega expects 1655 active on S4940 with O5645 and O5646 open; Omega publishes the arrival pin | **Matches** for the step state and the departure rows. **Mismatch on the pin** (below) |

### C4 mismatch (finding)

- SGC K5 (this file, row C4): candidate gate 5 pin **(34.705, -50.634, -149.092)**.
- Omega `omega-work-packets.md:121`: arrival point "published by UO-00 from UO-01's probe (first choice **(34.70, -51.86, -142.0)**, on the gate plaza in front of gate 5)".
- Omega `omega-packets/UO-00.md:29`: "It is the first-choice candidate B (34.70, -51.86, -142.0) only if UO-01 says so; copy the worknote, never this line."

Neither value was a published pin at the time. The two differed by about 1.2 m in y and about 7 m in z.

- **Resolved (2026-10-19):** Omega published x 34.70, y -51.28, z -142.00, heading 3.1409 (UO-00, sealed by UO-01); row C4 now carries it. Action::CrossWorldTeleport has no heading field, so only x, y, z reach the engine. The text below is the original finding, kept as history.
- **Resolution (proposed, superseded):** the coordinator aligns the two. Omega publishes the C4 pin from its UO-01 worknote; SGC then replaces the candidate in row C4 with that value. Until then US-32 stays BlockedDependency on the published pin, and no coder hard-codes either coordinate.
- **Owner:** coordinator (both plans are the coordinator's to align).

### Other agreements checked

- D-US11 (Humans hold 6786, Free Jaffa nothing) and Omega D-UO11 are the same decision: both **BlockedDecision**, owner plus SGC and Omega. Omega works either way; UO-07 adds one optional `remove_item 6786` if SGC grants it.
- F13 and the step rule: both plans use `complete_objective 1654 '5961'` only where 4939 has O5961 and O5962 both required (see README § Engine facts).
- The Dakara row C1 "Union writes no chain for it" applies to C2 (DK-30 owns the 4930 departure, D-UD2). No SGC chain is written for C2.
