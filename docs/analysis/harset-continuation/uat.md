---
title: Harset Owner-Manual UAT Guide
type: how-to
audience: the owner, running UAT by hand on a client; no agent and no lab
last_updated: 2026-10-18
companion_docs:
  - README.md
  - RESUME.md
  - work-packets.md
  - ../../guides/unified-uat.md
  - ../zone-restoration-operator-guide.md
---

# Harset owner-manual UAT guide

> Type: how-to. Audience: the owner, running every step by hand in a real client. No agent drives these steps and no lab is used.
> Companions: [ledger](README.md), [resume](RESUME.md), [packet contracts](work-packets.md), [HA-1360a seal](worknotes/HA-1360a-seal.md), [HA-1360b seal](worknotes/HA-1360b-seal.md), [a2c seal](worknotes/HA-1360a2c-seal.md), [owner decisions](worknotes/owner-decisions-2026-10-18.md), [unified UAT guide](../../guides/unified-uat.md) (house format, GM commands, known issues K1-K38), [Harset runbook C](../zone-restoration-operator-guide.md#c-harset-worlds-57-68-69-70).

This guide has two jobs. **Part A** checks what the campaign already merged. **Part B** collects the observations that unblock the Harset work still waiting. Nothing here records a result: every "Expect" is what the sources predict, and every box in the Result column is yours to fill.

## 1. What is tested and what is not

Read this before you start. It decides how much a failure surprises you.

| Thing | Lane-tested (default and `lab-chaos` suites, live-DB where noted) | Ever run in a client or lab |
|---|---|---|
| Letter turn-in transaction `commit_letter_turn_in` (HA-1360a1) | Yes, live-DB tests, with revert proofs | No |
| Session token and entry epoch before `InitPlayerState` (a2a) | Yes | No |
| `LetterTurnInRequest` routing, result frames, sticky refusal (a2b) | Yes | No |
| Strict `complete_mission_with_item` action (a3a) | Yes | No |
| Click, pending state and "processing" feedback (a3b) | Yes | No |
| Result consumer: dialog 4576, DSM slot clear, frames 105, 82, 81, 80 (a3c) | Yes | No |
| Seeds: DSM 120101, chains 6501, 6502, 6505, 6511, 1366 (b1, b2) | Yes, 42 live-DB tests, revert probes | No |
| World-entry continuation fence (a2c) | Yes, 254 default and 278 `lab-chaos` tests (A); 756 and 812 (B) | No |
| `remove_effect` executor arm (HA-03c) | Yes, lane tests only | No, and no seed chain uses it |
| Every Part B item | No (offline map and BSP analysis only) | No |

The whole Harset letter path has never run against a real client. Two predictions can fail for client reasons the lane cannot see: the client may not show an interaction prompt for a bind with no dialog (the a3b "processing" message exists to cover that), and system messages may not render at all (known issue K9).

Labels in the worknotes carry over here. **PROVISIONAL** means a design nobody approved; **INFERENCE** means read from text or engine habit; **BlockedEvidence** means a packet waits on the observation. Keep the labels until you have checked the thing.

## 2. Before you start

- Run on the colo build that contains the merged Harset branch. Seeds rebuild on deploy, and the dialog-map and mission-definition caches load at server start, so confirm with the operator that the **server process restarted after the seed change**. A hot reload is not enough.
- Characters reset on deploy (K4). Make fresh ones.
- A new character starts in the Cellblock stasis room and plays the intro movie (K6). The Praxis start is the baseline (K24).
- You may stage with GM commands (`/gmgiveitem`, `/gmmissionassign`, `.gotolocation`, see [unified UAT](../../guides/unified-uat.md)), but A1 and A2 only count when the **final run uses an ordinary non-GM account**.
- Type `.bug <note>` at every interesting moment. It bookmarks the time in SigNoz so you can find the rows later. Server log lines are named below; where a source gives no line to look for, the step says "nothing recorded by the source".
- Do not write account names, addresses or credentials in notes you attach to this guide.

## Part A. Validate what was added

### A1. Letter turn-in, Cellblock to Castle to Marsh

The chain: you hold mission 1360 at step 4038 with one Frost's Letter (item 3730), then click Marsh. The server runs one DB transaction, consumes the letter, completes 1360 once, and offers mission 1361 through a bind on slot 10 of DSM 5254.

| # | Prerequisites and character state | Do | Expect | Failure signature | Record |
|---|---|---|---|---|---|
| A1.1 | Fresh ordinary character in the Cellblock | Accept mission 1360 through its Cellblock dialog (1003/1121 grant and accept). Or stage with `/gmmissionassign` and note that you did | 1360 in the journal; a Frost's Letter (3730) is obtainable | No mission, or no letter source | Journal text; how you got the letter; whether staged |
| A1.2 | 1360 active, one 3730 in a bag you can open | Open the bag | One 3730 stack of 1 | Item absent or wrong count | Bag, slot, count |
| A1.3 | Same, at the Castle | Cross the Castle gate to Harset (needs the Harset address, see unified UAT Castle B19) | Mission 1360 advances 4037 to 4038 on the crossing (Castle chain 1366, event `stargate_crossed 'Harset'`) | Step stays 4037; the letter cannot be turned in later | Step before and after; server log chain rows if you can reach them |
| A1.4 | On Harset, mission at 4038 | Relog once, then log back in on Harset | Mission and item survive; Marsh's rebind (chain 6502, `player_loaded` for Harset_CmdCenter) puts the letter dialog set (DSM 120101, slot 10) back | After relog Marsh no longer reacts | Whether Marsh reacts after the relog |
| A1.5 | At Marsh, in range | Click Marsh once | At once a "processing" message in the chat window (CHAN9, sent before the first request). Then dialog 4576. Mission 1360 completes, the letter leaves your bag (count drops by 1) | No feedback on first press (a project rule breach); no dialog; item not consumed; mission not completed | Screenshot of dialog; bag before and after; whether the message appeared (K9 may hide it) |
| A1.6 | Right after A1.5 | Open the journal and Marsh's dialog list | 1360 gone from active, 1361 offered through slot 10 of DSM 5254 (chain 6511, an `add_dialog_set` bind, not a frame) | 1360 still active; 1361 not offered; 1360 offered again | Journal; Marsh dialog list |
| A1.7 | After A1.5 | Click Marsh again at once, then a few more times | No second completion, no second consumption (repeat count 0, one-shot) | Dialog 4576 again; item count falls below expected; duplicate mission event | Number of dialogs shown; bag count |
| A1.8 | After A1.5 | Relog, return to Marsh | State persists: 1360 stays complete, the receipt prevents a second turn-in | 1360 reopens; letter dialog returns (chain 6505 should rebind to 120101 only after 1361 completes) | Journal; Marsh dialog list |
| A1.9 | A second fresh ordinary character on Harset with 1360 at 4038 and **no** 3730 | Click Marsh | A refusal, no completion | Mission completes without an item | Refusal text or silence |
| A1.10 | Same kind of character with 3730 in a full or blocked bag, if you can build that | Click Marsh | Refused without losing the item | Item lost without the mission completing | Bag state; mission state |
| A1.11 | Ordinary character holding 3730 but at the wrong step (not 4038) | Click Marsh | Refused | Completion at the wrong step | Step; outcome |
| A1.12 | Any character, Marsh out of interaction range (walk away until the click is refused) | Try to click Marsh | No dialog; `interaction.out_of_range` in the server log | Dialog opens from afar | Log line |
| A1.13 | A1.5 in progress | Press the click, then log out at once | One of: completed once, or not at all, never twice. Dialog 4576 exactly-once across a crash is **not claimed** by the sources | Letter consumed with no completion, or two completions | Mission and bag after next login |

Inert items to confirm. Chains 6503 and 6504 (mission 567, item 2698) are disabled and must do nothing. Check that an owner-held 2698 gets no reaction from Marsh. The Castle route for 2698 stays an owner-open question.

**What the server log and SigNoz hold for this chain.** The tracing target `harset.letter_turn_in` emits these events when something goes wrong: `payload_mismatch`, `stale_request`, `transaction_uncertain`, `missing_db_pool`, `transport_uncertain`, `client_sync_uncertain`, `client_sync_query_failed`, `missing_result_channel`, `closed_result_channel`. The cell executor refuses with `invalid_action`, `missing_target`, `missing_session`, `ineligible_click` or `feedback_channel_unavailable`. The result consumer logs `stale_result`, `invalid_postimage` or `mission_tracker_locked`. A successful turn-in fires the chain event `mission_completed` once for 1360. The `.bug` bookmark from the step brackets the time window. The server log also holds `fire_interact_tag: matched` for interact chains. One failure the sources predict but cannot rule out: a NULL-dialog bind may not draw a prompt in the client. If A1.5 shows nothing on click but the log shows no refusal, record exactly that.

### A2. World-entry continuation fence (a2c)

The change splits the `on_client_ready` continuation into ten guarded stages (burst, gm_redirect, contact_lists, org_restore, plugin_hook, ignore_resync, mail_sweep, login_fanout, cinematic, aoi_flush) and drops sends to a session that a newer entry on the same address replaced. It fixes a general old race, not a letter blocker. It has **no player-facing path of its own**. The only thing you can observe is the absence of stale packets after a fast relog. Three steps stress it.

| # | Prerequisites | Do | Expect | Failure signature | Record |
|---|---|---|---|---|---|
| A2.1 | Ordinary character on Harset | Log out and back in three times in quick succession, as fast as the login screen allows | You land once, normally; chat, contacts, mail, organization list and the world look as after a single login; no duplicate NPCs or doubled UI | Doubled mail or contact entries, a frozen first frame, a disconnect on the second entry | What you saw per attempt; `.bug` after the last |
| A2.2 | Character with a pending mission state (1360 at 4038 with 3730) | Relog quickly, then click Marsh | Letter path works as in A1.5 | Click does nothing after a rapid relog (token or epoch stale); log `stale_request` | Outcome; any `stale_request` or `entry_superseded` row (debug level) |
| A2.3 | Two logins on the same address in turn, if you can | Log in, drop the client by killing the process, log in again at once | The new session runs; nothing from the old session reaches it | Old-session packets arrive in the new session | Anything odd in the first seconds |

Not fenced, so a small residue after a rapid relog is not a regression: the spawned mail sweep and login fanout tasks, the idempotent character writes (`first_login = 0`, mail expiry, organization online marking), and the letter's own bounded sender. The debug-level reason `entry_superseded` shows a dropped send; its absence at the default log level proves nothing.

### A3. The `remove_effect` executor arm (HA-03c)

State of affairs: the arm is merged and lane-tested only. **No seed chain uses it and no player path reaches it.** HA-03a (Mala'c duel design) and HA-03d (the seed graph for mission 1325) would use it, and neither is built. There is nothing to click and nothing to see in a client today.

What you can do: nothing in game. When HA-03d ships, the first check to add here is the neutral-override behaviour listed under B8, and that a duel's effect clears when the chain removes it. Record nothing for this item until then.

## Part B. Validate to unblock future Harset work

Ordered by how many waiting packets each observation feeds. Ties go to the cheaper check. The counts come from the gate column of [work-packets.md](work-packets.md) and are approximate. Every item is observation only: you change nothing, and a failing observation is recorded as BlockedEvidence, not repaired here. Mark each box pass, fail or record.

| Rank | Item | Unblocks | Design status |
|---|---|---|---|
| B1 | First authored-quest client catalog proof | HA-17a2, 17a3, 17b, 17c, 18a, 18b, 18c, 19 | The pilot is PROVISIONAL |
| B2 | Mission items: does the client send use-on-target | HA-07, HA-08a, HA-09, HA-11 | Unverified, none provisional |
| B3 | Market exit arch 1006 vs 1551 | HA-01a, then HA-03d, HA-05, HA-11 | Evidence, no design |
| B4 | Landing points free of clipping | HA-01a | 1006 alternate is PROVISIONAL |
| B5 | Spawn 308, raised points, containment walk | HA-02, then HA-03d, HA-05, HA-11 | Evidence |
| B6 | World-57 exterior door positions | HA-01a | Evidence |
| B7 | 5225 door cursor from a dialog bind alone | HA-01b1, then HA-01b | PROVISIONAL (INFERENCE medium) |
| B8 | Snare and the neutral override | HA-03a, HA-03b, HA-03d | INFERENCE medium |
| B9 | Kaerlia nameplate render and facing | HA-01b2, then HA-01b | PROVISIONAL |
| B10 | Lo'taur appearance (template 353 clone) | HA-15g, HA-15b, HA-15c | PROVISIONAL |

### B1. First authored-quest client catalog proof (HA-00c, pilot HA-17a)

Unblocks the largest set: the pilot (HA-17a2) and everything that waits on its owner-run check (HA-17a3, HA-17b, HA-17c, HA-18a/b/c, then HA-19). **PROVISIONAL**: the pilot graph (HA-17a1) is a design with no IDs allocated, and no authored quest is known to display, accept or advance in a client. Open owner question: may the Harset pilot itself be the first proof, or must Cellblock's first authored quest go first (the owner's "what stays open" list)?

- **What you can do today:** nothing from Harset, because no authored mission exists. Watch for Cellblock's first authored quest and note whether the journal shows it, the accept dialog appears, a step advances and it survives a relog.
- **When the pilot exists:** run the pilot's own script, [HA-17a1 section 9](worknotes/HA-17a1-pilot-design.md#9-owner-run-client-check-script-journal-dialog-advance-relog-reward), step by step. If its step 3 fails, that is the catalog proof failing, not the graph: record BlockedEvidence and do not edit chains.
- **Pass:** journal entry, accept dialog, advance and reward all show, and the state survives a relog.
- **Fail:** any of those missing. **Record:** the step that failed, a screenshot, and SigNoz rows `cooked_data.mission_catalog_inserted` and `cooked_data.mission_catalog_rejected`.

### B2. Use-on-target for mission items (HA-15e)

Unblocks HA-07 (item 2734 and 4690), HA-08a (2743), HA-09 (4396) and HA-11 (2818 and 2820). The sources say plainly that whether the real client sends a use-on-target request for mission-bag items is **unverified**, along with how it renders the abilities and the value of the enum `target_type_id`.

- **Do:** with a character that holds any mission-bag item that has a use action (stage via GM commands), click the item, then click a world target. Try an NPC and a prop.
- **Pass:** the item enters a targeting state or the server acts on the target (a log row naming the target). **Fail:** the click does nothing or consumes the item with no target. **Record:** item ID, target kind, what the cursor did, any packet-tap or log row (the sources give no named line; use `.bug` and look in SigNoz around it).

### B3. Market exit arch 1006 vs 1551 (HA-01a)

Unblocks HA-01a travel and placement, the gate for HA-03d, HA-05 and HA-11. The BSP probe found two Market candidates: arch 1006 at about (97.28, 3.52, 60.32) facing yaw -180, and arch 1551 at about (19.92, 4.80, 1.28) facing yaw -90. It is a one-glance owner check.

- **Do:** enter the Market (world 69) from Harset and walk to each arch in turn.
- **Pass:** you can tell which arch is the way back to the plaza (an opening that reads as an exit). **Record:** which arch (1006 or 1551), or "neither", or "both"; a screenshot of each. Storage arch 28 at (61.36, 5.12, -2.60) is the Storage exit; the doorways 2133 and 2387 inside Storage are not.
- A low-confidence note from the probe: a thin occupied plane at x 97.25 to 97.30 may seal arch 1006 in collision. If you can walk through it, record that.

### B4. Landing points free of clipping (HA-01a1)

Unblocks HA-01a. The offline check passed 1551 (20.0, 4.81, 8.5; free radius 4.0 m) and Storage (61.4, 5.12, 5.2; 4.0 m). The 1006 landing (89.5, 3.6, 60.3) has only 1.0 m free; the alternate (80.0, 3.61, 61.0) is **PROVISIONAL**. A pass offline does not prove the client puts you there or that your capsule clears.

- **Do:** arrive through each exit by walking, or use `.gotolocation` to the points. Stand still, turn a full circle, jump once.
- **Pass:** you stand on the floor, nothing intersects you, and you can walk away in every direction. **Fail:** inside geometry, falling, or stuck. **Record:** the point, a screenshot, and whether you could move.

### B5. Spawn 308, raised points and the containment walk (HA-02)

Unblocks HA-02, then the encounter packets that need duel and wave anchors (HA-03d, HA-05, HA-11). Spawn 308 and the five raised points were **not** re-measured offline. E03 failed offline; E04, E06, E07 and E13 passed with a nudge. A coordinator call on E03, the Market pocket box (needs shrinking to about x[16,24] z[9,22]) and the Storage region follows your findings.

- **Do:** as an **ordinary non-GM** account, walk the pen floor, the upper wing and each raised platform in world 70, then the Market pocket.
- **Pass:** you hold position on every platform and can leave each one; no point drops you through the floor. **Fail:** falling, clipping, or unreachable platforms. **Record:** for spawn 308 and each raised point, the coordinates you read with `.bug pin <name>`, ground or not.

### B6. World-57 exterior door positions (HA-01a)

Unblocks HA-01a. The positions are not recoverable from the interior data; the HA-D07 hand readings stand until you check. Landings that PASS offline: world 68 (0, 0.355, -20) and world 57 (0, -67.6, -231).

- **Do:** on foot, stand in the doorway of each of the two exterior doors and type `.bug pin <door>`. Walk through each and note where you arrive.
- **Pass:** you land on the floor at the listed point without a bounce. **Record:** the pinned coordinates for both doors and the arrival point per door.

### B7. Right-click cursor on a placed prop from a dialog bind alone (HA-01b1)

Unblocks HA-01b1, then HA-01b. **PROVISIONAL**: a server-only 5225 door is feasible on paper (INFERENCE, medium confidence) as a tagged prop plus an `interact_tag` chain plus a per-player NULL-dialog bind with the world-object interaction bit. If the cursor never appears, a client change is needed and that is a maintainer decision. Open items: which mesh reads as a door (template 22 is the console mesh; a door-shaped mesh is "asset needed"), whether the real door physically blocks the 2078 box, and that a wrong-step click is not refused.

- **Do:** when a build with the 5225 bind exists, hover the prop and right-click it, once at the right step and once at the wrong step.
- **Pass:** the cursor changes over the prop, the click gives first-press feedback and advances `1664` to `5226` (never completing mission 6060). **Fail:** no cursor, or the click does nothing. **Record:** cursor state, step before and after, the click-outcome row, and the existing `interaction.out_of_range` row.
- **Today:** there is nothing to run. Hold this item until the HA-01b seed exists.

### B8. Snare honoured, and the neutral override (HA-03a, HA-03b)

Unblocks the Mala'c duel (HA-03a), its contract (HA-03b) and the 1325 graph (HA-03d). Snare is a `MovementSpeedMod` effect and the sources say client honouring is **unverified**; it is not added (M4). Whether the neutral override suppresses the right-click attack route is INFERENCE (medium) and is "the first lab check for the implementation".

- **Do:** when a build exists, start the duel and try right-click attack while the override is on; get snared once by an ability that applies it and move.
- **Pass:** the override suppresses attack, or snare slows you visibly. **Fail:** you can attack under the override, or snare changes nothing. **Record:** the ability, your speed before and during, the combat log lines.
- **Today:** nothing to run. This also covers what A3 will eventually show.

### B9. Sgt. Kaerlia placement, nameplate and facing (HA-01b2)

Unblocks HA-01b2, then HA-01b. **PROVISIONAL**: the whole character is a provisional design (name_id 27315, template 370 components, primary position (-5, -68.95, 30) in world 57 with a fallback (8, -68.95, 36)). The heading convention is unverified.

- **Do:** when she exists, go to the spot.
- **Pass:** her nameplate reads as the intended name (a blank or a raw key means the client lacks name_id 27315), she stands on the floor facing the plaza. **Fail:** invisible, wrong name, floating, or facing a wall. **Record:** screenshot of the nameplate, her facing relative to the plaza.
- **Today:** nothing to run.

### B10. Lo'taur appearance (HA-15g, template 353 clone)

Unblocks the vendor clones in HA-15g, HA-15b (stock and price seal) and HA-15c. **PROVISIONAL**: the clone uses `BS_GoauldMale` and `AR_G_Underlings` on a class-pet template that has never rendered. The fallback is a Goa'uld Praxis look. The worknote calls this "a lab item for the owner."

- **Do:** when the clone exists, look at it from 5 m, then up close.
- **Pass:** it renders as a Lo'taur-style figure, upright, with a nameplate. **Fail:** invisible, a stock pet look, or an error. **Record:** screenshot and the nameplate text. A failure keeps the fallback.
- **Today:** nothing to run. The Lo'taur pet has known issues K12 and K35 in the unified guide.

### Other owner-gated items

These appear in the worknotes and ledger but are decisions, not observations. They are listed so none are lost; see [owner decisions](worknotes/owner-decisions-2026-10-18.md#what-stays-open): the 1353 treat source, prices inside O14-12, whether the pilot waits for the Jaffa-arc UAT, the 1664 abandon path without 1399, the Castle 2698 route (567 stays disabled), the 618 and 621 recipients, and Agnos (deferred).

## Result sheet

Copy this table into your notes and fill it. Leave a row blank until you have run it.

| Step | Pass / fail / record | Note |
|---|---|---|
| A1.1 to A1.13 | | |
| A2.1 to A2.3 | | |
| A3 | n/a today | |
| B1 to B10 | | |

## Maintenance

When Part A passes or fails, update [RESUME.md](RESUME.md) and the HA-1360b row of [work-packets.md](work-packets.md). When a Part B item resolves, the matching packet's gate changes in the ledger, not here. The ledger owner links this page from the harset-continuation README; a pointer from the Harset section of the [unified UAT guide](../../guides/unified-uat.md#harset) is a later, separate change.
