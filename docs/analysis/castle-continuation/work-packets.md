# Castle continuation packet gate contract

> **Type:** Reference. **Audience:** Castle packet authors and reviewers.
> **Last updated:** 2026-10-14.
> **Companions:** [Castle ledger](README.md), [dated source contract](evidence/castle-contract.csv), [shared ID table](../four-zone-planning/id-allocation.md).

The [planning packet menu](../four-zone-planning/castle-work-packets.md) at baseline `d3efb7d41aad0b6008397f112d224ce7430d7acf` defines the twelve families, exact implementation allowlists, checks and commit subjects. This ledger freezes the **evidence boundary** of that menu. It does not convert unresolved fields into executable instructions. No runtime worker is Ready on the strength of CS-00 alone.

## Dispatch requirements

The coordinator must review and sign the [dated source contract](evidence/castle-contract.csv) **and any later scoped amendment below**, with mission/items advisor review, before CS-02/03/06/08 dispatch. Each `UNRESOLVED` field in the dated snapshot names its gate as of the CS-00 evidence baseline. Keep that historical snapshot and [reconciliation](evidence/reconciliation.md) unchanged; a dated amendment here supersedes only the exact fields it names. Record approver, source, date and affected rows. A blank value or a guessed mechanic is not a resolution.

Packet branches start at the recorded Castle feature SHA; one worker/worktree/database each. Follow the shared execution contract for local feature integration, wave CI, up to three independent review rounds, and the single final campaign PR. CS-00 ends at a local commit; the coordinator owns integration and review. Implementation packets target about 40k working tokens within the 90k context cap and split before dispatch if larger.

## CS-00 scope and acceptance

Allowed changes: this README, work-packets, evidence files and `worknotes/CS-00.md`. No runtime, seed, client, test or shared planning edits. Acceptance consists of source/seed reconciliation, explicit uncertainty, map provenance, namespace collision checks, links, whitespace and independent review. Evidence checks do not substitute for player UAT or prove a source-compatible server transition.

## Proposed cross-zone interface: 1664

Owner direction fixes **Castle opening/exit; Harset debrief**. Proposed division: CS-06 accepts mission 1664 once at Castle exit and transfers it with step 5224 active; HA-01b owns steps 5224–5226, including objectives 6063, 6059/6060 and 6061/6062. HA-00b must confirm this division, NPC tags, archetype predicate, prerequisites and failure/relog semantics. The exact Castle acceptance event is **UNRESOLVED G-1664**: do not silently append acceptance to chain 1360 or grant on any generic Harset arrival.

The 2026-10-14 mission review recommends a bounded Castle event: chain
1360 completes mission 708 during `stargate_crossed` while still in world 8,
which synchronously emits `mission_completed 708`. A separate chain could
accept 1664 at zero delay when world 8, 708 completed and 1664 not active;
the seed supports no archetype restriction. This is a **candidate
`CIMMERIA_RECONSTRUCTION`**, not a sealed grant. `mission_completed` occurs
before `perform_gate_travel`; a later transfer refusal can leave 1664 active
in Castle. The player can redial Harset because the address remains known,
but failed mission persistence has no automatic replay. HA-00b must seal the
receiver and Kaerlia placement, then the packet must test the real crossing
event/message order, duplicate and wrong-world/destination refusal, failed
travel retry, persisted relog and client journal/arrival. An observed failed
travel blocks the wave. This candidate does not prove Harset arrival or close
G-1664.

The seed still disables 1664 steps 5224–5226 and objectives
6063/6059/6060/6061/6062. The runtime mission loader ignores those enable
flags, so an in-memory 5224 state would not prove client catalog or journal
presentation. Objective 6059 is explicitly optional while 6060 is the
required door interaction; the draft HA-01b receiver's treatment of both
as mandatory must be reconciled before either side is dispatched. The seed
names Kaerlia in step 5224 and has Harset dialog map 7016→6169, but no
Kaerlia actor/placement join is signed. A completion-time grant also precedes
the actual gate transfer and its mission persistence is best-effort; the
packet must prove how a refused transfer or failed persistence retries
without duplicate grants or a stranded journal entry.

