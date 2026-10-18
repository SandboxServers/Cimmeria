# Cellblock continuation ledger

> Opened 2026-10-14. Static baseline: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
> **Type**: Reference. **Audience**: Engineers and campaign coordinators.
> **Last updated**: 2026-10-17. **Companions**: [Wave resume](RESUME.md), [Work packets](work-packets.md), [W1b coder packets](w1b-packets.md), [W2 packets](w2-packets.md), [W3 packets](w3-packets.md), [W3 frozen literals](w3-frozen.md), [Owner lab checklist](owner-lab-checklist.md), [CB-N00 worknote](worknotes/cb-n00.md), [CB-N01 handoff](presentation-handoff.md), [CB-N05 proposed seal](worknotes/cb-n05-seal.md), [W4 packets](w4-packets.md), [W5 packets (CB-N06)](w5-packets.md), [W5 frozen literals](w5-frozen.md).
> CB-N00 is **Integrated** for the evidence/doc gate only. Fresh route acceptance is **UATPending**.

This is the live ledger for the one-time escape in world 12. The
[packet contracts](../four-zone-planning/cellblock-work-packets.md) and
[execution contract](../four-zone-planning/execution-contract.md) govern dispatch.
A completed source audit does not make a runtime packet Ready.

## Evidence and decisions

- [Source and route matrix](evidence-matrix.md): mission dispositions, historical hashes,
  current chain anchors and evidence limits.
- [Route baseline](route-baseline.md): prior observations, current faults, ownership,
  development population and unverified safe pockets.
- [Presentation handoff](presentation-handoff.md): asset, ring, class-start,
  autoplay and lab-reliability ownership with evidence and client row boundaries.
- [Decision log](decision-log.md): settled policy and outstanding design gates.
- [Wave resume](RESUME.md): reviewed commits, checks, next action and blockers.
- [CB-N00 worknote](worknotes/cb-n00.md): checks, scope and measurement handoff.
- [CB-N01 worknote](worknotes/cb-n01.md): presentation handoff checks and limits.
- [CB-N05 proposed seal](worknotes/cb-n05-seal.md): exact first authored story, copy, IDs, action order and review/UAT gates.

## Packet status

