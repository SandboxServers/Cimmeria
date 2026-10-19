# DK-31a: the Ha'tak sequences for mission 1652

> Type: desk note (evidence only, no code). Packet: DK-31a in [wave-6](../packets/wave-6.md). Feeds DK-31c (one `play_sequence` row per Ha'tak action). Read-only evidence: seed files, the Rust executor and wire path, the placement census. No Ghidra, no lab, no client.

## Short answers

1. `play_sequence`'s `target_id` is a **Kismet sequence id**: the `sequences.sequence_id` column. It is neither the `event_sets` row id nor the event id inside it.
2. Event set 1195 holds the **western** Ha'tak only if the census instance numbers follow the package suffix order. The repo does not settle it. Unknown, see section 2.
3. Unknown. The repo names the sequences "Unhide" and "Explosion" but records no initial hidden state for the meshes.
4. The rows DK-31c needs are listed in section 4. One of them, the accept-time unhide in chain 8322, is probably in the wrong world; see the caveat there.

## 1. What `play_sequence`'s target is on the wire

Chain side (content engine and executor):

- `crates/content-engine/src/loader/action.rs:77-79`: `"play_sequence" => Action::PlaySequence { sequence_id: row.target_id? }`. The chain row's `target_id` is the sequence id, stored as-is.
- `crates/cell-content/src/cell/content/executor/dispatch.rs:74-96`: `Action::PlaySequence { sequence_id }` writes `sequence_id.to_le_bytes()` as the first four bytes of the `ON_SEQUENCE` args (`KismetEventSetSeqID`, line 85). Nothing maps it through `event_sets`. The send at line 94-98 is an `EntityMethodCall` to `entity_id`, the acting player only.

Precedent (`db/resources/Content/Seed/sgc_w1_chains.sql`):

- Chain 3028, line 494: `(3028, 'play_sequence', 10009, NULL, '{}', 0, 3)`. The target column holds 10009.
- `db/resources/Events/Seed/sequences.sql:3561`: `(10009, 6000, 'SGC_W1-0000ffff.Main_Sequence.CartersLabDoors')`. So 10009 is the `sequences` row, with `event_id` 6000 ("Designer 0: Close Doors") and the Kismet script name. Line 3563 gives 10010 = event 6001 ("Designer 1: Open Doors"); chain 3030 plays 10010 (line 549).
- Trap: `db/resources/Events/Seed/event_sets.sql:1347` also has an `event_sets` row 10009 ("Beta Site E1 Stargate"). The number collides, but the executor never reads `event_sets` for `play_sequence`. The ring-transport path (`crates/cell-world/src/cell/ring_transport/wire_helpers.rs:41-62`) does look up `(event_set_id, event_id)` through `sequence_map`, but that is the ring route, not chain actions.

Conclusion: `play_sequence` takes `sequences.sequence_id`. For 1652 the rows must carry the sequence ids 2390-2393, not the set ids 1194/1195 and not the event ids 6000/6001.

## 2. Which set is western

Facts, each with its file:

- `db/resources/Events/Seed/event_sets.sql:1087` is set 1194; `:73` is set 1195. Both have empty names.
- `db/resources/Events/Seed/event_sets_sequences.sql:453` and `:455`: set 1194 holds sequences 2390 and 2391. Lines 457 and 459: set 1195 holds 2392 and 2393.
- `db/resources/Events/Seed/sequences.sql:1937`: 2390 = event 6000, `Dakara_E1.Main_Sequence.Prefabs.GA-Hatak00_Pf0_Seq`. Line 1939: 2391 = event 6001, same root. Line 1941: 2392 = event 6000, `..._Seq_0`. Line 1943: 2393 = event 6001, `..._Seq_0`.
- So set 1194 is the `GA-Hatak00_Pf0_Seq` root and set 1195 the `_Seq_0` root (this confirms audit finding 4, which says the seed binds `_Seq_0` to 1195).
- Positions: `docs/analysis/dakara-e1-rebuild/placements/data/Dakara_E1_landmarks.tsv:70` is a `GA-Props.GA-Hatak00_Pf0` at (-111.710, 353.916, 78.091), the western one. Line 71 is the same prefab at (367.160, 359.597, 136.130), the eastern one.
- `docs/analysis/dakara-e1-rebuild/placements/C-landmark-census.md:39-40` ties the western Ha'tak to "sequence instance 872" and the eastern one to "sequence instance 871".

Gap: nothing in the repo maps "sequence instance 871/872" to the `_Seq` or `_Seq_0` FName. The census gives instance numbers, `sequences.sql` gives root names, and no file joins them. `docs/engine/ue3-package-format.md:504-508` says a cloned root takes the next free FName number (`_Seq` becomes `_Seq_0`), but that is not the same number as the census instance.

Hypothesis, unverified: if the instance numbers follow the FName order (the higher one, 872, is the clone `_Seq_0`), then the western Ha'tak is set 1195 and the eastern is set 1194. The hypothesis is not confirmed by any file in the repo.

Evidence that would settle it: the client's `04_KISMET_EVENTSET_ACTORS.csv` (cited in audit finding 4, not in the repo) or the package's export table for the two `Sequence` objects, read offline. Do not decide it from the lab.

## 3. Hidden until "Unhide"?

Unknown. The repo has the names only:

- The census (`C-landmark-census.md:16`) lists "Designer 0: Unhide Hatak" and "Designer 1: Hatak Explosion".
- The audit (`audit.md:135`) names the same two events and says the meshes "hang" at the positions above. Neither says whether the meshes start hidden.
- The name "Unhide" suggests hidden at load, but the repo holds no initial `bHidden` or Kismet toggle state for them.

Evidence that would settle it: the map package's `StaticMeshComponent` hidden flag and the initial state of the "Unhide" Kismet action (offline package read). The packet forbids the client and the lab for this note.

## 4. The rows DK-31c would add

Sequence ids: unhide = event 6000 (2390 for `_Seq`, 2392 for `_Seq_0`); explosion = event 6001 (2391 for `_Seq`, 2393 for `_Seq_0`). Set 1194 is `_Seq` and 1195 is `_Seq_0`.

Explosions depend on the open question in section 2. Using the hypothesis (western = 1195, eastern = 1194): western explosion = 2393 and eastern = 2391. Replace these if the evidence says otherwise.

Proposed rows (`delay_ms` is a DK-31c call):

- Unhide, both Ha'taks, on the loaded-map restores: chain 8323 (`loaded Dakara_E1`, step 4933) and chain 8324 (step 4934). Each gets `play_sequence 2390` and `play_sequence 2392`. Both are in world 61, so the Ha'tak level is loaded there.
- Unhide on the accept, chain 8322: `play_sequence 2390` and `play_sequence 2392`. Caveat: chain 8322 is in world 62, and `db/resources/Worlds/Seed/worlds.sql:151` shows world 62 is `Dakara_E1_StoryRm`, not the map with the Ha'tak actors (world 61 is `Dakara_E1`, line 145). A sequence played there probably reaches no Ha'tak. DK-31c should either drop the accept-time unhide (the 8323/8324 restores cover it when the player next enters world 61) or move it to a world-61 chain. This note does not decide that.
- Explode the western one: chain 8325 (click `Dakara_E1_RingControl_XMinus`, step 4933), `play_sequence 2393` (or 2391 if the western root is `_Seq`), after the dialog 6108.
- Explode the eastern one: chain 8326 (click `Dakara_E1_RingControl_XPlus`, step 4934), `play_sequence 2391` (or 2393 if the eastern root is `_Seq_0`), after the dialog 6109.

`play_sequence` reaches the acting player only (section 1). If other players in the same world must see the Ha'tak change, DK-31c needs another route; this note does not evaluate one.

Steps: 4933 "western" and 4934 "eastern" are confirmed by `db/resources/Missions/Seed/mission_steps.sql:2839` and `:2841`.

## Checks

No code changed, so no lane check. The worknote is CRLF (checked before the commit).
