# Item sequence lookup — `items_event_sets` archaeology

**Scope:** Does the original SGW server use `resources.items_event_sets` for per-weapon kismet-sequence overrides (Item\_Equip / Item\_Unequip / Item\_Reload / Item\_Use)?  
**Driver:** PR \#338 P90→Pistol swap artifact — hypothesis that per-item sequences exist and differ from the shared archetype sequence.

---

## TL;DR verdict

The original game does **not** use `resources.items_event_sets` for Item\_Equip / Item\_Unequip / Item\_Reload / Item\_Use animations. That table maps combat-event-ids (EVENT\_ItemUse=5, EVENT\_ItemMelee=6, EVENT\_ItemRanged=7) to **per-item combat ability overrides**, not animation sequences. The `getItemSequence` lookup in `python/cell/SGWBeing.py` is archetype-keyed (constant lookup against `ARCHETYPE_ITEM_EVENT_SETS`), and every human archetype maps to the same event set 804. There is no per-weapon equip/unequip/reload sequence data anywhere in the seed. The Cimmeria archetype-keyed shape today IS the original game's shape. The P90→Pistol artifact must be solved differently.

---

## Schema mapping — what `item_event_id` actually is

`item_event_id` is a surrogate PK (auto-increment), not a sequence\_id. It carries no meaning beyond row identity.

The columns that matter are `(item_id, ability_id, event_id)`. The loader (`deprecated/python/common/defs/Item.py:136–140`) interprets them as:

```python
defs[evt['item_id']].events[evt['event_id']] = ability   # ability object, not sequence
```

`Item.events` is a dict of `{event_id: ability_def}`. It is serialized into the cooked XML as:

```xml
<ItemEventSet AbilityID="708" EventID="6" />
```

There is no `SequenceID` in this structure. The table is an **ability override table**, not a sequence table.

### Concrete example rows from the seed

| item\_event\_id | item\_id | ability\_id | event\_id | meaning |
|---|---|---|---|---|
| 1995 | 55 | 708 | 6 | pistol (item 55): melee-hit uses ability 708 |
| 1996 | 55 | 579 | 7 | pistol (item 55): ranged-fire uses ability 579 |
| 6 | 21 | 595 | 6 | SMG "SGHC 6" (item 21): melee-hit uses ability 595 |
| 7 | 21 | 559 | 7 | SMG "SGHC 6" (item 21): ranged-fire uses ability 559 |

Event-id mapping (`deprecated/python/Atrea/enums.py:456–460`):

```
EVENT_ItemUse    = 5
EVENT_ItemMelee  = 6
EVENT_ItemRanged = 7
```

**These are fire / melee-swing combat ability bindings, not animation events.**

### Distribution across the full seed (2,767 rows)

| event\_id | count | meaning |
|---|---|---|
| 6 | 1,299 | EVENT\_ItemMelee — per-item melee ability |
| 7 | 1,151 | EVENT\_ItemRanged — per-item ranged ability |
| 5 | 311 | EVENT\_ItemUse — per-item use ability |
| 3 / 2 | 6 | Other item-event categories |

`Item_Equip (4000)`, `Item_Unequip (4001)`, `Item_Reload (4002)`, `Item_Use/4003` **never appear** as event\_id values in `items_event_sets`. The note about `item_id=4000,4001,4002,4003` in the seed is a coincidence of integers — those are item IDs for items that happen to exist in the catalog; their event\_id values are 6 and 7, same as any weapon.

---

## Python reference — `getItemSequence`

**File:** `deprecated/python/cell/SGWBeing.py:517–523`  
**Function:** `getItemSequence(self, eventId)`

```python
def getItemSequence(self, eventId):
    """
    Returns the kismet event sequence for the specified item event.
    @return: Kismet event sequence or None, if no sequence was found
    """
    eventSet = DefMgr.get('event_set', Constants.ARCHETYPE_ITEM_EVENT_SETS[self.archetype])
    return eventSet.getSequence(eventId) if eventSet else None
```

**Call sites** (all in `SGWBeing.py` or `SGWPlayer.py`):

| Event | File:line | Called from |
|---|---|---|
| `Item_Reload` (4002) | `SGWBeing.py:871` | `onReloadWeapon` |
| `Item_Equip` (4000) | `SGWPlayer.py:904` | equip handler |
| `Item_Unequip` (4001) | `SGWPlayer.py:919` | unequip handler |
| `Item_Use` (4003) | `SGWPlayer.py:2167` | `useItem` |

**Lookup path:**

1. `Constants.ARCHETYPE_ITEM_EVENT_SETS[self.archetype]` — maps archetype integer to event-set id. Every human archetype (Any=0, Soldier=1, Commando=2, Scientist=3, Archeologist=4, Goauld=6, Sholva=7, Jaffa=8) maps to **804**. Asgard (5) maps to **1455** (`deprecated/python/common/Constants.py:103–106`).
2. `DefMgr.get('event_set', 804)` — loads the `EventSet` object for set 804 (`"Item handling generic event set"`, kismet file `KIS-abilities_human.KIS-handling`).
3. `eventSet.getSequence(eventId)` — returns the `KismetEventSequenceObject` matching `eventId`. The `EventSet` instances dict is populated from `resources.event_sets_sequences`, **not from `resources.items_event_sets`**.
4. Caller calls `self.playSequence(seq.seqId, self.entityId)` — sends the integer `seqId` to the client over `Event_NetIn_onSequence`.

