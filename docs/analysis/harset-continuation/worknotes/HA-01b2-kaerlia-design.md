# HA-01b2 provisional Sgt. Kaerlia design

Packet HA-01b2 under owner decision O18-01
([owner decisions](owner-decisions-2026-10-18.md)). Research only: no seed, ID, code,
lab or client change was made. Repo tip 29f9d3fdb, nothing committed.

Purpose: design a provisional NPC "Sgt. Kaerlia" at the Harset Stargate who gives the
mission 1664 step 5224 conversation, so the Castle-to-Harset debrief has a speaker. The
owner approved a provisional design; the final identity stays open.

## Labels

- FACT: read from a seed, source file or measured this session. Confidence high unless stated.
- INFERENCE: reasoned from facts, with a stated confidence.
- PROVISIONAL: a choice made here with no authoritative source. Each carries its evidence
  and is expected to change if the owner or the client disagrees.

## Summary

| Question | Answer | Label | Confidence |
|---|---|---|---|
| Who speaks step 5224? | Sgt. Kaerlia, text moniker 27315 | FACT (name), INFERENCE (placement) | high / medium |
| Which dialog is hers? | Dialog 6169 via dialog set 1992 and DSM 7016 | INFERENCE | medium-high |
| Gender | Female | PROVISIONAL | low-medium |
| Body and clothing | Human female, SGC uniform, no weapon | PROVISIONAL | medium |
| Faction and level | Faction 1, level 50 sentinel | PROVISIONAL | medium |
| Position | (-5, -68.95, 30), world 57 | PROVISIONAL, nav verified | medium |
| Does anything bind 6169 today? | No | FACT | high |

## 1. Identity and look

### Name

FACT: Texts moniker 27315 is `DN_npc_int_SgtKaerlia_Harset`, text "Sgt. Kaerlia". Monikers
27308 to 27314 are the step and objective texts of mission 1664. Step 5224 reads "Speak to
Sgt. Kaerlia by the Stargate for further instructions." Objective 6063 is its objective.

INFERENCE (medium): `name_id` resolves only against client-shipped text. Whether 27315 is
shipped is unproven. The `npc_int` family matches the Harriman moniker 26715, which does
render, so it likely is. A client glance at the nameplate is owed (open question 1).

FACT: a mob nameplate prefers `name_id` over `display_name`, so `display_name` is only a
fallback when `name_id` is NULL. New `texts.sql` ids never render in the client.

### Rank, species and body

FACT: the moniker carries the rank "Sgt.". The step comes from
`DS1_ms_A00_Castle_opc_09Debriefing`, where opc is OP-CORE.

INFERENCE (high): she is Tau'ri military, so the body set is `BS_HumanFemale.BS_HumanFemale`
if female.

### Gender (PROVISIONAL)

Choice: female. INFERENCE low-medium. Evidence:

- The name Kaerlia reads feminine.
- The Harset hub cast is mostly male (Hansen, Jacobs, Marsh, Nerus, Blackstock) with one
  female, Copplemann, so a second woman is not out of place.
- Reusable female SGC-uniform rows exist (below); no male-specific work is saved.
- Dialog 6169 has no gendered pronouns, so the text does not contradict it.

Change cost if wrong: swap the component list and body set on one template row.

### Appearance and clothing (PROVISIONAL)

Clone the component set of template 370 (Debug Hub Banker), the best non-hostile SGC
female base. FACT, its components:

- `AR_H_SGC.AR_HM_SL1_SL100`, `AR_H_SGC.AR_HM_ST1_ST100`
- `BS_HumanFemale.BS_HF_Base_Boots00_00`, `_Hands00_00`, `_Legs00_00`, `_Torso00_00`
- `BS_HumanFemale.BS_HF_Head_00` (generic head)

