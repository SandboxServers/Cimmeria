---
name: seed-name-id-and-asset-naming
description: entity_templates.name_id is client-PAK-resolved (new moniker ids cannot render); texts.sql monikers name the UE3 asset family, which is how to find map assets an English-keyword scan misses
metadata:
  type: reference
---

Two coupled facts about `resources.texts` / `entity_templates.name_id`, both confirmed while
authoring Castle World 8 story actors (CA05).

## `name_id` is resolved by the client, not the server

`crates/services/src/mercury/aoi/create.rs:211-218` writes `name_id` raw onto the AoI create
packet, and **only** when it is `Some(n)` with `n != 0`. The client resolves the id against its
own PAK string table.

Consequences:

- **You cannot mint a new display name server-side.** Adding a row to
  `db/resources/Texts/Seed/texts.sql` with a fresh `moniker_id` ships an id the client cannot
  look up. Only ids that already shipped in the client render. Recover an existing one instead.
- **NULL / 0 `name_id` fails silently** — the property is omitted and the NPC appears unnamed.
  No error, no log. Worth a live-DB guard on any new named actor (see
  `castle_ca05_story_actors_all_have_a_client_resolvable_name_id` in
  `crates/cell-catalog/src/cell/spawner/tests/live_db_loaders.rs`).
- `resources.texts` IS a server-side seed table, which makes it tempting to treat as
  authoritative for display strings. It is only a *lookup* of what the client already has.

Recovering the right id: grep `texts.sql` for the display-name moniker prefixes —
`DN_Mb_` / `DN_MB_` / `DN_MsMb_` (mobs; `_Uni_N` = unique named, `_St_N-M` = standard tier band),
`DN_npc_` (friendly NPCs), `DN_Ob_` (world objects). The existing Castle templates already follow
it (48 → 7035 'Capt. Copplemann', 149 → 7034 'Sgt. Gerschon'). Note some monikers have an empty
`text` server-side while the client still has a string, so an empty `text` column is not proof
the id is unusable.

## A texts.sql moniker names the UE3 asset family — use it to search the maps

The high-value trick. Cooked `.umap` name tables use the art department's asset names, which are
often a *different word* from the English display string. Three independent keyword scans over
all 145 Castle tiles searched for `comm` / `terminal` / `workstation` and concluded the
Communications room did not exist. It does. The moniker
`DN_Ob_D_HumanViewScreen_Castle_CommTerminal` = 'Communications Terminal' names the family:
**ViewScreen**. Re-scanning for the screen/monitor families found `EM-ViewScreen00/02/03`,
`GP-Monitor_Wall00`, `CA-Monitor_Wall_Screen00*`, `EM-CmdPostScreens00`, `Em-Screen00`.

So: **before concluding a map asset is absent, grep `texts.sql` for the object's moniker and
re-scan for the asset family it names.** Same applies in reverse — a recovered mesh family often
maps to an already-seeded `entity_templates.static_mesh` string you can clone verbatim
(`EM-ViewScreen02` → template 19's `Em-Props.EM-ViewScreen02`).

Also: `mission_objectives.display_log_text` is under-used ORIGINAL_DATA. One objective row
("Warden Muelbach ... is holed up in the bunker above Checkpoint Bravo") supplied a character's
title, gender and location at once, and another's literal noun ("NID **Officers**") picked the
right `name_id` between two candidates. Read the objective text before guessing any of those.

## Scanning mechanics

- Use `tools/upk_parser.py`'s `PackageReader` + `read_export_properties` (it picks the right
  tagged-property header offset per class kind — see [[ue3-staticmesh-extraction]]). Bypass
  `extract_actors()`'s class whitelist so no actor type is silently excluded.
- **A plain binary `grep` over a `.umap` is not a substitute** — the packages are
  chunk-compressed, so grep sees only fragments (it returns `_Screen00_` where the name table
  holds `CA-Monitor_Wall_Screen00A_FX`). It will produce false negatives that look authoritative.
- Actor `Location` → server units: `server.x = rawY/100`, `server.y = rawZ/100`,
  `server.z = rawX/100`. Absolute world space; chunk-filename offsets do **not** apply. Verify
  against an already-seeded coordinate before trusting a whole table — and watch for x/z
  transcription slips when writing the results up, since the varying axis is `server.z` in some
  rows and `server.x` in others.
- `SGWSpecCoverNode` actors carry `Location` and are extractable this way — the cover system's
  world positions are recoverable from the cooked maps.

Related: [[ue3-staticmesh-extraction]], [[stat-with-no-consumer-trap]] (same shape: a seed column
that exists but nothing consumes / renders it).
