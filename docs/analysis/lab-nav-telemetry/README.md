# Lab navigation telemetry (LN)

Source: the owner's 2026-10-22 Cellblock run on the colo (latest launcher and telemetry DLL), mined from SigNoz, plus static RE of the client's mouse and movement input ([controller-mouse-input-model.md](../../reverse-engineering/findings/controller-mouse-input-model.md)).

**Goal.** Make a human play session replayable as lab navigation knowledge. From SigNoz alone we should be able to see every mouse press, drag, orbit, turn, mouse-walk, click classification (select or interact) and the camera and pawn pose around it. We should also fix the run's server-side findings that get in the way of driving the lab.

**Privacy rule (all packets).** No key codes, characters or chat text, ever. No cursor coordinates. Mouse data is a per-transition or per-250 ms change row, never per frame. No account names or addresses in docs.

Packets and contracts: [work-packets.md](work-packets.md); wave 1b (owner decisions D-LN2, D-LN3): [work-packets-wave1b.md](work-packets-wave1b.md). Resume card: [RESUME.md](RESUME.md).

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | Debug Hub dialog NPCs: repeat RMB clicks re-send `onDialogDisplay` (ms_since_previous 754-2,586), but the owner saw no dialog. The base refuses the cooked-data miss for keys 60100/60104 (`cooked_data.miss_refused`, `unknown_key`, category 5 Dialogs). Both keys are seeded in `db/resources/Dialogs/Seed/`, so the base's cooked cache, not the seed, lacks them. **Cause (architect, 2026-10-22):** both sit in `QUARANTINED_DIALOG_OVERRIDES` (`crates/resources/src/base/dialog_overrides/mod.rs`, with 60101 and the Banker's Expand 60110), held back since the 2026-09-27 Castle_CellBlock map-load crash (#943), so they are never served. | `dialog.display` rows, `cooked_sync` rows, `dialog_overrides/mod.rs` L279 | LN-01, LN-20a..e (D-LN3) |
| F2 | Camera pose is sampled only at interact time (`client.camera.at_interact`), so orbit and turn between interacts are invisible. | client service rows | LN-07 |
| F3 | The pawn pose next to the camera pose is absent or zero in some rows. `read_pawn_pose` returns `None` on any null link of the EntityManager chain, and the camera actor's own location is zero on this build. | client rows, `hooks/camera/read.rs` | LN-07 (`pawn_ok`), LN-13 |
| F4 | The interact heading is about 3 s old: `avatarUpdateExplicit` only goes out about every 3 s while standing still, so the server's `dir` lags the client's turn. The cell keeps the last update time per entity in `MovementValidator.move_clock` (private). | `wire.txt` (dir -61/-58/-37/-93 between interacts) | LN-02 |
| F5 | Hostile NPCs leave no pose row: an RMB on a hostile is rerouted to an attack before the pose capture, and the interact context names no hostile nearby. | server rows, `interact.rs` :79-92 | LN-03 |
| F6 | Launcher log targets. Research: the server's `OTEL_FILTER` (`launcher=debug`, `launcher.key_dump=off`) and `CLIENT_TARGETS` already cover every `launcher.*` target the server emits, and the parity tests pin them. The gaps are the target catalog (no rows for `launcher.client_log`, `debug_log`, `session_meta`, `key_dump`, `bundle`) and the launcher's own `launcher.update` rows, which reach only its local file. | `filters.rs` L415-416, `otel.rs` L174-180, `shell/src/telemetry.rs` L84-129 | LN-04, LN-18 |
| F7 | `client.dispatch.method_dropped` WARN fires on the normal path (false positive). Only `onPlayerTeleport` is in `KNOWN_UNBOUND_CLIENT_METHODS`; which methods the run dropped is not yet extracted. | server WARN rows, `replay_native.rs` L87 | LN-19 |
| F8 | Black Market: items show on the first open only. On reopen after the My Auctions tab the client sends no `BMSearch`. The server answers every search correctly. **Root cause (LN-12, static):** our overlay's `BlackMarketMod.now()` returns `worldGetDeltaSeconds()`, a per-frame delta, so the 4 s search delay never ends: every reopen skips its search, Search/Next/Prev stay disabled and the 15 s pending timeout never fires. The test stub returned a clock, which hid it. | wire.in, `BlackMarket.lua` L125-131, L511 | LN-12 (Done), LN-12b (D-LN2) |
| F9 | Mouse input: only the throttled `MouseClick`/`MouseLook` CME names were captured, with no state, deltas, drag, click class or pose. | client rows | LN-06..LN-10 |
| F10 | Lab memory contradicts itself on the camera: one entry says the camera pose reads all zero, another that the Actors scan finds a valid camera. Both are true: the camera *actor location* is zero on this build, the camera *state* (yaw, pitch, zoom) through the Actors scan is valid, and the old WorldInfo `+0x35c` chain is 0 in a normal game (`read.rs` L105-108). | agent memory | LN-11 |
| F11 | Interact distance ran 1.25-3.84 m; lab sheets disagree (lab-uat skill 3-4 m, driving recipes 3.5-4.5 m, unified-uat 4 to 6 m). The server range is 5.0 m in 3D. | `org_vault_open_requested` distance and others | LN-11, LN-14 |
| F12 | Minor: the base's org-vault `not_in_org` reject is correct but logs WARN on a normal player action. | bank rows, `org_vault/open.rs` L319 | LN-05 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LN1 | Mouse input telemetry in the **player** DLL. Options: **A** poll-only (recommended): read the controller's button, drag, mode and hover fields from the existing `FEngineLoop::Tick` detour; no new hook sites, no fingerprint additions, no DirectInput hook. **B** A plus a read-only `GetDeviceData` post-call hook in the player build, for raw mouse counts and wheel. **C** lab-bridge only: the owner's own sessions give no data. | Decided (owner, 2026-10-22): **yes**, variant **A** | The owner said yes to input hooks in the player DLL. The design needs only A (memory polls from the existing tick detour), and the answer did not choose B, so LN-08..LN-10 are unblocked on the decision (they still wait for LN-06/LN-07) and LN-17 stays BlockedDecision (B only). If the owner meant B, one follow-up question unblocks LN-17. |
| D-LN2 | BM reopen (F8). Options: **a** server push on open; **b** fix our overlay Lua; **c** document it as a client bug. | Decided (owner, 2026-10-22): **b** | LN-12 found the root cause in our own overlay clock (F8), so (b) is a few lines of Lua covered by the BM client-patch decisions D1-D8; it also fixes the dead Search/Next/Prev buttons. Packet LN-12b. The server push (LN-12a) is dropped. The owner **waived the lab-load-before-publish rule** for this change: no lab step; re-signing the overlay manifest, publishing it and validating after release are owner steps. |
| D-LN3 | Debug Hub dialogs 60100/60104 (F1). Options: **a** keep refusing and lab-repro first; **b** serve them; **c** take the dialog route off those NPCs. | Decided (owner, 2026-10-22): **b, by lifting the quarantine** | The refusals are the #943 quarantine, not a cache gap. All four held dialogs (60100, 60101, 60104 and the Banker's Expand 60110) move into `DIALOG_OVERRIDES`; the existing runtime push at login pak sync delivers them (no pak rebuild, no client patch). `QUARANTINED_DIALOG_OVERRIDES`, `VAULT_EXPAND_DIALOG_SERVED` and the cell's suppressed-offer branch go. **Accepted risk:** the 2026-09-27 Castle_CellBlock entry crash (cause never named) may return; the owner validates after release, no lab step. Packets LN-20a..e. |
| D-LN4 | Rates: the pose is checked every 250 ms and emitted only when something changed; nothing while idle; no heartbeat. Button edges per transition; mode at most 8/s; hover at most 4/s. | Accepted (architect) | Enough to rebuild a drag from button rows (drag totals) plus pose rows; bounded at about 4 rows/s moving and 0 idle. |
| D-LN5 | Poll, don't hook: every controller-derived row comes from memory polls in the existing engine-tick detour, through the crate's `Mem` reads. | Accepted (architect) | Edges last 50+ ms and a tick is about 16-33 ms, so polling sees every press; the drag counters are reset only on press (RE § 2), so the release tick reads the final drag. No fingerprint change. LN-15 confirms statically, LN-13 live. |
| D-LN6 | `client.nav.*` and `client.input.*` are governor must-keep (`KeepReason::NavInput`); the rate cap is at the source (D-LN4). | Accepted (architect) | Budgeted classing (burst 20, then 2/s) would roll up a 4 Hz pose stream. Must-keep is never collapsed; the source caps keep it bounded. |
| D-LN7 | F6 resolution: catalog rows now (LN-04, no launcher files); exporting `launcher.update` from the launcher is LN-18, after launcher-uat-fixes closes. | Accepted (architect) | The server filter has no gap; the only code change is in launcher files. |
| D-LN8 | A `method_dropped` demotion is by resolved method name, with a dispatch-table citation per name (LN-19), never by index. | Accepted (architect) | A name list cannot swallow another class's method at the same index; BM `onBM*` drops may be F8 evidence and stay WARN unless proven unbound. |
| D-LN9 | The dialog-display/miss correlation lives on the base: `aoi_dispatch::entity_method_call` records the last `onDialogDisplay` per client address, and `serve_miss` reads it. | Accepted (architect) | Base-session does not depend on cell-world, where the friction watch lives. |
| D-LN10 | `org_vault_open_rejected` with `reason = not_in_org` logs at INFO; every other reason stays WARN. | Accepted (architect) | A player clicking a Banker of an org they are not in is a normal refusal with a chat line; the other reasons are faults. |
| D-LN11 | The quarantine lift ships as one integration PR (`lab-nav-telemetry/wave1b`: LN-20a, LN-20b, LN-20d code; LN-20c, LN-20e docs), and the quarantine list and the served flag are deleted, not left empty. | Accepted (architect) | A half-lift on `main` (dialogs served while the cell still suppresses the offer, or docs still saying "quarantined") would mislead the owner's post-release check. With no list left, a re-quarantine is a revert of the wave-1b commits plus a tester cache delete, written down in the crash finding (LN-20c). |
| D-LN12 | LN-12b also clears the search delay on every open, and ignores frame deltas of 1 s or more. | Accepted (architect) | The clock only runs while PreRender is subscribed, so a delay started before a close would otherwise outlive it; opens are server-initiated, so this adds no search spam. A long hitch frame must not end a 4 s delay in one step. |

## Packets

| Packet | Wave | Title | Status |
|---|---|---|---|
| LN-01 | 1 | Dialog cooked-miss: element name and display age on the refusal | Ready |
| LN-02 | 1 | Interact: heading age (ms since the last position update) | Ready |
| LN-03 | 1 | Interact: hostile-click pose row and nearby hostiles row | Ready (merge after LN-02) |
| LN-04 | 1 | Target catalog rows for the launcher replay targets (docs) | Ready |
| LN-05 | 1 | Org-vault `not_in_org` reject at INFO | Ready |
| LN-11 | 1 | Lab docs: camera-chain memory and interact distance gotcha; commits the campaign's untracked files | Ready (merge first in wave 1) |
| LN-12b | 1b | BM overlay clock: reopen searches again, Search re-enables | BlockedDependency (LN-11 merged) |
| LN-20a | 1b | Debug-hub dialogs 60100/60101/60104 served | Ready |
| LN-20b | 1b | Banker Expand dialog 60110 served; quarantine list and flag removed | BlockedDependency (LN-20a reviewed) |
| LN-20d | 1b | Comment sweep for the lifted quarantine | BlockedDependency (LN-20b reviewed) |
| LN-20c | 1b | Docs: debug hub and the crash finding | Ready |
| LN-20e | 1b | Docs: UAT guide, vault, catalog, commands | Ready |
| LN-06 | 2 | Client: controller reads (`find_player_controller`, `read_controller`) | BlockedDependency (wave 1 merged) |
| LN-07 | 2 | Client: nav tick, 4 Hz pose sampler, governor rows | BlockedDependency (LN-06) |
| LN-12 | 2 | RE: BM window reopen path | Done (static; report to `worknotes/ln-12.md`) |
| LN-12a | 2 | Server push of a default `BMSearch` on open | Dropped (D-LN2 = b) |
| LN-15 | 2 | RE: poll-model confirmations and the CEGUI mouse-owner field | BlockedDependency (LN-11 merged) |
| LN-19 | 2 | `method_dropped` known-unbound names from the run | BlockedDependency (triage step) |
| LN-08 | 3 | Client: button edges, drag totals and click class | BlockedDependency (LN-06, LN-07) |
| LN-09 | 3 | Client: mode-bit and hover transitions | BlockedDependency (LN-06, LN-07) |
| LN-10 | 3 | Client: `ui_owned` from the CEGUI mouse-owner field | BlockedDependency (LN-15, LN-07; architect touch-up) |
| LN-16 | 3 | Docs: mouse input rows and SigNoz queries | BlockedDependency (LN-08, LN-09) |
| LN-17 | 3 | Client: read-only GetDeviceData aggregate in the player build | BlockedDecision (D-LN1 = B only; not chosen) |
| LN-13 | 4 | Live confirmation of the input model and the new rows (lab; step 4 dropped) | BlockedDecision (ask the user before any lab use) |
| LN-18 | 4 | Launcher: forward `launcher.update` WARN/ERROR | BlockedDependency (launcher-uat-fixes closed) |
| LN-14 | 4 | Close-out | BlockedDependency |

## Tested vs not tested

- **Tested:** nothing in this campaign yet. The SigNoz findings come from one colo run.
- **Static only:** every address in the RE finding, and the poll model (D-LN5). LN-15 checks it statically, LN-13 live.
- **Static only (LN-12):** that `worldGetDeltaSeconds` is a frame delta (Ghidra chain plus every stock caller). The LN-12b fix is checked only by the overlay's Lua harness; the live check (a second `BMSearch` on reopen) waits for the owner to re-sign, publish and play.
- **Never tested, accepted risk (D-LN3):** whether serving the four dialogs again brings back the Castle_CellBlock entry crash. Its cause was never named; the owner validates after release, with no lab run.
- **Not checked:** which methods the run's `method_dropped` rows name (LN-19 triage), and which exact launcher rows F6 saw.