| Packet | Status | Dependencies and next seal |
|---|---|---|
| CB-N00 | Integrated | Evidence/doc gate passed two review rounds through `b59c4a124`; fresh route and lineage rows remain UNSEALED and client UATPending. No downstream runtime seal is granted. |
| CB-N01 | Integrated | Presentation and sibling ownership handoff passed independent review and focused status correction review; runtime and client rows remain UATPending. |
| CB-N02 | Writing | Family row. The 2026-10-15 fresh Human p3 run verified pistol grant, equip, reload and guard damage; #1667 did not reproduce and is not a child. CB-U02 stays UATPending. Children below. See [route baseline](route-baseline.md#2026-10-15-native-livewire-retry-and-prisoner-relog-fault). |
| CB-N02s | Integrated | Relog-restore audit, [cb-n02s.md](worknotes/cb-n02s.md); integrated at `e10fd1f0a`. Sealed gaps A, B, C. |
| CB-N02a | Integrated, UATPending | Chains 1026/1027 restore Prisoner 329's dialog on login; integrated at `e10fd1f0a`. CB-U03 client check pending. |
| CB-N02b | Integrated, UATPending | [w1b-packets.md](w1b-packets.md): chain 1028 restores the `329_CellDoorButton` bit at 638 step 2115 (strand). Integrated at `a8d516d26` (packet `f3ee0adce`). CB-U03 client check pending. |
| CB-N02c | Integrated, UATPending | [w1b-packets.md](w1b-packets.md): chain 1036 restores the Ambernol vial bit at 639 step 2145 (strand). Integrated at `654b2a3f2` (packet `1b7c05c28`). CB-U03 client check pending. |
| CB-N02d | Integrated, UATPending | [w1b-packets.md](w1b-packets.md): chain 1037 re-arms the retrieval drone at 639 step 2144 (degraded play). Integrated at `52bde32a6`. CB-U03 client check pending. |
| CB-N02s2 | Integrated | [work-packets.md](work-packets.md): bit-level check of the 622/640/641/680/687/688 restores and the relog state of sequences 1749 and 10001. Integrated at `0a2b5e803`. Verdict: no new gaps. Seals no children; W1c is not triggered by it. |
| CB-N03 | BlockedEvidence | Blocked on CB-U05. CB-N03s found no static fault (no 681/686 miss established; route baseline CB-B06). The lab row CB-U05 decides. |
| CB-N03s | Integrated | [work-packets.md](work-packets.md): `npc-ai-spawn-advisor` seal check. Integrated at `38bc2ad4f`. Verdict: no static fault, so CB-N03a and CB-N03b are not sealed; CB-U05 lab row pending. |
| CB-N03a | BlockedEvidence | CB-N03s sealed no seed fault, so W1c is not triggered. Exists only if CB-U05 shows a seed fault; then a new architect pass seals it. |
| CB-N03t | Integrated | Integrated at `090af3c5f`. [w4-packets.md](w4-packets.md#cb-n03t--cover-guard-seed-test-optional): static test that the other three cover guards (spawns 28, 32, 33) hold, defend and release when flanked on the real navmesh and cover seed. Strengthens CB-N03's evidence; CB-U05 still decides. |
| CB-N04 | Integrated, UATPending | Family row, cluster 1 only in W2 ([w2-packets.md](w2-packets.md)). O14-12 delegates cast and placement to the coordinator and domain reviewers. A second cluster is considered only after CB-U08 and CB-U09. |
| CB-N04s | Integrated | Integrated at `ce1381581` (from `de3528669`). `npc-ai-spawn-advisor` seal: cluster, cast, nav_inspect probes, guard and protected-area clearance, gate, copy, filled SQL. Worktree `cellblock-cb-n04s` on the W2 architect commit (child of `0fca3114f`). |
| CB-N04m | Integrated | Integrated at `86b5485bc` (from `8e29559e4`). `mission-systems-advisor` sign-off on the seal (gate, overlap, suppression, copy). After CB-N04s. |
| CB-N04a | Integrated, UATPending | Integrated at `139342746` (packet `908cef5a5` from `bb7605dba`, review fix `21f72c4e2` from `139342746`). CB-U08 UATPending (ask first). `packet-coder`: spawns 520-522, holder dialog 60160, screens 200160-200162, chains 11010-11012 (lowest needed) and the `cellblock_ambience` live-DB tests. After CB-N04m returns `approve` or `change`. CB-U08 UATPending afterwards. |
| CB-N05a | UATPending | Catalog infrastructure merged to `main` in #1734 and integrated here; the first real authored mission must pass journal, dialog, advance and relog before this gate closes. |
| CB-N05 | Integrated (W3-INT) | Itemless, zero-reward *The Remaining Pods* ([the seal](worknotes/cb-n05-seal.md)). Built before CB-U09/CB-U10 under D-CC13, split into CB-N05-1 to CB-N05-7 plus W3a-INT and W3-INT under D-CC14 ([w3-packets.md](w3-packets.md), literals in [w3-frozen.md](w3-frozen.md)). After W3-INT: Integrated, UATPending on owner-run CB-U09 (pacing, 10-minute beat, O14-14) and CB-U10 (native journal, dialog, advance, relog). |
| CB-U10 | UATPending (owner) | Owner-run lab rows for this wave: native journal, dialog 60150, first press, advance and relog in the real client ([owner-lab-checklist.md](owner-lab-checklist.md)). A blocking failure there blocks the campaign PR. |
| CB-N05b | BlockedDependency | Separate Castle evidence hand-in is held for Harset HA-1360's reviewed atomic item receipt primitive, Castle recipient ownership and a new item/reward/ID seal. It does not gate CB-N05. No W3 packet. |
| CB-N06 | Integrated, UATPending (CB-U12) | D-CC18: included as new Cimmeria content (`CIMMERIA_AUTHORING`). *Before the Patrol Returns* (mission 62060): after 638, both Op-CORE ambient prisoners offer one choice: hide them (second prisoner click) or jam the cell-door panel (`329_CellDoorButton`); either branch gives two Health Slappacks (item 2893) once and closes the other. Values in [w5-frozen.md](w5-frozen.md); packets in [w5-packets.md](w5-packets.md). After W5-INT: Integrated, UATPending (owner-run CB-U12). |
| CB-N06s | Integrated at `259127199` (from `bbdfd2fe9`) | `mission-systems-advisor` static seal of the W5 design (checks G1-G11). May run beside W3 and W4 from the W5 architect commit. `reject` closes CB-N06 `Done / omitted`. |
| CB-N06m | Integrated at `fedc51359` (from `6fc43280c`) | `items-systems-advisor` sign-off on the grant (item 2893 x2, full bag, first-press feedback, no farming). After CB-N06s `seal`. |
| CB-N06-1 to CB-N06-3 | Integrated at `03dc96faf`, `050f67f2f`, `de581adf9`; fixes `f893ea0c5`, `a83509799` | W5a data packets: catalog entry and mission seeds; dialog 60151; chains 11030-11041, rows and linters. After W4-INT and CB-N06m `approve`. |
| CB-N06-4 to CB-N06-7 | Integrated at `0fe86243b`, `d12b03b5b`, `8aacbde8a`, `04ea037c1` (from `2c3141cd3`, `c10160cce`, `21c21c8b9`, `2d3dd71c0`) | W5b proofs: resolve matrix, executor replay, cooked-sync transfer, services round trip (inventory and mission guard). After W5a-INT. |
| CB-N07 | Ready (Tier 0) | Family row. D-CC16: the default-safe tier is built from static evidence. The difficulty tier CB-N07b is omitted (D-CC19) ([w4-packets.md](w4-packets.md)). |
| CB-N07s | Integrated | Integrated at `556bb40c6`. `npc-ai-spawn-advisor` seal of the barracks alarm bark (Region12, mission 687 step 2355, chain 11020, screen 200170), assumptions A1-A4. May run beside W3. |
| CB-N07m | Integrated | Integrated at `4d759f735`. `mission-systems-advisor` sign-off on the CB-N07s seal. After CB-N07s `seal`. |
| CB-N07a | Integrated, UATPending | Integrated at `dde1a75ed`. `packet-coder`: chain 11020, screen 200170, `cellblock_barracks_response` live-DB tests. After W3-INT and CB-N07m `approve`/`change`. Owner-run CB-U11 UATPending after W4-INT. |
| CB-N07b | Done / omitted | D-CC19 (owner, 2026-10-17): no Tier 1 difficulty change. The barracks pack already pulls as one; the Tier 0 bark (CB-N07a) stays. |
| CB-N08 | BlockedDependency | Split into CB-N08a (UAT reconciliation), CB-N08b (close-out docs), CB-N08-CI (full local CI), CB-N08-R (campaign review) and ship ([w4-packets.md](w4-packets.md), D-CC17). After W5-INT and the owner's lab rows CB-U03, U05, U08-U12 (or O14-09). |

The fresh p3 client reached mission 638's Livewire door, but the native
minigame helper and screenshot call stalled. A later direct native Open reached
SmartFox and timed out; the relogged Prisoner 329 interaction reached the
matching server with no dialog response. These runs could not measure CB-U09
timing. On 2026-10-17 the owner stopped agent-driven lab work, so D-CC13 lets
CB-N05 be built first and leaves CB-U09 and CB-U10 to the owner
([owner-lab-checklist.md](owner-lab-checklist.md)). The Codex `lab-server` connection
pointed at a different server; direct access to the matching local server
provided the packet evidence. The client and lease were stopped and released.
See the [route baseline](route-baseline.md#2026-10-15-native-livewire-retry-and-prisoner-relog-fault).

The eleven packet families are scopes; an implementation child needs the complete
seal required by the packet contract. CB-N05 now has proposed exact copy and
individual ID claims under O14-04/O14-10, design review complete; W3 builds
them from the frozen literals in [w3-frozen.md](w3-frozen.md). No new placement, item or authored reward is
sealed. Chain block 11000–11499 remains a planning reservation outside the six
individually claimed CB-N05 chains. CB-N05a is the merged early standalone PR
exception. FZ-99 owns shared status docs.

## Orchestration decisions (D-CC)

Ported on 2026-10-16 from the adopting orchestrator's main-checkout overlay. These are orchestration-only and never renumber the zone's CB-D rows in the [decision log](decision-log.md).

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-CC1 | Who runs Cellblock after the zone session went idle. | Decided 2026-10-16: **B**. The WIP was checkpointed as `14658b3b1`; the adopting orchestrator runs the campaign. | Owner ruling. |
| D-CC2 | The main-checkout overlay stays uncommitted; this branch ledger is canonical. | Decided | One campaign branch, one ledger. The overlay was ported here in wave 1b. |
| D-CC3 | O14-08 standing lab approval covers the adopting orchestrator (CB-U03, CB-U09 and the other CB-U rows). | Decided 2026-10-16: yes. **Superseded 2026-10-17**: the owner runs every lab row; agents never drive the lab. | Owner ruling. See [owner-lab-checklist.md](owner-lab-checklist.md) and D-CC13. |
| D-CC4 | Seed-only `player_loaded` restores are tested at resolve level; an execute-level test only when a packet changes executor code. | Decided | Precedents 622/701/708; the executor paths have their own tests. |
| D-CC5 | Pilot re-run with measurement: CB-N02s about 78k tokens and 5 min, CB-N02a about 66k tokens and 22 min. | Done | CB-N00 had measured nothing. |
| D-CC6 | Review is split from fixing: read-only `packet-reviewer`, fixes by `packet-coder`, fresh re-review; no `packet-review-fixer`. Haiku agents stay under 100k total tokens (the 90k hook is a deliberate stop-gap). | Decided 2026-10-16 | Owner rulings. Applies from wave 1b. |
| D-CC7 | The drone relog restore (CB-N02d) sets aggression only, with no `generate_threat` and no sequence 10001 replay. | Decided | Chain 1009 precedent: the NPC engages through its normal hostile scan instead of being pulled across the map. |
| D-CC8 | Only the wave's integration packet edits the seed's chain-range header and `id-allocation.md`. | Decided | Lets the three W1b seed packets run in parallel and cherry-pick without conflicts. |
| D-CC9 | CB-N04's allowlist adds `castle_cellblock_chains.sql`, `dialogs.sql` and `dialog_screens.sql` (one seed-only holder dialog), and the test lives in `chain_replay_tests/cellblock_ambience.rs`. `crates/cell-chatter/` stays out. | Decided 2026-10-17 (architect, under O14-12) | Ambient chatter is on a schedule with no mission-state gate and repeats forever, so it cannot meet "reacts to escape state, no spam". The contract already asks for a chain test of the gate and suppression; a gated, fire-once `npc_bark` (precedent 1176-1178) is the existing verb that meets it. |
| D-CC10 | CB-N04a is built before the CB-U08 lab row. The static seal (nav_inspect probes, measured clearance from guards and protected anchors, mission sign-off) is the gate before dispatch. CB-U08 runs on the integrated branch and the campaign PR waits on it or on O14-09. | Decided 2026-10-17 (architect) | Two-client AoI and the camera check need the cast to exist; the contract gives that visual UAT to CB-N08. A failed CB-U08 opens a fix child. |
| D-CC11 | CB-N07: what death, difficulty and time impact is acceptable for a one-room encounter response. | **Moot** (D-CC19, 2026-10-17) | The owner omitted the difficulty tier, so no impact needs setting. |
| D-CC12 | CB-N04 cast rows set `is_stationary` = `true` and stay outside `Castle_Cellblock.Region1` (the debug hub floor). CB-N04a's placement test asserts the flag; its lane adds the `debug_hub_stationary` and `spawnlist_sequence` guards. | Decided 2026-10-17 (architect, W2 resume pass) | A mobile spawn off the patchy navmesh warns `spawn_off_mesh` on every instance and may take cover or path. A spawn inside Region1 breaks `live_db_debug_hub_stationary`. Precedent: spawn 10 and the hub rows 400-490. |
| D-CC13 | CB-N05 is built before CB-U09 and CB-U10. Both become owner-run UATPending rows ([owner-lab-checklist.md](owner-lab-checklist.md)); the campaign PR waits on them or on O14-09. A slow 62050 beat lets the coordinator drop 62050 or open a fix packet. A blank dialog, client crash, missing first-press response, lost progress or changed core Marsh briefing blocks the PR. Rollback: move dialog 60150 into the quarantine and delete the client's dialog cache pak. | Decided 2026-10-17 (architect, under the owner's lab-abandon directive; D-CC10 precedent) | No agent may wait on a lab result any more. CB-N05 is optional, itemless and zero-reward, so building it first risks only a removable beat; the static tests prove the catalog, the dialog bytes, the chains and the persistence. |
| D-CC14 | CB-N05's allowlist adds `overrides_cellblock.rs` (with `mod.rs` growing by at most 6 lines), `mission_additions_cellblock.rs` (two edits in `mission_additions.rs`), `resync_cellblock.rs` (instead of growing `resync.rs`), a `mission_62050/` test module and rig-stamped world rows. The packet is split into CB-N05-1 to CB-N05-7 with W3a-INT and W3-INT. | Decided 2026-10-17 (architect) | Keeps every touched file under the 700-line hard cap and every packet near 40k tokens; the frozen literals let three packets run in parallel. |
| D-CC15 | CB-N06: does it exist, and if so which quest and which prop. | **Decided** by D-CC18 (2026-10-17) | The owner answered before CB-U09/U10 and delegated the quest and prop. |
| D-CC16 | CB-N07 is split by impact. Tier 0 (CB-N07a): one gated, fire-once NID Guard alarm bark in the barracks (chain 11020, screen 200170 under holder 60160), with no spawn, aggression, threat, objective or time added, built now from static evidence and the CB-N07s/CB-N07m seal. Its allowlist adds `dialog_screens.sql` (one row) and `chain_replay_tests/cellblock_barracks_response.rs`. Tier 1 (CB-N07b) stays held on D-CC11. | Decided 2026-10-17 (architect, W4 pass) | D-CC11 asks about death, difficulty and time impact; a bark has none, so it needs no answer. The owner can still veto at CB-U11 (rollback: `enabled = false`). The barracks pack already pulls as one, so a scripted aggression adds nothing, and a reinforcement spawn would be an added unavoidable fight. |
| D-CC17 | [owner-lab-checklist.md](owner-lab-checklist.md) stands in for the contract's `uat-guide.md`. CB-N08's allowlist adds it and `docs/architecture/mission-pak-overrides.md` (one stale sentence after CB-N05). CB-N08 is split into CB-N08a, CB-N08b, CB-N08-CI and CB-N08-R to fit the 40k packet budget. | Decided 2026-10-17 (architect, W4 pass) | The owner runs the lab rows from that checklist; a second UAT guide would drift. |
| D-CC18 | CB-N06 is included as clearly labelled new Cimmeria content: provenance `CIMMERIA_AUTHORING` in every seed and doc, never presented as 2009 content (execution contract lines 18 and 83; O14-10 allows lore-safe premises). The owner delegated the quest and prop. The W5 architect chose *Before the Patrol Returns* (mission 62060) from static evidence: the two seeded Op-CORE ambient prisoners (spawns 520, 521) and the stock `329_CellDoorButton` panel, through existing dialog-set-maps 3862 and 7509; one exclusive choice; no punishment on abandon or decline; reward two Health Slappacks (item 2893), no XP. CB-N06s seals it before any build. Allowlist: [w5-packets.md](w5-packets.md) (`dialog_overrides/mod.rs` +5 lines, `interact_tag_linter.rs` +1 line). | Decided 2026-10-17 (owner; design by the W5 architect) | Owner ruling. The pre-chosen design lets the build packets be fully prescriptive; the seal verifies every claim, and CB-U12 lets the owner veto (rollback: disable chains 11030-11041, quarantine 60151, drop 62060 from `MISSION_ADDITIONS`). |
| D-CC19 | CB-N07b (Tier 1 difficulty change) is omitted: `Done / omitted`. The CB-N07a Tier 0 bark stays. | Decided 2026-10-17 (owner) | Owner ruling. Makes D-CC11 moot. |

**Corrections to [cb-n02s.md](worknotes/cb-n02s.md).** `route-baseline.md` is present on this branch, and its core route matches the worknote's assumption. The three objects the worknote left `unchecked` are restored on login: `Cellblock_WoodenCrate` by chain 1104 (687 step 2354), `Cellblock_TerminalX` by 1110 (688 step 2356), `Cellblock_ArmoryRingSwitch` by 1111 (688 step 80688). Gap A's setter is the 1014/1015 dialog-choice edge with no archetype split, so one restore chain is enough. Gap B's bit is cleared by `destroy_entity` in chain 1032, so a restore gated on step 2145 cannot outlive its window.

## Integration boundary

CB-N00 delivered its five allowlisted documents. Review fixes add the ledger link
in `docs/readme.md` and a sourced project-memory pointer; the four-zone index also
names this ledger directory. FZ-99 owns shared status documents.
No runtime, seed, shared planning, client or lab changes occurred.

Wave 0 integrates the static evidence only. `north_review` findings were fixed in
`b59c4a124`; fresh reviewer `north_rereview` found no actionable issues in the
full baseline-to-fixer diff. Downstream statuses stay blocked until their named
seals are complete; no runtime packet becomes Ready from this gate alone.
