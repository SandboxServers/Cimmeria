# Crafting: Session Resume

> Type: how-to. Audience: any later session, the coordinator and the owner.
> Updated: 2026-09-27 (CR-13 close-out). Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md), [worknotes](../worknotes/), [unified UAT guide, Crafting](../../../guides/unified-uat.md#crafting), [crafting system reference](../../../gameplay/crafting-system.md).

## State: every planned packet merged; CR-13 close-out in review; follow-ups CR-18 and CR-19 in progress; CR-14 owner UAT after the release

| Packet | Status | PR | Notes |
|---|---|---|---|
| Plan | Integrated | #851 | This ledger; ledger syncs #866 (after wave 0) and #899 (after wave 1) |
| CR-E1 | Integrated | #858 | Client evidence: respec flow, `CraftingOptions`, feedback paths, client clock |
| CR-E2 | Integrated | #868 | Blueprint-item mapping (193 resolved), Paradigm Guides already seeded, tools name-only |
| CR-01 | Integrated | #862 | Catalog, enums, serializers, argument parsing, request forward |
| CR-02 | Integrated | #864 | One game clock; no client patch needed (D-CR24); closes #271 |
| Telemetry contract | Integrated | #877 | The work-packets "Telemetry contract" and the first CR-14 queries |
| CR-03 + CR-04 | Integrated | #884 | Login bundle, ASP total, paradigm defaults, discipline learning |
| CR-05 | Integrated | #895 | Station gate, Field Crafting Tools, 140, `.allcraft` |
| CR-06 | Integrated | #897 | Induction engine and the consume-and-grant transaction |
| CR-12 | Integrated | #900 | 1 ASP at level 1, +1 per level gained |
| CR-15 | Integrated | #902 | Blueprint items and Racial Paradigm Guides |
| CR-08 | Integrated | #903 | Research and reverse engineering |
| CR-09 | Integrated | #904 | Alloying |
| CR-07 | Integrated | #905 | Craft |
| CR-11 | Integrated | #909 | Debug-hub stations, supplies vendor, `.craftkit`, `.learnblueprint` |
| CR-16 | Integrated | #932 | Grants fall through storage to the first carried bag; refused loot stays on the corpse |
| CR-17 | Integrated | #953 | Trade from the crafting bag (D-CR28); vendor buyback and sell lock order |
| CR-10 | Integrated | #979 | Two-step respec (`.respeccraft`, then the prompt's Yes); refund bound to the ASP spent |
| CR-13 | Review | #983 | This close-out |
| CR-18 | Writing | branch `craft/cr18-research-refusal` | Research with no eligible discipline refused before anything is consumed (D-CR29) |
| CR-19 | Writing | branch `craft/cr19-station-click` | Right-clicking a station opens its crafting window (D-CR30); whether a client patch is needed is still open |
| CR-14 | BlockedDependency | | The owner's UAT, [below](#cr-14-owner-uat-checklist), after the release deploys |

## Owner decisions

All six planning questions were answered on 2026-09-26 and are recorded in the [README](../README.md#owner-decisions): ASP 1 at level 1 plus 1 per level; free full respec; Common 5 and Racial Paradigm Guide items; blueprints from Blueprint items and research; stations and Field Tools; reverse-engineer recovery that rises with expertise. On 2026-09-27 the owner added D-CR28: crafting components live in the crafting bag, and that bag trades (CR-17) and mails (the social campaign, #933).

The two questions the close-out raised were answered on 2026-09-27:

- **D-CR29:** a research with no eligible discipline is refused before anything is consumed, reversing the legacy behaviour CR-08 kept. Follow-up CR-18.
- **D-CR30:** right-clicking a crafting station opens its relevant crafting window. The stations need interaction bits and a server answer to the click; whether a client patch is needed is still to be determined. Follow-up CR-19.

`/release` goes on whichever of CR-13 (#983), CR-18 and CR-19 merges last.

## Coordination

- Coordinator session: cimmeria-23 since 2026-09-27 (cimmeria-af until a system restart ended it). Campaign kicked off by cimmeria-19 for the owner.
- ID blocks: crafting templates 310-329, spawns 410-429, item list 310 and list rows 3101-3124 (floors 329 and 3299); debug hub (#846) 300-304 / 400-404; Harset 200-299 / 300-399; guilds 330-349 / 430-449; pets 350-369 / 450-469.
- Merge rule: squash-merge after green CI and the coordinator's own review. The Copilot-review-before-merge rule is suspended since 2026-09-27.
- Owner rule (2026-09-26): telemetry is a first-class deliverable (D-CR27); every packet has a telemetry acceptance line, and CR-14 carries the SigNoz queries.
- Worker rules: `%TEMP%\cimmeria-castle\CRAFT-WORKER-RULES.md`.

## Consumers of the shared inventory lock helper

`crafting::inventory_locks::take_inventory_locks(conn, player_id, bags)` (CR-15) takes the player-wide advisory key 0, then each bag's key in container order. Its order is the one every inventory writer must follow: advisory keys, then item rows, then `sgw_player`.

| Consumer | Bags | Merged in |
|---|---|---|
| Crafting transaction (`transaction/grant.rs`), item use (`item_use/transaction.rs`) | 1, 15 (and each product's bag) | #897, #902 |
| `.allcraft`, `.learnblueprint` | key 0 only | #909 (the `.allcraft` fix rode with CR-11) |
| Respec, spend, GM expertise grant | key 0 only | #979 |
| Mail send escrow (`mail/send/escrow.rs`) | 1, 15 | #912 (bag 15 added in #933) |
| Mail take and pay (`mail/claim.rs`), system mail (`mail/system/write.rs`) | the item's bag | #926, #929 |
| Trade swap (`trade/execute/swap.rs`) | 1, 15 | #953 |
| Vendor buyback and sell | 1, 16 (keys 0, 1, 16) | #953; this closes the inverted order reported in #928 |
| Black-market escrow (`black_market/escrow.rs`) | its container | #971 |

Moves (`inventory/move_/`) and vendor purchase take key 0 with their own inline query, in the same order. The generic grant path (`inventory/grant/persist.rs`: loot, content `grant_item`, `gmGiveItem`) takes only the per-bag key, which serializes it with every writer of the same bag; nothing found deadlocks with it.

## CR-14 owner UAT checklist

Run on the colo after the release deploys, as GM (access level 2 or higher), in the stasis-room debug hub of `Castle_CellBlock` (world 12), from a **new character**. At each oddity type `.bug <what you see>`. Step 17 (trade) needs a second player.

**Before you start.**

- Note the time you log in. Your character's `player_id` is on its `login_sync` row (`scope_name = 'crafting' AND event = 'login_sync'` at that time); every SigNoz query below filters on it.
- The crafting corner stands along the wall opposite the main hub row: the **Common Materials Components** vendor (1 naquadah per item) and four **BioMedical / Electronics / Power Systems / Materials Crafting Stations**. Any station allows all four verbs; the science in its name is a label. You are "at" a station within 5 units.
- `.allcraft`, `.craftkit` and `.learnblueprint` act on your **selected target**, never on you by default: select your own character first (click your portrait). If the client will not let you target yourself, have a second GM target you, and note it.
- Supplies land in the crafting bag (bag 15), except Crafted Pistol of the Whale (5481), which lands in the backpack. A stack of several non-stacking items bought in one purchase shows as one stack; that is a known vendor limit, and crafting counts it correctly.

| # | Do | Expect |
|---|---|---|
| 1 | Log in with the new character. Open the Applied Science window (Ctrl+J) | The ASP count reads 1. The tree is drawn, and the four root disciplines (Biomedical, Electronic, Power Systems, Materials Engineering) are learnable: every character starts at Common paradigm 5 |
| 2 | `/gmgiveappliedsciencepoints 5` | Chat: "gmGiveAppliedSciencePoints: +5 (total 6)". The count in Ctrl+J reads 6 without a relog |
| 3 | `/gmgivexp <amount>`, enough for at least one level | ASP rises by one per level gained, live. XP that crosses no level adds nothing |
| 4 | In Ctrl+J learn Materials Engineering, then click it again; then learn Biomedical Engineering | Materials Engineering shows expertise 1 and ASP drops by 1. The second click says "You already know Materials Engineering." Biomedical Engineering is learned, ASP drops by 1 more |
| 5 | Relog | Both disciplines at expertise 1, the ASP count and every blueprint are still there |
| 6 | With nothing in the crafting bag, open the Crafting window (J) away from the stations; walk to any Crafting Station; walk away | Away: the craft, research, reverse-engineering and alloy pages are unavailable. At the station they enable; away again they disable |
| 7 | Buy an **MAS-5 Field Crafting Tool** (8406); walk away from the stations. Then open your vault (`.bank`) and move the tool into it; move it back | The tool lands in the crafting bag. Away from the stations, craft, research and reverse engineering enable, alloy stays unavailable. With the tool in the vault they disable; back in the crafting bag they enable again |
| 8 | Buy **Blueprint: Steel Plating (Materials Subcombine A)** (6483) and use it (right-click). Buy a second one and use it | The first is used up and blueprint 25 (Steel Plating) appears in the J window. The second: "You already know this blueprint. The item was not used." It stays in your bag |
| 9 | In Ctrl+J click **Regenerative Energetics**. Buy **Racial Paradigm Guide: Goa'uld** (7808) and use it. Click Regenerative Energetics again | Before: "Regenerative Energetics requires Goa'uld paradigm level 2; yours is 1." The guide is used up (it prints no line of its own). After: "Regenerative Energetics requires Biomedical Engineering at expertise 50; yours is 1.", so the paradigm gate passed |
| 10 | Buy 13 **Steel Core (Materials)** (5254), or `.craftkit 25` with yourself selected. At a station, craft Steel Plating (blueprint 25), quantity 1. If you have 13 more cores, press Craft again during the bar | The 3-second induction bar shows. The cores go, and "You crafted Steel Plating (Materials Subcombine A) x1." Materials Engineering expertise goes from 1 to 2. A second craft pressed during the bar says "Crafting Steel Plating (Materials Subcombine A) x1 is queued behind 1 other crafting job(s)." and runs after the first |
| 11 | Put only 12 Steel Cores in the recipe and confirm | "You do not have enough components: 12 of 13 needed. Nothing was used." Nothing leaves your bags. If the client will not send the request at all, note that instead |
| 12 | Buy **Crafted Pistol of the Whale** (5481) and the **Materials Engineering Research Kicker** (5671). At a station, research the pistol with that kicker. Then try a second research with the **BioMedical Engineering Research Kicker** (5668) | The first: after the bar, "Research succeeded: Biomedical Engineering expertise increased to 6. You learned 1 new blueprint." The pistol and kicker are used; blueprint 1 appears in J. The second is refused at once: "Kickers cannot come from the same applied science as the item being researched. Nothing was used." |
| 13 | Buy 10 more pistols (5481). Put all 10 on the reverse-engineering page and confirm | Ten inductions run one after another; each ends with "Reverse engineering complete: recovered N components." (N at least 1), the pistol goes, and the components land in the crafting bag |
| 14 | `.learnblueprint 42` with yourself selected. Buy 1 **Cell (Bio-Medical)** (5192) and 5 **T1 Cell (Bio-Medical)** (5189). At a station, alloy blueprint 42 with the Cell as the current-tier item and the five T1 Cells. Repeat with only four T1 Cells | GM line: "learnblueprint [...]: blueprint 42 learned (... known)." The alloy: "Alloying complete: 2 x Blend (Bio-Medical Alloy)." and Biomedical expertise rises by 1. With four: "The quantity of elementary components per item quality was not met: 10 Normal, 5 Good, 2 Great or 1 Fantastic. Nothing was used." and your bags are resynced |
| 15 | Start a craft that takes components, and log out during the bar. Log back in | Nothing was consumed and no product arrived |
| 16 | Kill the **Crate** and loot it. Then fill the crafting bag (100 slots) with `.craftkit` kits: `.craftkit 25 <count>` adds 13 items per count, `.craftkit 42 <count>` one per count, and a kit that does not fit is refused whole, so finish with small ones. Kill the Crate again (it respawns 30 s after death) and loot its Cell | First loot: the Cell (5192) lands in the crafting bag; sometimes a guide or Blueprint: Steel Plating drops too. With the bag full: "Your crafting bag is full. The item was left on the corpse." and the Cell is still on the corpse |
| 17 | Two players, A and B. A offers a crafting component from the crafting bag in a trade; both lock and confirm. Repeat with B's crafting bag full | The component leaves A's crafting bag and lands in B's crafting bag. With B's bag full the trade closes for both: B reads "Trade cancelled: your crafting bag does not have room for the items you would receive.", A reads "Trade cancelled: your trade partner's crafting bag does not have room for your items." Nothing moves |
| 18 | Type `.respeccraft` and answer Yes. Then type `.respeccraft` again. Then learn a discipline, type `.respeccraft`, wait more than 60 seconds and answer Yes | The prompt shows a cost of 0. After Yes every discipline reads expertise 0 in Ctrl+J, and the ASP you spent learning disciplines (two points in step 4) comes back; blueprints (25, 1, 42) and the Goa'uld paradigm stay. The second `.respeccraft` says "You have no crafting disciplines to unlearn. Nothing was changed." The late Yes says "The crafting respec was not confirmed within 60 seconds. Type .respeccraft to start again." |
| 19 | `.allcraft` with yourself selected | GM line "allcraft [...]: N disciplines at 100, M blueprints, 5 paradigms at 7; craft anywhere is on until logout." Every page enables anywhere, and every discipline shows 100 |
| 20 | **After CR-18 (D-CR29).** With every discipline at 100 from step 19, buy another Crafted Pistol of the Whale (5481) and research it at a station | Refused at once with a line; the pistol stays and nothing is used, because no discipline of the pistol is below its tech competency (20). Before CR-18 it is used up and the line reads "Research complete. You learned nothing new: ..." |
| 21 | **Provisional, CR-19 (D-CR30).** Right-click a Crafting Station | Its crafting window opens. Until CR-19 lands a click does nothing; the exact window and whether a client patch is needed are still to be settled |

Mailing a crafting component from the crafting bag is the social campaign's step 3b ([unified guide, Mail, chat and duels](../../../guides/unified-uat.md#mail-chat-and-duels)).

**Things only a human can check:** the induction bar and its countdown; which window a station click opens (step 21); whether the pages enable and disable as you walk to and from a station, and with the tool; whether any text shows for `onErrorCode` 214 (no ASP), beside the chat line; whether the reverse-engineering page keeps its slots on confirm.

### SigNoz queries

Logs view. Every row starts from `service.name = 'cimmeria-server' AND scope_name = 'crafting' AND player_id = <your character id>` (the Rust `target` is stored as `scope_name`) and adds the filter shown. For a GM grant, `player_id` is the target's, and `gm_entity_id` names the GM.

| Step | Filter | What it shows |
|---|---|---|
| 1, 5 | `event = 'login_sync'` | What the login sent (`disciplines`, `paradigms`, `blueprints`, `asp`), and whether defaults were applied |
| 2 | `event = 'asp_granted'` | The GM grant, ASP before and after |
| 3 | `event = 'asp_earned'` | `level_before` / `level_after`, `asp_before` / `asp_after` |
| 4, 9 | `event IN ('learned', 'rejected')` | ASP before and after, or the refusal `reason` (`already_known`, `paradigm_too_low`, `prerequisite_expertise`) with the values compared |
| 6, 7 | `event = 'options_changed'` | The station and tool ids the client was given, and the `cause` (`moved`, `bag15_changed`) |
| 8, 9 | `verb = 'useItem' AND event IN ('blueprint_learned', 'paradigm_raised', 'rejected')` | Item-use results, with the item consumed |
| 10, 12, 14 | `job_id = <id>`, after finding the id with `event = 'queued'` | One job's life: queued, induction started and expired, completed or refused, items before and after, the roll for research |
| 11 | `event = 'rejected' AND reason = 'insufficient_components'` | The counts compared (`needed`, `available`) |
| 13 | `verb = 'reverseEngineer' AND event IN ('queued', 'completed')` over the step's time window | Ten `queued` and ten `completed` rows, one `job_id` each |
| 14 | `verb = 'alloying' AND event IN ('rejected', 'completed')` | `count_not_met` with `elementary_counts`, then the `completed` row with `quality_bucket = good` |
| 15 | `event = 'queue_dropped'` | `reason = logout`, the jobs dropped, nothing consumed |
| 16 | Replace the base with `scope_name = 'inventory' AND player_id = <id> AND event IN ('grant_container_chosen', 'grant_refused', 'loot_restored')` | The bag the Cell went to, and the refused pickup put back on the corpse |
| 17 | Replace the base with `scope_name = 'trade.atomic_swap' AND event IN ('trade.item_moved', 'trade.refused')` | `container_before = 15`, `container_after = 15`; then `reason = crafting_bag_full` |
| 18 | `event IN ('respec_prompted', 'respec', 'rejected')` | The prompt, the cleared disciplines with `asp_before` / `asp_after` and `asp_refund`, or `nothing_to_respec` / `respec_expired` |
| 19, GM grants | `event IN ('gm_allcraft', 'gm_craftkit', 'gm_learnblueprint', 'blueprint_learned')` | What each GM command granted, before and after, or why it was refused |
| 20 | `verb = 'research' AND event = 'rejected'` | After CR-18: the new refusal `reason`, with nothing consumed and no `completed` row |
| 21 | `event = 'station_opened'` (name provisional, CR-19) | The station's entity id and the window opened |
| any (this player) | `severity_text = 'WARN'` | Anything that failed an expectation: rollbacks (`persist_failed`), lookup misses, failed sends |
| any (server-wide) | `service.name = 'cimmeria-server' AND scope_name = 'crafting' AND severity_text = 'WARN'` | Warnings with no player: catalog load problems, requests dropped before the base (`no_player`, `malformed`) |

Metrics: `crafting_requests_total{verb, outcome}`, `crafting_jobs_total{verb, outcome}` and `crafting_rejections_total{verb, reason}`. CR-13 checked every event and filter in this table against the code; none of the queries has been run against a live SigNoz yet.

## Known gaps carried forward

- **The station or tool gate is checked at the request, not at completion.** Walking away from a station during a queued job still finishes it, and a tool traded away still covers crafts already queued (CR-07, CR-09, CR-17). A gate bypass, not a duplication; the fix is a gate re-check when the job completes.
- **Vendor purchase ignores `max_stack_size`** (CR-11): buying several Steel Cores at once makes one stack. Crafting consumes it correctly by quantity. The generic grant path has the same gap on fresh slots (CR-16). No issue filed yet.
- **Vendor purchase refusals are silent** to the player (not enough slots or naquadah: WARN only). Pre-existing, outside crafting.
- **Using a Blueprint item or a guide prints no success line.** The item disappears and the J window or the tree changes; the refusals do print a line.
- **96 Blueprint items are not seeded** (D-CR26): using one does what any other item does.
- **No backfill of earned ASP** for characters that levelled before CR-12 (the colo rebuilds from the seed, so none exist there).
- **The client's own `/respeccraft`** has no traced handler; `.respeccraft` is the supported way in (CR-10).
- **Whether `onErrorCode` shows any text** in the client is unresolved (CR-E1 Q3); the chat line is the signal.
- **Respawn reanchor** does not re-push the crafting state (CR-03); not seen to matter yet.

## Findings outside a packet

- #898: `ConnectedClientState.world_name` is never updated by gate travel. Crafting does not depend on it: gate travel drops the induction queue explicitly.
- #928 (buyback before the advisory key) is fixed by #953; the issue is still open.
- D-CR13's "a queue of at most 10" is implemented as 10 in all, the running induction included.
- `mail/send/escrow.rs` doc comments cite "crafting CR-16" and "Owner decision 2026-09-27", which the no-provenance rule for source comments forbids. Belongs to the social campaign.
- The seed comment above list 310 in `db/resources/Items/Seed/item_list_items.sql` still says "A purchase lands in the main bag (1)"; since CR-16 the `{17,15}` supplies land in the crafting bag.

## Housekeeping

Campaign worktrees left: `cr10` (CR-10, merged as #979, to retire) and `cr13` (this close-out). Retire each with `bash tools/build-lane/rm-worktree.sh <name>` the day its PR merges; it drops the target dir, the `sgw_<name>` database and the `external` junction. Every other campaign worktree is retired.
