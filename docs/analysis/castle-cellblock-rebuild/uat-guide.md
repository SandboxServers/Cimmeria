# Castle Cellblock In-Game UAT Guide

> Type: how-to. Audience: the project owner, running the real SGW client against a local server.
> Updated: 2026-09-18. Companions: [launch prompt and decisions](README.md), [spec audit](audit.md), [packet ledger](work-packets.md), [session resume handoff](handoffs/session-resume.md), [mission chains](../../content/mission-chains.md), [commands reference](../../commands.md).

This is the acceptance pass for everything the Castle Cellblock rebuild campaign has merged to `main`. You run it with the game open; each scenario tells you exactly what to click, what you should see, and what would count as a regression. Nothing here has been run in-client before — every scenario is pending.

## What this covers

| Packet | What merged | PR |
|---|---|---|
| C00 | Prison Boot movement-lock gate at zone entry, cleared by Livewire (mission 689, chains 1022-1025) | #646 |
| C01 | Purged the duplicate auto-export; exactly one Prisoner 329 dialog and one Marsh briefing per archetype; Region8 pistol-guard aggro (chain 1008) | #646 |
| C02 | Chain-replay baseline for 640 / 680 / 681-686 (no behavior change) | #646 |
| C03 | Stasis Sickness launched on zone load while 639 is uncured (chain 1112); cure path via chain 1034 | #646 |
| C04 | Frost's Letter (mission 1360) accepted on the Frost body loot (chain 1121) | #649 |
| C05 | Take-cover objective 2484 + drone-kill objective 2482 both gate step 2144 (chains 1033/1131/1132/1133) | #653 |
| C07 | Accept blurbs 2305 / 4000 / 2518 on missions 640 / 641 / 688 (chains 1151/1152/1154) | #648 |
| C08a | `content_actions.delay_ms` honoured by the executor | #646 |
| C08b | Straegis attack scene: Matinee 1751, Marsh despawn, dialog 2516 at 10.1 s (chains 1161/1162) | #650 |
| GC1a | Marsh dialog 2309 pre-departure; post-death blurb 5859 at 10.6 s (chains 1171/1172) | #655 |
| GC1b-0 | Engine: an NPC can follow a player; Marsh `move_speed` raised to 0.9 units/tick | #646 |
| GC1b-1 | Marsh rides the rings to region 3 (chain 1173) | #655 |
| GC1b-2 | Marsh follows the player topside; follow cleared at the Straegis scene (chains 1174/1175) | #655 |