Do not grant 1664 twice on redial/re-cross or from Harset receipt. Castle must persist acceptance before the handoff; Harset must restore the same mission state on relog and must not finish Castle's opening again. These are proposed implementation invariants (`CIMMERIA_RECONSTRUCTION`), not recovered source behavior. Any implementation must prove them with chain replay and live-DB guards. Client journal and gate presentation remain UATPending.

## Other external owners

| Boundary | Owner | Receiver responsibility |
|---|---|---|
| 1360 accept and letter 3730 in Cellblock | Cellblock existing chains 1003/1121 | CS-08 seals 4037→4038; Harset retains 4038 recipient. |
| 567 Castle accept, 2000/2012 and item 2698 | Castle CS-08, conditional on G-567 | Harset retains 4039; disabled 6503/6504 are not enabled until the grant is proved. |
| Chain 1109 inbound ring | Ring-transport campaign | CS-01 checks arrival; direct teleport remains fallback. |
| CB-N05b recipient in Castle | Cellblock | Castle consumes full quest and adds no second recipient action. |
| Mission catalog insertion | Cellblock CB-N05a early standalone PR | CS-00B verifies merged API and first authored client proof. |
| Shared status docs | FZ-99 | CS-07 reports only Castle ledger/content/UAT changes. |

## Restoration tests to seal after gate closure

Use the exact focused commands in the planning menu after final allowlists are known. CS-02/03 require chain replay for ordered success, wrong archetype/step, failed/cancelled minigame, duplicate interaction and relog; live-DB verifies enabled steps/objectives and actor/dialog/item joins. CS-06/08 additionally test cross-world persistence and receiving-side restore. Prove guards fail when their new chain/action is removed. Record chain fired, step advance, objective credit and refusal telemetry events so SigNoz can diagnose the path. No compilation or synthetic runtime test belongs to this docs-only packet.

## Closeout status (CS-07a, 2026-10-18)

The packet menu above is the original gate contract. Where the packets stand now is in the README's [Packet status table](README.md#packet-status) and [Closeout state](README.md#closeout-state-cs-07a-2026-10-18): every R4 packet is integrated (CS-INT4S `25beef08e`, INT4.1 `9a3a3b076`, INT4.2 `887f5d48b`), CS-07R and CS-07R-ID are done, and CS-07b, CS-07c and the combined four-zone review remain. This file's gates (G-MED, G-SYM, G-567, G-ROUTE, G-REVIEW) keep their seal records; the open ones are listed in the README. Shared status docs still wait for FZ-99 ([worknotes/CS-07.md](worknotes/CS-07.md) lists the handoff rows).
## Seal record

### CS-08a dated amendment

The [2026-10-14 bridge seal](worknotes/CS-08a.md) is the exact contract for mission 1360 and supersedes only its unresolved Castle bridge fields in the CS-00 evidence snapshot. Owner approval closes G-1360 as CIMMERIA_RECONSTRUCTION: Castle world 8 `stargate_crossed` key `Harset`, active mission 1360/step 4037, exactly one advance to 4038, zero item mutations. It freezes archetype/708 policy, metadata rows, telemetry, tests and the implementation allowlist including `crates/services/src/mission_round_trip_tests.rs`.

At the CS-08a seal, individual chain ID 1366 was reserved exclusively for CS-08a by owner approval, 2026-10-14; the rest of the restoration block was then unsigned. Independent mission, items and persistence/Rust reviews signed the Castle bridge phase on 2026-10-14; the coordinator accepted that narrow G-REVIEW. CS-08a1's Castle seed/replay/persisted-hydration phase is now integrated. Playable handoff and CS-08a completion depend on HA-1360a/b, whose committed interface must be recorded before CS-08a2 recipient integration. Current 6501 does not satisfy the approved policy. Client checks remain UATPending. The later entry seal below claims 1367–1370 without marking another runtime packet Ready.

