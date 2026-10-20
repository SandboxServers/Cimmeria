---
name: Castle continuation evidence boundary
description: World-8 continuation evidence locations and unresolved mission contracts.
type: reference
---

# Castle continuation evidence boundary

At the CS-00 evidence baseline on 2026-10-14, [the reconciliation](../../../docs/analysis/castle-continuation/evidence/reconciliation.md) and [mission contract table](../../../docs/analysis/castle-continuation/evidence/castle-contract.csv) left start, actor, item, route and cross-zone handoff fields unresolved. That dated snapshot allocated no chain IDs.

Later on 2026-10-14, the [live packet contract](../../../docs/analysis/castle-continuation/work-packets.md#cs-0203-entry-only-seal-full-missions-remain-blocked) signed a narrow post-704 Infirmary entry reconstruction and [the shared table](../../../docs/analysis/four-zone-planning/id-allocation.md) claimed chain IDs 1367–1370. The dispenser, tank, item/pouch and route contracts remain open, so the offers are not ready to ship. In the progression implementation, completing 1521's first-step sole objective 5175 can complete the mission; advancing to step 4456 credits it without prematurely finishing the graph. [CS-01 client evidence](../../../docs/analysis/castle-continuation/uat/CS-01.md) observed world-8 loading and Ogilvie presentation, but not natural routes or mission flow. Mission 1664's Castle grant / Harset receipt mapping still requires HA-00b confirmation.

Later domain review found stock monikers for the Med Dispenser (8896), Symbiote Tank (8897), and Romney's File Cabinet (7547), but no runtime bindings. `Hack` and `Bypass` currently use client-declared PlaceholderGame victory; a [candidate Livewire substitution and callback-time guard](../../../docs/analysis/castle-continuation/work-packets.md#cs-0203-conditional-graph-candidates-not-runtime-seals) are signed only as a bounded game choice. Direct minigame victory callbacks skip content conditions, so a new chain cannot rely on its launch guard to reject a stale result. The full mission graphs remain held.

CS-02a integrated an opt-in guarded callback resolver, but no Castle caller uses it yet. Authority review of the current result path found that a queued Start can target a reused entity, a stale entity-only Cancel can remove a new board, Base's UI send re-resolves the recipient after a precheck, and receipts need bounded retention. The [b1/b2 architecture candidate](../../../docs/analysis/castle-continuation/work-packets.md#cs-0203-conditional-graph-candidates-not-runtime-seals) keeps callbacks disabled through one-use Base receipt and Cell launch identity work. A duplicate Start must preserve the first board's generation; a refused first Start needs an acknowledgement for retry. The tokenless client `endCurrentMinigame` cannot identify an earlier board when received after relaunch. Offline Castle records still do not identify which actor/prefab starts either minigame; the monikers and chamber coordinates are insufficient to bind an interaction.

The configured QA client's cooked interaction archives add a narrow check: category-8 maps 5824–5832 reference dialog sets 1571/1572 and success dialog IDs 4984/4990, with cache version 6615, but no Med Dispenser or Symbiote Tank actor join. The 41 generic category-20 interaction records also lack it. [The exact receipt](../../../docs/analysis/castle-continuation/evidence/cooked-interaction-maps.md) keeps G-MED/G-SYM open; map IDs are not actors.

A second QA archive search found no Castle dispenser/tank/cabinet join in the likely cooked Kismet/interaction records or targeted Castle UMAP name tables. Cooked map 2817 references dialog set 512/dialog 2044/topic Romney's Files, but [the receipt](../../../docs/analysis/castle-continuation/evidence/cooked-interaction-maps.md#romneys-files-is-a-recipient-topic) and seed show it is Copplemann's disabled Harset turn-in for mission 567, not the Castle pickup or item-2698 grant. G-567 stays open.

The [CS-02b2 architecture gate](../../../docs/analysis/castle-continuation/work-packets.md#cs-0203-conditional-graph-candidates-not-runtime-seals) now requires a correlated Cell incarnation acknowledgment from CreateEntity. Gate travel can recreate Cell presence while Base instance/entity/player stay unchanged; an old ACK must not overwrite a replacement session. A typed internal launch key needs Base instance, Cell incarnation and launch generation through Start/ACK/Cancel/result. The client close RPC has no board token, so an old packet first received after relaunch remains ambiguous.

CS-06/1664 remains G-1664: a separate world-8 mission_completed 708 chain is only a reconstruction candidate because completion and best-effort persistence precede gate transfer. HA-00b has not signed receiver state or Kaerlia actor/placement. Stock objective 6059 is optional; 6060 is the required door interaction, so a Harset draft treating both as mandatory needs reconciliation. Steps 5224-5226 and objectives remain disabled, and runtime loader ignoring those flags does not prove client journal presentation. [The Castle interface note](../../../docs/analysis/castle-continuation/work-packets.md#proposed-cross-zone-interface-1664) holds the exact gate.
CS-02b0's [Base send packet](../../../docs/analysis/castle-continuation/worknotes/CS-02b0.md) established the reusable Mercury boundary: the sender captures a connection instance and key, checks the current entity/player before ACK and reliable-sequence reservation, and gates post-send TX registration by the same instance. ACK requeue and sequence rollback use instance identity only, since a same-instance character switch after reservation must keep the reliable stream contiguous. A failed minigame URL cancels by its exact SmartFox ticket. This does not authenticate the original launch at result time; b1/b2 remain required.

CS-02b1's packet implementation stores a one-use result receipt behind the same registry mutex as cancel, replacement and redemption. The internal `CellToBaseMsg::MinigameResult` now carries only the ticket; Base redeems first, compares the original entity/player/Base instance before any output and again after the awaited close before Cell forward. Normal SmartFox teardown leaves the receipt, while explicit removal invalidates it. Connected sessions retain a terminal-once marker after redemption or expiry; receipt TTL is 180 seconds and the cap is 4,096. A receipt identifies a reported server task, not a puzzle-validated victory or current Cell launch; CS-02b2 still owns Cell incarnation and generation. See the [packet worknote](../../../docs/analysis/castle-continuation/worknotes/CS-02b1.md).

The b1 review exposed a cancel/deposit classification race: reading the cancel watch before awaiting the registry mutex can mislabel a cancel-first refusal as `result_refused`. The terminal dispatch now carries the cancel state observed while that mutex decides refusal; lock-ordered tests cover cancel/deposit and cancel/redeem. The [packet worknote](../../../docs/analysis/castle-continuation/worknotes/CS-02b1.md) records the review checks and Base TTL/cap route guards.

2026-10-14 status: CS-02b1 merged into the Castle feature branch at `4f45ee100` after independent diff, authority and testing rereviews. The integration all-target check passed. The [packet worknote](../../../docs/analysis/castle-continuation/worknotes/CS-02b1.md#integration-record) keeps the exact checks; CS-02b2's Cell launch identity remains open.
At Castle integration HEAD `2bbea153c`, `CellToBaseMsg::GateTravel` and
`gate_travel/session_owner.rs::owns` use only reusable entity ID. The five
production travel constructors are in cell-interactions gate travel, space
transfer and respawn, plus cell-content ring transport and content transport.
A delayed old request can therefore reach Base after a same-ID Cell presence
is replaced. The [CS-02b2 Draft](../../../docs/analysis/castle-continuation/worknotes/CS-02b2.md)
proposes a source-presence check before travel side effects. It also records a
three-stage planning split: source-cap extraction, complete Cell presence
lifecycle, then minigame-launch binding. None of those stages is Ready.
The current create reply returns only space ID, and its instanced-world path
awaits a Cell-to-Base `SpaceData` send before replying; an occupied channel
can deadlock Base's serial GateTravel handler. This must be sealed with a
correlated reply and channel regression before runtime dispatch.
