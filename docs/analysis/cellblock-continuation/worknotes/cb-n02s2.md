# CB-N02s2 relog-restore bit check

> **Type:** Reference
> **Audience:** Cellblock coordinator and the CB-N02 implementers
> **Last updated:** 2026-10-16

Static check of every `set_interaction_type`, `set_aggression` and `add_dialog_set` setter in missions 622, 640, 641, 680, 687 and 688 against the restore that is supposed to undo a relog. [cb-n02s](cb-n02s.md) matched those restores by chain existence only; this note checks each bit. Source: `db/resources/Content/Seed/castle_cellblock_chains.sql`, `db/resources/Worlds/Seed/spawnlist.sql` and `db/resources/Entities/Seed/entity_templates.sql`. No lab, no code, no build. Nothing here was reproduced live.

## Method

- A fresh Castle_CellBlock instance is built on every login. Entity `interaction_type_flags`, aggression and per-player dialog sets all reset to the template or spawn defaults, so a restore must re-apply every bit its setter applied while the window is still open.
- Restore chains are `player_loaded` on `Castle_CellBlock`, scope player, gated on the setter's step or mission status.
- Template base `interaction_type` for the tags involved (parsed from `entity_templates.sql`; the first two numeric columns after the mesh/body/components columns): templates 13 (WoodenCrate), 18 (Ambernol vial), 10 (Col Marsh), 21 (guard corpse), 14 (Frost corpse), 19 (TerminalX, Terminal) and 22 (door button) are 0. Templates 3 (all three ring switches) and 8 (SMG-1A locker) are 32.
- Verdicts: `exact` (restore re-applies exactly what the window needs), `over` (restore or template default shows a binding outside its window), `under` (restore misses part of a bit), `missing` (no restore).

## Binding table

Masks: 256 `INT_MinigameLivewire`, 32 RingNetwork, 8388608 Col Marsh mission-available, 1073741824 `INT_MissionWorldObject`, 4194304 `INT_A_STORY_MISSION_PENDING` (interaction_flags.rs:66; the seed clears it on body search).

| Tag / dialog set | Mask / level | Setter (chain, step) | Clearer (chain, step) | Restore chain | Restore condition | Verdict |
|---|---|---|---|---|---|---|
| Frost dialog set 5229 (slot 14) | add_dialog_set | 1001, 622 accept (step 2113) | none | 1006 | `step_status` 622 `2113` active | exact. The set outlives the step in-session, but a relog past 2113 correctly does not re-add it. |
| Guard dialog set 5230 (slot 21) | add_dialog_set | 1003, 2113 to 80623 | none | 1007 | `step_status` 622 `80623` active | exact |
| GuardBody (tmpl 21) / FrostBody (tmpl 14) search | 4194304 clear | none (template 0) | 1003, 1005, 1010 | none needed | n/a | exact. The clear is a no-op against template 0, so there is nothing to restore. |
| NID guard aggression | level 1 (HOSTILE) + threat | 1008, Region8 enter, `once` | none | 1009 | `mission_status` 622 completed | exact. Spawnlist seeds NEUTRAL, so the restore is needed and sufficient. |
| `HackTheRings_Switch` (spawn 23, tmpl 3) | 256 | 1034, 639 completes | 1042, 2120 to 2215 | 1045 | `step_status` 640 `2120` active | exact |
| `HackTheRings_Switch` | 32 | 1042, 2215 | 1044, teleport_in region 2 | 1046 | `step_status` 640 `2215` active | exact in window. Template base is 32, so after 1044 clears it a relog re-lights it (over, harmless: 1043 is gated on step 2215). |
| `HackTheRings_Switch` | 1073741824 | 1042, 2215 | 1043, interact at 2215 | 1046 | `step_status` 640 `2215` active | exact, with a cosmetic over: a relog after the 1043 interact but before 1044 re-lights a cleared highlight. |
| `Preparation_ColMarsh` (tmpl 10) | 8388608 | 1044 (640 done); 1066 (step 3563) | 1053/1054 (641 accepted); 1058/1059 | 1063; 1064 | 640 completed AND 641 not_active; `step_status` 641 `3563` active | exact. No bit is needed at step 80641. |
| `Preparation_SMG1A` (spawn 11, tmpl 8) | 1073741824 | 1053/1054, step 2121 | 1055, pickup | 1062 | `step_status` 641 `2121` active | exact in window; template base 32 is over, harmless (see Observations). |
| `Preparation_Terminal` (spawn 16, tmpl 19) | 256 | 1058/1059, step 3564 | none, by design (shared marker) | 1065 | `step_status` 641 `3564` active | exact; the in-session over is intentional (seed comment ~1557). |
| `Preparation_RingSwitch` (spawn 17, tmpl 3) | 32 | 1061, 641 to 680 | 1072, teleport_in region 3 | 1074 | `step_status` 680 `2344` active | exact in window. Template 32 re-lights after 1072 (over, harmless: 1071 is step-gated). |
| `Preparation_RingSwitch` | 1073741824 | 1061 | 1072 | 1074 | `step_status` 680 `2344` active | exact |
| `Cellblock_WoodenCrate` (spawn 8, tmpl 13) | 1073741824 | 1097, mission_accepted 687 | 1192/1098/1193/1194/1099/1195 on search | 1104 | `step_status` 687 `2354` active | exact. 1191 reopens pending loot on later presses. |
| `Cellblock_TerminalX` (spawn 34, tmpl 19) | highlight | 1106, mission_accepted 688 | 1107 | 1110 | `step_status` 688 `2356` active | exact |
| `Cellblock_ArmoryRingSwitch` (spawn 79, tmpl 3) | highlight | 1107 | 1109 (out of scope) | 1111 | `step_status` 688 `80688` active | exact for the set; the clear is chain 1109, owned by the ring campaign. Template base 32 (see Observations). |

