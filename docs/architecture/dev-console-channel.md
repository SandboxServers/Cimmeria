---
title: "Dev `.`-Console Channel"
type: explanation
audience: engineers, GMs
last_updated: 2026-06-18
---

# Dev `.`-Console Channel (ADR)

> **Status**: Adopted in issue #523. Implemented in
> `crates/services/src/cell/console/` + `crates/services/src/base/console_authoring.rs`.
> **Confidence**: High for the channel/dispatch/auth and the read-only +
> authoring families; medium for the seed-commit Discord hook (designed, not yet
> wired) and the server/maintenance family (intentionally divergent — see below).

## Context

We have native `/gm*` slash commands for the ~62 dev commands that map to a
real cell-method index baked into `SGW.exe` (shipped in #518/#521). The 2009
client's slash roster (266 `Event_SlashCmd` classes) is **fixed** — we cannot
add new native slash commands. The remaining ~66 dev/authoring commands the
legacy Python server (`deprecated/python/cell/ConsoleCommands.py`) and the
[doko972/FanMMORPG](https://github.com/doko972/FanMMORPG) fork shipped have no
native binding and never can.

Both prior servers delivered those through a separate **`.`-prefixed console**:
the client does **not** intercept `.`-prefixed input (unlike `/`, which it
consumes locally) — it forwards it as an ordinary `CHAN_SAY` chat message. The
server can intercept it before broadcast. This is the analogue of the
`gm/feedback.rs` channel that unblocked the native query cluster (see
[gm-cell-method-adapt-plan](gm-cell-method-adapt-plan.md)).

## Decision

### 1. Channel + authorization

`cell::chat::handle_chat_message` intercepts, in its `CHAN_SAY` arm, any text
starting with `.` **when the sender's `CellEntity::access_level >= GameMaster`**.
A GM's `.`-line is routed to the console dispatcher and **not** broadcast (it
never appears in other players' chat); a non-GM's `.`-text falls through to
normal chat. Authorization is on the server-side `access_level` (sourced from
`account.accesslevel` at login, never a client byte) — the same trust model as
`cell::dispatch::gm_gate`. Every accepted command is logged at `info` for the
CAT-N audit trail (#473).

### 2. Registry-driven dispatch

`cell::console::COMMANDS` is the registry: `name → (min/max arg count, required
target type, summary)`, mirroring the legacy `Command` table. The dispatcher
parses `.<cmd> <args…>`, validates arg count and target type (the target is the
caller's currently-selected entity via `setTargetID`/`gmSetTarget`), and routes
to a family handler. All output returns to the GM only via
`console::send_gm_feedback` (`onPlayerCommunication` on `CHAN_FEEDBACK`) — the
same single-recipient channel the native `gm*` cluster uses. A coverage test
(`tests::every_spec_is_dispatched`) pins that no registered command falls
through to an unimplemented arm.

### 3. Authoring persistence: record → confirm → seed (NOT migrations)

Commands that change persistent data (`savespawn`, `delspawn`, the `path_*`
family) must not silently violate the repo's "seeds are the source of truth"
model (the DB is rebuilt from `db/resources/`; live writes are lost on rebuild;
**never** `db/scripts/*.sql` migrations). Each such command:

1. **Applies in memory** for immediate iteration (a freshly-assigned patrol
   starts walking now).
2. **Writes the live DB** via `CellToBaseMsg::ExecuteAuthoringSql` → a base
   handler runs the statement and reports rows-affected to the GM. The cell has
   no DB pool, so this crosses to base. The write is **transient** — it lets the
   developer see the change hold across reconnects within the deploy, and the
   next deploy rebuilds from seeds and wipes it.
3. **Records the canonical seed SQL** for a human to commit. Each statement is
   appended to a **per-session on-disk log** (`logs/seed-authoring-<session>.sql`,
   dir overridable via `CIMMERIA_AUTHORING_LOG_DIR`) as it happens, and buffered
   per-GM. `.seedconfirm` groups the buffer **per seed file** and emits each
   block; `.seedpending` lists; `.seedcancel` discards.

**The raw SQL is never shown in-game** — the client's chat isn't
copy-pasteable, so in-game feedback is status-only ("recorded — N pending",
"confirmed: M statements across K files"). The SQL goes out-of-band: the
per-session log and the server tracing log today.

**Trust model for the live write:** the channel is GM-gated, and the SQL is
*server-generated* — numeric values are formatted from cell-parsed `i32`/`f32`
and strings are escaped through `console::seed::sql_str`, so no raw client text
is concatenated. `world_id` is resolved at execution time via a
`SELECT … FROM resources.worlds WHERE world = '<name>'` subquery so the same
statement is valid in the seed file and live. This mirrors the legacy
`Atrea.dbQuery` authoring path.

### 4. Discord hook (designed, not wired)

All SQL emission funnels through one choke point (`console::seed`). The per-file
`.seedconfirm` payload is exactly what a `cimmeria-discord` `EventKind` (e.g.
`SeedAuthored`) would post to an authoring channel once the colo Discord
integration is enabled — a sink swap inside `seed::confirm`, not a redesign.
Discord is intentionally **not** wired yet (off in the colo; adding an event
type is its own checklist in [discord-notifications](discord-notifications.md)).

### 5. Patrol authoring (FanMMORPG `path_*`)

The patrol **runtime** already exists (`CellEntity::patrol_path`,
`AiState::Patrol`, `cell::service::npc_ai::npc_ai_patrol`). The `path_*` commands
are the authoring front-end. A path id == a `point_sets.set_id`; waypoints are
`point_set_points` rows ordered by `point_id`. `.path_assign` applies the
session waypoints to the targeted NPC's `patrol_path` immediately and records a
per-spawn override into `spawnlist.patrol_path_id` / `patrol_point_delay` (new
columns this issue), which the spawn loader now prefers over the template
default (`COALESCE(s.patrol_path_id, t.patrol_path_id)`). Per-waypoint edits
(`path_set_seq`, `path_set_tp`, …) address "the Nth waypoint" via
`ORDER BY point_id OFFSET n` and write `point_set_points.sequence_id` /
`teleport_*` (also new columns this issue).

#### Schema added (seed-edits, not migrations)

- `resources.spawnlist`: `patrol_path_id integer`, `patrol_point_delay real`.
- `resources.point_set_points`: `sequence_id integer`, `teleport_x/y/z real`,
  `teleport_sequence_id integer`, `teleport_delay real`.

## Per-command status

| Family | Status |
|---|---|
| Search (`searchitem`/`mission`/`template`) | **Done** — search runs base-side (`CellToBaseMsg::ConsoleSearch`, parameterized `ILIKE ... ESCAPE '\'` with `%`/`_`/`\` in the query escaped to match literally, plus explicit truncation feedback at the 25-result cap). |
| Roster (`players`, `listabilities`) | **Done** — `players` lists every online player across every loaded space on this CellApp (not cell-local); no "in transition" (connected-but-unplaced) tracking exists cell-side. `listabilities` resolves a player's known ability ids to names via the startup-loaded ability catalog. |
| Grants (`givecash`, `givexp`) | **Done** — both route through the shared `GrantCash`/`GrantXP` base sinks with `gm_feedback_to: Option<u32>` (P05) separating the DB/UI recipient (the selected target) from the GM feedback recipient (the caller); rejected requests still get immediate cell-side feedback, while successful grants have no optimistic cell-side line and wait for the base's post-commit confirmation instead. |
| Travel (`gotoxyz`) | **Done** — same-space authoritative teleport of the selected-or-caller entity, reusing the native `gmGotoXYZ`/`gmSummon` mechanism (`update_entity_position` + `note_authorized_teleport`, then `TeleportPlayer` gated on `is_player`); an NPC target has no client to snap but is still seen at its new position by other players through the normal AoI witness broadcast, since `update_entity_position` updates the spatial grid directly. No shared-message-struct change was needed — `TeleportPlayer` already carries no GM-feedback field, so `gotoxyz` just addresses its own immediate feedback line to the caller. |
| Placement (`location`, `rotation`) | **Done** — dual-mode read/set of the selected spawnable's position and orientation: no args reports, a complete three-tuple sets, and a partial (1- or 2-arg) tuple is rejected with no mutation (legacy gated its write on `z is not None` and silently reported instead — corrected per D02). `location` reuses `.gotoxyz`'s snap abstraction (`update_entity_position` + `note_authorized_teleport`, then `TeleportPlayer` for a player target) but restores `direction` afterwards, because `update_entity_position` overwrites facing from its `[i8; 3]` parameter. `rotation` writes `direction` as `[pitch, yaw, roll]` radians directly — `direction.y` is yaw, matching `pack_angle(direction[1])` and legacy's persisted `heading`. `BASEMSG_FORCED_POSITION` carries no orientation field, so `rotation` on a player target is server-side and witness-visible only, with no camera snap; witnesses see both commands' effects on the next AoI tick via `EntityMoved`. |
| Stat dumps (`stats`, `primarystats` … `stealthstats`) | **Done** — read `CellEntity::stats`. |
| Stat setters (`speed`) | **Done** — `.speed <0-500>` sets the selected Being's current `movementSpeedMod` *and* `rotationSpeedMod` together (100 = normal), current only (never `max`, matching legacy `setSpeed`), then publishes one `onStatUpdate` — `serialize_dirty` direct to a player target's client, `serialize_dirty_public` fanned out to an NPC target's AoI witnesses (both ids are in `PUBLIC_STATS`). Deviates from legacy in rejecting out-of-range values rather than silently clamping them, and validates both stats before writing either so the two-stat write is atomic. GM feedback goes to the caller (D03). `movementSpeedMod` now also scales the server-side NPC movement tick (`npc_movement_tick` steps by `move_speed × cur/100`, per `entities/defs/alias.xml`), so the stat has a real authoritative effect rather than a client-only one; `rotationSpeedMod` remains client-applied only — the NPC tick snaps yaw instantly and has no turn-rate integration. Not persisted: a speed change does not survive a relog. |
| Entity inspection (`info`, `facing`, `combatinfo`) | **Done** (`info`/`facing`) / **Partial** (`combatinfo`) — `info`/`facing` are read-only queries against `CellEntity` fields and legacy `SGWSpawnableEntity` geometry; `combatinfo` checks template/ability-set presence but omits legacy's weapon-presence and ability-type-bucket checks (no per-template weapon or per-ability-type concept exists yet — see the P02 handoff). |
| Entity authoring (`tag`, `name`, …) | **In-memory** — mutate `CellEntity`; appearance edits re-broadcast for players, surface on next AoI entry for NPCs. Pair with `.savespawn` to persist. |
| Net/AI debug (`net_seq`, `net_speak`, `threaten`, …) | **Done** — serialize the existing client method (`onSequence`/`onTimerUpdate`/`onMapInfo`/`onClientChallenge`) or poke the threat/follow/dialog systems. `debug_controller` is a no-op (no Rust debug controller). |
| Crafting (`learndiscipline`, `forgetdiscipline`) | **Done** via `GrantExpertise`. `allcraft` is a pointer (no consolidated blueprint-grant path cell-side). |
| Mission gaps (`missionfail`) | **Done**. `missionrewards` is a **preview** (reward dispatch tracked in #310). |
| Spawn lifecycle (`spawn`, `despawn`) | **Done** — `.spawn <templateId>` places one entity at the caller's exact position and facing via the existing `GmSpawnNpc` → `GmSpawnNpcReady` round-trip (the round-trip's message gained a `heading` field; the native `gmSpawnByCmd` keeps sending `0.0` since its wire signature carries no rotation). Feedback is deliberately deferred to the real creation result — the enqueue is silent, the base reports an unknown template, and the cell reports the new NPC id only once the spawn actually took; legacy's pre-creation "Spawning entity of type…" line is not ported. `.despawn` destroys the selected NPC through `SpaceManager::despawn_npc`, which fans `LeftAoI` out to every observing player immediately and scrubs the witness sets (rather than waiting for the next AoI tick to notice), and reports the real notified-observer count. It is NPC-only twice over: the spec is `Target::Mob` and the primitive independently refuses a player — legacy registered `.despawn` as `SGWSpawnableEntity`, which `SGWPlayer` derives from, so the legacy command could destroy a logged-in player. Neither command touches `resources.spawnlist`; that is `.savespawn` / `.delspawn`. |
| Spawn authoring (`savespawn`/`delspawn`/`spawnrandom`/`respawnall`) | **Done** — record→confirm + live write; `spawnrandom` reuses the `GmSpawnNpc` round-trip; `respawnall` is a runtime reset. |
| Patrol authoring (`path_*`) | **Done** — see above. |
| Server/maint (`save`/`reloadmap`/`reloadres`/`removerespawner`/`loglevel`/`logclient`) | **Divergent** — the Rust server handles these differently (incremental persistence, startup resource loading, env/`RUST_LOG` log level). Each reports the real mechanism rather than faking a no-op. Runtime log-level reload + resource hot-reload are future work. |

## Alternatives considered

- **Live DB write only** (legacy behavior): rejected — lost on rebuild, invisible
  to review.
- **Seed emit only** (no live write): rejected by the developer — they want to
  *see* the change hold within a deploy.
- **Typed per-operation cell→base messages** instead of `ExecuteAuthoringSql`:
  rejected for this pass — ~10 variants + handlers for a GM-gated, server-
  generated SQL string is more surface than the single executor warrants. Can be
  tightened later if the SQL surface grows.
- **Admin panel for DB-mutating commands**: still the right home for destructive
  bulk ops; the in-game console wins for position-derived authoring at your
  avatar. The two can coexist (the issue's open question).

## Consequences / follow-ups

- New cell→base messages: `ExecuteAuthoringSql`, `ConsoleSearch`.
- New `SpaceManager` fields: `authoring_changes`, `autosave_spawns`,
  `patrol_authoring` (all ephemeral, server-side).
- Follow-ups: wire the Discord `SeedAuthored` sink; runtime log-level reload
  (`loglevel`) + resource hot-reload (`reloadres`); a consolidated `allcraft`
  base grant; mission reward dispatch (#310, unblocks `missionrewards`).
