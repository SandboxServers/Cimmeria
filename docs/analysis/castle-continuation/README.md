# Castle continuation

> **Type**: Reference. **Audience**: Engineers and campaign coordinators.
> **Last updated**: 2026-10-18.
> **Companions**: [Wave resume](RESUME.md), [CS-00 worknote](worknotes/CS-00.md), [CS-07 closeout worknote](worknotes/CS-07.md).

Evidence baseline: `origin/main` and CS-00 base `d3efb7d41aad0b6008397f112d224ce7430d7acf`, inspected 2026-10-14. This is the live continuation ledger; the September rebuild ledger remains historical. CS-00 produces a reviewable evidence gate, **not a signed implementation freeze**.

## Contract and evidence

- [Packet gate contract](work-packets.md)
- [Mission contract table](evidence/castle-contract.csv)
- [Evidence reconciliation and open gates](evidence/reconciliation.md)
- [Cooked Castle dialog map receipt](evidence/cooked-interaction-maps.md) — packaged 1571/1572 dialog maps, with no actor binding.
- [Exact seed excerpts](evidence/seed-excerpts.md)
- [ID census](evidence/id-census.csv) and [branch snapshot](evidence/branch-snapshot.csv)
- [Wave resume](RESUME.md)
- [CS-00 worknote](worknotes/CS-00.md)
- [CS-08a dated bridge seal](worknotes/CS-08a.md) — owner-approved reconstruction and chain 1366 allocation; Castle phase independently reviewed; playable receipt depends on Harset/engine HA-1360a/b.
- [CS-02a guarded callback packet](worknotes/CS-02a.md) — opt-in condition resolver, held from runtime wiring until result receipt and mission gates close.
- [CS-02b0 Base send packet](worknotes/CS-02b0.md) — independently reviewed session-bound delivery implementation and checks; integrated.
- [CS-02b-prep message-reference packet](worknotes/CS-02b-prep.md) — documentation-only hard-cap prerequisite for the internal result enum; extraction independently reviewed and integrated.
- [CS-02b1 result receipt packet](worknotes/CS-02b1.md) — one-use Base receipt implementation, independently reviewed and integrated; Castle callbacks 1372/1375 were enabled later by CS-02L/R2 (b2-lite).
- [CS-02b2 Cell launch identity draft](worknotes/CS-02b2.md) — proposed b2-prep extraction, b2a presence/travel lifecycle and b2b minigame-launch split; typed-message, ordering and source-cap decisions remain open.
- [CS-01 bounded client probe and screenshot](uat/CS-01.md) — Castle load and Ogilvie rendering observed; route and mission UAT remain pending.
- [2026-10-14 CS-00 specialist findings](#cs-00-resumption-source-and-domain-findings) and [CS-01 bounded client probe](#cs-01-bounded-client-probe-2026-10-14).
- Governing [four-zone execution contract](../four-zone-planning/execution-contract.md), [Castle packet menu](../four-zone-planning/castle-work-packets.md), and [shared ID table](../four-zone-planning/id-allocation.md).

## Decisions

| ID | Status | Decision and reason |
|---|---|---|
| D-CS01 | Adopted owner direction | 1520 serves human/Op-CORE and Goa'uld; 1521 serves Jaffa. Source: four-zone README, Owner direction, 2026-10-13. Exact archetype predicates still require sealing. |
| D-CS02 | Four IDs allocated; later claims pending | O14-22 delegates individual Castle restoration-chain claims in 1367–1400 after a current stock/seed/shared-table/active-branch audit. Chains 1367–1370 are claimed for reviewed 1520/1521 entry; 1371–1373 are claimed by CS-00Ra (mission 1520), 1374–1377 by CS-00Rb (mission 1521); 1378–1400 remain free. Chain 1366 remains exclusively CS-08a under O14-18. |
| D-CS03 | BlockedEvidence | 1521 item 2718 versus 4652 versus pouch state remains unresolved. O14-03 permits a bounded, documented reconstruction after source and domain review; identical item names/descriptions do not identify a historical grant. |
| D-CS04 | BlockedEvidence | Post-704 acceptance, dispenser/tank event bindings, minigame types and radio trigger are not recovered. O14-03 permits a bounded, documented reconstruction after source and domain review; exact mechanics are not sealed. |
| D-CS05 | BlockedDependency | 1664 ownership is decided; mission review recommends a guarded Castle `mission_completed 708` grant before transfer, with failed-travel retry coverage. This is a candidate only; HA-00b receiver, Kaerlia placement and client metadata remain unconfirmed. |
| D-CS06 | Adopted boundary | 1376 remains held: B00 restart moniker plus disabled step 4098 does not prove compatible chronology. |
| D-CS07 | Adopted boundary | CS-08 owns Castle legs of 1360 and conditional 567; Harset owns recipient legs. Existing chain 1360 completes mission 708 and must not be mistaken for mission 1360. |
| D-CS08 | Approved reconstruction, 2026-10-14 | Owner approved Castle world 8 `stargate_crossed` key `Harset`, active 1360/4037 → 4038; retain 3730 until Marsh receives it. Marsh hand-in must require the letter. Exact Castle contract is [CS-08a](worknotes/CS-08a.md); original trigger remains unrecovered. |
| D-CS09 | Allocated, owner-approved 2026-10-14 | Chain 1366 reserved exclusively for CS-08a after main/four-zone branch/worktree collision recheck; shared table amended. O14-22 delegates later individual claims in 1367–1400 only after their own collision audit. |
| D-CS10 | Standing lab authorization; bounded probe completed | O14-08 authorizes the Castle coordinator to lease/start/restart the client and point it at a reviewed build. The first CS-01 probe is [recorded](uat/CS-01.md); server alignment and natural routes remain pending. O14-09 permits exact `UATPending` rows only after documented unavailable-lab attempts; observed gameplay failures block merge. |
| D-CS11 | Delegated authored and service design | O14-10/11 permit lore-safe authored premises, copy and modest rewards without player-facing provenance labels; seed/ledger provenance remains `CIMMERIA_AUTHORING`. O14-12/13 permit conservative service placement and persisted repeatable limits after economy/authority review. First authored mission client proof still gates additional authored implementation. |
| D-CA1 | BlockedDecision | Adopt the idle Castle zone session's branch and ledger as-is (recommended: do not restart CS-00). The owner confirms the old Castle main session is not running. Two coordinators on one branch would collide (O14-02); restarting would discard 64 reviewed commits. |
| D-CA2 | Decided | The main-checkout overlay folder is uncommitted. The branch ledger stays canonical; CS-INT0 ports the findings F1-F9, D-CA1..D-CA10 and the isolation recipe into it. One campaign branch, one ledger (execution contract). |
| D-CA3 | Decided | Castle is a second concurrent campaign. It never writes `.claude/state/active-campaign`; after compaction the Castle orchestrator reads `docs/analysis/castle-continuation/RESUME.md` and the branch ledger itself. The Cellblock session owns the pointer. |
| D-CA4 | Decided | Test isolation: one worktree and DB per packet (`sgw_castle_cs_<id>`), integration DB `sgw_zone_castle`, and a separate Castle server on non-default ports against `sgw_castle_cs_01b` for lab work ([recipe](work-packets.md#testing-isolation)). Repo rule plus F6 and F7. |
| D-CA5 | Decided | Lab use is authorized for this campaign (O14-08). The lease is taken per instance and released after each run. Castle uses an instance Cellblock is not leasing, and checks `lab status` first. |
| D-CA6 | Decided | Review rule: read-only `packet-reviewer`, fixes by a fresh `packet-coder` or domain agent, then a fresh re-review, at most 3 rounds (matches Cellblock D-CC6). |
| D-CA7 | Decided (after D-CA1) | Delete the leftover branches `castle/cs-00-contract` and `castle/cs-08a1-letter-bridge` through ops after D-CA1. Keep `castle/ledger-session-record` (historical). |
| D-CA8 | Decided | The CS-00B gate is split. Its evidence half (the merged #1734 API, the Castle authored-ID reservation in 10000-10999 and the stock-catalog collision audit) is Ready after CS-INT0. Its client-proof half stays UATPending until the first authored quest in any zone passes journal, dialog, advance and relog in the client. |
| D-CA9 | BlockedDecision | Route a Castle lab client to the Castle server with a non-destructive `CastleLocal` row (`http://127.0.0.1:18081`) in the shared lab install's `LoginInternal.lua`, and point `lab-account.p4.json` and `lab-account.p5.json` at it (`"server": "CastleLocal"`). Alternative: an exclusive lab window with `labd.env` switched. Reason: it edits shared lab install state that Cellblock's UAT also reads, and `labd.env` holds one server URL, so a switch would hijack Cellblock ([isolation](work-packets.md#lab-routing)). |
| D-CA10 | Decided | Add a `MINIGAME_PORT` env override (CS-INF1) on the Castle branch, so a second local server does not collide on `0.0.0.0:30000`. Server-only; the default stays 30000 and no client patch is needed. |
| D-CA11 | Decided (user, 2026-10-17) | **No lab until the end.** No packet drives the client or the lab. Each client-only claim becomes `UATPending (end UAT)` in branch `uat/end-uat.md`, which the user runs at the end. CS-01c and the CS-00B client proof stay UATPending. Reason: the user's direction. |
| D-CA12 | Decided (CS-02L `change`; the owner accepted the reading 2026-10-18: player-bound and server-counted, not skill-proof) | **b2-lite replaces the full CS-02b2 for Castle callbacks.** The internal `BaseToCellMsg::MinigameResult` carries the receipt's `player_id`; the Cell refuses a victory when the entity is missing or the player mismatches; and conditioned callback chains go through `fire_guarded_minigame_chain_by_id`. Reason: Base already proves E/P/I before and after the close (b0/b1). The guard re-evaluates world and step at fire time, and mutation is mission-state only. It reverses the branch's reviewed "b2 required" gate (`work-packets.md:197-271`), so trust and minigame advisors sealed it rather than the architect. |
| D-CA13 | Decided (architect recommendation; the orchestrator confirms) | Stop the Castle lab server (PID 36540) through CS-01d (not yet run). Keep `sgw_castle_cs_01b` and the `castle-cs-01b` worktree; restart with the CS-01b recipe for the end UAT. Reason: the lab is paused, and the server captured Cellblock's lab login (F11). The DB holds `Cellutb` and the end-UAT state. |
| D-CA14 | Decided | ops refuses `--force`, so the owner retires integrated packet worktrees: `pwsh tools/build-lane/rm-worktree.ps1 <name> --force` from the main checkout. `castle-cs-01b` is retired last, after the end UAT. Reason: F12. Never `git worktree prune`. |
| D-CA15 | Decided (architect); **amended 2026-10-18 (CD-7)** | Original: CS-08a2M moves the Frost's Letter tests into a new `frosts_letter_round_trip_tests.rs`, the campaign that reaches `main` second owns the recipient round trip, and the orchestrator points Harset's HA-1360b at the new file. **Amended:** Harset put its recipient tests in a new `crates/services/src/mission_letter_receipt_tests.rs` (plus `negative.rs` and `reaccept.rs`), which cover CS-08a2's two tests. CS-08a2 and CS-08a2M are dropped, Castle needs no recipient test, and nothing is relayed to Harset. Reason: moving the block out of `mission_round_trip_tests.rs` would silently duplicate the live-DB tests on merge (r3b plan F2/F3); the CS-08a2S verdict. |
| D-CA16 | **Answered by the owner 2026-10-18 (OD-3 = B): build now** | Authored quests after `medical_q1`: (A) hold them until a live catalog proof passes (the contract default), or (B) build them now and accept rework if the proof fails. The owner chose B. R3d (now R4) and CS-05b are unblocked; every authored quest is proven together in the CS-07c end UAT, and a catalog-proof failure there means rework, not a second planned UAT round. Reason: F19. |
| D-CA17 | Decided (architect) | Castle's authored XML goes in `mission_additions_castle.rs`; `MISSION_ADDITIONS` lists its entries in ID order; the tests go in `tests/mission_additions_castle.rs`; the chains go in `castle_side_stories_chains.sql` and `castle_repeatable_chains.sql`. Reason: F18. The Cellblock conflict shrinks to array entries that CS-07R unions (F1b rule below). |
| D-CA18 | Decided (architect) | CS-06a seeds chain 1398 only once Harset's HA-01b receiver is on `content/harset-campaign`. The campaign PR merges only once HA-01b is on `main` or in the same merge window. Otherwise CS-06a/b are reverted out of the PR. Reason: never ship a stranded mission. |
| D-CA19 | Decided (architect, R3b) | Bark and hub id sub-ranges, taken from the top of the CS-00Be sub-blocks so that authored quests keep growing upward from 10300/10000. Barks (CS-05S2b): chains 10200-10219, dialogs 10740-10749, screens 10900-10919. Hub services (CS-05S2a): chains 10220-10229, dialogs 10750-10759, dialog sets 10790-10799, screens 10890-10899, buttons 10990-10999. Template and spawn ids sit outside the Castle sub-blocks: a seal proposes ids above the seed max with a no-collision proof, and the coordinator confirms them (CD-6). |
| OD-1 (D-CA-SYM) | **Answered by the owner 2026-10-18: Option A** | The 1521 implant ships as an item (`add_item 5169` on completion; `worknotes/CS-03S.md` §7). CS-03dS (seal) and CS-03d (build) are unblocked and planned in R4. |
| OD-2 (G-567) | **Answered by the owner 2026-10-18: accept** | Mission 567's Castle leg is rebuilt as a `CIMMERIA_AUTHORING` reconstruction: a file-cabinet template and spawn, an offer binding, `interact_tag` → `give_item 2698` and `advance_step`, and the 2012 → 4039 trigger. CS-08bM is superseded by the R4 seal CS-08bS, then CS-08b1 and CS-08b2. 567 completes on Harset (chains 6503/6504), which ship enabled with the Castle leg under FZ-12 (D-FZ1). |
| OD-4 | **BlockedDecision (owner)** | A hub medic vendor needs a new medical `item_lists` row. Recommended: leave the medic out. Gates template 414 and spawn 13803 only. |
| OD-5 | Accepted for now | The hub quartermaster (template 415) reuses Debug Area list 1300, a cheap faucet. Flag it for an economy pass before a public release. |
| CD-1 | Coordinator decision; recommended **drop** | Chain 1399 (the 1664 catch-up on `player_loaded Castle`) would re-grant 1664 after an abandon. CS-06a seeds 1398 only (D-CA18), and it is blocked on HA-01b anyway. |
| CD-2 | Decided (coordinator, R3b) | Accept the CS-04a1s/d and c1/c2/c3 splits, CS-04a1t, CS-05S2a/b and the CS-07R rule amendment (F1b, next row). Hold CS-08a2M and CS-DOC3 (CS-DOC3 later shipped in R3c; CS-08a2M is dropped by CD-7). |
| F1b (CS-07R rule) | Decided (coordinator, R3b) | CS-07R's conflict rule takes the union, in ascending id order, at three points: `MISSION_ADDITIONS`, the registry `vec![...]` line in `resources/tests/mission_additions.rs` (now :130) and the array at `resources/tests/dialog_overrides.rs:87` (Castle's `[3995, 3996, 10700]` plus Cellblock's `60150, 60151`). CS-07R also takes Cellblock's side of `live_db_castle_population.rs` MISSION_ACTORS (11 → 9: Cellblock dropped Castle_MedDispenser and Castle_SymbioteTank). The R4 plan extends the union to the `DIALOG_OVERRIDES` tail. |
| CD-3 | Decided (R3b-2) | CS-05R1 (vendor store ops refuse an out-of-range or dead player) is carried in the Castle branch. `store_ops.rs` is byte-identical on `origin/main`, so the commit can also go to a standalone main PR; that is optional. |
| CD-4 | Decided (R3b-2) | CS-05a test 4 (the vendor round trip on template 415) is dropped: 415 reuses lists 1300/2, which template 1302 already uses. `live_db_vendor_arbitrage` and test 2 guard the prices; UAT row hub S2 covers the client path. |
| CD-5 | Decided (R3b-2) | The `medical_q1` replay work is split c1/c2/c3/c4, with c2 and c4 based on CS-INT3c rather than stacked on an unintegrated c1. |
| CD-6 | Decided (R3b-2) | Accept template 415 (hub quartermaster) and spawn 13804. Template 414 and spawn 13803 stay reserved for the medic (OD-4). |
| CD-7 | Decided (R3b-2) | CS-08a2 and CS-08a2M are dropped per the CS-08a2S verdict. D-CA15 is amended, F16 is resolved, and nothing is relayed to Harset. |
| D-CA20 | Decided (architect, R4) | Per-quest id ranges for R4, allocated from the 10300/10000 sub-blocks (`id-allocation.md`, Castle final pass). Built: `medical_q2` 10301, `defense_q1` 10310, `defense_q3` 10312, `return_q1` 10320, `return_q2` 10321, `repeatable_q1` 10330, plus the 567 and 1521 ranges. Reserved and unseeded: `medical_q3` (10302) and `defense_q2` (10311). |
| CD-8 | Decided (R4) | Scope: six R4 quests (`medical_q2`, `defense_q1`, `defense_q3`, `return_q1`, `return_q2`, `repeatable_q1`) plus Q1 makes 7. `medical_q3` and `defense_q2` are deferred with ids reserved: each needs a second flag carrier on Ogilvie or a prisoner that overlaps `medical_q2`. |
| CD-9 | Decided (R4) | Path legs are end-UAT rows. Component equality from `nav_inspect` is the static proof for every R4 actor; the `nav_path` leg needs a running server and becomes a CS-07c row. |
| CD-10 | Decided (R4) | No per-quest "t" packet. |
| CD-11 | Discharged by FZ-12 (D-FZ1) | The 567 PR gate was: the campaign PR carries CS-08b1/b2 only if Harset HA-D04 (chains 6503/6504 enabled) is on `main` or in the same merge window. The gate option (G-REVIEW rounds 1 and 2, CS-07bF1 and CS-07bF4) held eight chains disabled, with chain 1400 (the cabinet feedback dialog 10734) enabled. Owner decision D-FZ1 enables all nine Castle chains and Harset 6503/6504 together, pinned by `live_db_castle_567_chains_ship_enabled` and `live_db_chains_6503_and_6504_ship_enabled`. Client-side behaviour is unproven until the owner's lab UAT FZ-U1. Caveat (CS-07b3 m-3): a GM-forced step 2000 gets no cabinet feedback; do not widen 1400, because it would then fire together with 1397 now that the leg is enabled. |
| CD-12 | Decided (R4) | Dialog shape and budget: at most 2 authored dialogs per quest, one screen, no buttons; each "d" packet adds at most 4 lines to `dialog_overrides/mod.rs`. |
| CD-13 | Decided (R4) | One worktree per quest (`castle-cs-<q>`) holds s, then c1, then c2. The d packet runs in parallel in `castle-cs-<q>d`. |
| CD-14 | Decided (R4) | CS-R4P (an id-keyed registry test) runs in R4a and is integrated by CS-INT4S, so every "s" packet starts from it. |
| CD-15 | Decided (R4) | The final integration is split: CS-INT4.1 (567, 1521, `medical_q2`, `defense_q1`, `defense_q3`) and CS-INT4.2 (`return_q1`, `return_q2`, `repeatable_q1`, CS-DOC4). |
| CD-16 | Decided (R4) | The CS-07R union rule is extended beyond F1b to the `DIALOG_OVERRIDES` tail, `overrides_castle.rs` and `mission_additions_castle.rs`, followed by `cargo fmt -p cimmeria-resources`. |
| CD-17 | Decided (CS-07R-ID, 2026-10-18) | Castle templates 420/421 are renumbered to 424/425 (`0f1af91d4`); Tollana keeps 420/421. A guard test, `seed_id_uniqueness.rs`, now fails on a duplicate template id. |
| CD-18 | Decided (owner, 2026-10-18) | Castle merges into the local `campaign/four-praxis-zone-restoration` branch, and the combined campaign branch ships all four zones at once. An Opus (architect) review of the combined campaign comes first, producing prescriptive Haiku fix packets. |
## Packet status

| Packet | Status | Gate / next action |
|---|---|---|
| CS-INT0 | Done at this commit (ledger port) | Merge `f2aff4903` already at HEAD (no new merge); overlay ported; `sgw_zone_castle` reloaded. The returned SHA is the new `FEATURE_SHA`. |
| CS-00 | Integrated (entry amendment signed) | Delivered through `fae1614af`. Follow-up gates move to CS-00Ra/b (G-MED/G-SYM), then CS-00R2 (G-567/G-706/1376). |
| CS-00Ra (R1a) | Sealed (UNRESOLVED G-MED placement/game/radio) | Commits `312170e93`, `d54125e66`, `99bcbc249`. Dispenser placement, original game type and radio trigger stay `UNRESOLVED (G-MED)`. Chains 1371-1373 claimed. |
| CS-00Rb (R1a) | Sealed (D-CA-SYM open) | Commits `6405d8584`, `ac1e77dea`. Chains 1374-1377 claimed. Implant/pouch behavior stays the owner question D-CA-SYM. |
| CS-INF1 (R1a) | Integrated | Commits `6f60321a4`, `749314b07`. `MINIGAME_PORT` env override (D-CA10). |
| CS-INT1 (R1) | Integrated at this commit | Cherry-picked the reviewed R1a commits in order. The new tip is the CS-01b base. |
| CS-00B | Evidence half integrated (CS-00Be) / UATPending (client proof) | D-CA8. Gates CS-04a/b/c and CS-05. |
| CS-00R2 | Integrated (CS-INT2a) | G-567 BlockedEvidence; G-706 SUPPORTED as a reproducing model with the exporter join UNRESOLVED; 1376 lineage recorded. Evidence `evidence/g-567-706-1376.md`. |
| CS-02S | SEALED (R2a) | Mission 1520 frozen rows in `worknotes/CS-02S.md`. Gates CS-02c1. |
| CS-03S | SEALED (R2a, Option B skeleton) | Mission 1521 rows in `worknotes/CS-03S.md`; chamber placement stays BlockedEvidence (G-ROUTE). Gates CS-03c1. |
| CS-02L | `change` (R2a) | b2-lite stands for Castle callbacks 1372 and 1375 with the five contract edits applied in `r2-packets.md`. The D-CA12 reading awaits owner confirmation. |
| CS-01d | Unrun | Stop Castle server PID 36540 (D-CA13). Never reload or drop `sgw_castle_cs_01b`. |
| CS-INT2a | Integrated (this commit) | R2a seals, `009ab0536` (CS-01b), `r2-packets.md` and `RESUME.md` copied in. The new tip is the R2b base. |
| CS-MGF1 | Integrated (CS-INT2, `f0b7d7da0`, `1061286f2` as `bf4478e7d`, `97b878b8e`) | Receipt player id to the Cell; `chain_has_conditions`. D-CA12 accepted (b2-lite player-bound, server-counted). |
| CS-02c1 | Integrated (CS-INT2, `4b1c37af6`, `14e3e97bc` as `d631b74f7`, `5cb8b5fb8`) | 1520 seeds and entry replay tests. |
| CS-03c1 | Integrated (CS-INT2, `3b15ad417`, `8efc8ca7f` as `cb69d2f82`, `df44e504f`) | 1521 skeleton seeds and entry tests. |
| CS-MGF2 | Integrated (CS-INT2, `9c2cc3bd4` as `d5b963a9f`) | The player-bound victory guard. |
| CS-02c2 / CS-03c2 | Integrated (CS-INT2, `a2e732155` as `2c105f708`; `1aec5801c`, `b4d56d7a2` as `0f5f01496`, `bec83c9be`) | 1520/1521 graph replay and navmesh route tests. |
| CS-DOC2 | Integrated (CS-INT2, `30f6f150e`, `3a2c999ac`, `f9263ef17` as `27a9ac0db`, `3d870bddd`, `5177290c5`) | `mission-chains.md` rows; `uat/end-uat.md` rows. |
| CS-INT2 | Integrated (this commit) | Cherry-picked R2b/R2c in order onto `e78a44272`, no conflicts. reload-db, live-DB 1520/1521 and population, and nextest for cell, base-world-entry, content-engine and entity all green. |
| CS-01 | UATPending | Split into CS-01a (walk step file), CS-01b (isolated stack preflight) and CS-01c (lab walk by the orchestrator through the lab-uat skill). |
| CS-01a (R1a) | Integrated | Commits `d164faa25`, `a7417385e`, `89b1e6e00`. Mission-systems-advisor's `uat/CS-01-walk.md`. |
| CS-01b (R1b) | Integrated (CS-INT2a, `009ab0536`) | Preflight record in `uat/CS-01-preflight.md`. The Castle server (PID 36540) keeps running until CS-01d (D-CA13). |
| CS-01c (R1c) | UATPending (D-CA11) | Lab walk paused. It got through setup, the GM 688 grant and the Castle landing; walking was blocked by lab input. Rows go to the end UAT. |
| CS-02 | BlockedEvidence | G-MED (CS-00Ra) plus the CS-02b2 children, then CS-02c wiring. Do not offer a stranded mission. |
| CS-02c | BlockedDependency | Enable guarded Castle callback only after CS-02a/b0/b1/b2, actor/route/report seal and complete client-visible 1520 graph. |
| CS-02a / b0 / b-prep / b1 | Integrated | Branch worknotes. |
| CS-02b2 | BlockedEvidence | Draft only. Split it into b2-prep/b2a/b2b with an architect pass after CS-00Ra. |
| CS-03 | BlockedEvidence (G-ROUTE, end UAT) | The 1521 skeleton is integrated (CS-INT2). G-SYM is settled by OD-1 = A: the implant item is R4 (CS-03dS, CS-03d). The chamber walk is a CS-07c row. |
| CS-06 | BlockedDependency | G-1664 plus the Harset HA-00b receiver and Kaerlia placement. |
| CS-08a1 | Integrated / UATPending | Chain 1366 at `03100628b`. |
| CS-08a2 | Dropped (CD-7) | Harset's `mission_letter_receipt_tests` own the recipient round trip (CS-08a2S verdict). CS-08a2M is dropped too. |
| CS-08b | Integrated (CS-08bS seal, CS-08b1/b2, INT4.1 and INT4.2); shipped enabled (CD-11, FZ-12) | 567 Castle leg built and replay-tested; all nine chains and Harset 6503/6504 ship `enabled = true` under FZ-12 (D-FZ1); chain 1400 (feedback dialog) ships enabled (CS-07bF4). Client rows are UATPending (lab row FZ-U1). |
| CS-04a / b / c | Integrated (R4, INT4.1, INT4.2) | `medical_q1` through `return_q2` are built and replay-tested server-side; `medical_q3` and `defense_q2` are deferred (CD-8). Every client-visible claim is a UATPending row in `uat/end-uat.md`. |
| CS-05 | Integrated (R4, INT4.2) | Hub services, barks and `repeatable_q1` are integrated. The hub medic stays open as OD-4. |
| CS-07 | In progress (CS-07R, CS-07R-ID, CS-07a done; CS-07b G-REVIEW rounds 1 to 3 run, fixed by CS-07bF1 to F5; round 3 verdict ship) | See "Closeout state" below. CS-07c is the user's end UAT; the PR follows the combined four-zone review (CD-18). |
| CS-08 | BlockedDependency | Children below; CS-08b/567 retains G-567, G-ID and G-REVIEW. |
| CS-08a | BlockedDependency | Split below; playable handoff depends on Harset/engine HA-1360a/b possession and successful consume-before-complete. Client UATPending. |
| CS-06S | R3a: sealed, partial (`BlockedDependency (HA-01b)`) | Grant leg (points 1-4) frozen in `evidence/1664.md`; chain 1399 is frozen pending a coordinator decision; the receiver leg waits for HA-01b. Commits `b33f586e7`, `7346fbccc`. |
| CS-08bM | Superseded (OD-2) | Replaced by the R4 seal CS-08bS. Its branch was never created. |
| CS-03dS | Integrated (CS-INT4S) | Seal: G-SYM-CONSUME closed; G-SYM-GAME, G-SYM-DEBUFF and G-SYM-RADIO stay open. Build CS-03d integrated in INT4.2 (tip `72908a802`). |
| CS-05S | R3a: sealed, no leg `SEALED`; superseded | Services and barks were re-sealed by CS-05S2a/b (R3b); repeatables are re-sealed in R4 as CS-05S3 (D-CA16 answered B). `worknotes/CS-05S.md`. Commits `6deb6f2ea`, `a7ceaaa58`. |
| CS-04aD | R3a: partial seal | `medical_q1` `SEALED` and built. `medical_q2` is re-sealed in R4 as CS-04aD2 (new prisoner template and spawn); `medical_q3` is deferred. `worknotes/CS-04aD.md`. Commits `616c00257`, `7bde2cdd8`. |
| CS-04bD | R3a: design seal, no quest `SEALED` | D-CA16 is answered B. `defense_q1` and `defense_q3` are re-sealed in R4 as CS-04bD2a/b with new ids (this seal's ids collide with `medical_q1`); `defense_q2` is deferred. `worknotes/CS-04bD.md`. Commits `7894e2f8e`, `95efa3425`. |
| CS-04cS | R3a: partial seal | `return_q1` and `return_q2` are re-sealed in R4 as CS-04cS2, with a return trigger that does not need HA-01b; the route's path leg is a CS-07c row. `worknotes/CS-04cS.md`. Commits `ae1ba77c6`, `47dc0fc64`. |
| CS-INT3a | Integrated (`224e7692e`) | R3a seals cherry-picked in order, R3 files copied in. R3a review verdict was "needs coordinator decision" (`.claude/reports/castle-continuation/r3a-wave-review.md`). |
| CS-04a1s | Integrated (CS-INT3b, `63f28454b`, `72e338620`) | `medical_q1` catalog entry and Missions seeds. |
| CS-04a1d | Integrated (CS-INT3b, `fd7e6bdd2`) | `medical_q1` dialog 10700 and its override. |
| CS-08a2S | Integrated (CS-INT3b, `92c55cdb0`, `0f5c6e985`) | HA-1360 recipient interface recorded; its verdict drops CS-08a2 (CD-7). |
| CS-05S2a | Integrated (CS-INT3b, `bee4c0cb8`, `2f3f54e17`, `b2f6bf48d`, `c73133f66`) | Return-hub services re-seal: quartermaster 415 / spawn 13804 (CD-6, OD-5). |
| CS-05S2b | Integrated (CS-INT3b, `95120d92d`) | Return-hub barks re-seal (D-CA19). |
| CS-INT3b | Integrated (`95120d92d`; RESUME `8d93c3193`) | R3b-1 cherry-picked in order. Review r3b1: Ship. |
| CS-05R1 | Integrated (CS-INT3c, `ae25d9ae4`, `d14a29a92`) | Vendor store ops refuse an out-of-range or dead player (CD-3). |
| CS-05a | Integrated (CS-INT3c, `3e923803f`) | Return-hub services and activity; test 4 dropped (CD-4). |
| CS-04a1c1 | Integrated (CS-INT3c, `7eec70d6c`, `e95c54447`) | `medical_q1` chains 10000-10006 and the core replay. |
| CS-04a1c3 | Integrated (CS-INT3c, `7742478d4`, `3ac7516f7`) | `medical_q1` progress persists per character. |
| CS-04a1t | Integrated (CS-INT3c, `1fdfb8f04`) | `medical_q1` and dialog 10700 reach a stock client through cooked sync (`resync_castle.rs`). |
| CS-05c | Integrated (CS-INT3c, `d51928ab8`, `721141239`) | Return-hub barks 10200/10201. |
| CS-INT3c | Integrated (`721141239`; RESUME `0674a57be`) | R3b-2 cherry-picked in order. Review r3b2: Ship. |
| CS-04a1c2 | Integrated (CS-INT3c2, `4781a3ae0`, `a49e069ad`) | `medical_q1` lifecycle replay, including chain 10000 (704 completion offers the medic). |
| CS-04a1c4 | Integrated (CS-INT3c2, `f7bf4b5ad`) | `medical_q1` refusals and the medic range gate. |
| CS-DOC3 | Integrated (CS-INT3c2, `26812d3b9`, `c8511c1a6`, `2fad73404`) | `mission-chains.md` and end-UAT rows (Q1-U1..U9, hub S1..S5, HB1..HB2). The hub rows reuse the ids S1-S5 of the 1521 table; cite them as "hub S1..S5". |
| CS-INT3c2 | Integrated (`2fad73404`, the current `FEATURE_SHA`) | R3c cherry-picked in order; one pure-addition conflict in `chain_replay_tests/mod.rs`, both lines kept. Lane: `castle_side_medical_q1` 21 passed, `castle_medic` 2 passed, resources/base-session nextest 81 passed. Review r3c: Ship. |
| R4 | Integrated (CS-INT4S, INT4.1, INT4.2) | Plan: `.claude/reports/castle-continuation/r4-dispatch-plan.md`. Seals integrated at `25beef08e`; quests, 567 and the 1521 implant integrated at `9a3a3b076` and `887f5d48b`. |
| CS-INT4S | Integrated (`25beef08e`, the `FEATURE_SHA` for R4b onward) | Cherry-picked CS-R4P, CS-08bS, CS-03dS, CS-04aD2, CS-04bD2a/b, CS-04cS2 and CS-05S3 (25 commits). All seals SEALED; open items are listed under "Closeout state". |
| CS-INT4.1 | Integrated (`9a3a3b076`) | 28 picks: 567, 1521 and the first three quests. Lanes: `castle_side` ok, `mission_567` ok, `mission_1521` 23, `castle_hub_barks` 7, `live_db_castle` 40, `live_db_content_loaders` 5, `castle_medic` 12, nextest 150, `castle_navmesh` 7. |
| CS-INT4.2 | Integrated (`887f5d48b`) | 18 picks plus a `medical_q1` test fix with a revert proof; two picks skipped as empty duplicates. Lanes: `castle_side` 171, `mission_567` 22, `mission_1521` 23, `live_db_castle` 50, `return_q1` 19, `return_q2` 24, `repeatable` 28, nextest 174, `castle_navmesh` 7. |
| CS-07R-ID | Done (`0f1af91d4`) | Template ids 420/421 renumbered to 424/425 (CD-17); guard test `seed_id_uniqueness.rs` added. nextest 1565 passed, `castle_side` 171. |
| CS-07R | Done locally, never pushed (merge `0a851a410`, fix `a8753bbe5`) | Merged the local `campaign/four-praxis-zone-restoration` (`b28b4fa11`) into Castle. The first attempt blocked on the 420/421 collision; its resolved merge is kept as local branch `wip/cs-07r-merge-unresolved-ids` (`3f9b7c49e`, retirable). |
| CS-07a | Done (this commit) | Ledger, RESUME, `id-allocation.md`, `unified-uat.md` and `worknotes/CS-07.md` closed out. |
| CS-07b | Next | G-REVIEW, at most 3 rounds. |
| CS-07c | UATPending (user) | The user runs `uat/end-uat.md`. |
Mission and natural-route client acceptance remains **UATPending**. The CS-08a1 first-session lab attempt stopped before world entry on a bridge connection failure; no CS-08a client result is claimed. The later bounded CS-01 probe below established only Castle loading and Ogilvie rendering. FZ-99 owns shared status documents; chain 1109 belongs to ring transport; CB-N05b owns its Castle recipient action. No Castle packet duplicates those owners.

## Closeout state (CS-07a, 2026-10-18)

Branch `content/castle-campaign`, worktree `zone-castle`. The head before G-REVIEW was `a8753bbe5`; the G-REVIEW round 1 fixes sit on top (see "G-REVIEW round 1" below); neither it nor the merge `0a851a410` was pushed. The local `campaign/four-praxis-zone-restoration` (`b28b4fa11`) exists only on this machine; `origin` has no such branch.

**Built and tested (server side, by replay and live-DB tests).** Missions 10300, 10301, 10310, 10312, 10320, 10321, 10330 (Q1, `medical_q2`, `defense_q1`, `defense_q3`, `return_q1`, `return_q2`, `repeatable_q1`), the 567 Castle leg, the 1521 implant chain 1386, the hub quartermaster (415), the hub supply officer (424), the defense_q3 giver (425) and the hub barks 10200/10201. Final lanes at the CS-07R fix commit: clippy on six crates ok; `nextest --profile=ci` on six crates 2890 passed; `castle_side` 171; `mission_568` 20; `mission_567` 22; `mission_1521` 23; `mission_706` 9; `mission_708` 38; `hub_barks` 7; `live_db_castle` 50; `content_loaders` 5 (on retry); `castle_medic` 12; `na13` 9; cellblock 79; tollana 4; harset 84; `castle_navmesh` 7.

**Never tested.**

- Everything a client shows or does: journals, dialog windows, glyphs, item icons, route walks and the natural-route flow are UATPending rows in [uat/end-uat.md](uat/end-uat.md) (D-CA11, no lab).
- Two wave-review verdicts are still open `recheck`s with no re-review report: CS-04c1c2 (`a093dcab6`, fix `13a994b3e`) and CS-04c2c2 (`904d43dcf`, fix `bb7eb8296`). The CS-DOC4 doc rework (`a41feb202`, `f3fe7f1b0`) was rechecked by PU-04b and its four wrong rows are fixed in PU-02.
- The 657-line `reliable_fit.rs` re-application in the CS-07R rerun was verified only by compile and tests, not read by hand.
- Seal open items: CS-08bS dialog ids 10733/10734 and screens 10865/10866 sit outside the D-CA20 allocation, dialog 2043 replaces 2042, and the file cabinet sits 6.0 u inside Romney's 18 u aggro ring (OPEN-5). CS-03dS keeps G-SYM-GAME, G-SYM-DEBUFF and G-SYM-RADIO open; the inventory tooltip, mailed-item placement and native-use silence are UATPending. `defense_q3` uses fallback giver 425 and spawn 14096. The `return_q1`/`return_q2` path legs are UATPending (CD-9).
- The `repeatable_q1` sentinel window was moved to `0x7000_9A30..3F` in INT4.2; the `na13` once-chains expectation was corrected in INT4.1. Both are proven by their tests only.
- CS-01d (stop PID 36540) is unrun. CS-07b and CS-07c have not started.

**Gates.**

| Gate | State |
|---|---|
| D-CA18 | CS-06a/b wait on Harset HA-01b. Met by absence: no Castle seed adds chain 1398 or grants mission 1664 (the stock `missions.sql` row 1664 "Debriefing" predates the campaign). |
| CD-11 | The 567 leg ships enabled (all nine Castle chains and Harset 6503/6504, FZ-12, D-FZ1); the old gate recipe under CD-11 above is historical. |
| OD-4 | Hub medic is open (template 414, spawn 13803 reserved). |
| CD-1 | Chain 1399 is dropped. |
| CD-8 | `medical_q3` and `defense_q2` are deferred; ids reserved in [id-allocation.md](../four-zone-planning/id-allocation.md). |

**Remaining sequence** (user decision 2026-10-18, CD-18):

1. CS-07b (G-REVIEW).
2. Merge Castle into the local four-praxis branch.
3. An Opus (architect) review of the combined four-zone campaign, producing prescriptive Haiku fix packets.
4. Haiku fixes plus a Sonnet review loop.
5. CS-07c end UAT (user).
6. Ops opens the PR. If all CI is green, merge and release.

**Known pre-existing issues, untouched by this campaign.** Workspace `clippy -D warnings` fails in `cimmeria-scene-editor`, `cimmeria-content-editor` and `cimmeria-app/src-tauri` (lint drift). The plan filters `castle_706_708` and `live_db_only_chains_1008_and_1044` match no test (use `mission_706` and `mission_708`). CS-04cS2 has two seal defects: it says Dakara E1 is world 62 (the repo says 61) and it names `castle_return_chains.sql` (the file is `castle_side_stories_chains.sql`).

**Persistence note for the release.** The persistent-player-data campaign owns the persistence changes that gate the first persistent `/release`: waves 1-2 are on `main` (#1832-1838) per the CS-07a brief, while wave-2-container fixes, PP-13 (`-WipeDatabase`) and PP-09..12, 14, 16 were not confirmed merged when this was written. Check that campaign's `RESUME.md` before releasing. Its docs are not edited here. Shared status docs (`gap-analysis*`, `project-status.md`, `zone-audit.md`) stay with FZ-99; see [worknotes/CS-07.md](worknotes/CS-07.md).

## G-REVIEW round 1 (CS-07b)

Review report `.claude/reports/castle-continuation/CS-07b2-review.md` found one blocker, one major and five minor items. Fix rounds, all local, none pushed:

| Round | Commits | What |
|---|---|---|
| CS-07bF1 | `6b677cdbc`, `5f089d601` | B-1: the nine 567 Castle chains (1394-1397, 1400, 10240-10243) ship `enabled = false`; replay tests enable their own loaded copies; new `live_db_castle_567_chains_ship_disabled` (renamed `..._ship_gated` in CS-07bF4). m-3: `seed_id_uniqueness.rs` now guards dialogs, dialog sets and maps, screens (by `screen_id`), missions and chains, plus the Castle range tests. |
| CS-07bF2 | `7b676b438` | The 115 Castle live-DB tests gained the `live_db_` name prefix so the nextest live-db group (`test(~live_db)`) selects them; the set of tests is unchanged. |
| CS-07bF3 | `1eab79abc` and the two commits after it | M-1: `mission-pak-overrides.md` lists all nine `MISSION_ADDITIONS` entries. m-1: the stale "callbacks remain disabled" line. Docs synced with F1/F2. |
| CS-07bF4 | `cc76b10d2`, `d5bc251f3` | M-1: `seed_id_uniqueness.rs` reads the multi-line `VALUES (` form and asserts row-count floors (450 chains, 4500 maps). m-3: `castle_block_ids_do_not_creep_past_10999` deleted. D-1: chain 1400 ships enabled, the other eight stay disabled; test renamed `live_db_castle_567_chains_ship_gated`. n-1 typos. |
| CS-07bF5 | docs `12af405fa`, and the revert `b21eb0013` | M-2: revert of the regen commit `2c038da1c` (`regen.py --check` is stale again by design; PRs do not commit generated blocks). Docs fixed for D-1 (chain 1400 enabled), m-2, n-2, D-2b. |
| CS-07b3 | round 3 review (read-only, `CS-07b3-review`, scope through `12af405fa`) | Verdict ship: 0 blockers, 0 majors, 3 minors, 3 nits. Applied in the PU-02 docs commit: m-2 (RESUME and this README; heading at mission-chains.md:1632 left as is), n-1 (`12af405fa` named), m-3 (caveat in the CD-11 enable recipe here and in RESUME). Open: n-3 (`CASTLE_CHAIN_FILES` omits five Castle files). Closed by PU-01 (pending merge): m-1 (`dialog_set_maps` floor 4500 does not catch a reverted next-line reader), n-2 (overlong doc line in `mission_567.rs`). |

Still open from the round: m-2 (`interaction/mod.rs` is over the 700-line cap; D-2, a split packet or the Castle test mods in a new aggregator) and m-4 (commit trailers; squash at PR time). CS-07b1 also recorded two reds in Harset's `letter_turn_in_route.rs`, which Castle did not touch: `unpaired_id_fields_only_shrink` (16 unpaired ID fields, baseline 0) and `every_source_target_reaches_signoz_at_its_level` (`harset.letter_turn_in` at DEBUG reaches no OTLP index). Both are unproven as pre-existing, because nobody ran them on `b28b4fa11`; they are assigned to the Opus four-zone review.

## CS-00 resumption: source and domain findings

The 2026-10-14 archaeology, mission, items and movement advisors reviewed the
current seeds, legacy script and route evidence without changing runtime data.
Legacy `Castle.py` handles `Castle.Infirmary` entry only by showing room text
5188; it does not accept 1520/1521. The mission advisor signs a bounded
`CIMMERIA_RECONSTRUCTION` of separate post-704 Infirmary-region offers for
1520 (archetypes 1–4 and 6) and 1521 (archetype 8), with separate first-step
region replay chains 1368 and 1370. Current `fire_enter_region` carries world,
mission and archetype context. Current `fire_dialog_choice` also carries
archetype; an older replay comment claiming it does not is stale. This
review closes the design aspect of G-ENTRY under O14-03, while the
implementation/test gate remains open. For 1521 the replay must advance
directly to 4456: merely completing sole objective 5175 can complete the
entire mission prematurely.

The items advisor signs only the 1521 story-step boundary: tank bypass,
implant and report may be represented by persisted mission progress, with
**zero item, pouch-stat or effect mutation** until evidence supports one.
Neither item 2718 nor 4652 nor 5169 is proved as the mission grant. No Castle
dispenser/tank actor binding, validated minigame type, radio edge or client
chamber route was recovered, so G-MED/G-SYM/G-ROUTE remain open. Mission 567's
office pickup goal is source-supported, but its exact actor/grant is not; a
bounded 2698 pickup needs placement, repeat/fail lifecycle and Harset's safe
recipient contract before CS-08b dispatch. Mission 1376 remains on a build
lineage hold. These are evidence signatures, not runtime G-REVIEW or client
passes. Details and required checks are in [work-packets.md](work-packets.md#2026-10-14-owner-decision-reconciliation).

The stock text seed names a Med Dispenser (moniker 8896), Symbiote Tank
(8897), and Romney's File Cabinet (7547) in Castle. These support the object
concepts but identify no current template, spawn, event set, interaction tag,
or pickup grant in the checked seed and legacy Castle script. Four Symbiote
Chamber prefabs in the map are location evidence, not proof of which prop is
the mission tank. Mission 1520's radio objective 5174 and dialog 4985/map
5826 exist, but their trigger is unbound. See the [stock prop supplement](work-packets.md#2026-10-14-stock-prop-supplement-no-binding-seal).

## CS-01 bounded client probe, 2026-10-14

O14-08 supplied standing lab authorization. Peers confirmed no scheduled use;
the default lease was acquired, then released. The [UAT record](uat/CS-01.md)
and [screenshot](uat/CS-01-ogilvie.png) preserve the client observation. The client reached character
select after a 67-second ready phase. Its first test-character world entry
reported a bridge dispatch timeout; one retry reported the character in world
12. A GM `.gotolocation Castle` loaded world 8 at the seeded story ring pad
(466.365, 70.397, 991.466). A later GM setup at (362, 70, 886) rendered and
targeted Ogilvie (entity 100102) at spawn 197's coordinate. From (366, 70,
886), `client_move_to` toward (386, 55.38, 940) stopped against room geometry
at about (371.07, 70.16, 889.75), 52.4 m short. The helper walks a straight
line; this result does **not** prove that the natural chamber doorway is blocked.

The local lab-server endpoint reported zero sessions while this client was in
world. Therefore this probe has client/screenshot evidence only: it did not
establish that the lab-server endpoint observed the same server, did not run
the natural 701–708 path, and did not test Castle→Harset crossing, mission
1360, return arrival, second-observer AoI or a new service location. The
client/server alignment and the natural chamber/DHD routes remain
`UATPending`; no gameplay failure is inferred from the straight-line helper
stopping at a wall. No branch build was installed or pointed at this client.

Offline route review gives inspection landmarks, not a walk: Ogilvie
`(362, 70.18, 886)`, cooked-map SecurityLock `(399.79, 58.91, 919.41)` and
the chamber prefab cluster near `(398.83, 55.04, 929–941)`. No documented
stair/door link joins them. Castle's navmesh is **advisory**, so its isolated
chamber component does not itself prove inaccessible client geometry. The
existing mesh test covers DHD→Bravo bunker, not natural Armory→DHD. Harset's
return gate row is likewise accepted without an enforceable Castle navmesh
landing check. G-ROUTE needs a natural client walk into and out of the
chamber, Armory→DHD, then Harset→Castle landing and movement to a safe
service site, with relog and an observer.

## Wave-0 integration boundary

The evidence/doc gate passes two review rounds: `north_review` findings are fixed
in `fae1614af`, and fresh reviewer `north_rereview` finds no actionable issues in
the full baseline-to-fixer diff. This documents the source matrix delivery only.
G-REVIEW and unresolved runtime gates stay blocked for the other packets. Independent
mission/items review and coordinator signatures on their contract rows are still required.
CS-08a1 has a separate owner seal and independent review; its authorization does not
come from the Wave-0 documentation gate.

## CS-08a design seal boundary

The owner-approved [CS-08a contract](worknotes/CS-08a.md) supersedes CS-00's unresolved mission-1360 bridge fields. G-1360's Castle design is resolved as CIMMERIA_RECONSTRUCTION; historical evidence snapshots still record the unrecovered original event. G-ID is closed only for individual chain 1366. Independent mission, items and persistence/Rust reviews plus coordinator signature close G-REVIEW for the Castle bridge phase only. The implementation adds a reviewed test-only wrapper around production mission hydration. Live-DB tests prove the Castle bridge, saved 4038/objective state, retained letter and fresh Harset hydration. Current Marsh chain 6501 can complete without possession, so HA-1360a/b remain playable dependencies. Client and telemetry acceptance remain UATPending.

## 2026-10-17 resume

Findings F1-F9 from the main-checkout overlay (`docs/analysis/castle-continuation/README.md`, not committed to `main`). Each row keeps the overlay's dated evidence; a tip or count cited in a row is the value at the time of that check.

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | **The campaign was already started.** `content/castle-campaign` exists at `de750bf07` (2026-10-14 19:30 -0500) in worktree `zone-castle`: 64 commits ahead of `origin/main`, 59 behind. It holds the full continuation ledger, evidence, the CS-01 probe record and worknotes for CS-00, CS-02a, CS-02b-prep, CS-02b0, CS-02b1, CS-02b2 and CS-08a. The brief's "nothing is started" premise is wrong. | `git log origin/main..content/castle-campaign`; branch `RESUME.md` | D-CA1 |
| F2 | The Castle zone session (O14-02) has been idle about three days. Its worktree is clean apart from an untracked `castle-pg.log`, and no Castle server, client or Postgres process is running. Per campaign-packet section 0, nobody messages it; the owner says whether this session adopts it. | `git -C zone-castle status`; process list 2026-10-17 | D-CA1 |
| F3 | `#1734` (CB-N05a) is already merged into the Castle branch (`94b4a5754` is an ancestor, via merge `a2d6debb6`). Since the merge base `817d3792c`, `main` has not changed `docs/analysis/four-zone-planning/`, `db/database.sql`, the Content/Missions seeds, `mission-chains.md` or `unified-uat.md`, so merging `origin/main` again should not conflict on Castle's files. | `git merge-base --is-ancestor`; `git diff --stat 817d3792c origin/main -- ...` (empty) | CS-INT0 |
| F4 | Leftover branches: `castle/cs-00-contract` (1 commit, cherry-picked into the campaign as `d5b679061`), `castle/cs-08a1-letter-bridge` (merged at `03100628b`) and `castle/ledger-session-record` (September, historical). Their worktrees are already retired. They are not in-flight work. | `git branch --list castle*` | D-CA7 |
| F5 | Harset's HA-1360a/b work (needed for CS-08a2) sits on `content/harset-campaign` (`7965f653d`, 2026-10-14) with seven `harset-ha-1360*` worktrees. Their tips are not ancestors of the Harset branch, so they were cherry-picked. The receiving interface has not yet been recorded in the Castle ledger. | `git log origin/main..content/harset-campaign --grep=1360` | CS-08a2 |
| F6 | Cellblock is live in another session: `zone-cellblock` (`content/cellblock-campaign` `452c15340`, 2026-10-17), eight `cellblock-cb-n0*` worktrees and `.claude/state/active-campaign = cellblock-continuation`. Castle must not touch those worktrees, DBs, server, the main-checkout `docs/analysis/cellblock-continuation/` overlay or the pointer file. The only shared files are `db/database.sql`, `id-allocation.md`, `docs/content/mission-chains.md` and `docs/guides/unified-uat.md`, and they collide only at each zone's final PR. CB-N05b (Castle recipient) is Cellblock's and is held on HA-1360. | Cellblock overlay `RESUME.md`, README F8 | D-CA3 |
| F7 | The 2026-10-14 CS-01 probe ran the default lab client against an unidentified server: `server_sessions` showed zero sessions while the client was in world 8, and the build hashes were not captured. A walk only counts when the client's server, the queried lab-server endpoint and the Castle build are proven to be the same process. | branch `uat/CS-01.md` | CS-01b |
| F8 | At session start the `lab-server` MCP (`CIMMERIA_LAB_MCP_URL`, default: the colo host on `:8444/mcp`, IP not recorded) was ECONNREFUSED. The local lab daemon `cimmeria-lab.exe` is up on `127.0.0.1:8779`. The shared test Postgres on `:5433` is up (Docker). | `.mcp.json`; `Get-NetTCPConnection` | CS-01b |
| F9 | Open evidence gates on the branch: G-MED (1520 dispenser actor, minigame, radio), G-SYM (1521 tank, minigame, implant), G-ROUTE (Infirmary to chamber, Armory to DHD, Harset to Castle landing), G-1664 (exit grant and HA-00b receiver), G-567 (Romney's Files grant), G-706 (why the ZIP classifier omits it) and the full G-REVIEW. The only claimed chains are 1366-1370. | branch `README.md` Decisions, `work-packets.md` seal record | CS-00Ra/b, CS-01a |
| F10 | **The lab is paused (2026-10-17).** The user's direction is to abandon lab work, return to content restoration, and leave all UAT to the user at the end. The CS-01 walk got through setup, the GM 688 grant and the Castle landing; walking was blocked by lab input. Those rows stay UATPending. | orchestrator brief 2026-10-17 | D-CA11 |
| F11 | The Castle lab server (PID 36540, `castle-cs-01b\cimmeria-server.exe`, started 2026-10-17 18:52) is still running on 18081/32932/18443/8452/30100 against `sgw_castle_cs_01b`. Cellblock's CB-U08 lab attempts logged into it by mistake. Cellblock's test character `Cellutb` lives on that DB. CS-01b's commit `009ab0536` is not cherry-picked. | process list; Cellblock overlay RESUME | D-CA13, CS-01d |
| F12 | The R1a worktrees `castle-cs-00ra` (`ed0c44657`), `-00rb` (`b9492dfee`), `-01a` (`966a4fd7d`) and `-inf1` (`4c13b9d13`) are integrated but still on disk, because ops refuses `rm-worktree --force`. | ops return | D-CA14 |
| F13 | The runtime never reads `mission_steps.step_enabled` or `mission_objectives.is_enabled`; only `console_authoring` tests read them, and mission 708's step 2417 is `false` yet works. Enabling 1520/1521 needs no flag flips. | grep `crates/`; `mission_steps.sql:2052-2064` | CS-02c1, CS-03c1 |
| F14 | Of the seeded minigame victory callbacks (1017, 1025, 1042, 1061, 1234, 1293, 13005, 1357, 3024, 7005), none has `content_conditions`. Routing conditioned callbacks through `fire_guarded_minigame_chain_by_id` (which exists, `event_dispatch/direct_invoke.rs`) keeps legacy parity. Today the Cell fires a victory for a missing entity with player 0 (`base_messages/minigame.rs:38-54`). | seed scan; code read | D-CA12, CS-MGF1/2 |
| F15 | Only chain 1366 is seeded. 1367-1377 are claimed but not seeded, per the "entry chains ship only with a completable branch" rule. `data/spaces/castle.nav` is tracked, so `nav_inspect` and `castle_navmesh.rs` can prove placement and routes without the lab. | `castle_carry_in_chains.sql`; `git ls-files` | CS-02S, CS-03S |
| F16 | Three test files are already past the 500-line soft cap: `crates/services/src/mission_round_trip_tests.rs` (638), `chain_replay_tests/mission_1360.rs` (555) and `resources/tests/mission_additions.rs` (610). HA-1360b's draft extends the first in place, so it would grow toward the 700 hard cap. **Resolved 2026-10-18 (CD-7):** Harset put its recipient tests in new files, and CS-08a2/08a2M are dropped, so no Castle packet grows `mission_round_trip_tests.rs` (672 lines at R3b). | line counts at `da2b82c70`; Harset `work-packets.md:172+` | CS-08a2M, D-CA15 |
| F17 | `mission_completed` is a working trigger (`class_start_sgc/ordinance_rewards.rs:99`), and `Action::CompleteMission` awaits `fire_mission_completed` inside the executor arm (`mission_1324.rs:803-806`). The 1664 candidate is mechanically plausible. What the world reads at fire time, and what a failed travel leaves behind, are still unproven. | code read | CS-06S |
| F18 | Production `MISSION_ADDITIONS` is one `const` array (`mission_additions.rs:14`), and `resources/tests/mission_additions.rs:125` asserts that it is empty. Cellblock's CB-N05 fills the same array, so both final PRs conflict there. | code read; Cellblock `cb-n05-seal.md` | D-CA17 |
| F19 | The contract holds every authored quest after the first until a live-client catalog proof. Under D-CA11, that proof only comes at the end UAT, which forces a second UAT round. Cellblock's own first proof (CB-N05) is BlockedEvidence on the user's CB-U09 run. | execution contract; Cellblock overlay | D-CA16 |