Context rows, outside the six missions but part of the same relog window:

| Tag | Mask | Setter | Clearer | Restore | Verdict |
|---|---|---|---|---|---|
| 638 prisoner dialog set (slot 17) | add_dialog_set | 1011/1012 | 1018/1019 | 1026/1027 | exact (sealed as CB-N02a) |
| `329_CellDoorButton` (tmpl 22) | 256 | 1014/1015 | 1017 | none yet | under/missing: already sealed as CB-N02b (chain 1028) |
| `ArmYourself_AmbernolVial` (tmpl 18) | 1073741824 | 1031 | 1032 | none yet | missing: already sealed as CB-N02c (chain 1036) |
| Drone aggression | level 1 | 1032 | none | none yet | missing: already sealed as CB-N02d (chain 1037) |
| Col Marsh despawn (686/687) | entity | 1161 | n/a | 1162 | exact precedent |

Observations (none of them a new gap):

- Template 3 gives all three ring switches base `interaction_type` 32 and template 8 gives the SMG-1A locker 32. So every clear of bit 32 (1044, 1072) is a no-op across a relog, and the fresh state already shows the RingNetwork icon outside the mission windows. Every ring interact chain (1043, 1071) is gated on its step, so the click does nothing out of window. The ring campaign owns the switch presentation and chain 1109; this is recorded for them, not sealed here.
- Restore chains are not `once`, except 1026/1027 (`once` is per cell-entity life, so a fresh instance fires them again). Each login therefore fires them once per fresh instance, which is what is wanted.

## Sequences 1749 and 10001

### 1749 (cell door open, chain 1017)

- Setter: chain 1017 (Livewire victory) runs `advance_step` 2116, `play_sequence` 1749, then clears the button bit. No `player_loaded` chain replays 1749.
- Is the state needed to progress after a relog at 2116? No, per the seed. Step 2116 advances only through `dialog_choice` chains 1020/1021 and then 1018/1019, all gated on `step_status` 638 `2116`, and the dialog set is restored by 1026/1027. Nothing in the seed reads the door's open state.
- The door's open state is client-map state: the QA map `fffefffd` has a `Prisoner329CellDoor` event driving SetBool, PlaySound and Interp, with no server-side persistence ([presentation-handoff](../presentation-handoff.md), doors row). Whether a relog leaves the door visually closed or physically blocking is unproven ([route-baseline](../route-baseline.md) line 321: door swing and release state not physically checked).
- Owner: Asset audit packets CA-03A (door trace, Ready) and CA-06D (relog replay, BlockedEvidence; branch `origin/docs/cellblock-asset-audit`), relog scenario CA-U17. Cellblock opens a CB-N02 child only if CA-U17 or CB-U03 reproduces a stranded player. Precedent if the owners decide a replay is needed: chain 1002 already replays `play_sequence` 10000 on every load once 622 is complete. No client patch is proposed, and no child is sealed on a speculative fault.

### 10001 (TakeCoverIndicator, chain 1032)

- Chain 1032 (step 2145, vial interact) plays 10001, then advances to 2144; the hide counterpart is 10014 (chains 1132/1133).
- After a relog at 2144 the indicator is gone. It is a hint only: cover progress is driven by `player_entered_cover`, independent of the indicator, so the state is not needed to progress.
- Owner: presentation/asset audit. The drone re-arm that does matter for the fight is already CB-N02d (chain 1037).

## Sealed children

None. Every row above is `exact` (with cosmetic or template-default overs that cannot strand a player or open an out-of-window interaction). The three real gaps in these missions' neighbourhood (door button, vial, drone) are already CB-N02b, CB-N02c and CB-N02d. CB-N03a is not sealed by this packet, so W1c is not triggered by CB-N02s2 (CB-N03s may still seal one).

## Verdict

no new gaps

## Measurement

- Working tokens: not exposed by the harness.
- UTC: start not captured before a context compaction; end 2026-10-16T16:46:35Z (`Get-Date -AsUTC` at the last evidence read, before the write).
- Lane runs: 0.
- Review rounds: 0 at write time.
