---
title: "Ambient chatter"
type: reference
audience: content authors, engineers
last_updated: 2026-10-05
---

# Ambient chatter

Ambient chatter is groups of NPCs talking among themselves in say chat on a
schedule. A player standing near them reads the conversation in the chat
window, each line prefixed with the speaking NPC's name. The first group is
the Debug Area's [System Lords' summit](debug-area.md#system-lords-summit)
(DA-09).

The 2009 client has no NPC-to-NPC chatter route, and the shipped dialog
screens are all addressed to the player, so the groups and their text are
Cimmeria seed data. A line reaches the client as
`onPlayerCommunication(speaker, SPEAKER_None, CHAN_say, text)` (client
method 28), the route player say chat and the `npc_bark` content action
already use ([content-engine-vocabulary.md](content-engine-vocabulary.md#npc_bark-params)).
No client patch is involved.

## How a group plays

- A group plays its **exchanges** (short scenes) in `exchange_id` order and
  starts again from the first after the last.
- An exchange starts only while a connected player is within `hear_radius`
  of one of the group's speakers. With nobody there the group checks again
  every 2 seconds and keeps the same exchange, so the next player who walks
  up hears a scene from its first line.
- Once an exchange starts, its lines are spoken in `line_index` order, each
  `delay_ms` after the one before it (the first line's delay counts from the
  start). A line goes to every connected player within `hear_radius` of the
  NPC speaking it at that moment. A player who walks away mid-scene stops
  hearing it, and the scene still finishes.
- After an exchange's last line the group is quiet for `exchange_gap_secs`.
- A line whose speaker is dead or missing is skipped, and the scene goes on.
  The first such skip per group and speaker logs a WARN.
- The schedule restarts with the server: every group begins at its first
  exchange.

## Seed tables

Both tables are in the `resources` schema, under `db/resources/Dialogs/`.

`ambient_chatter_groups`, one row per group:

| Column | Meaning |
|---|---|
| `group_id` | Primary key |
| `world_id` | `resources.worlds` row the speakers stand in. The world must have a shared space on the cell (instanced worlds are not supported) |
| `name` | For log lines |
| `hear_radius` | Metres from a speaking NPC within which a player hears it; 0 to 100, default 20 |
| `exchange_gap_secs` | Quiet time between exchanges; at least 5, default 30 |

`ambient_chatter_lines`, one row per line, primary key
`(group_id, exchange_id, line_index)`:

| Column | Meaning |
|---|---|
| `exchange_id` | The scene the line belongs to |
| `line_index` | Order within the scene, from 0 with no gaps |
| `speaker_tag` | `spawnlist.tag` of the NPC who speaks it, in the group's world. The chat window shows that NPC's name (its template's `name_id` text) |
| `delay_ms` | Pause before the line; 0 to 60000 |
| `text` | The line, shown as written; non-blank and at most 200 characters |

The loader is `load_ambient_chatter`
([`cell-catalog/src/cell/spawner/ambient_chatter.rs`](../../crates/cell-catalog/src/cell/spawner/ambient_chatter.rs)),
run once at cell startup, so a seed edit needs a restart. The tick is the
`cimmeria-cell-chatter` plugin ([`crates/cell-chatter/`](../../crates/cell-chatter/)),
registered in the facade's plugin table.

## Adding a group

1. Spawn the speakers in `spawnlist`, each with a unique tag in its world.
   Give their templates a `name_id` whose text is the name players should
   read, and make them stationary unless walking chatter is wanted.
2. Add the group row and its lines. Size each `delay_ms` to the time it takes
   to read the line before it. The summit uses 2.5 s plus 55 ms a character,
   clamped to 3-9 s, and 0 for an exchange's first line.
3. Keep the speakers within `hear_radius` of where a listener stands, and
   keep listeners out of hostile aggro range.

The live-DB guards in
[`live_db_ambient_chatter.rs`](../../crates/cell-catalog/src/cell/spawner/tests/live_db_ambient_chatter.rs)
check every group. Each exchange must be complete and start at once, and
every speaker tag must name exactly one spawn in the group's world with a
display name and a faction players cannot attack.

## Logs

Everything logs on target `chatter`:

| Event | Level | When |
|---|---|---|
| `chatter.ready` | INFO | First tick: how many groups, and how many have a loaded world |
| `chatter.exchange_started` | INFO | A scene starts: group, exchange (named by the start of its first line), listener count and names |
| `chatter.line` | DEBUG | Each line: speaker entity and name, listener count |
| `chatter.speaker_missing` | WARN | `reason = no_living_npc_with_tag`, once per group and tag |
| `chatter.speaker_unnamed` | WARN | `reason = speaker_has_no_name`, once per group and tag |
| `chatter.group_world_missing` | WARN | `reason = world_not_loaded`: the group stays silent for the process lifetime |
| `chatter.send_failed` | WARN | The cell-to-base channel is closed |
| `catalog_load_failed` | ERROR | The cell could not load the tables; no group speaks |

To find out whether a player heard the summit, filter SigNoz on
`event = 'chatter.exchange_started'` and read `listener_names`.
