# Bank and Vault Restoration

> Type: how-to. Audience: the Claude Code coordinator and implementing engineers.
> Updated: 2026-09-27. Tracking issue: [#859](https://github.com/SandboxServers/Cimmeria/issues/859). Companions: [audit](audit.md), [work packets](work-packets.md), [session resume](handoffs/session-resume.md), [organizations ledger](../organizations/work-packets.md) (org vaults build on its API), [documentation index](../../readme.md).

## Purpose

This campaign restores banking. It covers the personal vault (container `INV_Bank` = 17) opened at a Banker NPC, and, in a later wave, the Team (19) and Command (20) organization vaults and treasuries.

The client already has the whole UI: `Vault.lua`, `Team.lua` and `Command.lua`. The work is server-side only, so **no client patch is needed** for any packet in this ledger.

Out of scope:

- the auction container (18), which belongs to the Black Market;
- a UI field for an org cash-withdrawal cap. That needs a client patch plus a new client-to-server message, which is a maintainer decision. It is parked; see D-BV15.

## What was found

Against `main` @ `004bccb4`. The [audit](audit.md) has the evidence for each row.

| Area | State on `main` | Packets |
|---|---|---|
| Client window | Exists. It opens only when the server sends `onVaultOpen` (106). Closing it sends nothing. | (reused) |
| Opening at a Banker | Missing. The client sends a generic `interact`, and nothing on the server answers a Banker. `NpcInteractionType` has no Banker variant. The `ON_VAULT_OPEN` constant exists but is never called. | BV-02 |
| Capacity | Contradictory. `BAG_SIZES` tells the client container 17 has 100 slots. `bag_max_slots` says 0, so every `moveItem` into 17-20 is rejected. | BV-01 |
| Loading contents | Already works. Login loads every `sgw_inventory` row whatever its container and sends it in `onUpdateItem`. | (reused) |
| Move authority | Missing for the bank. Also, the buyback container (16) can be moved out of for free, which is issue #798, in the same function. | BV-01, BV-03 |
| Expansion (40 to 100) | The client has no expansion UI. The visible size comes only from the container size the server declares. | BV-05 |
| Org vaults and treasury | Blocked on the organizations campaign: there is no org table yet (#568). | BV-07 to BV-10 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-BV01 | **APPROVED** (owner, 2026-09-27) | No bank fees. | The client has no fee UI, and the legacy server charged nothing. |
| D-BV02 | **APPROVED** (owner) | The personal vault starts at 40 slots. It grows in +10 steps up to 100, each step bought at a Banker. The price is a placeholder, 100 naquadah per step, stored in a seed table so it can be tuned without code. | A `Team.lua` comment reads "40 to 100, in intervals of 10". The client shows whatever size the server declares (audit A-09). |
| D-BV03 | **APPROVED** (owner) | The vault opens at a Banker NPC. A GM-only `.bank` console command also opens it anywhere. `isBankingOverride` is not used, because nothing reads it (audit A-12). | UAT needs a way to reach the bank from anywhere, and players should need a Banker. |
| D-BV04 | **APPROVED** (owner; crafting input) | Vendors, trade, mail and crafting use only carried inventory; the bank is storage only. Items that are only stored, such as Field Crafting Tools, may sit in 17. Container 15 (Crafting) stays movable to and from 1. | This keeps bank items out of every other service, which matches the existing trade whitelist. |
| D-BV05 | PROPOSED (coordinator) | **Vault session.** Opening the vault pins the Banker. Every move into or out of 17 needs an open session **and** a fresh proximity check against the pinned Banker, within the interact distance, on the same space. A space change, a logout or a new interaction target ends the session. A GM session opened with `.bank` has no Banker and skips the proximity check. | CAT-D-02: loot pinned the target once and never checked again. The client sends nothing when the window closes, so the session must end on the server's own signals. |
| D-BV06 | PROPOSED | **One capacity source.** `bag_max_slots` in `crates/wire/src/containers.rs` is the only table, and `BAG_SIZES` derives from it. The personal vault's size is per player, in `sgw_player.bank_slots`: `smallint NOT NULL DEFAULT 40`, checked to be between 40 and 100 and a multiple of 10. It is declared in `onBagInfo` at world entry and on resync. | Two tables drifted apart on 17-20 (audit A-04). |
| D-BV07 | PROPOSED | **Player-movable containers** are an explicit allowlist in the move path. Moves into or out of 16 (buyback) are refused, which closes #798. 17 is movable only through the vault session. 18, 19 and 20 are refused until their own wave. Every move that is legal today stays legal. | CAT-D recommends validating through the owning service, and #798 is the same function. |
| D-BV08 | PROPOSED | Mission items cannot enter the bank. Bound items can. | A bank is where bound items are kept safe. Mission items drive quest state. |
| D-BV09 | PROPOSED | A Banker is `INT_BANKER` (bit 2) plus a new `entity_templates.vault_scope` column: `personal` by default, or `team` or `command`. The legacy `EInteractionType` numbering is not used. | There is no Team or Command banker bit. The legacy id numbering is not kept in sync with the bitmask (`spawn.rs` warns about this). |
| D-BV10 | PROPOSED | The personal vault opens in the cell, the same way as the trainer: pin the Banker, then send `onVaultOpen(banker_id, banker_position)`, with no base round trip. BV-E1 confirms whether `onBagInfo` for 17 has to be re-sent first. | Container 17 rows are already loaded at login. |
| D-BV11 | **APPROVED** (owner) | Two colo releases: one when the personal-bank waves (BV-01 to BV-06) merge, and one when the org vaults merge. `/release` is posted from PowerShell. | Lets the owner test the personal bank without waiting on the organizations campaign. |
| D-BV12 | **APPROVED** (owner) | Org vault permission defaults: the leader has every bank bit. Default ranks get `DepositBank`, `DepositCash` and `ViewBankLogs`. Withdrawal is opt-in. | Stops members emptying a vault. |
| D-BV13 | **APPROVED** (owner) | An org cannot disband while its vault holds items or cash. The refusal carries a visible reason. | No item loss, no duplication, no mail dependency. |
| D-BV14 | **APPROVED** (owner; was D-ORG17) | The Team vault grows from 40 to 100 in +10 steps. The Command vault is fixed at 100. | The `Team.lua` comment for Team, and legacy `BAG_SIZES` for Command. |
| D-BV15 | **APPROVED** (owner; was D-ORG19) | No cap on org cash withdrawals; every transfer is logged for `ViewBankLogs` holders. A UI cap field is a parked follow-up: the Lua edit is small, but it needs a new wire message and a maintainer decision. | The client has no cap UI. |
| D-BV16 | **APPROVED** (owner, via cimmeria-19; confirmed by the user) | Autonomous run. Workers use isolated worktrees and their own test databases. Each PR is squash-merged once CI is green. | Owner kickoff. |
| D-BV17 | PROPOSED | Issue #798 is fixed inside BV-01. | It is the same allowlist in `move_/mod.rs`. |
| D-BV18 | **APPROVED** (owner, 2026-09-27) | When an org's last member deletes their character and the org's vault is not empty, cimmeria-fa's D-ORG12 trigger keeps a memberless org that still holds the vault, for GM recovery. Vault tables do not cascade-delete with the org. An empty vault disbands normally. | This is the one disband path D-BV13 cannot refuse. Nothing is silently destroyed. |
| D-BV19 | **APPROVED** (owner rule, relayed by cimmeria-19, 2026-09-27) | Telemetry is first-class. Every packet must make "player X did Y at time T and it failed" answerable from SigNoz alone. That means the path that ran, a stable `reason=` on every refusal, the before and after values for cash, items and capacity, and correlating ids (`account_id`, `player_id`, `entity_id`). It follows `instrumentation-discipline.md`, `negative-logging-convention.md` (a `LogCapture` test per seam) and `observability.md` (the `OTEL_FILTER` rows). | So the owner can debug from telemetry, with no repro and no debugger. |
| D-BV20 | **APPROVED** (implemented in BV-01, PR #872) | D-BV06 as built. The one capacity table is `cimmeria_entity::inventory::bag_max_slots`, not a table in `crates/wire/src/containers.rs`: `cimmeria-wire` depends on `cimmeria-entity`, so `BAG_SIZES` can only be derived from a table in the lower crate. `cimmeria_wire::containers::bag_max_slots` (and `base::resources::bag_max_slots`) re-export it. 17-20 are all 100, as in `Constants.py`. `sgw_player.bank_slots` is as D-BV06 specifies and is declared for 17 in `onBagInfo` at world entry and on resync. | Supersedes D-BV06's table location only. |
| D-BV21 | **APPROVED** (implemented in BV-01, PR #872) | D-BV07 as built. `player_movable(container_id) -> Movable {Yes, No, VaultSession}` in `move_/container_policy.rs` is checked for the target before the slot-range check and for the source after the locked row read. `Yes` is 1-15, `No` is 16, 18, 19, 20 and every unknown id, and 17 is `VaultSession`, which refuses until BV-03 wires the session. A refusal resends only the refused item, under the per-player move lock and a `FOR UPDATE` lock on the item's row. Grants into 17-20 are refused too, because loot and content grant into an item's first `container_sets` entry, which is 17 for the seeded `{17,15}` items. #798 is closed by it (D-BV17). | Supersedes the PROPOSED status of D-BV07 and of D-BV17 (#798 is fixed in BV-01, as D-BV17 proposed); adds the grant refusal. |
| D-BV22 | ADOPTED (coordinator; implemented in BV-02, PR #921) | **Banker precedence.** `INT_BANKER` wins over the vendor bits in `static_interaction_for_flags`. Anything that answers before the static dispatch still wins: a trainer list, an `interact_tag` or `interact_template` chain, a per-player dialog bind, and `INT_DHD`. | The banker bit names one role; the nine vendor bits are store tabs a quest giver can carry as decoration. |
| D-BV23 | ADOPTED (coordinator; implemented in BV-02, PR #921) | A non-GM's `.bank` gets the bank-specific refusal ("`.bank` needs GM access. Visit a Banker to open your vault.", `vault_open_rejected reason=not_gm`) ahead of the generic non-GM console refusal. It is the one `.`-line a non-GM types that is answered rather than broadcast; `.bankdump` from a non-GM gets the generic line. | The refusal tells a player where the vault really is. |
| D-BV24 | ADOPTED (coordinator; implemented in BV-03, PR #935) | **The cell-to-base vault verdict.** The cell computes `cimmeria_wire::cell::vault::VaultAccess` (`Open { scope, banker_id, distance }` or `Closed { reason, banker_id, distance }`) on every forwarded `MoveInventoryItem`, `UseInventoryItem`, `RemoveInventoryItem` and `RemoveInventoryItemByType`, with a fresh proximity check, and attaches it. The base consults it only where 17 is touched. Only an `Open` session of scope `Personal` opens 17; any other scope is `vault_scope_mismatch`. A base entry with no verdict (the in-process right-click auto-equip) passes `VaultAccess::NO_SESSION`, so it fails closed. | The session and positions live in the cell, the transaction on the base. No round trip, and no base-side session mirror to go stale. |
| D-BV25 | ADOPTED (coordinator; implemented in BV-03, PR #935) | **Stack merge** is legacy parity across all containers, not only the vault. It needs the same type, room under `max_stack_size`, and equal `bound`, `durability` and `charges`. A partial merge is allowed. | A bound stack merged into an unbound one would become sellable, tradable and mailable. |
| D-BV26 | ADOPTED (coordinator; implemented in BV-03, PR #935) | Content by-type removal (`RemoveInventoryItemByType`) never searches container 17, even with the vault window open: it always sends `NO_SESSION`. Removal by instance and `gmRemoveItem` take the live verdict. | Otherwise whether a turn-in eats a banked copy would depend on UI state. D-BV04: the bank is storage only. |
| D-BV27 | ADOPTED (coordinator; BV-03, PR #935) | **`bank_slots` only grows.** It is read without a lock in the move transaction and in `reservable_slots`, which is safe only while it never decreases. BV-05 must keep it grow-only; anything that shrinks it (a GM "set bank slots", say) needs `FOR SHARE` at those reads, or the move lock. | A stale value can then only refuse a slot the player is about to own. |
| D-BV28 | **APPROVED** (owner, 2026-09-27) | **BV-09 payer.** A Team vault +10 step (100 naquadah, the D-BV02 price) is paid from the org treasury, `sgw_organizations.cash`, and only the leader may buy it. No new permission bit. | Answers BV-09's open question. |
| D-BV29 | ADOPTED (coordinator; from the social campaign's D-SS07) | **Vault mail aliases** are Wave 4 work, after BV-07. The seam is `resolve_recipient_flags` in `crates/base-methods/src/base/world_entry/methods/mail/send/mod.rs` (SS-M1, PR #894), which the Bank campaign replaces; it also owns `VaultButNoItem`, `VaultPlusCash` and `SentToVault`. Until then any `MAIL_ToVault` bit is refused with `reason=vault_alias_unsupported`. `MAIL_ToCommandRank6` is 4092, not 4096, so it carries the vault bit (social audit A-12); BV's replacement must decide what that value means. | The social campaign shipped the refusal and handed the seam to us. |
| D-BV30 | **APPROVED** (owner, 2026-09-27; as built in SS-M2, PR #912) | Mail attachments come from the carried bags only: `MAILABLE_CONTAINERS` is the backpack (1) and the crafting bag (15). Sources 16 to 20 are refused with `item_in_buyback` or `item_in_vault`. | D-BV04 applied to mail. The owner made 15 a mail source for crafting components. |

PROPOSED rows are adopted at these defaults unless the owner objects. A change is recorded as a new row, never by editing an old one.

## Coordinator launch prompt

You are the Claude Code coordinator for the bank campaign. Implement [work-packets.md](work-packets.md) as small reviewed PRs.

1. Record `git rev-parse origin/main` and re-check the audit's file references. Before touching the shared files in [work-packets.md § Contended files](work-packets.md#contended-files), message cimmeria-23 (crafting, formerly cimmeria-af) and cimmeria-1f (organizations, formerly cimmeria-fa).
2. **Wave 0:** BV-01 and BV-E1, in parallel.
3. **Wave 1:** BV-02, once BV-01 is on `main`.
4. **Wave 2:** BV-03 and BV-04, once BV-02 is on `main`.
5. **Wave 3:** BV-05, then BV-06, which carries the first `/release`.
6. **Wave 4:** BV-07 to BV-10. BV-07 builds on ORG-02 (#881) and ORG-06 (#941), both merged; it does not need ORG-07 (confirmed by cimmeria-1f). BV-08 waits on ORG-07, which brings the CM 19 cell forward.
7. Every worker gets its packet, the contract section and the audit rows it cites, plus the worker rules file `%TEMP%\cimmeria-castle\BANK-WORKER-RULES.md`. Workers push; the coordinator opens the PRs, reviews them, and merges them on green CI. Copilot review is suspended while its spend is exhausted, so the coordinator's review stands in for it.
8. When blocked, leave `handoffs/<packet>.md` with the exact next action, and update [session-resume.md](handoffs/session-resume.md).

## UAT

The owner runs the checklist in [session-resume.md](handoffs/session-resume.md#uat-checklist) on the colo after each release, as a GM, and uses `.bug <note>` at each oddity. The coordinator then reads SigNoz for the `bank` target.

## Where confidence is low

- Whether a mid-session `onBagInfo` really resizes an open window. BV-E1 found the `InventoryUpdateContainerSize` subscription but not the native emit, so this is inferred until UAT confirms it. The world-entry declaration alone is enough when the vault opens.
- The Expand dialog's button is UI only. The server does not check `button_id`, so BV-05's purchase handler must enforce every rule itself (BV-E1 review).
- Org vault storage: `sgw_inventory.character_id` is `NOT NULL`, so org-owned items need a table or ownership design (BV-07, with the database-persistence advisor).
- The `organizationTransferCash` arguments. It is **cell** method 19 (`docs/protocol/cell-method-dispatch-table.md:142`, `crates/wire/src/cell/cell_methods/organization.rs:17`); an early research pass read the client method table instead. cimmeria-1f's ORG-07 owns the decode and forwards `TransferCash { player_id, entity_id, org_id, amount }`.
- `onOrgMoveItemResult` and `onClearOrgVaultInventory` have no Lua subscriber and may be dead protocol (audit A-13).
