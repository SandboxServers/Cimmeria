# T-01 source, map and placement contract

> **Title:** T-01 source, map and placement contract
> **Type:** Reference
> **Audience:** Tollana packet authors and reviewers
> **Last updated:** 2026-10-14
> **Companions:** [ledger](../README.md), [placement register](../tollana-placement-contract.csv), [ID register](../tollana-id-register.csv)

## Decision and evidence boundary

Under O14-20, select missions **568, 569 and 570** as the bounded OPC
control-technology *implementation candidate*. The recovered journal text
supports the sequence of Copplemann's search, Lethander's data disk and
Travell's camp (`db/resources/Missions/Seed/mission_steps.sql:5205-5233`).
The proposed 568 -> 569 -> 570 unlocks are `CIMMERIA_RECONSTRUCTION`:
the archive has no per-mission build label or prerequisite edge. The source
IDs, monikers and journal text alone are `RESTORED_SOURCE` candidates. This
selection does **not** change the T-00 matrix's 51 verified `hold` rows or
authorize enabling a mission. Mission-system review approves the bounded
candidate and 568-first pilot, subject to the gates below.

The other 27 core/cross-zone intentions are held by ID: OPC 590, 598,
601-603; Goa'uld 604-611; DRK 612-621; Loyalist Jaffa 1626, 1629, 1632,
1643. The 21 side/lineage records remain held: 743, 744, 860, 863-874,
876, 877, 879, 882, 1301 and disabled 1723. Conditional Harset-moniker
missions 1364 and 1366 remain separate holds. No held row is Done. These
holds retain all four faction paths in the placement/dependency map. T-01
does not infer that the other factions share a compatible chronology.

Mission 590 is a specific item/escort hold. Step 1977 of 570 says to obtain
Control Technology from Travell; steps 2398/4000 of 590 revisit Travell's
arrangement and delivery. The handoff does not establish whether these are
the same item state. Step 1983 also requires Sagrine's return. A later T-04
contract needs item and movement review before treating 590 as a sequel.

All 51 source rows have unknown mission build lineage. The T-00 seed graph
has 240 disabled steps and 259 disabled objectives, no direct mission
acceptance-dialog join, and no Tollana content chain. The missing source
SQLite databases and build field remain evidence limits. O14-03 permits a
reviewed bounded reconstruction, not an assertion of 2009 gameplay.

## Map and route survey

The extracted `Stargate Worlds (0.58674) (beta)` Tollana map has 506 `.umap`
files. `Tollana-00000000.umap` hashes to
`3e39e5a6cfbbf63555170af17992c3e6383f21b06f93185a4f7d641026bc341b`,
matching build 58674 in the supplied
`WORLDS/RECOVERED/Tollana/01_MAP_PACKAGES.csv`. This proves the local root
package belongs to that archived map build; it does not assign build 58674
to any mission or prove that the current lab client has identical packages.
The configured lab client's Tollana root package instead hashes to
`eabb468962accee3b43694830e463524eb38fd865c4e5f36ab62d55f6c3476a2`,
matching the archive's **QA4046** map row. This is a measured client/map
lineage distinction; neither map hash gives a mission build label.
`tools/extract_actors.py` over both the build-58674 and configured QA4046 maps,
filtered for prefab, interactable, NPC, mob, note and PlayerStart actors,
returned no named Repair Facility,
Copplemann, Lethander, Travell, Curia or ring *quest actor* placement. A
PlayerStart is present but is not the server arrival rule. The extractor's
bounded class/name filter is not proof that no relevant static mesh exists.

The current seed puts gate 4 at `(211.045, 0.606, -547.644)`, DHD spawn 39
at `(209.748, -1.008, -552.898)`, ring consoles 155 at
`(235.244, -1.109, -571.308)` and 156 at
`(-766.127, 9.5, 379.945)`, all in world 19. The independent
`docs/analysis/debug-area/map-selection-study.md` reports the gate and west
gallery rig on nav component 89. Its middle native rig at
`(-139.91, 1.66, 312.33)` is component 885 and has no seeded ring console
or transport region. These are survey anchors, **not** quest cast positions
or a proven walking route. World 19 uses advisory navmesh; a nearby polygon
does not prove a usable NPC start, interaction clearance or player path.