Optional: add `BS_HumanFemale.BS_HF_Hair_01`, used by templates 177 (Castle_Medic) and 219
(Storage Lo'taur). Skin tint -256076032 as on the hub cast. No weapon component: she is a
quest giver at a safe gate. Template 170 (Castle_Muelbach) has the same clothing plus
`WP-Human.WP_SMG_1A` but is faction 10, so it is not cloned directly. Templates 33
(Sam Carter) and 48 (Copplemann) use named unique heads and are not reusable.

## 2. Template shape (PROVISIONAL)

ID needed: template id. Candidate 260 `Harset_Kaerlia` in
[the allocation table](../id-allocation.md); FACT it is absent from the seed, and template
360 is the Debug Hub pet trainer, a different namespace from spawn 360. Not allocated here.

| Column | Value | Reason |
|---|---|---|
| `template_name` | `Harset_Kaerlia` | allocation candidate |
| `name_id` | 27315 | the moniker above |
| `body_set` | `BS_HumanFemale.BS_HumanFemale` | section 1 |
| `components` | clone of 370 | section 1 |
| `faction` | 1 | hub cast is faction 1; only 10 is damageable; faction 0 is an auto-aggro dead zone |
| `level` | 50 | seed sentinel for unknown level; level only drives HP, XP and level hooks |
| `class` | mob | the hub cast shape; `spawnable` and `being` classes never AI-tick |
| `interaction_type` | 134217728 | talk bit; without a bit the client never sends the click |
| `event_set_id` | 570 | as on the hub cast |
| `speaker_id` | 3219 | the only speaker whose lines are the 6169 NPC lines |
| `has_dynamic_properties` | true | per-player dialog binding |
| vendor, trainer, repair, loot, ability set, weapon | NULL | quest speakers carry none |
| `vault_scope` | default | not a banker |

Notes:

- FACT: speaker 3219 has an empty name, so the plate name must come from `name_id`. No
  template references speaker 3219, and nothing else references set 1992 or DSM 7016.
- FACT: the talk-bit precedent is INT_NonAStoryMissionAvaliable = 134217728, the Harriman
  clerk (template 58) in `ss-u3.md`.
- FACT: the `interact_tag_linter` requires a `set_interaction_type` for every `interact_tag`
  NPC tag in the same seed file, so the tag must be paired with one.
- The "!" story cursor (INT_AStoryMissionActive, flags 16777216 on DSM 7016) is delivered
  per player by `add_dialog_set`, not on the template.

## 3. Placement (PROVISIONAL, nav verified)

ID needed: spawn id. Candidate 360 (spawn ids 360 to 379 are absent from the seed; [id-allocation](../id-allocation.md) already assigns 361 to 379 as candidates to other packets, 360 is this NPC's). ID needed: tag.
Proposed `Harset_Kaerlia`, to register in the tag worknote.

FACT anchors from [placements](../placements.md):

- Stargate arrival (-0.076, -67.274, 38.011), heading 3.141.
- Point set 1001 `Harset.Stargate`: centre (-0.372, -67.364, 37.353), radius 2.5, height 10.
  Arrival must not trigger pending dials, so the NPC stays outside it.
- Respawner 20 at (-8, -68.99, 34), inferred, with a ground guard.
- DHD spawn 37 at (4.407, -68.957, 31.525). Plaza floor y about -68.92.
- The Castle hub keeps NPCs 5 or more units from the respawner, and the respawn pad pulls
  the nearest-NPC spacing.

### Offline mesh probes

FACT: measured with `nav_inspect --probe` on `data/spaces/harset.nav` (372 components, the
gate area is component 11). Log
`C:/Users/Steve/AppData/Local/cimmeria-build/logs/zone-harset/20261018-142823-53708.log`.
Method follows [HA-02](HA-02-offline-revalidation.md): on-mesh with horizontal gap h and
vertical offset dy.

| Point | x, y, z | Component | h (m) | dy (m) | Verdict |
|---|---|---|---|---|---|
| R20 | -8, -68.99, 34 | 11 | 0.00 | -0.16 | ok |
| ARR | -0.076, -67.274, 38.011 | 11 | 0.00 | -0.04 | ok |
| DHD | 4.407, -68.957, 31.525 | 11 | 0.00 | -0.37 | ok |
| A | -5, -68.95, 30 | 11 | 0.00 | -0.13 | ok |
| B | 8, -68.95, 36 | 11 | 0.00 | -0.12 | ok |
| C | -6, -68.95, 32 | 11 | 0.00 | -0.12 | ok |
| D | -4, -68.95, 35 | 11 | 1.80 | -0.12 | ok, marginal |
| E | 6, -68.95, 34 | 11 | 0.00 | -0.12 | ok |

### Choice

- Primary A (-5, -68.95, 30): about 5 u from R20, about 9.5 u from the DHD, about 9.4 u from
  the arrival point, outside point set 1001, on the gate component. INFERENCE (medium): near
  enough that a new arrival sees her, far enough to keep the pad clear.
- Fallback B (8, -68.95, 36): about 8.3 u from the arrival and 4.6 u from the DHD.
- Reject C: only about 2.8 u from R20, inside the spacing the Castle hub keeps clear.
- `is_stationary` true on the spawn row. `respawn_secs` NULL is not acceptable for a quest
  giver the player must reach: set a respawn only if the tag worknote decides she can die;
  at faction 1 she cannot be damaged, so none is needed.
- Heading: face the arrival point. The NPC heading convention is unverified, so no number is
  given here and a client glance is owed (open question 2).

Mesh connectivity proves reachability only, not that level geometry is right; ground truth
is a client look.

## 4. Dialog join

### FACT

- Dialog 6169: flags 0, `event_set` NULL, DUIST_DefaultDialog, no `dialog_screen_buttons`
  rows. Five screens:
  - 0, NPC (speaker 3219): "Keep moving... Lots of wounded coming through here, we need room."
  - 1, player (speaker 0): "What is this place?"
  - 2, NPC: "You're on Harset now - once the site of one of Ra's archeological digs, it is now
    your hot, sweaty, sand-blasted home. Feel free to explore."
  - 3, player: "Where do I report?"
  - 4, NPC: "Ah, right. Make sure you report in at the command center for debriefing. I'll
    upload a path to your PDA."
- DSM 7016 maps dialog set 1992 to dialog 6169, topic "Debriefing", flags 16777216.
- Nothing binds set 1992 or DSM 7016 today. Candidate join only, per
  [HA-00b](HA-00b.md) line 17.
- The H20 comment in `harset_jaffa_chains.sql` (lines 531 to 593) records why the arrival
  chain was not authored and suggests a "gate medic" template with speaker 3219.

### INFERENCE

- Medium-high: 6169 is the step-5224 conversation. Screen 4 "upload a path to your PDA"
  matches objective 6059 (follow the minimap path in step 5225), and the topic reads
  "Debriefing".
- Medium: speaker 3219 is Kaerlia. The NPC lines are an arrival briefing at the gate.
- Low: the "gate medic" label in the H20 comment. It is a prior note's reading of "wounded
  coming through", not evidence of a medic role. Do not name her a medic.

### Mission steps for context

- 5224: speak to Kaerlia (objective 6063).
- 5225: reach the Command Center (6059 optional minimap path, 6060 mandatory right-click
  door, task_type 1).
- 5226: report for debriefing (6061 Col. Marsh, 6062 Moh'katan).

Kaerlia owns only the 5224 to 5225 edge. She never touches 5225 or 5226.

## 5. Interaction guards

### What the engine does with a click

FACT, from `cell_methods/player/interaction/interact.rs` and `cell-interactions/.../dispatch/interact.rs`:

1. Trainer check, then the `interact_tag` chain, then the `interact_template` chain, then
   `handle_interact`, then `fire_dialog_open`.
2. `handle_interact` checks the per-player `available_interactions` (set by `add_dialog_set`,
   slot = template id), then the static `interaction_type`.
3. A target with no bound dialog and no static type dead-ends silently, with only friction
   telemetry.
4. Every matching chain fires. A chain that claims the click skips `handle_interact`, so a
   `display_dialog 6169` chain needs `target_entity_id`, which `fire_interact_tag` stamps.
5. `OnDialogOpen{dialog_id}` and `OnDialogChoice{dialog_id}` exist as triggers; 6169 has no
   buttons, so a choice trigger is unlikely.

### Consequence

A player without mission 1664 who clicks her would get silence. Project rule: every press
gets visible feedback. So the design needs an explicit mechanism for everyone else.

### Design (PROVISIONAL)

Two `interact_tag` chains on tag `Harset_Kaerlia`, plus a baseline:

- Chain K1, the grant-of-progress edge: conditions mission 1664 active at step 5224 and
  world 57. Actions `display_dialog 6169` then `advance_step 1664 5225`. Advance on the
  interact itself (or on dialog open), not on a choice.
- Chain K2, feedback for everyone else: `npc_bark` (precedent in
  `debug_area_plaza_chains.sql` lines 95 and 117; it needs a dialog screen row,
  `ID needed: dialog screen`) or a monologue `display_dialog` (`ID needed: dialog`). Do
  **not** use `system_message`: FACT, `executor/dispatch.rs:312-315` is a log-only stub
  whose wire format is unknown (see [HA-01b1](HA-01b1-door-affordance.md#q4-first-press-feedback-and-what-needs-the-client)).
  Variants by state:
  - 1664 at 5225 or 5226: "Report to the Command Center."
  - no 1664: a neutral line that she has nothing for them.
- Because every matching chain fires, K1 and K2 must have mutually exclusive conditions,
  or K2 would also fire on the 5224 click.
- Optional per-player `add_dialog_set 7016` with slot = the template id for the "!" cursor,
  as sketched at the H20 comment. It must be paired with the matching removal when the step
  moves on.
- Pair the tag with `set_interaction_type` for the linter.
- Baseline: the talk bit 134217728 on the template so the client sends the click at all.

Engine-contract traps to honour (R6): `once` resets on relog, so the guard is the mission
step, not `once`; item or bind cleanup goes before the mission mutation; never mutate shared
world 57 actors per player.

Why advance at the interact: dialog 6169 has no buttons, so the player may close the
window before any choice. Advancing on open means the PDA-path line is always followed by
the new step.

Relog and duplicate clicks: after K1, the step is 5225, so K1's step condition is false and
a repeat click falls to K2's "Report to the Command Center" variant. That makes the pair
idempotent without `once`.

## Open questions

1. Does the client render `name_id` 27315 on the nameplate? A lab or client glance owes it.
2. NPC heading convention (which axis a heading value faces), so she faces the arrival.
3. Castle's grant edge for 1664 (CS-06): who grants the mission and at what step.
4. The abandon path without chain 1399 (O18-02): if 1664 is abandoned, K2's no-mission
   variant applies; confirm that is acceptable.
5. Final identity: whether Kaerlia keeps this provisional look if the owner supplies one.
6. Whether 6169 is truly her dialog (INFERENCE medium-high): confirm against the client
   dialog tree if one becomes available.

## Tests the implementation owes

Each must fail when the guarded behaviour is reverted.

- `kaerlia_click_at_step_5224_advances_to_5225`: fails if K1 advances at any other step or
  not at all.
- `kaerlia_click_without_1664_gets_feedback`: a player without the mission receives a bark
  or monologue dialog; fails if K2 is removed (silent dead end).
- `kaerlia_click_at_5225_or_5226_does_not_regress_or_skip`: the step stays; fails if K1 is
  not step-gated.
- `kaerlia_duplicate_click_and_relog_idempotent`: a second click and a relog do not
  advance twice; fails if K1 relies on `once`.
- `kaerlia_k1_k2_mutually_exclusive`: on the 5224 click exactly one chain fires.
- `kaerlia_spawn_on_mesh_outside_point_set_1001`: the spawn is on the world 57 mesh,
  outside point set 1001, and at least 5 u from respawner 20.
- `kaerlia_template_has_no_vendor_or_trainer_fields`: the template shape holds NULL for
  the vendor, trainer, loot, ability and weapon columns and faction is not 0.
- `kaerlia_name_id_resolves_in_texts`: `name_id` 27315 exists in the Texts seed.
- `interact_tag_linter_accepts_kaerlia_tag`: the tag is paired with `set_interaction_type`.

## Telemetry the implementation owes

- One interact outcome row per click with chain id, player, mission state and a reason:
  `advanced`, `no_mission`, `wrong_step`, `duplicate`.
- A row for the bark or monologue dialog sent, with the variant name.
- The existing dialog-open row for 6169, so the advance can be matched to the open.
- A silent-dead-end counter staying at zero for her entity: a non-zero friction row for
  `Harset_Kaerlia` means the guard pair has a hole.
