# Cellblock continuation: owner lab checklist

> **Type:** How-to (owner-run UAT)
> **Audience:** The owner, running the Cellblock lab rows by hand
> **Last updated:** 2026-10-17
> **Companions:** [README.md](README.md) (row status), [route-baseline.md § bounded fresh acceptance rows](route-baseline.md#bounded-fresh-acceptance-rows) (the row definitions), [w3-packets.md](w3-packets.md), [CB-N03 seal](worknotes/cb-n03-seal.md), [CB-N04 seal](worknotes/cb-n04-seal.md), [CB-N05 seal](worknotes/cb-n05-seal.md), [w5-packets.md](w5-packets.md) and [w5-frozen.md](w5-frozen.md) (CB-N06)

Owner directive, 2026-10-17: agents no longer drive the lab. The owner runs every row below once the agents are done, and records the result in the README row (pass, fail with what was seen, or skipped). Agents never start a lab session for these rows.

**State at close-out (CB-N08b):** the owner has run no row. Every "Owner result" cell reads `Not run (O14-09)`; replace it with `Pass`, `Fail: <what you saw>` or `Skipped: <why>` and the date, and copy the result into the README row. The same state is in [worknotes/cb-n08-acceptance.md](worknotes/cb-n08-acceptance.md).

| Row | Proves | Run after | Unblocks | Owner result |
|---|---|---|---|---|
| CB-U03 | Relog restores the 638/639 interactions | now (W1b integrated) | CB-N02 Done | Not run (O14-09) |
| CB-U05 | Mess Hall and Hallway05 guards flank and use cover | now | CB-N03 (CB-N07b is omitted, D-CC19) | Not run (O14-09) |
| CB-U08 | The ambient cast stands where sealed and barks once | now (W2 integrated) | CB-N04 Done; a second cluster | Not run (O14-09) |
| CB-U09 | A fresh core escape takes 30-45 minutes, plus the 62050 and 62060 beats | W3-INT (62060 part after W5-INT) | the pacing gate (O14-14) for CB-N05 and CB-N06 | Not run (O14-09) |
| CB-U10 | Mission 62050 works in the native client | W3-INT | the campaign PR; CB-N05/CB-N05a Done | Not run (O14-09) |
| CB-U11 | The barracks alarm line plays once, only during 687 step 2355 | W4-INT | CB-N07a Done | Not run (O14-09) |
| CB-U12 | Mission 62060 *Before the Patrol Returns* (new Cimmeria content) works in the native client: one choice, one reward | W5-INT | the campaign PR; CB-N06 Done | Not run (O14-09) |

## Setup (every row)

1. **Run exactly one `cimmeria-server`.** On 2026-10-17 a second server from the `castle-cs-01b` worktree also listened on auth port 13001, the client logged into it, and every lab result that day was invalid. Before you start, check: `Get-NetTCPConnection -LocalPort 13001 -State Listen | ForEach-Object { (Get-Process -Id $_.OwningProcess).Path }`. It must print one path, inside `.claude\worktrees\zone-cellblock\`. Stop any other server first.
2. Build and run the server from `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (branch `content/cellblock-campaign`) against its own database `sgw_zone_cellblock`.
3. Reload that database from seeds so it has every wave's rows: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock; pwsh tools/build-lane/reload-db.ps1`. **This wipes characters.** Restart the server afterwards.
4. Use **fresh characters** made after the reload (a Human and a Loyalist Jaffa for CB-U09, U10 and U12). Characters from another worktree's database do not exist here.
5. For CB-U10 and CB-U12: delete the client's `Cache.en-US\CookedDataDialogs.pak` before the first login, so the client takes the served dialogs 60150 and 60151 fresh. Delete it again if you roll back.
6. GM commands need client patch 012. Without it, use the `.`-console equivalents (`.gotolocation`, and so on).

## GM shortcuts

| Purpose | Command |
|---|---|
| Stand next to Prisoner 329 (spawn at -289.8, 65.5, -113.1) | `/gmgotolocation Castle_CellBlock -287 65.5 -113` |
| Stand next to Col. Marsh (spawn at -191, 54.7, -138.6) | `/gmgotolocation Castle_CellBlock -189 54.7 -138.6` |
| Mess Hall guards (about -96, 34.6, -95) | `/gmgotolocation Castle_CellBlock -110 34.6 -95` (stand off, west) |
| Hallway05 guards (about -101, 24.7, -47) | `/gmgotolocation Castle_CellBlock -101 24.7 -30` |
| Stand by the two Op-CORE prisoners (spawns 520, 521 at -271 and -254, 65.45, -135) | `/gmgotolocation Castle_CellBlock -265 65.45 -135` |
| Cell-door panel `329_CellDoorButton` (spawn 22 at -287.3, 67.3, -115.0) | the Prisoner 329 shortcut above; the panel is 3.2 m from him |
| Give a mission | `/gmmissionassign <missionId> 1` |
| Jump a mission to a step | `/gmmissionadvance <missionId> <stepId>` |
| Kill the target, refill health | `/gmkilltarget`, `/gmsethealthmax`, `/gmsethealth <n>` |

Prefer playing the real route. Use the shortcuts only to reach a row's start state, and say in the result which ones you used.

## SigNoz

Filter every query on `service.name = 'cimmeria-server'` and the time window of your run.

| What | Query |
|---|---|
| Mission flow | log body contains one of `Content: accepting mission`, `Content: advancing step`, `Content: complete objective`, `Content: completing mission`, `Content: abandoning mission` |
| Offers and dialogs | `Content: adding dialog set`, `Content: removing dialog set`, `Content: displaying dialog` |
| Step changes | span name `mission.advance_step` |
| Catalog insert at startup | attribute `event = cooked_data.mission_catalog_inserted` (expect `mission_id` 62050, and 62060 after W5-INT); `cooked_data.mission_catalog_rejected` must be absent |
| Item grant (CB-U12) | `Content: granting item` with item 2893, count 2 |
| Ambient cast | DEBUG `Spawned instance NPC from DB` with tag like `Cellblock_Ambient_%`; INFO `instance_populated` |
| Ambient bark | span `content.npc_bark`, INFO `Content: npc bark` with `chain_id` 11010 and `screen_id` 200160; WARN with `reason` in (`screen_not_cached`, `empty_text`, `actor_not_player`) must be absent |

## CB-U03 — Relog restores (chains 1026-1028, 1036, 1037)

At each step below: reach it, log out, log back in, then use the named object on the **first** press.

1. 638 step 2114, Human: Prisoner 329 offers his dialog (chain 1026). Jaffa: the same with the Jaffa dialog (1027). An unconfirmed 2026-10-17 note says 329 gave no dialog here; that run used the wrong server, so check this first.
2. 638 step 2115: the `329_CellDoorButton` Livewire panel opens (1028).
3. 638 step 2116: the med station: take cover and kill in both orders; the cure gives real feedback and the sickness state clears.
4. 639 step 2144: the Prisoner Retrieval Unit is armed again and can be killed (1037).
5. 639 step 2145: the Ambernol vial can be picked up (1036).

Pass: every object answers the first press after relog. Record any that needed a second press.

## CB-U05 — Flank and cover (Mess Hall, Hallway05)

Guards (chunk IDs, not spawn IDs, so they are not in `spawnlist.sql`): `MessHall_Guard1` (chunk 1200046), `MessHall_Guard2` (chunk 1200053), `Hallway05_Guard1` (chunk 1200056), `Hallway05_Guard2` (chunk 1200044).

1. Enter the Mess Hall from the west and pull both guards. Watch that a guard moves to cover (`cover.hold` or `spawn_reserved` in the server log), stays in cover while you are in front, and that a guard tries to flank you west of x = -97.
2. Step around a guard in cover: the log shows `cover_released_flanked`, and `fire_player_flanked_npc` matches.
3. Kill both: objective 2725 completes. Repeat at Hallway05: objective 2731.
4. Negatives: no guard runs through a wall, leaves the room more than a few metres, or stands frozen with no target.

Pass: steps 1-3 seen on both rooms, no negative. Note how long each fight took.

## CB-U08 — Ambient cast (spawns 520, 521; chain 11010)

1. Fresh character. Talk to Prisoner 329, then walk east.
2. Stop about 6 m west of the cast at (-271, 65.45, -135) and (-254, 65.45, -135). Both stand still, name plate `Op-CORE Soldier`, friendly.
3. During 639 step 2117, on first entry to Region2, one line appears in chat once (chain 11010, screen 200160). Leave and re-enter: no repeat. Relog: at most once more. After the vial step 2145: no line.
4. With a second client in the same area: both see the cast; only the crossing player gets the line.

Pass: placement, plate, once-only bark, AoI on the second client, and the SigNoz ambient rows above.

## CB-U09 — Core-run pacing (after W3-INT)

1. Fresh Human: play the core escape from the stasis pod to the 688 exit, skipping optional content (do not take 62050). Record wall-clock time. Target 30-45 minutes.
2. Same on a fresh Loyalist Jaffa.
3. Then time the 62050 beat alone on one of them: from the first Prisoner click after 638 to the completion at Marsh. After W5-INT also time the 62060 beat (from the first click on an Op-CORE prisoner to the reward). Budget: 10 minutes or less for both optional beats together (O14-14).

Pass: both core runs inside 30-45 minutes and the optional beats inside 10 together. A slow beat does not block the PR by itself: tell the coordinator, who can drop 62050 or 62060 or open a fix packet.

## CB-U10 — Mission 62050 in the native client (after W3-INT)

Run on a fresh Human and a fresh Loyalist Jaffa; setup step 5 first.

1. **First offer.** Complete 638. Go to Prisoner 329: a usable offer marker or topic (`Free Prisoner 329`) is there on the **first** look and first press.
2. **Accept.** One click on Prisoner 329: the journal shows *The Remaining Pods* with step "Tell Col. Marsh there may be prisoners in the other pods." and objective "Warn Col. Marsh."; dialog 60150 shows one screen, Prisoner 329's report, and closes cleanly. The offer is gone.
3. **Repeat click.** Click Prisoner 329 again: nothing new (no second accept, no second dialog).
4. **Relog** at step B: the journal still shows step B; the offer does not return.
5. **Complete.** Talk to Col. Marsh (after 640): his normal briefing (4001 Human, 5022 Jaffa) still shows and 62050 completes. Relog: still complete; no new offer.
6. **Abandon** (second character or a fresh one): accept, abandon from the journal: the offer returns at Prisoner 329; re-accept works.
7. **Skip.** Ignore 62050 and finish the core route to 688: nothing blocks.
8. Evidence: the SigNoz mission-flow rows; the startup `cooked_data.mission_catalog_inserted` event; if you capture packets, the cooked-sync heads `(3, 62050)` and `(5, 60150)`.

**Blockers (not "pending"):** a blank dialog, a client crash, a missing first-press response, lost progress, or a changed core Marsh briefing blocks the campaign PR.

**Rollback if 60150 crashes the client:** tell the coordinator (a fix packet moves 60150 into the dialog quarantine), and delete `Cache.en-US\CookedDataDialogs.pak` on the client.

## CB-U11 — Barracks alarm bark (chain 11020)

Run after W4-INT, on any fresh character that reaches mission 687. The barracks (`Castle_Cellblock.Region12`) spans about x -149 to -115, z -147 to -94; its guards are `Barracks_Guard1`-`3` (spawns 25, 26, 36). The crate (687 step 2354) sits just north of it at (-130.5, 24.7, -92.1).

1. **During 2354** (before the crate search): walk into the barracks. No alarm line appears.
2. **Activation.** Search the crate. If you are already inside the barracks when step 2355 starts, one line appears at once; otherwise it appears on your first step into the barracks: `NID Guard: Intruder in the barracks! One of the prisoners got loose. Weapons free!` (chat, say channel), once.
3. **No repeat.** Leave and re-enter during 2355: no second line. Relog during 2355: at most one more line.
4. **Fight unchanged.** The three guards fight as before: no extra guard, no guard pulled from another room, objective counts unchanged.
5. **After.** Clear the barracks (687 completes): re-entering gives no line.
6. SigNoz: span `content.npc_bark`, INFO `Content: npc bark` with `chain_id` 11020 and `screen_id` 200170; no WARN `screen_not_cached`.

Pass: steps 1-5. If you dislike the line or it fires at the wrong time, say so: the rollback is `enabled = false` on chain 11020, and CB-N07 closes as `Done / omitted`.

## CB-U12 — Before the Patrol Returns in the native client (after W5-INT)

New Cimmeria content (D-CC18), not 2009 content. Run on a fresh Human and a fresh Loyalist Jaffa; setup step 5 first. The two Op-CORE prisoners stand at (-271, -135) and (-254, -135); the cell-door panel is beside Prisoner 329.

1. **No early offer.** Before 638 completes, click each Op-CORE prisoner: nothing happens.
2. **Offer.** Complete 638. Each prisoner is now usable on the **first** look and first press (topic `Mend Slave Cages`, if the client shows it).
3. **Accept.** One click on either prisoner: the journal shows *Before the Patrol Returns* with step "Help the prisoners hide, or jam the cell-door panel." and objective "Help the prisoners or jam the panel."; dialog 60151 shows one screen ("The two prisoners are too weak to run…") and closes cleanly. **No item yet.**
4. **Choose (character A: aid).** Click a prisoner again: two Health Slappacks arrive in the bag on that click, and the mission completes. Then click both prisoners and the panel: nothing new, no second reward.
5. **Choose (character B: sabotage).** After accepting, click the cell-door panel: two Health Slappacks arrive on that click, and the mission completes. Then click a prisoner and the panel again: nothing new. The panel does not start the 638 Livewire minigame.
6. **Relog** after step 3 (choice still open): the prisoners and the panel still answer the first press. Relog after the reward: no offer, no second reward, the journal shows it complete.
7. **Abandon** (a fresh character): accept, abandon from the journal: the panel stops answering, the prisoners offer again, and re-accepting works.
8. **Skip.** Ignore 62060, accept 641 at Col. Marsh: the prisoners stop offering and the core route to 688 is unchanged.
9. **Pacing.** Time the beat for CB-U09 step 3.
10. SigNoz: the mission-flow and dialog rows, the item grant row (item 2893, count 2), and the startup catalog insert for 62060.
11. **Walk-back (observe).** Between 640 (ring-out) and 641, walk from the ring back to the prisoners. If you cannot, note it: the offer is then reachable only from 638 to 640 and the aid/sabotage beat shrinks.
12. **Full bag (observe).** Once with a full bag, run steps 3-4. The mission completes once and the reward is refused, which shows only in the log as `grant_refused`. Note it; a lost reward blocks the campaign as listed below.
13. **Accept on click (observe).** One click accepts and opens dialog 60151 together. A second click on a prisoner is the aid branch, so check that the dialog was read before the second click.
Items 11-13 are the known risks from the [CB-N06 seal](worknotes/cb-n06-seal.md) (G8 and G11). They supersede that note for CB-U12.

**Blockers (not "pending"):** a blank dialog, a client crash, a missing first-press response, a lost or doubled reward, both branches paying out, or any change to the core route blocks the campaign PR.

**If you dislike the quest, the copy or the reward**, say so: a fix packet sets `enabled = false` on chains 11030-11041, moves 60151 into the dialog quarantine and drops 62060 from the served catalog; delete `Cache.en-US\CookedDataDialogs.pak` on the client. CB-N06 then closes `Done / omitted`.

## Rows owned by CB-N08

CB-U01, CB-U02, CB-U04, CB-U06 and CB-U07 belong to the close-out packet CB-N08 and are listed in [route-baseline.md](route-baseline.md#bounded-fresh-acceptance-rows). None has a result: each reads `Not run (O14-09)` in the [acceptance reconciliation](worknotes/cb-n08-acceptance.md#route-baseline-rows).