The placement CSV has one row for each 568-570 candidate location,
actor, prop and encounter named in the seeded steps, plus separate survey or
held-path rows. Unlocated quest rows have empty
coordinates and `UNVERIFIED` status. Before T-02 or T-03a becomes Ready,
inspect the current cooked map and client scene at each proposed coordinate,
measure floor/collision, check NPC start polygon and player route from gate or
working transport, keep at least interaction clearance around the actor, and
test client render and two-player AoI. Gate and ring approaches remain neutral;
stationary NPCs do not become neutral merely because they do not patrol.
The directional leads conflict: mission 569's step 1968 says northeast, while
dialogs 2155/2156 say a destroyed building to the south. Mission 570's step
1974 says southwest, while dialog 2161 says southeast
(`db/resources/Dialogs/Seed/dialog_screens.sql:9702-9706,9734`). The CSV
keeps both destinations unplaced and does not equate either with the other
or with Travell's camp. T-03b/c must reconcile the dialog/step lineage and
client route before selecting coordinates.

The 2026-10-14 lab survey used the default client's QA4046 map and the
server selected in that client. A GM world-19 placement at
`(230, -1, -540)` showed a rendered gate-area walkway; `(570, -1, -530)`
showed a rendered urban pad. These are client render observations, not native
walk routes or NPC interaction proof. A same-world GM jump to
`(-770, 9, 380)` changed client position to world 19 at that coordinate,
but the screen showed sky/black void and the player fell. The telemetry bridge
became unreachable during a recovery command; recovery was not verified. This
**fails that jump probe** and blocks using the west-gallery coordinate as a
placement. It does not establish whether a native approach or ring arrival
to the seeded ring-156 area works. The lab lease was released. The separate
lab-server endpoint returned zero sessions and no matching log lines, so
there is no server-log attestation for the probe.

The Curia climax is a `CIMMERIA_RECONSTRUCTION` staged in world 19. Text
monikers for the shield computer, wall safe, Curia console and Gardens ring,
plus effects 2336/2337, support intended objects, but give no verified
coordinate. World 88 is a flat stub and world 9 Tol-POI-06 is outside this
design. The CSV records Curia roles without inventing a council interior or
placing them. T-08e and T-09 remain dependent on a reviewed client-safe
server-authoritative phase design or O14-21 fallback preserving mission 621's
objective and two-player isolation.

## Individual ID claims and cross-zone boundary

The companion ID register claims only chain 9001 for the 568 pilot,
template 420 and spawn 19001 for **Tollana's own** Copplemann. The proposed
tag is `Tollana_Copplemann`. Template 48 / `CmdCenter_Copplemann` belongs to
Harset; its template-scoped dialog bindings 6503/6504 must not be inherited.
Copy only reviewed Copplemann visual/name/speaker identity (name ID 7035,
speaker 968) into the new template. Dialog and item IDs remain unallocated
until exact copy and item ownership are reviewed.

The collision scan used first-tuple `INSERT INTO` IDs in all matching seed
files of `origin/main` and the active Cellblock, Castle, Harset and Tollana
integration checkouts on 2026-10-14. It found no use of chain 9001, spawn
19001 or template 420. The shared table reserves 9000-9799 for Tollana, and
is amended for this one chain claim. These are server-owned namespaces;
the stock client mission catalog is relevant to *new mission IDs*, and no
new mission ID is allocated here. Recheck every claim immediately before
seeding; this is a point-in-time audit, not an atomic lock.

Harset HA-00b signs the existing recipient tags: `Harset_Hansen` (world 57,
spawn 300/template 212); `CmdCenter_Marsh` (68, 344/10);
`CmdCenter_Nerus` (68, 347/53); `CmdCenter_Baal` (68, 340/42); and
`CmdCenter_Mohkatan` (68, 343/54). Tollana owns both ends of missions 603,
611, 618, 621, 1626, 1632 and 1643, with Harset recipient chains in
9800-9999, and owns complete conditional 1364/1366. Objectives 2438 (618)
and 4734 (621) have blank recipient text; neither is bound to a guessed NPC.

## Pilot dispatch gate

### Mission 568 dialog and prop topology

Dialog set 545 contains source leads for mission 568, but its rows do not
bind an acceptance mission. Map rows 2656/2657 point to dialogs 2150/2151;
2151 speaks of traveling to Tollana, has `Accept` buttons on screens 108369
and 108373, and uses unnamed speaker 959. Its `accepts_mission_id` is NULL.
Map row 4959 points to dialog 2153 with the story-active indicator;
speaker 1100 is `Captain Copplemann`, and the final screen names the Repair
Facility. Map row 6393 points to dialog 2154 with the turn-in indicator;
Copplemann discusses a food duplication unit. Both dialogs also have NULL
`accepts_mission_id`; their event sets 548/550 are named dialog VO, not a
proven mission-action binding. The relevant rows are in
`db/resources/Dialogs/Seed/{dialog_set_maps,dialogs,dialog_screens,dialog_screen_buttons}.sql`
at maps 2656/2657/4959/6393 and dialog screens 9674-9700.