### 2026-10-14 owner decision reconciliation

[O14-03](../four-zone-planning/owner-decisions-2026-10-14.md) delegates bounded, source-reviewed `CIMMERIA_RECONSTRUCTION` for missing Castle wiring. G-ENTRY, G-MED, G-SYM and G-ROUTE still require an exact evidence/implementation seal; the delegation does not select an actor, item, minigame or coordinate. O14-22 delegates individual restoration-chain allocations in 1367–1400 after the stock catalog, shared-table, seed and all active zone branches are checked and the claim is recorded. Existing chain 1366 remains CS-08a's exclusive claim under O14-18. O14-08 is standing lab authorization for this campaign; coordinate a single shared lease and record the build and observed result. O14-09 permits exact `UATPending` rows only for unavailable lab attempts, never for observed failed gameplay.

O14-10/11 close the owner premise/copy/reward and player-label policy for lore-safe authored quests, subject to the first authored quest's catalog/journal/dialog/advance/relog client proof. O14-12/13 delegate conservative services and persisted repeatable limits with economy and authority review. The historical planning menu and dated CS-00 evidence snapshot remain the source claims they were when written; this paragraph controls subsequent dispatch.

### CS-02/03 entry-only seal; full missions remain blocked

The 2026-10-14 mission advisor and coordinator sign this **bounded
`CIMMERIA_RECONSTRUCTION`** after archaeology review. Legacy `Castle.py`
binds `Castle.Infirmary` only to room text 5188. Current
`fire_enter_region` supplies authoritative world, mission and archetype
context; server-known position validates the region hint. Mission 704 must be
completed. Chain **1367** offers 1520 in world 8, inside that region, only to
archetypes **1–4 and 6** while 1520 is not active; chain **1369** offers 1521
under the same region/world/prerequisite guards only to archetype **8** while
1521 is not active. Exclude archetypes 5 and 7 explicitly. Zero-delay
`accept_mission` persists the first step. The current dialog-choice dispatcher
also supplies archetype; older replay comments saying otherwise are stale, but
the chosen trigger is the source-grounded Infirmary region.

| Dated source-contract row and fields superseded | Signed replacement | Basis and reviewer, 2026-10-14 | Fields still unresolved |
|---|---|---|---|
| 1520: `archetype`, `start_condition`, `chain_id_block`, G-ENTRY/G-ID parts of `gate` | Archetypes 1–4 and 6; completed 704 + validated `Castle.Infirmary` enter in world 8 + 1520 not active; entry chains 1367/1368. | O14-03 reconstruction; `Castle.py:293–301`, `fire_enter_region`, current 704 completion chain 1295; Castle mission advisor and coordinator. O14-22 audited claim in shared table. | G-MED actor/game/report, full G-REVIEW, client UAT. |
| 1521: same fields | Archetype 8; completed 704 + validated `Castle.Infirmary` enter in world 8 + 1521 not active; entry chains 1369/1370. | Same source/owner decision and reviewers; items advisor signs only the no-item-mutation boundary. | G-SYM actor/game/implant, G-ROUTE, full G-REVIEW, client UAT. |

The old CSV's `UNRESOLVED` cells remain an accurate dated **source snapshot**;
the table above is the signed later contract for just these fields. It does
not alter mission/dialog/item/actor rows elsewhere in that CSV.

