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
| Search (`searchitem`/`mission`/`template`, `players`) | **Done** — search runs base-side (`CellToBaseMsg::ConsoleSearch`, parameterized `ILIKE`); `players` is cell-local. |
| Stat dumps (`primarystats` … `stealthstats`) | **Done** — read `CellEntity::stats`. |
| Entity authoring (`tag`, `name`, …) | **In-memory** — mutate `CellEntity`; appearance edits re-broadcast for players, surface on next AoI entry for NPCs. Pair with `.savespawn` to persist. |
| Net/AI debug (`net_seq`, `net_speak`, `threaten`, …) | **Done** — serialize the existing client method (`onSequence`/`onTimerUpdate`/`onMapInfo`/`onClientChallenge`) or poke the threat/follow/dialog systems. `debug_controller` is a no-op (no Rust debug controller). |
| Crafting (`learndiscipline`, `forgetdiscipline`) | **Done** via `GrantExpertise`. `allcraft` is a pointer (no consolidated blueprint-grant path cell-side). |
| Mission gaps (`missionfail`) | **Done**. `missionrewards` is a **preview** (reward dispatch tracked in #310). |
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
