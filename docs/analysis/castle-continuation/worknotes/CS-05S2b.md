# CS-05S2b worknote: return-hub barks re-seal

> Type: worknote. Packet CS-05S2b (R3 re-seal of the CS-05S barks leg). Branch worktree `castle-cs-05s2b`, base `224e7692e`.
> Written 2026-10-18 from repo seed files only. No lab, no live server, no seed or code edit. Provenance of the bark copy: CIMMERIA_AUTHORING.

## Verdict

**Barks: `SEALED` with two barks (B1 Ogilvie, B2 Castle Medic), both on `enter_region Castle.Infirmary`.** The brief allows 2-4. A third bark was not sealed: the only unused Castle regions (`CastleFrontCourtyard` and the others) have no existing world-8 spawn inside them (the courtyard box is x 469-510, z 632-674; Castle_Coppleman is at 352, 952 and Castle_SgtGerschon at 429, 996), so a speaker there would be a name with no body. Two true co-located barks beat four invented ones.

One disclosure the CS-05c coder and the end-UAT reader must keep: **there is no persisted once-per-player guard.** `content_triggers.once = true` is enforced per player per cell-entity life only (`executor/once_gate.rs`, #802; memory note `content-engine-once-semantics`). It re-arms on relog and on world change, and a once-chain whose conditions fail stays armed. So "once per player" here means once per Castle visit session. The mission-completion conditions only stop the bark firing before the player has earned it; they do not stop it replaying after a relog. That is accepted for an ambient chat line (it has no side effect and moves no state). A truly persisted guard would need a tutorial-shown style row, which is a new action and out of scope.

## No-collision proof (every ID, `git grep -n -w` on `224e7692e`)

Chosen: chains 10200, 10201; dialogs 10740, 10741; screens 10900, 10901. Inside the brief's blocks (chains 10200-10219, dialogs 10740-10749, screens 10900-10919).

Chain ids, over `db crates docs`:

```
10200: db/resources/Events/Seed/debug_area_ring_events.sql:81 (sequences.sequence_id), :84 (event_sets_sequences); Texts/Seed/texts.sql:462 (texts.moniker_id)
10201: crates/navmesh-extractor/tests/it/terrain_castle.rs:123,124 (Heights.Num); debug_area_ring_events.sql:88,91 (sequences); texts.sql:464 (moniker)
```

Dialog ids: `10740` -> `texts.sql:1662` (moniker only); `10741` -> `texts.sql:1664` (moniker only); `docs/analysis/cellblock-asset-audit/parser-coverage-followup.json:8984` holds a bare `-10741` (a negative offset in a JSON audit, not an id). No `dialogs.dialog_id` row.

Screen ids: `10900` -> `texts.sql:2019` (moniker only); `10901` -> `texts.sql:2021` (moniker only). No `dialog_screens.screen_id` row.

Every hit is in a different id namespace (texts monikers, kismet `sequences`, terrain height counts). None is a `content_chains.chain_id`, `dialogs.dialog_id` or `dialog_screens.screen_id`. The `docs` and `.claude/agent-memory` sweep (cellblock, harset, tollana refs included, plus `ring-transport-system.md:456-457`, which names sequence ids 10200/10201/10202) found nothing else. The Harset and Tollana blocks sit at 6500+ and 7000+; Cellblock's 1176-1195 are untouched.

Existing chain ids on `enter_region Castle.Infirmary` at `224e7692e` (the only sibling set for this key): 1367, 1368 (`castle_1520_chains.sql:4,9`) and 1369 (`castle_symbiote_chains.sql:5`). `player_loaded 'Castle'` chains 1381-1383 are a different event key and are not siblings. Chains 1384-1399 have no `enter_region` trigger in any Content seed.

## Speakers (spawnlist rows)

| Bark | Speaker | Spawn | Row |
|---|---|---|---|
| B1 | `Ogilvie` (speakers.sql:107, id 951; template 179 nameplate text 7342) | 197 | `Worlds/Seed/spawnlist.sql:616`, world 8, (362.0, 70.18, 886.0), tag `Castle_Ogilvie` |
| B2 | `Castle Medic` (template 177, name_id 6964) | 198 | `spawnlist.sql:617`, world 8, (352.0, 70.18, 885.0), tag `Castle_Pop_Infirmary_Medic1` |

Both rows are inside point set 2051 `Castle.Infirmary` (`spawnlist.sql:610-615`). `npc_bark.speaker` is a literal string, as in chains 1176-1178.

## B1 and B2 frozen design

| | B1 chain 10200 | B2 chain 10201 |
|---|---|---|
| Trigger | `enter_region` `Castle.Infirmary`, scope `player`, `once = true` | same |
| Gates | world 8; `mission_status 1520 completed`; `archetype lte 6` | world 8; `mission_status 1521 completed`; `mission_status 1520 not_active`; `archetype eq 8` |
| Screen | 10900 (dialog 10740) | 10901 (dialog 10741) |
| Text | "The Ambernol run bought these people time, and I will not forget who made it. Keep your head down out there." | "The tank held, and so did the people who depend on it. Rest while you can; the Castle needs every warrior standing." |

The text is two sentences each, lore-consistent (1520 is the Ambernol delivery to the Infirmary; 1521 is the Moh'katan symbiote tank, offered to archetype 8 only by chain 1369), uses no new opcode, and carries `speaker_id` 0 like the debug-hub and Cellblock bark screens.

## Pairwise disjointness on `enter_region Castle.Infirmary`

The engine fires every matching chain, with conditions frozen at resolve time and `priority` never excluding (memory note `multi-chain-dispatch-semantics`). So disjointness needs a contradictory condition pair.

| Pair | Contradiction |
|---|---|
| 10200 vs 1367 | `mission_status 1520 completed` vs `1520 not_active` |
| 10200 vs 1368 | `1520 completed` vs `1520 active` |
| 10200 vs 1369 | `archetype lte 6` vs `archetype eq 8` (and 1369 needs `1521 not_active`) |
| 10201 vs 1367 | `archetype eq 8` vs `archetype gte 1 AND lte 6` |
| 10201 vs 1368 | `1520 not_active` vs `1520 active` |
| 10201 vs 1369 | `1521 completed` vs `1521 not_active` |
| 10200 vs 10201 | `archetype lte 6` vs `archetype eq 8` |

Q1's 10000-10006 are not seeded at `224e7692e` (no `content_chains` row, searched above). Both barks are side-effect-free chat lines, so even a future overlap with a Q1 chain on this key corrupts no state; CS-05b must still keep its own chains off a 1520/1521 completed pair if it wants a clean single response. The 1367/1368/1369 offer and credit chains are unchanged and are not edited.

## Frozen SQL (1366 seed shape, one statement per line)

Dialog and screen rows (append to `Dialogs/Seed/dialogs.sql` and `Dialogs/Seed/dialog_screens.sql`, or to the new chain file in `\ir` order after both; see the allowlist note below):

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10740, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Castle hub - Ogilvie bark holder (never displayed)');
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10741, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Castle hub - Castle Medic bark holder (never displayed)');
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10740, 10900, 'The Ambernol run bought these people time, and I will not forget who made it. Keep your head down out there.', 0, 0);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10741, 10901, 'The tank held, and so did the people who depend on it. Rest while you can; the Castle needs every warrior standing.', 0, 0);
```

Chains:

```sql
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10200, 'Castle hub bark: Ogilvie after 1520 (CIMMERIA_AUTHORING)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10200, 'enter_region', 'Castle.Infirmary', 'player', true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10200, 'world', 8, NULL, 'eq', NULL, 0), (10200, 'mission_status', 1520, NULL, 'eq', 'completed', 1), (10200, 'archetype', NULL, NULL, 'lte', '6', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10200, 'npc_bark', NULL, NULL, '{"screen_id": 10900, "speaker": "Ogilvie", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10201, 'Castle hub bark: Castle Medic after 1521 (CIMMERIA_AUTHORING)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10201, 'enter_region', 'Castle.Infirmary', 'player', true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10201, 'world', 8, NULL, 'eq', NULL, 0), (10201, 'mission_status', 1521, NULL, 'eq', 'completed', 1), (10201, 'mission_status', 1520, NULL, 'eq', 'not_active', 2), (10201, 'archetype', NULL, NULL, 'eq', '8', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10201, 'npc_bark', NULL, NULL, '{"screen_id": 10901, "speaker": "Castle Medic", "channel": "say"}', 0, 0);
```

Allowlist note for the coordinator: the CS-05c allowlist (`r3-packets-2.md`, CS-05c) names `castle_hub_barks_chains.sql`, a `database.sql` `\ir` line, `chain_replay_tests/castle_hub_barks.rs` and a `mod` line. It does not name `Dialogs/Seed/dialogs.sql` or `dialog_screens.sql`. The four dialog and screen rows above live there, so CS-05c either gets those two files added to its allowlist (preferred; matches the debug-hub precedent) or ships them as the first statements of the new chain file after confirming that file loads after `Dialogs/Seed`. The `npc_bark` text lookup reads the `dialog_screens` startup cache, so a missing screen row means a silent no-op, not an error.

## CS-05c test list (`chain_replay_tests/castle_hub_barks.rs`, pattern `npc_bark.rs`)

1. B1 fires once for a player with 1520 completed and archetype 3 on `enter_region Castle.Infirmary`; the decoded `onPlayerCommunication` carries speaker `Ogilvie`, channel say, and the screen 10900 text. Fails if chain 10200 is removed.
2. B1 does not repeat on a second entry in the same session (the `once` gate). Fails if `once` is set false.
3. B1 does not fire with 1520 not completed (active, not_active), and not for archetype 8. Fails if the 1520 condition is removed.
4. B2 fires once with 1521 completed, 1520 not_active and archetype 8; speaker `Castle Medic`, screen 10901 text. Fails if chain 10201 is removed.
5. B2 does not fire for archetype 3, and not with 1521 active. Fails if the archetype or 1521 condition is removed.
6. Disjointness: with 1520 completed and archetype 3, exactly chain 10200 among {1367, 1368, 1369, 10200, 10201} produces a bark and no accept; with 1521 completed and archetype 8, exactly 10201 does.
7. Seed agreement: the two screens exist with the frozen text under their dialogs (`spawner::load_dialog_screen_text`).

## End-UAT rows

| Row | Steps | Expected | State |
|---|---|---|---|
| UAT-HB1 | Complete 1520 on a non-Jaffa character, leave and re-enter the Infirmary | One Ogilvie chat line (the B1 text) in the say channel, no dialog window; no second line on the next entry in the same session | UATPending |
| UAT-HB2 | Complete 1521 on a Jaffa character, re-enter the Infirmary | One Castle Medic chat line (the B2 text); no Ambernol offer fires | UATPending |

Nothing was run on a client, so no row is claimed as passed.

## Open items

1. Coordinator: widen the CS-05c allowlist to the two Dialogs seed files (above).
2. Accepted limitation: the once guard is session-scoped; a relog replays the bark.
3. A third bark needs a Castle speaker spawn inside an unused region; none exists today.