There is **no per-item override path** in `getItemSequence`. The function never touches `item.events`. The `Item.events` dict (populated from `items_event_sets`) is accessed by the combat and ability subsystems to determine which ability fires when a weapon is used or swings — it has no role in animation.

**`ability_id` participation:** Every row uses a weapon-combat ability id (e.g., 579=ranged pistol shot, 708=ranged SMG shot). These are sentinel `any`-style: a per-item override for which combat ability triggers on that event type.

**Per-fire vs cached at equip:** Not relevant here; the Item.events dict is loaded at server startup and cached in-memory for the life of the process.

---

## Ghidra cross-check — client wire receiver

**Function:** `register_NetIn_onSequence @ 0x00d76f40`

The registration table entry at `0x019c7f44` shows the event name string `"Event_NetIn_onSequence"` alongside the emit-signal vtable. The CME signal type is:

```
CME_EventSignal<VEvent_NetIn_onSequence>::TypedEmitInfo
```

This is a Direction=NetIn (server → client) single-integer event. The client's subscription chain receives a `seqId` integer, looks up the corresponding `CookedKismetEventSequenceData` entry in its cached event set zip (populated from the cooked XML the server sends at spawn), and drives the kismet sequencer to play the named animation.

**Confirmed:** the client treats the `seqId` as an opaque integer lookup key into its preloaded cooked-kismet data. There is no per-item or per-weapon dispatch inside the client; it simply finds the kismet sequence by integer id and plays it. Changing the server-sent `seqId` would change the animation. However, since the original server sends a uniform `seqId` for all human archetype weapons (from event set 804), there are no per-weapon `seqId` values in the original data to switch to.

---

## Implementation guidance for Cimmeria

### No change to the lookup path is warranted

The Cimmeria `archetype_item_event_set` function and `fire_item_sequence` in `crates/services/src/cell/cell_methods/player/world.rs:181` faithfully mirror the original Python. The original game sends the same `seqId` (1872 for `Item_Equip`, 1873 for `Item_Unequip`) regardless of weapon. The P90→Pistol artifact is not a server data problem — it is a kismet sequence design issue in the shipped client assets: sequence 1872 was authored assuming a back-holster start pose, which looks correct for P90→anything but creates a visible from-back-reach on Pistol→anything.

### Where to look for the actual fix

The artifact lives in the kismet animation asset, not in server sequence selection. Options in increasing feasibility:

1. **Client-side PAK inspection:** Check whether a separate `Item_Equip` variant sequence exists (e.g., a pistol-specific draw-from-hip) that was registered under a different event-set id in the shipped kismet files. If so, the server would need to know which event-set id to use per weapon class — that is a new per-weapon-class lookup, not a per-item one.

2. **Suppressing the equip animation on P90→Pistol swap specifically:** If no variant sequence exists, the correct server behavior may be to skip `Item_Equip` on pistol-draw-after-P90 (since the P90-unequip animation leaves the hand in a compatible position for pistol idle). This is a server-side timing / conditional fire change, not a sequence-data change.

3. **Accept the artifact as shipped behavior:** There is meaningful probability the shipped 2009 client showed this same artifact for P90→Pistol swaps; the game never reached open launch. Logging a known-issue note and deferring is a valid option.

### `items_event_sets` — correct current use

If Cimmeria needs per-item combat ability dispatch (which ability fires on `EVENT_ItemRanged` per weapon), the `items_event_sets` table feeds `Item.events` and is the correct source. That path is currently used by the ability launch system, not the animation system.

---

## Open questions

1. **Does a per-weapon-class equip sequence exist in the shipped kismet PAK?** Inspecting `KIS-abilities_human.KIS-handling` under `external/` (populated by `setup.ps1`) would resolve this. If a separate event-set id or sequence id exists for pistol draw vs. rifle draw, a weapon-class-keyed lookup (not item-keyed) would be the original intent.

2. **Was the from-back pose artifact present in the 2009 beta?** No client recording is available. If it was present in beta, suppressing is the wrong call.

3. **Does event set 1455 (Asgard) differ in structure?** Not relevant to this bug, but worth confirming its sequences exist in the seed for completeness.

---

## Evidence trail

| Claim | Source |
|---|---|
| `getItemSequence` is archetype-keyed only, no item override | `deprecated/python/cell/SGWBeing.py:517–523` |
| All human archetypes → event set 804 | `deprecated/python/common/Constants.py:103–106` |
| `items_event_sets` loader assigns ability, not sequence | `deprecated/python/common/defs/Item.py:136–140` |
| `event_id` distribution in seed (5/6/7 only) | `db/resources/Items/Seed/items_event_sets.sql` sampled |
| Pistol (item 55) has event\_id=6,7 rows only | `items_event_sets.sql:3995–3997` |
| Item\_Equip(4000) never appears as event\_id | grep of full seed, zero matches |
| Client receives opaque seqId | `register_NetIn_onSequence @ 0x00d76f40`, CME signal anatomy |
| `EventSet.getSequence` uses `event_sets_sequences`, not `items_event_sets` | `deprecated/python/common/defs/EventSet.py:26–46` |
