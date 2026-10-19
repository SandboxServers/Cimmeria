# player-capture-bots: architect report

> **Superseded 2026-10-18** by [architect-revision-report.md](architect-revision-report.md), after the owner's decisions. The packet ids and the wave plan below are out of date; use README.md and the work-packets files.

> Type: design report. Written 2026-10-18 by the architect. Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md). Ideas: [IDEAS.md](IDEAS.md). Resume card: [RESUME.md](RESUME.md).

## Goal

Capture server-side telemetry whenever a real (non-lab) player successfully interacts with an NPC, item, terminal or object. The row records the player's position and heading, and the camera where the camera can be reached. It feeds lab UAT, so the lab can fix camera placement, body placement and facing. The design is server-authoritative first, uses no names or IPs, and is debuggable from SigNoz alone. Beyond that, rank further ideas for using human play, up to real-input playerbots.

## What the client actually reports

- **Position and three angle bytes only.** `avatarUpdateExplicit` (40 bytes, about 10 Hz) carries position, velocity, packed yaw, pitch and roll, flags and cells. Since P49 the server unpacks the angles into `CellEntity.direction = [pitch, yaw, roll]` radians.
- **No camera data on any method.** `interact`, `useAbility` and `setTargetID` carry ids only. `useAbilityOnGroundTarget` carries a ground point.
- **The camera is client memory only.** The lab supervisor reads it over the lab bridge (`crates/lab/src/supervisor/world/memory.rs`). The player DLL reads none of it. A player camera channel therefore needs a DLL change, which is decision **D-PC3**.
- **Open RE question (PC-06, Ghidra only).** Is the wire yaw/pitch the pawn rotation or the controller (view) rotation? If it is view rotation, the server already has the aim direction for free, and the DLL only needs zoom.

## Design (core campaign)

**The row.** One INFO row, target `interaction.pose`, written on every accepted interact (after the range gate, with the route recorded as trainer, chain_tag, chain_template or fallback) and on every item use aimed at another live entity. Each row carries:

- the player pose, the target pose and the derived geometry in the lab's own vocabulary: `radius_m`, `bearing_deg`, `dy_m` (D-SV3) and `facing_error_deg`;
- the `cimmeria.session_kind` tag;
- ids only for the player (D-PC2);
- a 250 ms per-player throttle.

**Where the code goes.** A new module, `crates/cell-world/src/cell/interaction_pose/`. Two call sites: `interact.rs` in `cimmeria-cell-methods`, and `inventory_events.rs` in `cimmeria-cell`. The new INFO target exports under the existing `OTEL_FILTER` leading `info` without edits.

**Size.** Six packets in two waves, plus a live check and a close-out:

| Wave | Packets |
|---|---|
| 1 | PC-01 geometry; PC-02 capture and emit (stacked on PC-01, shipped as one PR); PC-06 RE |
| 2 | PC-03 interact site; PC-04 item site; PC-05 docs |
| Live check | PC-07: SigNoz query after a colo release, user-gated |
| Close-out | PC-09 |

**Wave 3, PC-08a..c.** Opt-in DLL camera capture, sketched only, blocked on D-PC3.

## Decisions for the owner

| ID | Status | Question |
|---|---|---|
| D-PC3 | **BlockedDecision** | Ship opt-in camera capture in the player telemetry DLL? It covers zoom, the yaw and pitch offsets, and the pawn location at each `interact` or `useItem`, joined to the server row. It needs privacy and consent wording, a client-side change and a live lab UAT. Recommend deciding after PC-06: if the wire carries view rotation, the scope shrinks to zoom. |
| D-PC8 | **BlockedDecision** (soft) | Which colo release or playtest serves as the live proof (PC-07)? Rows only exist after real players play on a build with wave 2. |
| D-PC1 | Accepted, owner may veto | Lab sessions also write `interaction.pose`, tagged `session_kind = lab`, as the comparison set. |
| D-PC2 | Accepted (from the brief), owner may tighten | No names or IPs on this row. `account_id`, `player_id` and `entity_id` stay as ids, because instrumentation rule 5 requires them. Note: SigNoz rows elsewhere (for example `session.start`) carry names for the same ids under D-NT2, so this row is pseudonymous, not anonymous. Hashing the ids would need a separate decision. |
| Ideas | Owner approval | IDEAS.md ranks ten ideas. Suggested now: idea 2a (`session_kind` on `movement.position_sample`, S) and idea 3 (human friction into lab regression rows, S). Ideas 1 and 6 should follow PC-07. Playerbot stages 2 to 4 need their own architect pass. |

## Adjacent fixes folded in (PC-05)

- `.claude/agent-memory/rust-gameserver-dev/cell-entity-direction-semantics.md` still says inbound direction is never unpacked and that teleports zero facing. Both have been fixed in code: P49 unpacks the angles, and teleports use `update_position_preserving_facing`.
- `docs/engine/navmesh-build-pipeline.md:608-609` says `movement.position_sample` was not exported. `OTEL_FILTER` now exports it at DEBUG.

## Risks and file-cap notes for the orchestrator

- `interaction/mod.rs` (871 lines), `base_messages/mod.rs` (728), `automated-uat.md` (868) and `unified-uat.md` (1,395) are over the hard cap. The packets avoid them, using `#[path]` sibling tests and `tests/mod.rs`. `instrumentation-discipline.md` is at 697 lines, and PC-05 adds no lines to it.
- The throttle is process-global, so tests use distinct entity ids.
- The unpaired-id scan (`cimmeria-server unpaired_id`) runs in PC-02, PC-03 and PC-04. Each new id field is paired with a name or marked `nt:id-only`.
- PC-02 is stacked: it starts from PC-01's branch with a fast-forward merge, and the two ship as one PR, so no restack is needed.
- The orchestrator asks the user before any lab use, colo deploy or release (PC-07, PC-08b).

## Files written

- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\player-capture-bots\README.md`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\player-capture-bots\work-packets.md`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\player-capture-bots\IDEAS.md`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\player-capture-bots\RESUME.md`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\player-capture-bots\architect-report.md`