Not covered, because it is not built: **C06** (flank objectives 2725 / 2731), **GC1c** (lockdown energy field), **GC3** (mission-completion XP). See [Known limitations](#known-limitations--not-validated).

## Before you start

### 1. Reload the database so the merged seed applies

The content chains live in `resources.content_*` rows loaded at boot. If your `sgw` database predates these PRs, the server will happily run the old chains and every scenario below will fail for the wrong reason. Reseed first:

```powershell
pwsh setup.ps1 -SkipBuild -ForceDatabase
```

That drops and recreates `sgw` and reloads `db/database.sql` plus every seed file under `db/resources/`. If the data directory itself is suspect, the nuclear form is `pwsh setup.ps1 -ResetDatabase -SkipBuild -NoLaunch`.

Do **not** use `db.bat start` to bring Postgres up for this. `db.bat` points at `external/postgresql_server/data`; the bootstrap seeds `<repo>/server/pgdata`. They are different instances and `db.bat` will start the empty one. Connection string for anything you want to inspect by hand: `postgres://w-testing:w-testing@localhost:5433/sgw`.

### 2. Start the server

```powershell
.\cimmeria-server.exe
```

Run it from the repository root — the log directory is resolved relative to the working directory.

### 3. Know where the logs are

Logs land in `<repo root>\logs\`. There is no rotation; the previous session's files are moved to `logs\archive\<timestamp>\` every time the server starts, so a fresh run always gives you clean files.

| File | What lands in it |
|---|---|
| `logs\content.log` | Every content-chain action (`cimmeria_services::cell::content` at TRACE). This is the file you grep for almost every scenario. |
| `logs\missions.log` | Mission lifecycle (`cimmeria_services::cell::missions`) |
| `logs\server.log` | JSON, all modules, INFO and above — the place `aoi.create_send_failed` warnings show up |
| `logs\spawner.log` | Spawns, gate travel, ring transport |
| `logs\aoi.log` | Cell service + space manager |
| `logs\combat.log` | Combat and abilities |

Several actions you will want to confirm log at **DEBUG**, not INFO — `Content: set interaction type`, `Content: set aggression`, `Content: move waypoint` and `Content: deferring action` among them. `content.log` is filtered at TRACE for the content module, so they land there regardless. If you want them on the console too, start the server with `RUST_LOG=debug`.

Note: `docs/building.md` tells you to tail `logs\cimmeria-server.log`. That file does not exist — use `logs\server.log`.

### 4. Make the characters you need

Archetype branching matters in four places (missions 638, 641, 687, and the Aftermath reward). Make **two fresh characters**:

- One **Jaffa** (archetype 8).
- One **non-Jaffa Tau'ri** — Soldier (1), Commando (2), Scientist (3) or Archeologist (4). Any of the four takes the same branch.

There is no GM command that resets mission state. `/gmmissionreset`, `/gmmissionclearactive` and `/gmmissionclearhistory` exist in the client's `SGWGmPlayer.def` but have no server handler. **Delete the character at character-select to start the tutorial over** — `sgw_mission` and `sgw_inventory` cascade off `sgw_player`, so a delete + reroll is a genuinely clean run. Per-mission surgery with `/gmmissionclear <id>` then `/gmmissionassign <id> 1` also works when you only need to redo one beat.

Praxis Goa'uld (char defs 10 and 19) also start here. They are excluded from the forced Prison Boots and they take the **Tau'ri** branch everywhere else, because Cimmeria's chains gate on `archetype neq 8` rather than the original Python's `archetype < 5`. Testing one is optional.

### 5. GM commands you will actually use

Every seed account (`test`, `cady`, `tester`) is access level 2 = GameMaster, so the client's native `/` console is available. All of these are confirmed implemented in `crates/services/src/cell/cell_methods/gm/`; the full catalogue is [docs/commands.md](../../commands.md).

| Need | Command |
|---|---|
| Teleport within the Cellblock | `/gmgotoxyz <x> <y> <z>` |
| Teleport to another world | `/gmgotolocation Castle_CellBlock <x> <y> <z>` (world names match case-insensitively) |
| See where you are | `/gmshowtargetlocation` |
| Grant a mission | `/gmmissionassign <missionId> 1` |
| Jump to a step | `/gmmissionadvance <missionId> <stepId>` — the second argument is the **step id** (e.g. `2144`), not a step index |
| Abandon a mission | `/gmmissionclear <missionId>` |
| Inspect mission state | `/gmmissionlist`, `/gmmissionlistfull`, `/gmmissiondetails <missionId>` |
| Target something | `/gmsettarget <entityId>` |
| Kill a targeted NPC properly | `/gmkilltarget <entityId>` (runs the canonical death sequence, so `entity_dead_tag` chains fire) |
| Remove an NPC | `/gmdespawn <entityId>` |
| Give yourself an item | `/gmgiveitem <designId> <qty>` |

`/gmmissioncomplete`, `/gmsetlevel` and `/gmsetgodmode` are declared client-side but **not implemented** — don't plan a scenario around them. To survive a fight you don't want to fight, use `/gmsethealthmax <n> 0` then `/gmsethealth <n> 0`.

### 6. How to read a scenario

**ID** reuses the T-numbers from [audit.md's acceptance-test mapping](audit.md#acceptance-test-mapping-spec-validation_tests-sheet) where one exists; scenarios for behaviour the 2009 spec never had get a new `T25`+ id. **Must NOT happen** is the regression guard — if you see it, the scenario fails even if everything else looked right. **Server evidence** is a literal string you can grep for; every one listed was verified to exist in the source.

---

## Milestone M1 — Baseline repair (C00, C01, C02, C03)

### T25 — Prison Boot movement lock at zone entry (C00)

**Packets:** C00. **Chains:** 1022, 1023, 1024, 1025. **Mission:** 689 "Prison Boot Lock" (hidden, never shows in the quest log).

**Preconditions:** brand-new character (either archetype except Goa'uld). This is the very first thing that happens, before you can do anything else.

**Steps:**

1. Create a character and enter the world. You spawn in the stasis room at roughly `(-334.23, 73.47, -228.03)`.
2. Try to walk. Note whether you can.
3. Open your inventory (`b`) and look at the equipped boots slot. Item 3438 "Prison Boots" should be worn.
4. Double-click / right-click the worn Prison Boots to use them. Note whether the client offers a "use" action at all for an item in an equipped slot.
5. If a Livewire minigame opens, complete it.

**Expected:**

- Mission 689 is accepted server-side on load and stays out of the quest log (`is_hidden = true`).
- Ability 1597 "Prison Boot" is launched at you on every load until the gate clears.
- On Livewire victory, ability 1598 fires and mission 689 completes.

**Important caveat before you judge this one.** Effects 1939 (the lock), 1942 (the unlock) and 3081 (the item swap) all have `script_name = NULL` and `pulse_count = 1`, which makes them single-shot and non-pulsing. `register_active_effect` returns early for those *without sending any wire packet at all*. So on the server side, launching 1597 and 1598 is currently a complete no-op: no movement lock, no status icon, no item swap. **The real question this scenario answers is: does the client do anything of its own when the player's boot ability list changes?** If it does not, that is the expected result today and it tells us a follow-up packet needs a server-side movement gate keyed on `mission_status 689 neq completed`. Record what you actually see; do not treat "I could walk" as an automatic fail.

Two things are explicitly unverified and this test is what resolves them:

- Whether the client exposes a "use" action for an item worn in an equipped slot (chain 1024 triggers on `item_use 3438`). If it does not, the fix is swapping chain 1024's trigger to an `interact_tag` on a boot-control-panel spawn — no ability/effect/mission data changes.
- Whether the item swap to item 5865 renders. Item 5865 is the intended unequippable twin, but **its `name` column is the literal placeholder `NO ITEM NAME`** with the real name sitting in `description`. If the swap ever does fire, you will see an item called "NO ITEM NAME" in your inventory. That is a seed-data defect, not a server bug.

**Must NOT happen:**

- Mission 689 appearing in the visible quest log.
- The Livewire minigame opening a second time after the gate has cleared (chain 1024 gates on `mission_status 689 neq completed`).
- Mission 689 being re-accepted on a later login.

**Relog check:** log out and back in *before* clearing the gate — ability 1597 must be re-launched (`chain 1023` fires on `active`). Log out and back in *after* clearing — 1597 must **not** be re-launched (`completed` stops it). These two are the whole point of the packet.

**Server evidence:** `logs\content.log`, grep `Content: launched ability` (fields `ability_id=1597`, `chain_id=1023`, and `ability_id=1598`, `chain_id=1025`) and `Content: accepting mission` with `mission_id=689`, `chain_id=1022`.

### T01 / T02 — Zone entry, mission 622 and Stasis Sickness (C03)

**Packets:** C03 (plus the pre-existing 622 chains). **Chains:** 1001, 1002, 1112.

**Preconditions:** fresh character, first load into Castle_CellBlock (world 12).

**Steps:**

1. Enter the world.
2. Read the dialog that opens.
3. Check your quest log.
4. Check your buff/debuff bar for a Stasis Sickness icon.

**Expected:**

- Dialog **2982** plays: two screens, opening "The last thing you remember is going into stasis after another session with that sadist, Romney…".
- Mission **622 "Arm Yourself!"** is accepted, on step **2113** "Search the nearby corpses to locate a weapon."
- Cpl. Frost's body (`ArmYourself_FrostBody`, spawn 19, template 14) is searchable; the NID Guard's body (`ArmYourself_GuardBody`, spawn 15, template 21) is **not yet** — chain 1001 deliberately binds only Frost's dialog set.
- Ability **1372 "Stasis Sickness - Stage 1"** is launched at you.

**Caveat, same shape as T25.** Effect 1634 is scriptless and single-shot, so launching 1372 sends nothing to the client. The M1 milestone text in [README.md](README.md#validation-and-uat-gates) says "the Stasis Sickness icon appears on load"; that wording predates the C03 investigation and is optimistic. **Record whether any icon appears at all** — that is the finding. The server-side behaviour (launch on load while 639 is uncured, stop after the cure) is what chain 1112 guarantees and what the log line proves.

**Must NOT happen:**

- Dialog 2982 playing twice, or mission 622 being accepted twice.
- The Guard's corpse being searchable before Frost has been searched.
- Sequence 10000 (the stasis-room door) playing on this first load — chain 1002 only fires when 622 is already `completed`.

**Relog check:** relog on step 2113. Frost's dialog binding must come back (chain 1006 re-binds dialog set 5229 to template 14) and his corpse must still be clickable. Ability 1372 is re-launched.

**Server evidence:** `logs\content.log`, `Content: accepting mission` `mission_id=622 chain_id=1001`; `Content: displaying dialog` `dialog_id=2982`; `Content: launched ability` `ability_id=1372 chain_id=1112`.

### T03 / T04 — Frost, the Guard, and Frost's Letter (C04)

**Packets:** C04. **Chains:** 1003, 1005, 1004, 1121. **Missions:** 622, 1360.

**Preconditions:** T01 passed; on step 2113.

**Steps:**

1. Right-click Cpl. Frost's corpse at `(-328.30, 73.47, -210.27)` and pick the "Search Cpl. Frost's Corpse" topic.
2. Check the quest log.
3. Right-click Frost's corpse again.
4. Right-click the NID Guard's corpse at `(-322.51, 73.47, -209.83)`, topic "Search the Guard's Corpse".
5. Open your inventory, find the **SI 3 9mm Pistol** (item 55) in your backpack, and equip it.

**Expected:**

- Dialog **3995** shows ("There are no obvious wounds on Cpl. Frost, though it appears he died…").
- **Frost's Letter** (item 3730) lands in your mission inventory (container 0).
- Mission **1360 "Frost's Letter"** appears in the log on step **4037** "Find a way to get Cpl. Frost's Letter to his family." This is C04's whole deliverable.
- Mission 622 advances to step **80623** "Search the NID Guard's body for a weapon" and the Guard becomes searchable.
- Searching the Guard shows dialog **3996**, grants the pistol to your backpack, and advances to step **80622** "Equip the pistol from your inventory."
- Equipping the pistol plays sequence **10000** (the stasis-room door opens) and completes mission 622.

**Must NOT happen:**

- Mission 1360 being accepted twice, or re-accepted on a second click of Frost's body. Chain 1121 gates on both `step_status 622/2113 = active` and `mission_status 1360 = not_active`.
- Frost's corpse re-granting the letter on a re-click.
- The Guard re-granting the pistol on a re-click.
- Mission 622 completing before you manually equip the pistol.

**Relog check:** relog while on step 80623 — the Guard's dialog binding must come back (chain 1007, dialog set 5230 to template 21). Mission 1360 must still be active and stay active for the rest of the zone.

**Server evidence:** `logs\content.log`, `Content: accepting mission` `mission_id=1360 chain_id=1121`; `Content: granting item` for 3730 then 55; `Content: advancing step` `mission_id=622 step_id=80623` then `80622`; `Content: playing sequence` `sequence_id=10000`.

### T05 / T06 — Prisoner 329, one dialog per archetype (C01)

**Packets:** C01. **Chains:** 1011, 1012, 1013. **Mission:** 638 "Speak to Prisoner 329".

This is the scenario the whole C01 purge exists for. Before the purge, the auto-exported chains 5004/5005 bound **both** archetypes' dialog sets to the prisoner and issued `accept_mission 638` up to four times.

**Preconditions:** mission 622 complete, out of the stasis room. Run this **once as Jaffa and once as Tau'ri** — it is the single most archetype-sensitive check in the zone.

**Steps:**

1. Walk down toward the cell block until you cross into `Castle_Cellblock.Region2`.
2. Open the quest log.
3. Right-click Prisoner 329 and look at the topic list.
4. Pick the "Free Prisoner 329" topic and read it through.

**Expected:**

- Mission **638 "Speak to Prisoner 329"** is accepted **exactly once**, on step **2114** "Speak to Prisoner 329."
- The prisoner offers **exactly one** "Free Prisoner 329" topic.
  - Tau'ri: dialog **2300**, opening "You feel uncomfotable, don't you? Nauseous?…" (the typo is in the 2009 data — do not report it as a bug).
  - Jaffa: dialog **5021**, opening "You feel dizzy, don't you? Stomach aching?…", and the second screen is the player line "My symbiote will cure me. I will not trust a Goa'uld."
- Finishing the topic advances to step **2115** "Hack the Cell Door controls…" and puts the Livewire wrench cursor on `329_CellDoorButton` at `(-287.29, 67.26, -115.04)`.

**Must NOT happen:**

- **Two topics on the prisoner.** A Tau'ri seeing the symbiote line, or a Jaffa seeing the Tau'ri line, is the exact bug C01 fixed (issue #216).
- Mission 638 being accepted more than once — you would see duplicate accept toasts or a mission-offer refusal warning in the log.
- Any system message on entering Region2. Chain 1013 fires `system_message 5040` but the executor's `SystemMessage` arm is a stub with the wire format still unresolved (issue #268), so nothing should render.

**Relog check:** relog on step 2114 — the prisoner's dialog set is re-bound by mission-accept state, so the topic must still be there and must still be the single correct one.

**Server evidence:** `logs\content.log`, grep `Content: adding dialog set`. You should see **exactly one** line, with `dialog_set_id=2794` (Tau'ri) or `dialog_set_id=5866` (Jaffa), from `chain_id=1011` or `1012` respectively. Two lines is a fail. Also `fire_enter_region: matched` in the same file with `region_tag=Castle_Cellblock.Region2`.

### T07 — Cell door Livewire

**Chains:** 1016, 1017, 1020, 1021, 1018, 1019.

**Preconditions:** T05 passed, on step 2115.

**Steps:**

1. Right-click `329_CellDoorButton` and play the Livewire minigame to victory.
2. Talk to Prisoner 329 again.
3. Agree to the escape.

**Expected:**

- Victory advances to step **2116**, plays sequence **1749** (the cell door), and clears the wrench cursor off the button.
- Talking again shows the follow-up: Tau'ri gets **2299**, Jaffa gets **5020**.
- Agreeing shows blurb **2298** "Secure some Ambernol in order to counteract your Stasis Sickness.", completes 638, and accepts **639 "Find Ambernol"** on step **2117** "Look in the Med Station down the hall for some Ambernol."
- The prisoner's topic is removed after you commit.

**Must NOT happen:**

- A Tau'ri player being routed through the Jaffa follow-up 5020 (or vice versa). This was a real drift: the earlier archetype fix corrected the topic dialog but missed the post-Livewire follow-up.
- The prisoner still offering the "Free Prisoner 329" topic after you agree.

**Relog check:** relog on step 2116 before agreeing — the follow-up path must still work.

**Server evidence:** `logs\content.log`, `Content: playing sequence` `sequence_id=1749`; `Content: removing dialog set`; `Content: accepting mission` `mission_id=639`.

### T26 — Region8 NID guard aggro (C01, defect B3)

**Packets:** C01. **Chain:** 1008. **Region:** `Castle_CellBlock.Region8` (point set 2039).

The auto-export keyed this on `Castle_Cellblock.Region8` with a lowercase `b`. Region matching is an exact string compare, so the scripted ambush never fired in-client. Chain 1008 restores the Python's exact call order and values.

**Preconditions:** any character, anywhere in the zone that reaches Region8. The guard is `ArmYourself_NIDGuard`, spawn 20, at `(-289.46, 68.54, -154.28)`.

**Steps:**

1. Walk into Region8. No mission state is required — the trigger is unconditional.
2. Watch the NID guard.

**Expected:** the guard turns aggressive and comes at you without you shooting first. Aggression is set to level 1 and 1000 threat is generated on you, in that order.

**Must NOT happen:**

- The guard standing inert when you cross the region boundary — that is the pre-fix behaviour.
- The guard aggroing when you enter Region2 or any earlier region.

**Relog check:** the trigger is `once = true` per player. After a relog, re-entering Region8 need not re-aggro a guard that is already fighting or already dead; what matters is that a fresh character gets the aggro on first entry.

**Server evidence:** `logs\content.log`, `fire_enter_region: matched` with `region_tag=Castle_CellBlock.Region8` (capital B), then `Content: set aggression` (DEBUG, `entity_tag=ArmYourself_NIDGuard`, `agg_level=1`, `chain_id=1008`) followed by `Content: generate threat on NPC from player` (`threat_level=1000`). Threat before aggression, or `threat_level=5000`, means the wrong chain is loaded.

---

## Milestone M2 — Tutorial objectives (C04, C05)

### T08 — Ambernol pickup, the drone, and the cover objective (C05)

**Packets:** C05. **Chains:** 1032, 1033, 1131, 1132, 1133. **Mission:** 639, step **2144** "Defend yourself from the drone!".

Step 2144 has two objectives and needs **both**: **2482** "Defeat the Prisoner Retrieval drone" and **2484** "Take cover behind the desk!". Run this scenario **twice**, once in each order.

**Preconditions:** mission 639 active. Walk into `Castle_Cellblock.Region11` (the med station) to advance from step 2117 to step **2145** "Retrieve the vial of Ambernol from the desk."

**Steps (order A — cover first):**

1. Right-click the Ambernol vial (`ArmYourself_AmbernolVial`, spawn 12) at `(-234.04, 66.52, -124.70)`.
2. Stand behind the med-station desk — the cover nodes sit around `(-231.8, 65.45, -124.2)` to `(-234.7, 65.47, -124.7)`, i.e. right where the vial was. Detection radius is 5 m horizontally with 2 m of vertical tolerance, so you only need to be at the desk, not on an exact spot.
3. *Then* kill the drone.

**Steps (order B — kill first):** same, but kill the drone before taking cover.

**Expected on pickup (both orders):**

- **Ambernol Vial** (item 19) enters your mission inventory; the vial object is destroyed.
- The drone (`ArmYourself_PrisonerRetrievalUnit`, spawn 10, at `(-220.26, 66.74, -121.38)`) goes aggressive and focuses on you (aggression 1, threat 1000).
- Dialog **2297** shows: "You take the vial of Ambernol from the counter."
- Sequence **10001** plays — the **TakeCoverIndicator** appears.
- Mission 639 advances to step **2144**.

**Expected for the first of the two objectives, whichever it is:**

- Its checkbox ticks in the objective list.
- The mission **stays on step 2144**.
- If it was the cover objective, sequence **10014** plays and the TakeCoverIndicator disappears.

**Expected for the second:**

- Mission 639 advances to step **2343** "Use the Ambernol in your mission inventory… to cure yourself of Stasis Sickness."
- If cover was second, sequence 10014 plays now.

**Known client-side wrinkle.** The second objective is completed implicitly by `advance_step`, not by an explicit `complete_objective` call. Issue **#656** tracks that `advance_step` never sends `ON_OBJECTIVE_UPDATE` for implicitly completed objectives — so the second objective's checkbox may **not** visibly tick even though the step correctly advances. Record what you see; a non-ticking second checkbox with a correct step advance is the known issue, not a C05 regression.

**Must NOT happen:**

- Step 2144 advancing to 2343 after only **one** of the two objectives. That is the pre-C05 behaviour and the core thing this scenario guards.
- Mission 639 **completing outright** and skipping step 2343 entirely. That is the auto-complete trap: calling `complete_objective` for both objectives ends the whole mission. If you find yourself cured-and-done without ever using the vial, this is a hard fail.
- Sequence 10014 playing **twice**. Lean out of cover and back in before killing the drone — the indicator must not re-hide, and no second `PlaySequence(10014)` may fire.
- The indicator hiding on cover taken at some *other* piece of cover in the space. The chain is keyed to cover set **1381** (`Castle_CellBlock_MedStationDesk`) specifically.

**Relog check:** relog mid-step-2144 with one objective already done. No chain replays sequence 10001 or 10014 on load — they are one-shot, gated purely on live events. The completed objective must still read as completed, and completing the remaining one must still advance to 2343.

**Server evidence:** `logs\content.log`. Cover first: `fire_cover_entered: matched` with `cover_set_id=1381`, then `Content: complete objective` `mission_id=639 objective_id=2484 chain_id=1132` and `Content: playing sequence` `sequence_id=10014`; then on the kill, `Content: advancing step` `mission_id=639 step_id=2343 chain_id=1131`. Kill first: `Content: complete objective` `objective_id=2482 chain_id=1033`, then `Content: advancing step` `step_id=2343 chain_id=1133`. Exactly one `sequence_id=10014` line across the whole sequence.

### T09 — The cure (C03)

**Packets:** C03. **Chain:** 1034.

**Preconditions:** on step 2343 with the Ambernol Vial in mission inventory.

**Steps:**

1. Press `b`, switch to Mission Inventory, and use the **Ambernol Vial**.
2. Check the debuff bar.
3. Check the quest log.
4. Right-click `HackTheRings_Switch` at `(-218.08, 67.04, -122.72)`.

**Expected:**

- Ability **1374 "Cure Stasis Sickness"** is launched at you (before the vial is consumed).
- The vial is consumed — **exactly one** of it.
- Mission 639 completes; mission **640 "Hack the Rings"** is accepted on step **2120** "Hack the Ring Transport controls."
- The ring switch gains the Livewire wrench cursor.
- Blurb **2305** shows (see T10).

Effect 1636 is scriptless and single-shot like the rest, so expect **no** visible debuff change. What matters here is that the Stasis Sickness load-gate stops: see the relog check.

**Must NOT happen:**

- The vial not being consumed (it should leave your mission inventory) — a stale seed once shipped exactly that.
- **Two** vials disappearing if you somehow have a stack of two.
- The vial being consumed *before* the cure ability fires.

**Relog check:** this is C03's acceptance criterion. Relog after the cure — ability 1372 must **not** be re-launched, because chain 1112 gates on `mission_status 639 neq completed`. Relog *before* the cure (mid-639) — 1372 **is** re-launched.

**Server evidence:** `logs\content.log`, in order: `Content: launched ability` `ability_id=1374`, `Content: RemoveItem → RemoveInventoryItemByType` for item 19, `Content: completing mission` `mission_id=639`, `Content: accepting mission` `mission_id=640` — all `chain_id=1034`. Then relog and confirm **no** `ability_id=1372` line.

---

## Milestone M3 — Narrative beats (C07, C08, GC1)

### T10 — Hack the rings, and the first accept blurb (C02, C07)

**Packets:** C02, C07. **Chains:** 1041, 1042, 1043, 1044, 1045, 1046, 1151.

**Preconditions:** mission 640 active on step 2120.

**Steps:**

1. Read the prompt that appeared when 640 was accepted.
2. Right-click `HackTheRings_Switch` and win the Livewire minigame.
3. Right-click the switch again.
4. Once you land, look around for Col. Marsh.

**Expected:**

- Blurb **2305** "Hack the Ring Transporter controls to leave this floor." shows **exactly once** on accept. This is C07's deliverable for mission 640.
- Livewire victory advances to step **2215** "Use the Ring Transporter to leave the Cells.", swaps the wrench cursor for the RingNetwork icon, and adds the quest highlight.
- Right-clicking the switch triggers ring region **1** (`CellblockRing1`, `(-215.46, 65.92, -121.40)`) and clears the highlight.
- You arrive at ring region **2** (`CellblockRing2`, `(-192.66, 55.26, -154.84)`), the Preparation floor.
- Mission 640 completes and Col. Marsh (`Preparation_ColMarsh`, spawn 7, at `(-191, 54.72, -138.59)`) gets his "talk to me" indicator.

**Must NOT happen:**

- Blurb 2305 showing twice, or showing on the accept of any other mission.
- The Livewire minigame opening a second time after the hack (chain 1041 gates on step 2120).
- The rings firing before the hack (chain 1043 gates on step 2215).
- Mission 640 completing twice, or Marsh's indicator being re-set after 640 is already complete.

**Relog check:** relog on step 2120 — the wrench cursor must be restored (chain 1045). Relog on step 2215 — both the RingNetwork icon and the quest highlight must be restored (chain 1046). Interaction bits are not persisted, so without those chains the switch would be dead on login.

**Server evidence:** `logs\content.log`, `Content: displaying dialog` `dialog_id=2305 chain_id=1151`; `Content: triggering transporter`; `fire_teleport_in: matched` `region_id=2`; `Content: completing mission` `mission_id=640 chain_id=1044`.

### T11 / T12 — Marsh's briefing, one prompt per archetype (C01, C07)

**Packets:** C01, C07. **Chains:** 1051, 1052, 1053, 1054, 1055, 1066, 1062, 1063, 1152.

Run **once as Jaffa and once as Tau'ri**. Before the C01 purge, a Jaffa interacting with Marsh resolved dialog 5022 **five times** plus one stray Tau'ri 4001.

**Preconditions:** mission 640 complete; standing in the Preparation room.

**Steps:**

1. Right-click Col. Marsh.
2. Read the briefing through to the accept.
3. Check for a prompt.
4. Right-click the P90 locker (`Preparation_SMG1A`, spawn 11, at `(-201.25, 56.08, -131.61)`).
5. Right-click Marsh again — note whether he is talkable.
6. Open your inventory and equip the **SGHC 6 SMG** (item 21).
7. Right-click Marsh again.

**Expected:**

- **Exactly one** briefing dialog: Tau'ri gets **4001** ("Well, you woke up faster than I thought you would…"), Jaffa gets **5022** ("You're a big one, aren't you?…", which brings in Moh'katan on screen 8).
- Accepting gives mission **641 "Preparation"** on step **2121** "Prepare yourself for the escape.", highlights the P90 locker, and clears Marsh's indicator.
- Blurb **4000** "Get outfitted for your escape from the Cellblock." shows **exactly once** on accept — C07's deliverable for 641. One chain covers both archetype accept paths.
- The locker grants the SMG to your **backpack** (container 1), not the bandolier, and advances to step **80641** "Equip the P90 from your inventory."
- Marsh is **not talkable** between pickup and equip.
- Equipping the P90 advances to step **3563** "Speak to Col. Marsh." and restores his indicator.
- Talking again gives **3999** (Tau'ri) or **5023** (Jaffa), then step **3564** "Use the Terminal to open the remaining Stasis Pods."

**Note on the seed comments.** Chains 1052/1054/1057 are labelled "(sci)" in the SQL. The condition is `archetype eq 8`, which is **Jaffa**, not Scientist. The behaviour is correct; the comment label is wrong. Flagged as documentation debt, not a test failure.

**Must NOT happen:**

- More than one briefing dialog, or the wrong archetype's dialog.
- The briefing re-firing and re-accepting 641 after you have already advanced past step 2121 — that was a real loop bug, fixed by gating on `mission_status` rather than `step_status`.
- A **second** P90 from re-clicking the locker.
- Blurb 4000 on any other mission's accept.

**Relog check:** relog on step 2121 — the locker highlight must return (chain 1062). Relog with 640 complete and 641 not yet accepted — Marsh's indicator must return (chain 1063). Relog on step 3563 or 3564 — Marsh's / the Terminal's markers must return (chains 1064/1065).

**Server evidence:** `logs\content.log`. Grep `Content: displaying dialog` for the interact — **exactly one** line, `dialog_id=4001 chain_id=1051` or `dialog_id=5022 chain_id=1052`, and zero of the other. Then `Content: displaying dialog` `dialog_id=4000 chain_id=1152`, and `Content: granting item` `item_id=21` with `container_id=1`.

### T13 — Stasis terminal, and mission 680

**Chains:** 1060, 1061.

**Preconditions:** on step 3564.

**Steps:**

1. Right-click the Preparation terminal (`Preparation_Terminal`, spawn 16, at `(-187.71, 55.85, -141.50)`) and win the Livewire minigame.
2. Check the quest log.

**Expected:**

- Dialog **3998** shows: "You succesfully bypass the security systems - the Stasis Pods for the East Wing slowly begin to open." (the typo is in the 2009 data).
- Mission 641 completes; mission **680 "Escape the Cellblock"** is accepted on step **2344** "Find a way out of the Castle!"
- `Preparation_RingSwitch` (spawn 17, at `(-193.92, 56.32, -152.16)`) gets both the RingNetwork icon and the quest highlight.

**Must NOT happen:**

- An accept blurb on mission 680. **Chain 1153 is reserved but deliberately not authored** — blurb 2308's text references "follow Marsh" and it was held back pending GC1. Seeing a blurb here would mean someone shipped 1153 without updating this guide.
- Stasis pods being driven server-side — they are client Kismet with no recovered event id. Whether they visibly open is a client question.

**Relog check:** relog on step 2344 — both ring-switch bits must return (chain 1074).

**Server evidence:** `logs\content.log`, `Content: displaying dialog` `dialog_id=3998`, `Content: completing mission` `mission_id=641`, `Content: accepting mission` `mission_id=680` — all `chain_id=1061`.

### T27 — Marsh's pre-departure line (GC1a)

**Packets:** GC1a. **Chain:** 1171.

**Preconditions:** mission 680 active on step **2344**, still in the Preparation room, Marsh still standing at his spawn.

**Steps:**

1. Right-click Col. Marsh.

**Expected:** dialog **2309** plays — three screens, speaker **261 "Col. Marsh"**:

1. "That's about all we can do from here. C'mon, let's move out."
2. (player) "What about the prisoners we just freed?"
3. "They'll follow later. The NID got sloppy and fell asleep because they thought they were safe on this frozen rock…"

Speaker 261 is the only one of Marsh's dialogs with a properly populated speaker name. His earlier lines (4001 / 5022 / 3999 / 5023) use speaker **256**, whose name column is an empty string — so **expect those earlier dialogs to render with a blank speaker label** unless the client falls back to the NPC's entity name. Worth noting which way it goes; it is a seed-data observation, not a chain bug.

**Must NOT happen:**

- Dialog 2309 firing once you have advanced past step 2344 (i.e. after the ring hop).
- Dialog **4003** ("That sparkly energy field is what's wrong. It's blocking our way out.") appearing anywhere. It is deliberately excluded — the lockdown barrier it describes does not exist (GC1c is blocked on evidence), so playing it would tell you your way is blocked when nothing blocks it.
- Dialog **5019** appearing. Also deliberately excluded: its final screen is the out-of-scope "Future Self" time-travel content and `display_dialog` cannot show only part of a dialog.

**Relog check:** relog on step 2344 — the line must still be available.

**Server evidence:** `logs\content.log`, `Content: displaying dialog` `dialog_id=2309 chain_id=1171`.

### T28 — Marsh rides the rings and follows you topside (GC1b-1, GC1b-2)

**Packets:** GC1b-1, GC1b-2 (and GC1b-0's engine change). **Chains:** 1072, 1173, 1174.

This is the highest-risk scenario in the guide. **Read the [Known risks](#known-risks) section before running it.**

**Preconditions:** mission 680 on step 2344. Marsh at his Preparation spawn `(-191, 54.72, -138.59)`.

**Steps:**

1. Right-click `Preparation_RingSwitch` to trigger ring region 2.
2. When you land at ring region **3** (`CellblockRing3`, `(-89.69, 45.19, -161.53)`), **stop and look around immediately.** Specifically look for Col. Marsh.
3. Walk a short way and look again.
4. Walk the topside route — Ring 3 → Mess Hall → Hallway01 → onward — and keep checking whether Marsh is behind you.
5. Watch whether he walks around corners or through walls.

**Expected:**

- Mission 680 advances to step **2345** "Lockdown! Find another way out of the Castle!"; the ring switch's bits clear.
- Marsh is snapped to `(-91.689, 45.188, -161.533)` — about 2 units off the landing pad, so he appears **beside** you, not on top of you.
- Marsh starts following you, and keeps pace. His move speed was raised to 0.9 units/tick in GC1b-0, about 10% over the world-12 player run speed of 8.125 u/s, so he should close the gap rather than fall behind.
- He paths around geometry. The Preparation room and the topside route are genuinely disconnected navmesh components — that is why the ring exists — but everything from Ring 3 onward is one connected component.

**Must NOT happen:**

- **Marsh being invisible.** He has not been in your AoI before this moment, so the reposition is effectively a fresh AoI entry — the same shape as the open invisible-corpse bug. If you cannot see him, do not walk on: check the log (below) and note it.
- Marsh drifting straight through walls. That is the degenerate single-waypoint straight-line fallback that happens when `find_path` returns nothing.
- Marsh falling steadily further behind each hallway (the pre-GC1b-0 speed deficit).
- Marsh staying in the Preparation room.

**Relog check — this one is a known gap, and you should exercise it deliberately.** There is **no** `player_loaded` restore chain for the escort. If you relog after the ring hop but before mission 686 completes, Marsh will respawn at his original Preparation-room position with no follow state, and he will not come back. That is a documented, accepted gap needing a coordinator decision on whether it deserves its own packet — record it, but it is not a new finding.

**Server evidence:** `logs\content.log`, `fire_teleport_in: matched` `region_id=3`, then `Content: move waypoint` (DEBUG, `entity_tag=Preparation_ColMarsh`, `destination=-91.689003,45.1879997,-161.533005`, `chain_id=1173`) and `Content: set follow target` (`use_player=true`, `chain_id=1174`). For the invisibility risk, grep `logs\server.log` for `aoi.create_send_failed` — fields `witness_id`, `entity_id`, `phase` (`create_base` or `cascade`) and `reason` (`entity_to_addr_miss` / `client_disconnected` / `send_error`). If Marsh is invisible and that warning never fires, the drop is downstream of those seams and is worth a fresh note on issue #582.

### T14 — Mess Hall (681)

**Chains:** 1073, 1085, 1086, 1087.

**Preconditions:** on step 2345, topside.

**Steps:**

1. Walk into `Castle_Cellblock.Region9` (the Mess Hall).
2. Kill `MessHall_Guard1` at `(-96.25, 34.59, -91.59)` and `MessHall_Guard2` at `(-95.89, 34.59, -98.81)`.

**Expected:**

- On **entering** Region9: mission 680 completes and mission **681 "Mess Hall Controller"** is accepted, in that order. Cimmeria fires this on region **enter**; the 2009 Python used region exit. That is a deliberate deviation — do not report it.
- The second guard death completes 681 and accepts **682 "Hallway01 Controller"**.

**Must NOT happen:**

- Mission 680 completing again if you backtrack out of and into Region9.
- Mission 681 completing after only one kill.
- **Mission 681 appearing as a visible quest-log entry with a "Kill the Guards" step you are expected to track.** It is the only one of the six controller missions with `is_hidden = false` and `is_a_story = true`, unlike 682-686 which are all hidden. Record whether it shows in the log — a visible controller mission would be a UI defect worth its own issue.
- The flank objective **2725** "Take position behind the long table to flank the guards and negate their cover." doing anything. It is **not built** — see [Known limitations](#known-limitations--not-validated).

**Relog check:** relog mid-Mess-Hall. The kill counter (`messhall_kills`) is content-engine state; confirm that killing the remaining guard after a relog still completes 681.

**Server evidence:** `logs\content.log`, `Content: completing mission` `mission_id=680 chain_id=1073` then `Content: accepting mission` `mission_id=681`; `Content: incremented counter` (DEBUG) `counter_name=messhall_kills`; then `Content: completing mission` `mission_id=681 chain_id=1087`.

### T15 — The hallway chain (682-686)

**Chains:** 1081, 1082, 1083, 1088, 1089, 1090, 1091, 1092, 1093, 1094.

**Preconditions:** 681 complete, 682 accepted.

**Steps:**

1. Leave `Castle_Cellblock.Region3`, kill `Hallway01_Guard` at `(-128.85, 39.55, -73.53)`.
2. Kill `Hallway02_Guard` at `(-113.49, 39.55, -63.04)`.
3. Enter `Castle_Cellblock.Region4`, kill `Hallway03_Guard` at `(-98.58, 39.55, -77.09)`.
4. Kill `Hallway04_Guard` at `(-61.35, 34.59, -69.03)`.
5. Enter `Castle_Cellblock.Region5`, kill both `Hallway05_Guard1` at `(-100.16, 24.67, -43.90)` and `Hallway05_Guard2` at `(-101.50, 24.67, -51.30)`.

**Expected:** a clean one-guard-per-mission cascade — 682 → 683 → 684 → 685 → 686, each completing on its guard's death and accepting the next. Hallway05 is the only two-guard one; the second kill completes **686**. These are hidden controller missions and should never appear in the quest log.

**Must NOT happen:**

- **Each region transition accepting its mission more than once.** Before the C01 purge, the Region3-exit / Region4 / Region5 / Region6 triggers each resolved two to five accepts. Exactly one each.
- Any controller mission (682-686) appearing in the visible quest log.
- Flank objective **2731** doing anything on Hallway05 — not built.

**Relog check:** relog between hallways; the next region transition must still accept exactly one mission.

**Server evidence:** `logs\content.log`, `Content: accepting mission` — grep per mission id and confirm **one** line each for 682, 684, 686. `fire_exit_region: matched` for Region3; `fire_enter_region: matched` for Region4/5/6.

### T16 / T17 — The Straegis attack scene (C08b, GC1a, GC1b-2)

**Packets:** C08b, GC1a, GC1b-2. **Chains:** 1161, 1162, 1172, 1175.

Four chains fire on the same `mission_completed 686` event. They are ordered by chain id, and the timing is the point of the scenario — **have a stopwatch or a screen recording.**

**Preconditions:** Hallway05 cleared, so 686 completes. Marsh should be following you (T28).

**Steps:**

1. Kill the second Hallway05 guard.
2. Do not move. Watch and time.
3. When control returns, look for Marsh.
4. Walk on into `Castle_Cellblock.Region6`.

**Expected, in this order:**

1. **t = 0 s** — the StraegisAttack Matinee plays (sequence **1751**, `Castle_Cellblock-fffffffe.Main_Sequence.StraegisAttack`, roughly 10.01 s of camera-only Director track).
2. **t = 0 s** — Col. Marsh is despawned. He was following you, so unlike the original plan he vanishes **in front of you**, not off-screen in the Preparation room.
3. **t = 0 s** — his follow target is cleared (a harmless no-op, since the despawn already ran).
4. **t ≈ 10.1 s** — dialog **2516** shows: "Something just took Marsh - it snatched him through a rift in this and space and left behind a puddle of blood…" (the typos are in the 2009 data).
5. **t ≈ 10.6 s** — blurb **5859** shows: "Find a way out of the Cellblock. Without Marsh."
6. Mission 686 completes; mission **687 "Aftermath"** is accepted, and the wooden crate is highlighted.

**Camera caveat.** `play_sequence` hardcodes `ViewType = 0` (KISMET_VIEW_Witness) for every content chain; there is no per-action override. The cinematic docs suggest `3` (KISMET_VIEW_EventInvoker) is the usual value for non-combat camera cinematics like ring and stargate sequences. No legacy source exists for StraegisAttack to settle it. **If the camera is broken or disorienting, say so explicitly** — the fix is threading a `viewType` field through `Action::PlaySequence`, and this scenario is the evidence that justifies it.

**Must NOT happen:**

- Dialog 2516 or blurb 5859 appearing **during** the Matinee. 5859 racing 2516 was found and fixed in review; 5859 is now at `delay_ms` 10600 against 2516's 10100.
- 5859 appearing **before** 2516.
- Control not returning to the player after the Matinee.
- Marsh still standing there afterwards.
- A creature, a blood decal or a data disc. D-CB07 chose camera-only — none of the three has a recovered actor, event id, template or item id.

**Relog check — the important half of this scenario.** Log out and back in with 686 completed and 687 not yet accepted. The per-player Castle_CellBlock instance is rebuilt from `resources.spawnlist` on every relog, so **Marsh would otherwise respawn**. Chain 1162 re-despawns him. Expect: Marsh gone, Matinee **not** replayed, dialogs 2516 and 5859 **not** replayed. Once 687 is accepted, chain 1162 stops matching — that is correct and intentional.

**Server evidence:** `logs\content.log`. On the kill: `Content: playing sequence` `sequence_id=1751 chain_id=1161`, `Content: destroying tagged entity` `entity_tag=Preparation_ColMarsh chain_id=1161` (with a `witnesses_notified` field — that count should be non-zero), then two `Content: deferring action` DEBUG lines with `delay_ms=10100 chain_id=1161` and `delay_ms=10600 chain_id=1172`, followed ten seconds later by two `Content: firing deferred action` INFO lines and the matching `Content: displaying dialog` for 2516 then 5859.

---

## Milestone M3 continued — Aftermath and the Armory

### T18 / T19 — Aftermath crate and the barracks (687)

**Chains:** 1097, 1098, 1099, 1100-1103, 1104.

Run **once per archetype** — the reward branches on Jaffa vs everyone else.

**Preconditions:** mission 687 accepted on step **2354** "Search the crate for any useful items."

**Steps:**

1. Right-click `Cellblock_WoodenCrate` (spawn 8) at `(-130.45, 24.67, -92.07)`.
2. Check your backpack.
3. Kill all three barracks guards: `Barracks_Guard1` `(-118.41, 24.67, -118.35)`, `Barracks_Guard2` `(-131.48, 24.67, -116.97)`, `Barracks_Guard3` `(-136.38, 24.67, -135.67)`.

**Expected:**

- The crate is highlighted on mission accept.
- **Tau'ri (and Goa'uld):** dialog **3942** "You search through the Crate and discover a stealth suit and a nasty-looking serrated knife.", and six items into the **backpack** — Covert Stealth Helmet (3347), Vest (3359), Pants (3372), Gloves (3387), Boots (3401) and a Combat Knife (3325).
- **Jaffa:** dialog **3943** "…a staff weapon and a piece of plate armor for the chest.", and two items — Armored Prison Jacket (3482) and Serpent Staff (2797).
- Step advances to **2355** "Eliminate the guards in the barracks."; the crate highlight clears.
- The **third** guard's death completes 687 and auto-accepts **688 "Secure the Armory"**.

Goa'uld taking the Tau'ri branch is intentional — Cimmeria's chains gate on `archetype neq 8` where the 2009 Python used `archetype < 5` and gave Goa'uld nothing at all.

**Must NOT happen:**

- Both reward sets being granted, or the wrong archetype's set.
- The crate re-granting on a second click.
- 687 completing before all three guards are down.
- Per-class variants (dialogs 2517 / 4408 / 4409) appearing. Those were never wired to this graph in any revision we have evidence of; GC2 is closed with no packet.

**Relog check:** relog on step 2354 — the crate highlight must return (chain 1104).

**Server evidence:** `logs\content.log`, `Content: displaying dialog` `dialog_id=3942` or `3943` (exactly one), `Content: granting item` ×6 or ×2, `Content: incremented counter` `counter_name=barracks_kills` ×3, `Content: completing mission` `mission_id=687 chain_id=1103`.

### T20 — Secure the Armory (688) and the third accept blurb (C07)

**Packets:** C07 (plus the pre-existing 688 chains). **Chains:** 1105, 1106, 1107, 1108, 1109, 1110, 1111, 1154.

**Preconditions:** 687 complete.

**Steps:**

1. Read the prompt that appears when 688 auto-accepts.
2. Right-click `Cellblock_TerminalX` (spawn 34) at `(-52.66, 25.76, -151.13)`.
3. Optionally kill `Cellblock_ArmoryGuard1` (spawn 27) at `(-49.69, 24.67, -127.11)`.
4. Right-click `Cellblock_ArmoryRingSwitch` (spawn 79) at `(-54.88, 26.08, -163.84)`.

**Expected:**

- Blurb **2518** "Help Op-CORE secure the Armory by opening the doors." shows **exactly once** on accept — C07's deliverable for 688.
- Mission 688 on step **2356** "Secure the Castle's main armory for Op-CORE.", with required objective **2734** "Use the terminal to open the armory doors." and optional objective **4647** "Eliminate the NID guards."
- The terminal is highlighted; using it advances to step **80688** "Use the ring transport to escape.", clears the terminal highlight and highlights the ring switch.
- Objective 4647 is optional; killing the guard is not required to proceed.
- The ring switch completes 688 and moves you to Castle.

**Must NOT happen:**

- Mission 688 **completing at the terminal**, before you reach the ring switch. Step 2356 has only one required objective, so calling `complete_objective` on 2734 would trip the all-required-done auto-complete and end the mission early. Chain 1107 uses `advance_step` precisely to avoid this. If the mission ends at the terminal, that is a hard fail.
- Blurb 2518 on any other accept.
- The ring switch doing anything while you are still on step 2356.
- A ring transport animation on the exit. That is deliberate — neither platform has a wired `GLB-RingTransporterBase` prefab, so the exit is a direct cross-world teleport. It is on the prerelease known-issues list.

**Relog check:** relog on step 2356 — terminal highlight restored (chain 1110). Relog on step 80688 — ring-switch highlight restored (chain 1111).

**Server evidence:** `logs\content.log`, `Content: displaying dialog` `dialog_id=2518 chain_id=1154`; `Content: advancing step` `mission_id=688 step_id=80688 chain_id=1107`; `Content: completing mission` `mission_id=688` and `Content: cross-world teleporting entity` — both `chain_id=1109`.

---

## Milestone M4 — Boundary

### T21 / T22 — Arrival in Castle with Frost's Letter intact (C04, C09 UAT-only)

**Packets:** C04 (the mission), C09 (UAT check only — the Castle-side authoring belongs to the sibling Castle campaign as CA01, which merged in PR #659).

**Preconditions:** T20 passed; you are arriving in Castle (world 8).

**Steps:**

1. Arrive. Check where you land.
2. Open the quest log.
3. Look for Sgt. Gerschon (`Castle_SgtGerschon`, spawn 112, template 149) at `(429.64, 70.11, 996.56)`.
4. Check your character's appearance and your inventory.

**Expected:**

- You arrive at `(466.365, 70.397, 991.466)` on the Castle ring platform — within interaction range of Gerschon.
- **Mission 1360 "Frost's Letter" is still active**, on step 4037. Step 4038 "Give Cpl. Frost's Letter to Col. Marsh" is Castle-side content and is out of this campaign's scope.
- Frost's Letter (item 3730) is still in your mission inventory.
- Gerschon is live and offers the mission-701 handoff (that is CA01's acceptance, not this campaign's — just confirm the handoff lands).

**Must NOT happen:**

- Mission 1360 being abandoned or lost by the cross-world hop. Mission state rebuilds from `query_saved_missions` on the far side exactly like a relog does, so it should survive for the same reason any other active mission does.
- Arriving out of interaction range of spawn 112, or off the platform.
- Character appearance/colour corruption across the transfer — the v3 spec flagged a "character-creation colour bug" as a P0 check for this transition. Look at your character before and after.

**Relog check:** relog in Castle. 1360 must still be active.

**Server evidence:** `logs\content.log`, `Content: cross-world teleporting entity`; `logs\missions.log` for the mission rebuild on the far side.

### T23 — Relog safety sweep

**Packets:** all.

This is not a separate playthrough — it is the discipline of logging out and back in at **every** step boundary above and confirming the restore chain repaints what it should. Interaction bits are never persisted, so every chain that sets one needs a `player_loaded` partner.

| Restore chain | Gate | What must come back |
|---|---|---|
| 1006 | 622 step 2113 | Frost's dialog binding |
| 1007 | 622 step 80623 | Guard's dialog binding |
| 1023 | 689 not completed | Prison Boot lock re-applied |
| 1045 | 640 step 2120 | Livewire wrench on the ring switch |
| 1046 | 640 step 2215 | RingNetwork icon + quest highlight |
| 1062 | 641 step 2121 | P90 locker highlight |
| 1063 | 640 complete, 641 not active | Marsh's "talk to me" indicator |
| 1064 | 641 step 3563 | Marsh's marker |
| 1065 | 641 step 3564 | Terminal's Livewire marker |
| 1074 | 680 step 2344 | Both ring-switch bits |
| 1104 | 687 step 2354 | Crate highlight |
| 1110 | 688 step 2356 | Terminal highlight |
| 1111 | 688 step 80688 | Ring-switch highlight |
| 1112 | 639 not completed | Stasis Sickness re-launched |
| 1162 | 686 complete, 687 not active | Marsh stays despawned |

**Must NOT happen:** any one-shot cinematic replaying on login. No `player_loaded` chain plays sequence 10001, 10014, 1749, 1751 or 10000 (except chain 1002's deliberate 10000 replay when 622 is already complete), and no dialog blurb re-shows.

**Known gap:** GC1's escort state (Marsh's reposition and follow) has **no** restore chain. See T28.

---

## Results

Fill this in as you go. "Blocked" means you could not reach the scenario.

| Scenario | Pass / Fail / Blocked | Notes |
|---|---|---|
| T25 — Prison Boot gate (C00) | | |
| T01 / T02 — Zone entry + Stasis Sickness (C03) | | |
| T03 / T04 — Frost, Guard, Frost's Letter (C04) | | |
| T05 / T06 — Prisoner 329, Tau'ri | | |
| T05 / T06 — Prisoner 329, Jaffa | | |
| T07 — Cell door Livewire | | |
| T26 — Region8 guard aggro (C01) | | |
| T08 — Cover objective, cover-then-kill (C05) | | |
| T08 — Cover objective, kill-then-cover (C05) | | |
| T09 — The cure (C03) | | |
| T10 — Hack the rings + blurb 2305 (C07) | | |
| T11 / T12 — Marsh briefing + blurb 4000, Tau'ri | | |
| T11 / T12 — Marsh briefing + blurb 4000, Jaffa | | |
| T13 — Stasis terminal, mission 680 | | |
| T27 — Marsh's pre-departure line (GC1a) | | |
| T28 — Marsh rings + follows topside (GC1b) | | |
| T14 — Mess Hall (681) | | |
| T15 — Hallway chain (682-686) | | |
| T16 / T17 — Straegis scene (C08b + GC1a) | | |
| T18 / T19 — Aftermath, Tau'ri | | |
| T18 / T19 — Aftermath, Jaffa | | |
| T20 — Armory + blurb 2518 (C07) | | |
| T21 / T22 — Castle arrival, 1360 intact | | |
| T23 — Relog safety sweep | | |

## Known limitations / not validated

These are expected absences. Do not file them as bugs from this pass.

- **C06 — flank objectives, pending.** Objectives **2725** ("Take position behind the long table to flank the guards and negate their cover.", step 2348, mission 681) and **2731** ("Take a flanking position to negate the guards' protective cover.", step 2353, mission 686) have no chains. Chain ids 1141-1150 are reserved and unused. Per D-CB05 the flank objectives are meant to be *tracked but not gating*, so even when C06 lands the kill counter still completes the mission. Their sibling objectives **2724** and **2730** have a single space as their display text and will render blank in the objective list — that is seed data.
- **GC1c — lockdown VFX, not built.** No energy-field actor and no Kismet event id have been recovered. Sequence 10000 is already bound to the mission-622 stasis door and cannot double as the lockdown route. Dialog 4003 is deliberately not played because it describes a barrier that does not exist. Step 2345's log text still reads "Lockdown!" — that is just the step's text, not a promise of a barrier.
- **Chain 1153 — mission 680's accept blurb 2308, reserved and not authored.** Held back because its "follow Marsh" text only reads correctly with GC1 in place. GC1 has since merged, so re-adding 1153 is now a small follow-up.
- **GC3 — mission-completion XP, blocked on design.** No XP is awarded for completing 680 / 681 / 686. The spec's observed 52 XP is an observation, not a DB value; every `reward_xp` is 0 and the formula is unknown.
- **Effects are inert.** Effects 1634 (Stasis Sickness), 1636 (cure), 1939 (boot lock), 1942 (lock clear) and 3081 (boot item swap) all have `script_name = NULL` and `pulse_count = 1`. `register_active_effect` returns early for non-pulsing effects without sending any wire packet. Server-side, launching any of these abilities is a no-op today. Whether the client renders anything of its own is exactly what T25 and T01 are for.
- **Objective checkboxes may not tick on implicit completion** — issue **#656**. `advance_step` completes the old step's remaining objectives without emitting `ON_OBJECTIVE_UPDATE`. Affects C05's second objective and mission 688's objective 2734.
- **`system_message` does not render** — the executor arm is a stub with the wire format unresolved (issue #268). Chain 1013's message 5040 on Region2 entry will not appear.
- **No ring ceremony on the Cellblock → Castle exit.** Deliberate: neither platform has a wired ring prefab, so chain 1109 does a direct cross-world teleport. Prerelease known issue.
- **Straegis scene is camera-only.** No rift creature, no blood decal, no data disc — none has a recovered actor, event id, template or item id.
- **No escort restore across relog.** Marsh's ring-hop reposition and follow state are not restored on login (T28). Documented gap awaiting a coordinator decision.
- **Symbiote Loss (ability 1926 / effect 2480) is out of scope.** Never wired in the shipped build; the ability has a placeholder name. Jaffa players will not lose a symbiote.
- **Stasis Sickness Stage 2 (ability 1373) is out of scope** — it needs an engine timer primitive that does not exist.
- **Stasis pods are not driven server-side.** Whether they visibly open after the terminal hack is entirely client Kismet.

## Known risks

**Issue #582 — static NPCs invisible until relog.** The open Castle Cellblock bug is the GuardBody corpse not rendering until the player relogs. A June colo repro disproved the original address-gate hypothesis (the AoI warnings never fired), which puts the drop somewhere downstream in create + appearance delivery. Issue #582 added the `aoi.create_emit` (DEBUG) and `aoi.create_send_failed` (WARN) seams to localize it on the next repro.

**Where this bites in this guide:**

- **T28, Marsh's ring hop.** Marsh has never been in your AoI before chain 1173 snaps him to the Ring 3 pad, so it is effectively a fresh AoI entry — exactly the failure shape. GC1b-1's packet notes call out exercising this deliberately.
- **T03, the NID Guard corpse**, which is the original repro subject.
- Any freshly spawned or repositioned entity you are expected to see and cannot.

**If something you should see is invisible:**

1. Note it in the results table before doing anything else.
2. Grep `logs\server.log` for `aoi.create_send_failed`. Record `entity_id`, `phase` (`create_base` vs `cascade`) and `reason`.
3. If that warning **did** fire, the drop is at the emit and the reason field names it.
4. If it did **not** fire, the packets were sent successfully and the drop is downstream — that is the more useful finding, and it goes on issue #582.
5. Relog and confirm the entity appears. "Invisible until relog" is the signature.

Content-driven despawns previously used the bare `destroy_entity` path rather than `despawn_npc`, which reproduced the same shape. C08b's follow-up commit widened the fix to the remaining call sites, so `Content: destroying tagged entity` now reports a `witnesses_notified` count — if that count is zero when other players or you should have seen the despawn, that is worth recording too.