`accept_mission` replays the containing region for its newly active first
step (`accepting_a_mission_replays_the_first_steps_regions`). Chain **1368**
credits only 1520 objective 5171 while step 4452 and that objective are active;
Ogilvie's later interaction advances to 4453 and credits hidden objective
5172. Chain **1370** uses `advance_step 1521 4456` while step 4455 is active;
this credits its sole objective 5175. **Do not** call `complete_objective`
for 5175: the progression code can complete the whole mission when the
current step's sole required objective finishes. The entry subpacket must
exercise the real region dispatcher, accepted mission's immediate replay,
duplicate region hints, wrong world/prerequisite/archetype, relog and
reversion of each offer/credit chain. It must enable the exact shipped
step/objective metadata needed for the test, with client-visible state in
`docs/content/mission-chains.md`.

This seal reserves four individual chain IDs in the [shared table](../four-zone-planning/id-allocation.md),
after the documented cross-branch audit. The implementation is **not Ready**:
offering a mission whose dispenser/tank graph is missing would strand the
player. CS-02 and CS-03 first need exact G-MED/G-SYM actor/minigame/report
contracts, client route/placement evidence where relevant, domain authority
review, and the remaining packet allowlists. The entry chains must ship only
with a completable branch or remain disabled until that branch is ready.

### Remaining signatures

Coordinator and mission advisor: **signed only for the CS-02/03 entry reconstruction above**; G-REVIEW remains open for full mission graphs. Items advisor: **signed only the no-item-mutation evidence boundary for 1521**. HA-00b's 1664 receiver proposal is recorded; Castle grant and Kaerlia identity/placement remain **UNRESOLVED G-1664**. G-MED/G-SYM/G-ROUTE remain open. G-ID is closed for audited chains 1366–1370 only; no other restoration ID is claimed.

### 2026-10-14 stock prop supplement; no binding seal

