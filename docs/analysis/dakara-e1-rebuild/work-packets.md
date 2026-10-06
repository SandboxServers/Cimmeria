# Dakara_E1 Rebuild Work Packets

> Type: how-to. Audience: Claude Code coordinator and packet workers.
> Written 2026-10-06 against `main` @ `ddd549797`. Companions: [ledger and decisions](README.md), [audit](audit.md), [Castle ledger](../castle-rebuild/work-packets.md), [Harset ledger](../harset-rebuild/work-packets.md), [testing playbook](../../../TESTING.md), [parity ledger protocol](../legacy-command-parity/work-packets.md#dispatch-rules).

## Dispatch Rules

This ledger reuses the dispatch, ownership, worknote, handoff and acceptance rules of the [legacy command parity ledger](../legacy-command-parity/work-packets.md#dispatch-rules). Initial state: documentation only; no implementation, build, test or client UAT has run. Worktrees live under `.claude/worktrees/dakara-<packet>`; branches are `dakara/<packet>-<slug>`; every compiling `cargo` call goes through the [build lane](../../../CLAUDE.md#build-lane-and-concurrency) and every live-DB test uses the worktree's own database. PRs open against `main`, one per packet, not stacked.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision** (needs an OD-DK answer from the [ledger](README.md#owner-decisions)), **BlockedDesign** (needs a design gate), **BlockedEvidence**, then **Writing**, **Review**, **Integrated**, **UATPending**, **Done**. None is Done.

Writers and advisors run as defined in `.claude/agents/`: `rust-gameserver-dev` writes Rust and seed packets; `mission-systems-advisor` advises every chain packet; the others are named per packet. `testing-validation-engineer` reviews each regression strategy. No packet worker launches the game client, the launcher or a lab tool; lab UAT is the owner's or the coordinator's lab session.

## Worker Input And Ownership

Common read-only inputs: the [audit](audit.md); [content-engine.md](../../content/content-engine.md) section 3 and the [vocabulary](../../content/content-engine-vocabulary.md); [content-chains.instructions.md](../../../.github/instructions/content-chains.instructions.md); [interaction-flags.md](../../content/interaction-flags.md); [gate-travel.md](../../gameplay/gate-travel.md); the [Harset placement method](../harset-rebuild/placements/METHOD.md); and the agent-memory notes `multi-chain-dispatch-semantics`, `dialog-chain-authoring-rules`, `interact-dialog-routing-traps`, `advance-step-vs-complete-objective` and `condition-column-layout` under `.claude/agent-memory/mission-systems-advisor/`.

The external archive is not an input. It is not in the repo and nothing in it needs importing; the client's cooked maps and PAKs are the evidence, read with the repo's tools.

Seed files, split from day one (four chain files on one theme):

| File | Owner packets |
|---|---|
| `db/resources/Content/Seed/dakara_e1_space_chains.sql` | DK-01, DK-04, DK-05 |
| `db/resources/Content/Seed/dakara_e1_arrival_chains.sql` (1570, 1645, 1646) | DK-10, DK-11, DK-12 |
| `db/resources/Content/Seed/dakara_e1_betrayal_chains.sql` (1647-1650) | DK-21, DK-13, DK-14, DK-15 |
| `db/resources/Content/Seed/dakara_e1_climax_chains.sql` (1651-1654) | DK-30 to DK-33 |
| `db/resources/Entities/Seed/entity_templates_dakara_e1.sql` | DK-03, DK-20 |
| `db/resources/Worlds/Seed/spawnlist_dakara_e1.sql` | DK-04, DK-05, DK-20 |
| `db/resources/Events/Seed/point_sets_dakara_e1.sql` (and its points) | DK-04, DK-12, DK-21 |

Each is `\ir`-included from `db/database.sql` beside its Debug Area counterpart. Existing rows that change (gate 25's arrival pin, respawner rows) are edited in their existing files.

Id blocks, free on `main` and on the two open Class Start branches as of 2026-10-06:

| Kind | Block | Split |
|---|---|---|
| Chains | 8001-8499 | 8001-8039 space, travel and notice; 8040-8079 mission 1570; 8080-8119 1645; 8120-8159 1646; 8160-8219 1647; 8220-8249 1648; 8250-8299 1649; 8300-8339 1650; 8340-8379 1651; 8380-8419 1652; 8420-8469 1653; 8470-8489 1654 |
| Entity templates | 440-479 | cast and props 440-459; hostiles and defenders 460-479 |
| Spawnlist | 600-699 | |
| Point sets | 2200-2279 | |
| Respawners | 611-619 | the rest of CS-02's 610-619 block |

Seed rules: edit `db/resources/` directly, no `db/scripts/` migration; region keys are byte-identical to `point_sets.name` and start `Dakara_E1.`; every row carries a comment naming its evidence label from the [ledger](README.md#evidence-labels); every spawn row and template sets `respawn_secs`; loot tables stay NULL; run `crates/content-engine/tests/it/interact_tag_linter.rs` after every seed change.

## Common Acceptance

Every chain packet ships a chain-replay test (TESTING.md type 6) under `crates/cell-content/src/cell/content/chain_replay_tests/` that (a) asserts the exact resolved action list for the matching state, (b) asserts no resolution for the adjacent wrong state (other archetype, wrong step, mission already completed), (c) asserts the `player_loaded` restore chain repaints every bind the packet sets, and (d) where an executor arm matters, runs the actions through `execute_actions` and asserts the emitted `CellToBaseMsg`. A test that passes with the packet's seed rows deleted is not a guard. Every coordinate ships a navmesh guard in the shape of `crates/cell-world/src/cell/harset_placement_tests.rs` and CS-02's `dakara_start_tests.rs`: the seeded row against the real `dakara_e1.nav`, with a path-reaches check to the gate plaza, not `is_point_valid` alone. Seed relationships get a live-DB guard (type 3, `require_db_or_skip!`, `live_db` in the name, no hard-coded seed ids beyond the packet's own block). Anything that puts a new message on the wire needs a byte-exact wire-format test (type 2). Client-visible behaviour is UAT-gated per the [milestones](README.md#validation-and-uat-gates), and its steps go into [unified-uat.md](../../guides/unified-uat.md) with this ledger's ids in the same PR.

## Foundation: Remove The Dead End

### DK-00

**Status:** Written 2026-10-06 (this plan; branch `dakara/da-00-plan`). **Scope title:** Audit, ledger and packets.

### DK-01

**Status:** Ready (authoring). UAT waits on PR #1273. **Scope title:** A new Free Jaffa can leave Dakara, come back, and is told where things stand. **Depends:** OD-DK02's recommended default (Omega Site); #1273 merged before UAT. **Advisor:** mission-systems-advisor, movement-teleport-advisor, server-authority-enforcer.
**Entries:** [executor/stargate.rs](../../../crates/cell-content/src/cell/content/executor/stargate.rs) (`grant_stargate_address`, idempotent); chain 1357's fourth action and `chain_replay_tests/mission_708/stargate.rs` (the precedent and its executed test); [stargates.sql](../../../db/resources/Worlds/Seed/stargates.sql) rows 25 and 5 and the `arrival_*` columns added by Harset H01; point set 1005; spawn 38 (DHD) and spawn 41 (Omega Site DHD); `send_system_mail` params in the [vocabulary](../../content/content-engine-vocabulary.md); the Debug Area gate's arrival pin as the pattern; H55's [worknote](../harset-rebuild/worknotes/H55.md) ("a `player_loaded`-triggered `grant_stargate_address` chain in that zone's seed").
**Scope:** (1) Chain 8001, scope `space` 61, trigger `player_loaded` key `Dakara_E1`, condition `archetype eq 7`, action `grant_stargate_address` with `target_id = 5`. Label `PROJECT_FINAL_RECONSTRUCTION`: the client's story hands Omega Site out in mission 1655. (2) Pin gate 25's arrival: `arrival_x/y/z/yaw` on the plaza in front of the gate, outside point set 1005, on the same navmesh component as respawner 610, facing away from the gate. First read how `gate_travel` places an arrival whose gate row lies inside its own volume and record the finding; pin regardless, as the Agnos and Debug Area rows do. (3) The arrival notice, once per character: chain 8002, same trigger and gate, one `send_system_mail`. Sender, subject and body are `NEW CONTENT`, three sentences at most: the command is not staffed yet, the DHD beside you dials Omega Site, Omega's DHD brings you back. No cash, no item. `send_system_mail` is the one verb that carries free server-side text, and it answers every later firing with a cooldown-refusal chat line ("You can ask again in ..."), which a `player_loaded` chain would repeat at each login. So this part carries one small Rust change: an optional param on the mail action that keeps the persisted cooldown claim and sends no line on refusal, with a long `cooldown_secs`. If the coordinator wants a seed-only first PR, parts 1 and 2 ship as DK-01a and the notice as DK-01b; parts 1 and 2 alone already let the character leave and return. (4) No mission, no dialog bind, no NPC.
**Acceptance:** loader and base unit tests for the quiet-refusal param, and a live-DB test that a second firing inside the window writes no mail and sends no line. Replay test: archetype 7 on `Dakara_E1` resolves exactly the two chains' actions; archetype 1 and archetype 8 resolve nothing; `player_loaded` on another world resolves nothing. Executed test: the grant emits `updateStargateAddress(5, 1, 0)` and `CellToBaseMsg::GrantStargateAddress`, and a second run emits nothing. Live-DB guard, by relationship: every gate a `player_loaded` chain grants exists, is not a dial hub, is on a world the server can enter, and that world has a DHD-flagged spawn (the return path); gate 25's arrival pin is outside every stargate-flagged point set on its world. Navmesh guard for the pin. **UAT (lab, M1):** the checks in the [ledger's M1 row](README.md#validation-and-uat-gates). **Exclude:** the SGC address (DK-30); any mission; respawner rows (CS-02 owns 610).

### DK-02

**Status:** Complete (2026-10-06; PR pending). Ledger: [placements/](placements/README.md); worknote: [DK-02](worknotes/DK-02.md). **Scope title:** Zone evidence pack and placement ledger. **Depends:** a client copy on the build host. **Advisor:** npc-ai-spawn-advisor, movement-teleport-advisor, game-archaeology-specialist.
**Entries:** [navmesh-extractor README](../../../crates/navmesh-extractor/README.md) (`extract_map`, `nav_inspect`, `obj_slab`, `archetype_census`); `crates/upk` (`extract_actors`, `extract_kismet`); the [Harset method](../harset-rebuild/placements/METHOD.md) and its ledger columns; the audit's tent groups, ring transporters and Ha'tak positions; `data/spaces/dakara_e1.nav`, `dakara_e1_storyrm.nav`.
**Scope:** read-only extraction, then one document set under `docs/analysis/dakara-e1-rebuild/placements/`. (1) Prefab landmark census for world 61 with BigWorld positions: every `JF-`, `HB-`, `GA-` building, gate and wall prefab, the merchant tent, anything named for a repository, a gate, a med tent or the Superweapon. (2) For each thing the story needs, a candidate row with evidence class, confidence, floor Y, navmesh component and whether a path reaches the gate plaza: the command tent and its flap (Rak'nor's line in dialog 6110 puts it "just to the east of the Stargate, next to the healing tent"), the Med Tent beside it, Moh'katan's tent and its flap, the Naquadah Repository and Loth'ta's camp, the supply stores, three SG-18 search sites, five drop locations, the Eastern tents, the Western and Eastern Gates, the two Ha'tak plazas (the ring transporters are exact), the Superweapon courtyard, Rak'nor at the plaza. (3) World 62: the interior's standable floor, the exit flap, Bra'tac's and Moh'katan's spots, the terminal. (4) Whether the client's `Cache/covernodes_*.pak` hold Dakara_E1 sets, and the true cover-node count. (5) A `## No idea` list for everything without usable evidence. Tie "Eastern" and "Western" to a map axis only if the map says so (a named prefab, the minimap art); otherwise say it is unknown.
**Acceptance:** the ledger exists with one row per coordinate and no invented row; the census is reproducible from the commands in the worknote. No seed, code or test changes. **UAT:** none. **Exclude:** seeding anything.

### DK-03

**Status:** Ready. **Scope title:** Cast templates and interactable props. **Depends:** none (templates carry no position). **Advisor:** npc-ai-spawn-advisor, aoi-witness-broadcast for prop visibility.
**Entries:** templates 54 and 59; `texts.sql` name ids 26717-26737; speakers 781 and 2335 (Loth'ta); the Cellblock and Castle prop templates (interactable corpse, console, crate) as patterns; Harset D-H16 and D-H17.
**Scope:** new templates in 440-459: Loth'ta (speaker as used by dialogs 5808, 5812, 5813, 5829), Rak'nor (`name_id` 26720; speaker 2956, inferred from dialogs 6110/6111), Jaffa Captain (speaker 2959, inferred from dialogs 6106/6107), Ba'al hologram (`name_id` 26719), a non-hostile Free Jaffa Warrior (`name_id` 26723); props: tent flap (four name ids), Drop Location (26727), three SG-18 remains (26724-26726), Command Terminal, Moh'katan's Terminal, Ring Control, Power Supply, Vocuum. Friendly faction, stationary, `respawn_secs` set, no ability set beyond a default, no loot. Where a name string's text is empty in the client (Bra'tac, Moh'katan, Loth'ta, Jaffa Captain), use `display_name` and say so in the row comment. Do not edit templates 54 or 59 (shared with Harset).
**Acceptance:** live-DB guard that each new template's `name_id` resolves to a non-empty text or has a `display_name`, and that no Dakara template is hostile to archetype 7; loader unit test if a new column value is used. **UAT:** with DK-05. **Exclude:** spawn rows, hostiles (DK-20).

## Population And Travel

### DK-04

**Status:** BlockedDependency (DK-02, DK-03), BlockedDecision (OD-DK03, OD-DK04). **Scope title:** Named areas, tent travel and the Med Tent. **Advisor:** movement-teleport-advisor, mission-systems-advisor.
**Entries:** the six discovery strings (`texts.sql:36658-36672`, `:36692`); `DN_Respawner_DakaraE1_MedTent`; chains 6006 and 6007 in [harset_space_chains.sql](../../../db/resources/Content/Seed/harset_space_chains.sql) (the `cross_world_teleport` door pair); respawner 25 (world 62); `entities/spaces.xml:19`.
**Scope:** point sets for the named areas DK-02 could place; tent flap spawns on world 61 and the exit flap on world 62; `interact_tag` chains that `cross_world_teleport` to `Dakara_E1_StoryRm` and back to a pinned exterior point. The return point depends on which flap the player came in by, and a content counter does not survive the world change: use one shared exit point if DK-02 finds the two tents adjacent, otherwise split the exit chain on mission state, and record the choice; respawner 611 `Med Tent` on world 61; check respawner 25 against the interior found in DK-02.
**Acceptance:** replay tests for each flap both ways and for the wrong-world non-match (`OnRegionEnter` and tags do not filter by world); navmesh guards for every point; live-DB guard that no respawner on worlds 61 or 62 is at the origin. **UAT (M2):** both flaps both ways, with a relog inside the tent. **Exclude:** who stands in the tent (DK-05, DK-15).

### DK-05

**Status:** BlockedDependency (DK-02, DK-03). **Scope title:** Static population and relog restores. **Advisor:** npc-ai-spawn-advisor, mission-systems-advisor.
**Entries:** the placement ledger; dialog sets 1656 and 1832-1842; `add_dialog_set` and the NULL-dialog flag-only bind (CA02).
**Scope:** spawn rows for Bra'tac and Moh'katan (command tent), Loth'ta (Repository camp), Rak'nor (gate plaza; the client's greeter, dialogs 6110/6111), all stationary with headings derived from the approach direction; no bind by default (binds belong to the mission packets); a world-61 and a world-62 `player_loaded` restore skeleton the mission packets extend.
**Acceptance:** navmesh guards; live-DB guard that every Dakara story spawn has a tag starting `Dakara_E1_`, a template in the campaign's block or 54/59, and `respawn_secs`. **UAT (M2):** each NPC is where the ledger says, named, and still there after a relog. **Exclude:** hostiles, vendors, ambient crowds.

### DK-06

**Status:** Ready for its evidence step; the change is BlockedDesign on that step. **Scope title:** Radio dialogs. **Advisor:** bigworld-engine-advisor, mission-systems-advisor, game-archaeology-specialist.
**Entries:** [executor/dialog](../../../crates/cell-content/src/cell/content/executor/dialog) speaker resolution; dialogs 5815/5816, 5824, 5825, 5833; `docs/reverse-engineering/findings/` on `onDialogDisplay`; the `npc_bark` action.
**Scope:** first, evidence: what the client does with `onDialogDisplay` for an NPC-speaker dialog when the speaker entity is the player or absent, from the client handler, not from the legacy server. Then the smallest change that lets a chain show such a dialog with no entity in range, or the finding that it cannot be done server-side, in which case the radio steps fall back to `npc_bark` lines of the same screens. How the radio is raised (on step activation after a delay, or by an interaction) is recorded as `RECONSTRUCTION` either way.
**Acceptance:** a findings note with client addresses; if code changes, a wire-format test for the display and an executor unit test for the no-entity path. **UAT (M3):** the Hammond radio in 1570. **Exclude:** new dialog rows.

## Missions: Arrival Story

### DK-10

**Status:** BlockedDependency (DK-04, DK-05), BlockedDecision (OD-DK01). **Scope title:** Mission 1570 on Dakara, steps 4906 to 4903. **Advisor:** mission-systems-advisor, items-systems-advisor.
**Entries:** steps 4906, 4904, 4902, 4901, 4905, 4903 and their objectives (5936 is the optional "Speak with Rak'nor"); dialog set 1656 rows 6726-6733, 6889, 6890, 7153-7155; dialogs 5810-5817, 6110 (Rak'nor to a Jaffa), 6111 (Rak'nor to a Human), 6224-6226; item 6778; the three remains props.
**Scope:** entry for a Free Jaffa (option B of OD-DK01): on `player_loaded Dakara_E1`, `archetype eq 7`, 1570 `not_active` and 1645 `not_active`, accept 1570 and advance past steps 4641 and 4642, so the first open step is 4906. Entry for anyone else is the route campaign's step 4642 handing over on arrival; this packet authors the Dakara steps so that either entry works and splits every sibling chain on `archetype` (7 and 8 take the Jaffa dialogs 5811, 5813; the rest take 5810, 5812). Rak'nor at the plaza (6110/6111) completes the optional objective and points to the tent; Bra'tac in the tent (5810/5811) advances to 4904; Loth'ta (5812/5813) advances to 4902; the search sites and remains grant Dog Tags and advance through 4901; the radio step 4905 uses DK-06; Bra'tac (5817) completes. Restore chains for every step.
**Acceptance:** `mission_1570.rs` replay tests: both archetype branches at each step, the Free Jaffa entry exactly once, no entry for a Human, no re-accept after completion, every restore. Live-DB test that the entry leaves steps 4641 and 4642 completed and 4906 active after a relog. **UAT (M2).** **Exclude:** steps 4641 and 4642 themselves; hostiles at the search sites (OD-DK05).

### DK-11

**Status:** BlockedDependency (DK-05, DK-10). **Scope title:** Mission 1645 Withdrawal Orders. **Advisor:** mission-systems-advisor, items-systems-advisor.
**Entries:** steps 5168, 4907; objectives 5938, 5633, 5634 (optional, four tasks); dialog set 1832; dialogs 5980, 5807, 5808; item 6779.
**Scope:** offer bound to Moh'katan once 1570 is completed (`RECONSTRUCTION`: no offer rule survives); 5807 accept grants 6779 and advances to 4907; Loth'ta (5808) takes the item and completes. The optional path objective is completed by entering the Repository area, or left open if DK-02 found no route evidence.
**Acceptance:** `mission_1645.rs` replay tests; item grant and removal asserted through the executor. **UAT (M2).**

### DK-12

**Status:** BlockedDependency (DK-11). **Scope title:** Mission 1646 Moh'katan's Scouts. **Advisor:** mission-systems-advisor.
**Entries:** steps 4908, 4909; objectives 5635, 5939-5943; dialog set 1833 (five world-object rows on dialog 5821); item 6780.
**Scope:** five Drop Location props, each an `interact_tag` chain that shows 5821, completes its own objective and clears its own glow; the fifth leaves the step with `advance_step 4909`, never by letting `complete_objective` close the last required objective, which would complete the whole mission; Moh'katan (5820) completes. One tag per prop, and each prop chain is gated on its own objective so a second click does nothing.
**Acceptance:** `mission_1646.rs`: any order of the five, no double count on a second click, restore after relog with a partial set. **UAT (M2):** the owner corrects all M2 positions in one pass afterwards.

## Missions: Betrayal

### GDK1

**Status:** BlockedDecision (OD-DK05); opens after the M2 playtest. **Scope title:** Encounter design gate. **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.
Collect what exists (template 35 and its kit, the `spawn_set` and `spawn_entity` actions, `entity_dead_tag` counters, the cover data found in DK-02, what a level-1 to level-3 Shol'va can survive with the v6 start kit), then propose for 1647, 1652 and 1653: hostile and defender templates, counts, wave triggers, level band, win and fail conditions. Output is a decision record in the ledger and child scopes for DK-20, DK-21 and DK-32. Nothing is seeded by the gate.

### DK-20

**Status:** BlockedDesign (GDK1). **Scope title:** Hostile roster, cover sets and encounter kit. **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.
**Scope:** templates 460-479 as GDK1 decides, spawn sets and points, world-61 cover sets if DK-02 found the data, all `RECONSTRUCTION`. **Acceptance:** live-DB guards on the roster (levels in band, `respawn_secs`, faction hostile to archetype 7 and friendly defenders not), navmesh guards, the existing aggro and leash suites on a Dakara fixture. **UAT (M3).**

### DK-21

**Status:** BlockedDesign (GDK1). **Scope title:** Mission 1647 Enemy at the Gates. **Depends:** DK-20, DK-06, DK-12. **Entries:** steps 4911, 4912, 4910, 4916, 4915, 4913, 4917, 4914; dialog set 1834; dialogs 5982, 5822-5827, 6106, 6107. **Scope and acceptance:** written by GDK1.

### DK-13

**Status:** BlockedDependency (DK-21, by story order). **Scope title:** Mission 1648 Loth'ta's Withdrawal. **Entries:** steps 4919, 4918; dialog set 1835; dialogs 5983, 5828-5830; item 6781. **Scope:** Bra'tac offers, Loth'ta refuses and hands over 6781, Bra'tac and Moh'katan (5830) complete. **Acceptance:** `mission_1648.rs`. **UAT (M3).**

### DK-14

**Status:** BlockedDependency (DK-13, DK-06). **Scope title:** Mission 1649 Moh'katan. **Advisor:** mission-systems-advisor, minigame-systems-advisor, items-systems-advisor.
**Entries:** steps 4923, 4921, 4922, 4920; objectives 5638-5640; dialog set 1836; dialogs 5984, 5831-5838; items 6780, 6782, 6783; the Livewire launcher and victory pairs (chains 1016/1017, 1060/1061).
**Scope:** Bra'tac (5832); Teal'c by radio (5833, DK-06); the Command Terminal as a Livewire hack whose victory chain shows 5834 and grants 6782; the drop sites again (5835 at one, 5836 at the others; which one still holds a message is `RECONSTRUCTION`); decoding as an `item_use` chain on 6780 that shows 5837 and grants 6783; Bra'tac (5838) completes.
**Acceptance:** `mission_1649.rs`, with the three objectives in any order and the launcher not resolving at the wrong step. **UAT (M3).**

### DK-15

**Status:** BlockedDependency (DK-14), BlockedDecision (OD-DK04). **Scope title:** Mission 1650 Confront Moh'katan. **Advisor:** mission-systems-advisor, aoi-witness-broadcast.
**Entries:** steps 4927, 4924, 4926, 4925; dialog set 1837; dialogs 5839-5842, 5985; item 6787; Moh'katan's Terminal.
**Scope:** the confrontation (5840: Ba'al, Bra'tac, Moh'katan) inside the player's own tent instance with mission-scoped actors; search the terminal for 6787; Bra'tac (5842) completes. What happens to Moh'katan afterwards is not in the data: she is absent from the player's later tent scenes and unchanged for everyone else.
**Acceptance:** `mission_1650.rs`; a test that the scene's actors are spawned by tag for the acting player's instance and never touch shared spawns. **UAT (M3):** two players at different points of the arc.

## Missions: Climax And Exit

### DK-30

**Status:** BlockedDependency (DK-15, B-DK4). **Scope title:** Mission 1651 Renewed Attack. **Entries:** steps 4932, 4930, 4928, 4931, 4929, 5171; dialog set 1838; dialogs 5843-5845, 5847; item 6784. **Scope:** the Dakara steps (4932, 5171) and the SGC address grant (gate 27 or 23, whichever world the route campaign chooses) here; the SGC steps with the route campaign, which owns a way to dial out of the SGC. Show the Ha'taks from 4932 on. **Acceptance:** `mission_1651.rs` for the Dakara steps; a round-trip test that 1651 state survives two world changes. **UAT (M4).**

### DK-31

**Status:** BlockedDependency (DK-30). **Scope title:** Mission 1652 Fireball. **Advisor:** mission-systems-advisor, movement-teleport-advisor.
**Entries:** steps 4933, 4934; dialog set 1839; dialogs 5986, 5846, 6108, 6109; items 6784, 6785; event sets 1194 and 1195 (sequences 2390-2393); the two ring transporter positions in the audit; `play_sequence`.
**Scope:** a Ring Control prop at each transporter; overloading a generator (how is `RECONSTRUCTION`: an item use or the control's dialog); the matching Ha'tak explosion sequence for the acting player; confirm from the executor whether `play_sequence` takes a sequence id or an event-set id before seeding. Guards at the plazas only if GDK1 says so.
**Acceptance:** `mission_1652.rs`; an executed test that the right sequence goes to the acting player only. **UAT (M4).**

### DK-32

**Status:** BlockedDesign (GDK1). **Scope title:** Mission 1653 Superweapon. **Depends:** DK-31, DK-20, DK-06. **Entries:** steps 4936, 4938, 4937, 4935; objectives 5641-5643; dialog set 1840; dialogs 5848, 5849. **Scope and acceptance:** written by GDK1. The courtyard is on world 61 (its discovery string is a Dakara_E1 string); world 65 is not used.

### DK-33

**Status:** BlockedDependency (DK-32, B-DK4). **Scope title:** Mission 1654 exit boundary. **Entries:** step 4939; dialog set 1841; dialogs 5850-5853, 5987. **Scope:** Bra'tac's offer on Dakara and the gate-travel objective; Hammond's half belongs to the route campaign. **Acceptance:** `mission_1654.rs` for the Dakara half. **UAT (M5).**

## Design Gates Not Opened

- **GDK2, rewards.** Inherits the Cellblock's GC3. No packet.
- **GDK3, vendors and ambient population.** Inherits Harset's GH2. The merchant tent stays a prop.

## Explicit Non-Goals (record as decisions, do not open packets)

- Worlds 24, 30, 63, 64 and 65 and the fifty Dakara E2 and E3 missions.
- The SGC and Omega Site halves of 1570, 1651, 1654 and 1655.
- Importing anything from the external archive.
- New dialog rows, cooked-data overrides or client patches.
- Moving world 61 to navmesh `enforce`.
- Changing the Free Jaffa start world or level.

## Scheduling And Closeout

Lanes that can run at once from day one: DK-01 (seed), DK-02 (evidence, read-only), DK-03 (seed, different files), DK-06's evidence step. DK-04 and DK-05 follow DK-02 and share `spawnlist_dakara_e1.sql`, so one worker takes both or they run in sequence. Mission packets run in story order because each offer is gated on the one before; DK-10, DK-11 and DK-12 are one worker's run. Pause for the owner at each milestone and record skipped scenarios.

### DK-99

**Status:** Planned. **Scope title:** Close-out. Update `docs/gap-analysis.md` with its area file and `docs/project-status.md` once, correct the Dakara_E1 and StoryRm sections of [zone-audit.md](../../content/zone-audit.md) in full, add the mission rows to [mission-chains.md](../../content/mission-chains.md), write the final PASS / BLOCKED matrix into the ledger, and retire the campaign's worktrees.
