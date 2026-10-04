# Named Telemetry Campaign

> Type: how-to (campaign launch and ledger). Audience: the coordinator session and packet workers.
> Updated: 2026-10-04. Companions: [work-packets.md](work-packets.md), [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [negative-logging-convention.md](../../architecture/negative-logging-convention.md), [discord-notifications.md](../../architecture/discord-notifications.md), [observability.md](../../architecture/observability.md).

## Goal

Every ID the server writes to a log line or posts to Discord appears with its name next to it, resolved by ordinary code when the line is written. A human reading SigNoz or Discord, or an agent reading either, never has to look an ID up by hand:

```text
before: ability_id=880 target=4123 space_id=12 reason=out_of_range
after:  ability_id=880 ability_name="Staff Blast" target=4123 target_name="Jaffa Guard"
        space_id=12 world_name="Castle_CellBlock" reason=out_of_range
```

Discord gets the same pairs, rendered as `Name (#id)`. The rest of the restoration team reads Discord, but only developers on the VPN can reach SigNoz, so **Discord messages never link to SigNoz** or to any other VPN-only host. Each message has to make sense on its own.

What the campaign covers:

- **Entities and content:** items, abilities, effects, missions, steps and objectives, dialogs, NPC templates, worlds and spaces, stargates, spawn sets, containers and loot lists, archetypes, ammo types, organizations, characters.
- **Wire and state codes:** Mercury message IDs and entity-method indexes get their method names. Bitflag fields get flag names (`BSF_InCombat|BSF_Dead`). Error codes get their `error_texts` moniker.
- **Discord:** both paths. Typed `emit_*` events, and the warn/error tracing layer.
- **Client telemetry:** addresses and entity IDs in replayed client rows, resolved at ingest on the server, so no client patch is needed.
- **Lab tools:** `server_entity_get`, `server_entity_query` and `server_witnesses` return names next to IDs, so agents spend fewer tokens on correlation.
- **Enforcement:** a CI scan that blocks a new unpaired ID field, with a baseline that only shrinks.

## Where things stand (baseline `a679e748c`, 2026-10-04)

Nothing has been built yet. What the planning pass found:

- **IDs outnumber names about 30 to 1.** In `tracing` fields across `crates/`: `account_id` 1,036, `entity_id` 929, `item_id` 438, `space_id` 203, `template_id` 169, `ability_id` 133, `effect_id` 94, `mission_id` 50, `dialog_id` 34. Name fields: `player_name` 48, `character_name` 40, `item_name` 7, `template_name` 6, `ability_name` 2. NT-03's scanner produces the real per-file baseline.
- **Rule 5 already stamps identity, but only as IDs** ([instrumentation-discipline.md § Rule 5](../../architecture/instrumentation-discipline.md#rule-5--every-log-describing-player-activity-carries-account_id--player_id)): `account_id` and `player_id` on every player log, from two resolvers (`SpaceManager::player_identity`, `base::session_identity::identity_for_entity`). This campaign adds a sibling rule for names and reuses its patterns: resolve late, snapshot before teardown, absent rather than `0`/`"None"`.
- **Names have to go on the event, not just the span.** The OTLP bridge copies only the event's own fields into the log record. Spans don't cross the base↔cell channel. The likely parent spans are debug-level. All of this is in Rule 5 § "These go on the log EVENT". The Discord layer works the same way: `DiscordLayer::on_event` reads only the event's fields (`crates/discord/src/layer/mod.rs`). So span-level names help the Traces view and nothing else.
- **The seed has every name we need:** `items.name`, `abilities.name`, `effects.name`, `missions.mission_label`, `dialogs.name`, `dialog_sets.name`, `speakers.name`, `entity_templates.name` / `template_name` / `name_id`, `monikers.name`, `texts.text`, `error_texts.moniker_name`, `stargates.name`, `respawners.name`, `spawn_sets.name`, `containers.name`, `item_lists.name`, `applied_science.name`.
- **The in-memory defs mostly drop them.** `SpaceManager` holds `mission_defs`, `ability_defs` and `item_defs`. `AbilityDef` has `name`, but `MissionDefEntry` and `WeaponDef` have none. `CellEntity` carries `name_id`, not a resolved name. Name helpers are scattered and ad hoc: `archetype_name`, `ammo_name`, `racial_paradigm_name`, `get_entity_world_name`, `online_name`.
- **Discord typed events are half-named.**
  - All five mission emits pass `mission_name: None`.
  - `emit_item_used` sends `item_type_id` with no name. `emit_dialog` sends `dialog_id` with no name. `emit_character_created` sends archetype as a bare integer.
  - Going the other way, player death, respawn and level-up send a character name but no `player_id`. NPC death sends a name but no `entity_id` or template. Killers are names only.
  - Account fields are already paired (`account_value` → `name (#id)`), and are the model for the rest.
- **The Discord tracing layer posts raw fields.** It keeps up to 24 of them (`MAX_FIELDS` 25, minus one for the target), so doubling the fields would cut off the tail. NT-11 folds each `x_id`/`x_name` pair into one embed field.
- **Discord has no SigNoz links today.** NT-11 adds a guard so none ever get in.

## Design summary

The full contract is NT-00 (Rule 6). All five decisions were settled on 2026-10-04. In short:

1. **Pair, don't replace.** The ID stays the join key. The name sits next to it under the same prefix: `ability_id` + `ability_name`, `target` + `target_name`, `space_id` + `world_name`. NT-00 publishes the canonical key table.
2. **One lookup path.** A `NameBook` is loaded at boot from `db/resources`, shared by base and cell, and refreshed on content reload (NT-01). Lookups return `Option<&str>`. tracing records an `Option` field only when it is `Some`, so an unresolved name is left out, never written as `"unknown"`.
3. **A missing name means bad data.** A log line with `template_id` but no `template_name` points at a seed hole. NT-50 ships a saved SigNoz query for this.
4. **Names are never metric labels.** Rule 4 stands. `world_name` stays the one approved label.
5. **Resolve late, and before teardown.** Same as Rule 5. The 10 Hz movement accept path pays nothing.
6. **Discord stands on its own.** The server is private and team-only, so it shows the same names as SigNoz, account login names included (D-NT2). Every object appears as `Name (#id)`. There are no links to SigNoz, the admin API or any VPN-only host, and an embed-wide test enforces it. Player IPs and whisper text stay hidden, as today.

## Decisions

| ID | Question | Recommendation | Status |
|---|---|---|---|
| D-NT1 | Where does `NameBook` live? | A new small crate, `cimmeria-names`, depending only on `cimmeria-entity` and sqlx. | **Decided 2026-10-04:** new crate. |
| D-NT2 | Discord shows the account **login name** (`steve (#6)`). Keep it? | The Discord server is private, team only. | **Decided 2026-10-04:** keep login names in Discord, paired with the account ID. |
| D-NT3 | Should Discord error embeds carry the `trace_id` as **plain text** (not a link), so a developer can paste it into SigNoz? | Yes, as a short footer line. | **Decided 2026-10-04:** yes, plain text, never a link. |
| D-NT4 | Should the NT-03 scan block CI or only warn? | Block. | **Decided 2026-10-04:** block new unpaired IDs; the per-file baseline may only shrink. |
| D-NT5 | For NPCs, log the player-facing name or the template name? | Both. | **Decided 2026-10-04:** both, on every NPC line: `entity_name` = the player-facing `name_id` text, plus the `template_id` + `template_name` pair. |

## Coordinator launch prompt

You coordinate this campaign. Work from [work-packets.md](work-packets.md) one packet at a time; each packet is one PR.

1. Record `git rev-parse HEAD` in the ledger below and compare it with the baseline. Re-check any cited path that changed under `crates/discord/`, `crates/server/src/logging/`, `crates/cell-world/src/cell/space_manager/` or `docs/architecture/instrumentation-discipline.md`.
2. Read [CLAUDE.md](../../../CLAUDE.md), [AGENTS.md](../../../AGENTS.md), [TESTING.md](../../../TESTING.md), [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md) and [discord-notifications.md](../../architecture/discord-notifications.md). Give each worker only the sections its packet cites.
3. Phase 0 runs in order: NT-00, then NT-01 and NT-02, then NT-03. Phase 1 (Discord) starts once NT-01 merges. The Phase 2 system sweeps are independent of each other and run in parallel, one worktree and one lane slot each.
4. Each worker ships with `bash tools/build-lane/ship.sh pr -C <worktree> -m <msg>`. Merge with `ship.sh merge <PR> --retire <worktree>` once the minimum CI passes.
5. Update the ledger in each packet's own PR. `docs/gap-analysis.md` and `docs/project-status.md` change only in NT-50.

## Ledger

| Packet | Status | PR | Notes |
|---|---|---|---|
| NT-00 Rule 6 and key table | Ready | | |
| NT-01 NameBook | Ready | | |
| NT-02 Name helpers on the existing resolvers | BlockedDependency (NT-01) | | |
| NT-03 Unpaired-ID scan and baseline | BlockedDependency (NT-00) | | |
| NT-10 Discord typed events | BlockedDependency (NT-01) | | |
| NT-11 Discord tracing layer: fold pairs, no internal links | BlockedDependency (NT-00) | | |
| NT-20 Sweep: combat and effects | BlockedDependency (NT-02) | | |
| NT-21 Sweep: missions, content, dialog | BlockedDependency (NT-02) | | |
| NT-22 Sweep: inventory, loot, vendor, crafting | BlockedDependency (NT-02) | | |
| NT-23 Sweep: world, movement, AoI, travel | BlockedDependency (NT-02) | | |
| NT-24 Sweep: session, auth, world entry | BlockedDependency (NT-02) | | |
| NT-25 Sweep: NPC AI, spawner, pets, cover | BlockedDependency (NT-02) | | |
| NT-26 Sweep: social, orgs, mail, trade, BM, duel | BlockedDependency (NT-02) | | |
| NT-27 Sweep: GM console and minigames | BlockedDependency (NT-02) | | |
| NT-30 Opcode and method names | Ready | | |
| NT-31 Flag, enum and error-code names | BlockedDependency (NT-01) | | |
| NT-40 Client telemetry resolved at ingest | BlockedDependency (NT-01, NT-30) | | |
| NT-41 Lab tools return names | BlockedDependency (NT-01) | | |
| NT-50 Close-out | BlockedDependency (all) | | |