Three mission-world-object map rows in the same set reference dialogs 4211,
4213 and 4214. Dialog 4211 describes removing a core from a functioning
food duplication unit; the other two describe unusable devices
(`dialog_screens.sql:17194-17200`). A separate text moniker names a Food
Duplication Unit (`db/resources/Texts/Seed/texts.sql:54184`). These are
prop and interaction candidates, not evidence of a placed actor, an item ID
or which objective completes. Set 545 also contains Praxis dialogs 5682/5684,
and its map rows have empty alignment/faction arrays; copying the whole set
onto Tollana Copplemann would expose unrelated choices. Template 48 uses
speaker 968 (`Copplemann`), while dialogs 2153/2154 use speaker 1100
(`Captain Copplemann`); retain those source identities pending client portrait
verification rather than rewriting the screens by inference.

The T-03a contract must choose an exact offer/grant and turn-in binding,
step-gate each copied map row, and prove the client dialog and journal result.
No dialog set or object interaction is enabled by this evidence checkpoint.
The mission-568 seed has explicit `reward_naq=0` and `reward_xp=0`, sets
`award_xp=true`, and has no item reward group
(`db/resources/Missions/Seed/missions.sql:1459`). The original XP formula and
resulting amount remain unresolved
(`docs/analysis/issue-triage-2026-09-25/findings/content.md`). Any chosen fixed
XP amount or non-zero NAQ reward without further evidence is
`CIMMERIA_RECONSTRUCTION` and must be reviewed. Its repeat-on-fail flag also
requires grant and turn-in idempotency across a fail/reaccept cycle.

T-03a/mission 568 remains `BlockedEvidence` until a reviewed child contract
seals: distinct Tollana Copplemann placement and neutral faction response;
Repair Facility actor/fragment positions and a navigable route; acceptance
dialog and exact copy; authoritative fragment item grant/ownership/consume
mapping; both enabled tasks 2939 and 5389 under currently **disabled,
optional** objective 2312, with an explicit objective enablement and
requiredness decision; the required sibling objective 2311 on step 1965;
modest reward;
enabled-flag policy; replay and live-DB tests that fail on revert; relog and
two-player isolation; SigNoz chain/step/objective/refusal events; and native
client journal/dialog/interaction UAT. Item IDs 2343/2742 are *name
candidates*, not proven task mapping. Mission 569 and 570 need separate
actor, combat, item and dialog reviews before their own coder packets.

The current content `remove_item` action issues an asynchronous base request.
The base can return without removing anything if that character lacks the
item, while the chain continues; the DB condition loader cannot author the
apparent `HasItem` check
(`crates/cell-content/src/cell/content/executor/inventory.rs:89`,
`crates/base-methods/src/base/world_entry/methods/inventory/core/remove_by_type.rs:143`).
The DB conversion arms are in
`crates/content-engine/src/loader/condition.rs` and omit `HasItem`.
A simple `remove_item` then `advance_step` or `complete_objective` chain
would let a missing item advance the mission. Native `ConsumeItemForUse`
acknowledges a committed inventory removal, but its acknowledgment can be
lost and mission persistence is a separate asynchronous write. Reusing it
unchanged could consume an item without durable progress
(`crates/base-methods/src/base/world_entry/methods/inventory/core/consume_for_use.rs:18`,
`crates/cell-content/src/cell/missions/persist.rs:92`). Before dispatch,
the engine/persistence owners must seal a new combined server-owned turn-in
operation: validate actor ownership, exact item type/count/container, active
mission and expected step; lock and mutate inventory plus mission state in
one database transaction; and durably deduplicate retry before applying
client frames. The cell tracker must reconcile so a stale `MissionUpdate`
cannot overwrite the committed row. Grant actions must be step-gated and
persisted. An item-use trigger must not create a second implicit consume
path. Tests must reject missing/wrong-character items, concurrent duplicates
and wrong step, roll back both mutations on write failure, and cover a lost
acknowledgment, relog and two-player isolation.
The runtime `complete_objective` operation records objective progress, not
individual `mission_tasks` rows. The task seed therefore does not prove two
separately tracked fragment completions. Required objective 2311 could permit
the step to advance while optional 2312 is incomplete; the child contract
must resolve that graph explicitly before enabling either objective.

T-12 authored stories additionally wait for Cellblock's first new-mission
catalog client proof under O14-10. No seed or replay test counts as client
proof. T-01 placement remains `BlockedEvidence`; mission-client UAT is
pending. The bounded map survey above does not satisfy either gate.