The stock text seed names `DN_MsOb_Castle_mini_InfirmaryMedDispenser`
(moniker 8896, “Med Dispenser”),
`DN_MsOb_Castle_mini_InfirmarySymbioteTank` (8897, “Symbiote Tank”), and
`DN_Ob_ItG_Human_Castle_FileCabinet_Fac` (7547, “Romney's File Cabinet”)
at `db/resources/Texts/Seed/texts.sql:57107–57109,54162`. These are stronger
evidence for the intended object concepts than dialog verbs, but the checked
`entity_templates.sql`, `spawnlist.sql`, `event_sets.sql`, legacy `Castle.py`
and CA05 map worknote give no exact runtime actor/tag/event or pickup grant.
The number 7547 in `dialog_set_maps.sql` is a separate namespace, unrelated
to the file cabinet moniker. CA05 places four Symbiote Chamber prefab
instances around `(398.83, 55.04, 929–941)` without identifying the quest
tank; its Infirmary generator candidates do not match seeded region 2051.

For mission 1520, step 4454, task 6376 and objective 5174 describe the radio
report; dialog 4985 uses screens 96929–96931 through map 5826. No accept ID,
button, event set or trigger binds that report in the checked seed. For
mission 567, step 2000, objective 2338, tasks 2933/2934/5696 and item 2698
support the office-files pickup concept, but provide no acquisition event,
actor coordinate or grant. Source: `db/resources/Missions/Seed/mission_steps.sql`,
`mission_tasks.sql`, `mission_objectives.sql`,
`db/resources/Dialogs/Seed/dialog_screens.sql`, `dialog_set_maps.sql`,
`docs/analysis/castle-rebuild/worknotes/ca05.md:124–125,510–511` and the
2026-10-14 Castle archaeology review. This supplement leaves G-MED, G-SYM and
G-567 unresolved; any exact new binding needs a separately signed
`CIMMERIA_RECONSTRUCTION` and client check.

### CS-02/03 conditional graph candidates; not runtime seals

The 2026-10-14 mission and items reviews identified a bounded story-state
graph using existing stock steps and dialogs. For **1520**, the signed entry
chains offer the mission and credit 5171. A validated Ogilvie interaction at
spawn 197/template 179 could show dialog 4983; its guarded close advances
4452→4453 and credits hidden 5172. A separately reviewed Med Dispenser
interaction while 4453 is active could start a server-validated game; a
validated victory advances to 4454, crediting 5173, then shows 4984. Its
close could show radio dialog 4985; the guarded 4985 close completes 1520 and
credits 5174. The stock steps 4452–4454 and objectives 5171–5174 are disabled
and would need enabling; tasks 6372–6376 are already enabled. Dialog 4982 on
the synchronous 704 completion is a separate unsealed cue. The radio dialogs
have no recovered event binding, and displaying 4985 from the dispenser
could show the wrong actor portrait; client presentation must decide that.

For **1521**, the signed entry chains offer the Jaffa mission and advance
4455→4456, crediting sole objective 5175. A reviewed tank interaction while
4456 is active could start the validated game; victory advances to 4457,
crediting 5176, and shows dialog 4990. A later, separate server-owned tank
interaction could represent immediate pouch implantation by advancing to
4458, crediting 5177; a guarded Moh'katan radio dialog 4991 close could
complete 5178 and 1521. This reconstruction persists mission state only:
zero item grant/removal, pouch stat, effect, XP or cash action. Steps
4455–4458 and objectives 5175–5178 are disabled and would need enabling;
tasks 6377–6381 are already enabled. Dialog set 1572 maps 4989/4990/4991,
but exact radio and actor triggers are unrecovered.

The seed verbs “hack” and “bypass” do **not** identify a safe SmartFox game
type. Current `Hack` and `Bypass` use `PlaceholderGame`, where a client
`_cmd=victory` reports success without a server puzzle check; the legacy
Python versions also use Placeholder. Current `Livewire` validates moves on
the server. Under O14-03 the coordinator and minigame advisor sign **Livewire
and `INT_MinigameLivewire` cursor 256 as a bounded game-choice reconstruction
only** for these two beats. This does not seal an actor, route or complete
mission graph, and never claims the original game type. Client UI, win,
cancel and retry checks remain required.

`fire_chain_by_id` runs victory callbacks without evaluating their
`content_conditions`, while `advance_step` does not check the expected
current step. A narrow cell-content callback seam must resolve the loader's
synthetic `__direct_invoke_<chain_id>` custom event against the player's
**current** world and mission context, requiring world 8 and the exact active
step 4453 or 4456 before execution. It must fail closed if the entity is
missing or is not a player; the current result path can otherwise invoke a
chain as player 0. Keep legacy callback parity for chains without conditions.
Prove stale, duplicate, cancelled, failed, forged and cross-player results
cannot advance either mission, and require server-authority review. Source:
`crates/minigame/src/minigame/games/`,
`crates/content-engine/src/loader/mod.rs`,
`crates/cell-content/src/cell/content/event_dispatch/mod.rs`,
`crates/cell/src/cell/service/base_messages/minigame.rs`, and
`docs/content/content-engine-vocabulary.md`. These candidates remain
disabled until every branch is completable and G-MED/G-SYM/G-ROUTE and client
checks close.

The 2026-10-14 authority review found a second, upstream gap. The SmartFox
registry validates a login ticket and Livewire validates puzzle moves, but
Base currently uses the result ticket only to suppress a stale UI close and
forwards the result to Cell without a one-use receipt. `MinigameResult` carries
no original player ID, game name or ticket; Cell cannot reject entity-ID
reuse or a canceled/replaced session from that message alone. Split the
prerequisite: **CS-02a** adds an opt-in condition-aware callback resolver with
no Castle callback enabled; **CS-02b0** binds minigame Mercury sends to a
Base connection instance and cleans failed launches by exact ticket;
**CS-02b1** establishes one-use server-produced
result receipts in the registry and atomic Base redemption; **[CS-02b2](worknotes/CS-02b2.md)** proposes
a source-cap extraction, then a fenced Cell presence lifecycle (including
GateTravel source identity), then launch-generation binding through Start,
Cancel and result. These three child boundaries are Draft, with no worker
Ready. **CS-02c** may wire Castle callbacks after the b2 runtime gates and
other Castle evidence gates pass. Keep existing direct callback behavior for
unrelated chains. Callback mission actions must have `delay_ms=0`, because a
guard evaluated at receipt time does not authorize mutation after a later
deferred drain. No part of the Livewire game-choice signoff waives this
receipt/identity gate.

A b1 receipt proves that the registered server task emitted one terminal
`GameOutput`; it does not prove every game's puzzle result is trustworthy.
The current PlaceholderGame accepts client `_cmd=victory`, so Castle callback
activation still needs its separately reviewed Livewire choice and b2 identity
gate. The b1 packet must not describe receipt redemption as victory validation.

The b0 packet is an **integrated and independently reviewed implementation** in
[CS-02b0](worknotes/CS-02b0.md). The documentation-only
[CS-02b-prep](worknotes/CS-02b-prep.md) first moves long internal-message
Rustdoc into a linked protocol reference so the ticket-only b1 enum edit can
meet the 700-line source cap. Its packet review passed after a source-link check correction; its implementation passed independent diff and documentation review and is integrated at `d33a04041`. [CS-02b1](worknotes/CS-02b1.md)
is integrated after focused tests, revert proofs and independent diff,
authority and testing rereviews. b2 remains a Draft architecture candidate;
its proposed CS-02b2-prep/b2a/b2b split is not the integrated
CS-02b-prep message-reference extraction. The Cell-to-Base GateTravel request
and Base owner helper currently use only reusable entity ID, so b2a must
reject a stale source before travel side effects, across all five production
constructors. b2b must retain b1 receipt authority while binding it to the
accepted Cell presence and launch generation.
In b1, the minigame registry deposits at most one terminal
receipt while the claimed ticket is current, keyed by ticket and containing
the registered entity, player, game and callback chains. A ticket-only
internal notification lets Base atomically redeem it before forwarding.
Normal SmartFox teardown must preserve an issued receipt; public remove,
cancel or replacement invalidates it before redemption. The reviewed b1
contract selects 180 seconds and 4,096 receipts as fail-closed retention
bounds.
The preceding b0 packet rejects a queued `StartMinigame` whose entity and
player IDs do not match its current connected session, mints a Base
connection-instance token and binds each minigame URL/close send to a fixed
session snapshot. It cannot bind a late close to the original launch; b1
must retain that instance and the original entity/player in the result
receipt and compare all three before a close. b1 is partial hardening:
it cannot prove
Cell's current launch after a result has been redeemed.
**b1 does not authorize a Castle callback.**

For b1, persist b0's original Base connection instance in the
registered session and derive the receipt's entity, player, game, callback
chains and instance from that registration, with result and UI flag from the
server outcome. Deposit at most once under the registry lock while the claimed
ticket is current, then notify Base by ticket only. A missing, forged or
already redeemed ticket must produce zero UI sends and zero Cell forwards.
Normal SmartFox teardown preserves a deposited receipt; explicit cancel or a
replacement registration invalidates it even after teardown removed the
session. Bound retention with a concrete TTL and capacity, enforce both at
deposit/redemption/sweep and log fail-closed eviction with redacted tickets.
Test cancellation before/after deposit, teardown before Base drain,
replacement and relog before drain, forged notification fields, duplicate
redemption, closed-channel cleanup, TTL/cap eviction and deterministic
cancel-versus-redeem races. On valid redemption, both UI close and Cell
forwarding must use the receipt's **original** Base instance/entity/player.

In b2, a process-unique CellEntity incarnation and fresh Cell-owned launch
generation travel through the internal Start, Cancel and result messages.
Bind the Base connection-instance token to the Cell player at entity
creation and echo it on Start and Cancel. Base rejects a queued internal
Cancel unless its launch identity still matches. A duplicate Start must
retain the first board's generation when Base refuses the second request;
a refused first Start needs an internal
accept/refuse acknowledgement so Cell can clear pending state and retry.
The current `CreateEntity` reply returns only `space_id`, while
`EntityCreated` is trace-only; b2 must return the fresh Cell incarnation in
a correlated successful create acknowledgment and retain it in Base before
accepting Start or Cancel. Both login world entry and gate travel recreate
Cell presence, and gate travel can keep the same Base instance, entity and
player. A missing create acknowledgment leaves launch disabled rather than
falling back to a space ID. Stamp the incarnation at Cell entity birth before
the acknowledgment; when Base receives it, recheck the initiating Base
instance/entity/player before storing it, since an older create response can
arrive after connection replacement. Do not derive identity from a clock or
persist it as an entity property. Prefer a typed internal launch key containing
Base instance, Cell incarnation and Cell-owned generation; no token enters
entity properties or client Mercury. Define whether the Start ACK is queued
before the URL or how Cell handles a result before ACK, and compensate a
failed URL after an accepted Start. Destroy, disconnect and gate travel must
invalidate the current launch and receipt without touching a newer key.
The Cell result handler compares both
plus player ID to the current live player, clearing or advancing launch state
on cancel, replacement and destruction. The launch must also bind to a Base
session instance: checking player ID before `onEndMinigame` is insufficient
because the send helper looks up the recipient again, and the same character
may relog. Verify ownership at send for both launch URL and close. Wall-clock
`created_at` is not a uniqueness proof.
These changes must keep Mercury `onStartMinigame(URL)`/`onEndMinigame` and
the SmartFox login bytes unchanged. Tests need duplicate/redeemed outcomes,
teardown before Base drain, bounded receipt eviction, queued Start after
entity reuse, stale Cancel after relaunch, address takeover between recipient
check and send, duplicate Start while the first board runs, refused Start
then retry, cross-player entity reuse, same-player relog and same-entity
relaunch. `endCurrentMinigame` carries no board token: a queued internal
Cancel can be generation-checked, but an old client close RPC first received
after a new board becomes current remains ambiguous. Domain, engine,
minigame and authority advisors reviewed this architecture
on 2026-10-14; exact lifetime, file allowlists, ordering and build-lane
checks still need packet seals and independent review before dispatch.
Add b2 negative tests for an old-world Start after gate travel with the same
Base instance/entity/player, a dropped create ACK, first Start refusal then
retry, fast result before ACK, stale Cancel after relaunch, and result after
destroy/recreate. Each stale key must cause zero callback and zero close and
must leave the current registry board intact. The current Cell result handler
fires a missing-entity victory as player 0; guarded Castle callbacks must
fail closed there.

### CS-01 offline route boundary

`db/resources/Worlds/Seed/worlds.sql` marks Castle world 8's navmesh
`advisory`. Its chamber guards occupy a navmesh component apart from the
Infirmary, but this is not an enforceable player-walk rejection or proof of
an open door. Cooked-map inspection anchors are Ogilvie `(362, 70.18, 886)`,
`CA-SecurityLock00_Pf0` `(399.79, 58.91, 919.41)` and the chamber prefab
cluster `(398.83, 55.04, 929–941)`; no sourced stair/door or waypoint chain
connects them. The DHD `(806.27, 55.10, 517.24)` has a tested mesh path to
Bravo bunker, not from Armory. Gate-row arrival back from Harset is accepted
as unvalidated on this advisory world, so only the client can establish a
safe landing. G-ROUTE remains open for natural Infirmary→chamber and back,
Armory→DHD, and Harset→Castle→service-site walks, plus relog and observer
presentation. Source: `docs/analysis/castle-rebuild/worknotes/ca05.md`,
`docs/engine/castle-navmesh-connectivity.md`,
`crates/entity/tests/castle_navmesh.rs` and the 2026-10-14 Castle
movement review. No new `nav_inspect` route proof is claimed.
