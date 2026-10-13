# Hub pass UAT through the lab (round 2)

> Type: explanation and runbook (campaign ledger). Audience: the coordinator who runs the packets, the Sonnet verifiers, and the owner who signs the rows off.
> Planned 2026-10-12 against the colo build `v2026-10-12.3` (deployed 23:04Z; the colo database is rebuilt from the seeds on every deploy, so every character below is created fresh). Nothing here has been run yet: the Status column of every table says so.
> Companions: [work-packets.md](work-packets.md) (dispatch rules and one section per packet), [briefs/](briefs/README.md) (the driver briefs, ready to paste), [verify/](verify/HP-M1.md) (the Sonnet verifier briefs), [unified-uat.md](../../guides/unified-uat.md) (the rows), [Haiku lab UAT round 1](../haiku-lab-uat/README.md) (the recipes this plan copies), [lab-harness close-out](../lab-harness/closeout.md) (what the lab tools can and cannot do), [automated-uat.md](../../guides/automated-uat.md) (result vocabulary and native tiers N1-N3).

## Purpose

Sign off the rest of "Session 2, the hub pass" of [unified-uat.md](../../guides/unified-uat.md#suggested-test-order) on the colo, through the live research lab, with cheap agents:

1. A Haiku `lab-driver` drives one packet (15 to 25 tool calls) from a written brief and reports raw facts.
2. A Sonnet verifier grades every row against server evidence (ClickHouse over ssh, read-only database queries), never against the driver's claims.
3. A Haiku looker reads the saved screenshots and answers yes/no questions for the look-and-feel checks.
4. A bug the verifier finds becomes a GitHub issue with an implementation plan.

Round 1 ([report](../haiku-lab-uat/README.md)) did the bank, consumables and ability-tree rows this way. This round does the social rows (mail, chat), pets and deployables. Bank, consumables and ability trees are not repeated.

## Scope

Rows come from [unified-uat.md](../../guides/unified-uat.md): [Mail, chat and duels](../../guides/unified-uat.md#mail-chat-and-duels), [Pets](../../guides/unified-uat.md#pets), [Deployables](../../guides/unified-uat.md#deployables). Duel rows (11-13) are out of scope: they need two real players.

| Row | What it tests | Packet | Notes |
|---|---|---|---|
| Mail 1 | new-mail notice with the mailbox closed and open (`.mail` to yourself); a name that does not exist | HP-M1, HP-M4 | the bad name goes through the real compose window in M4 |
| Mail 2 | take 100 naquadah; the real send with 25 postage | HP-M1, HP-M4 | `.mail cash` mints, so the postage is only testable through the compose window |
| Mail 3 | take an item | HP-M1 | the full-bags retry is not run (see below) |
| Mail 3a | a mission-only item (Gopher Head, item 10) cannot be mailed, plain or COD | HP-M4 | |
| Mail 4 | COD: cannot take before paying, pay, take, payment mail | HP-M2 | |
| Mail 5 | return an unpaid COD; it cannot be returned twice | HP-M3 | |
| Mail 6 | GM `.mailbox`, `.mail_expire` on a COD, a plain mail and a returned mail | HP-M3 | |
| Chat 7 (solo) | tell yourself, tell an offline name | HP-C1 | |
| Chat 9 | welcome line colour and tab, no popup, one tab per channel | HP-C1 | the flood-limit sentence is not runnable (see below) |
| Chat 9a | `/say`, `/emote`, `/yell` each show once | HP-C1 | `/say` is typed with real keys |
| Chat 9c | repeat join, bad leave, the 10-channel cap | HP-C2 | |
| Chat 10 | `.announce`, `.announce space`, `/gmsendgmshout`, `.location` colour | HP-C2 | without `.mute B`. Broadcasts reach every online player |
| Pets U1, U2b, U3 | `.pet summon 350`, `.giveability 2826`, the cast with its 6 s warmup | HP-P1 | |
| Pets U4, U6, U12 | follow and leash, stances (buttons and console), `.pet info` | HP-P2 | |
| Pets U5, U7, U8 | defend the owner, pet-bar order, kill XP | HP-P3 | U8 is XP and credit only |
| Pets U14, U15, U16 | Jaffa, Lo'taur, Prime; Lo'taur fight recorded only; a refused pet ability | HP-P4 | U15 is known broken (K12): recorded, not graded |
| Pets U17-U20 | Holy Warrior, To The Death, Heed Our Calling, no pet | HP-P5 | |
| Pets U9, U10 | owner death; relog and zone change; the U2b relog half | HP-P6 | the second-character half of U10 is dropped |
| Deployables DP-U1, DP-U2, DP-U4 | place one, what it looks like, 30 s lifetime | HP-D1 | the ground cast has never been done in a real client |
| Deployables DP-U3, DP-U5, DP-U8 | it hurts Rats; cooldown and range refusals; cannot be attacked | HP-D2 | |
| Deployables DP-U6 | gone on death, logout, zone change | HP-D3 | |

### Rows judged not runnable, or only partly

| Row or half | Why | What would make it runnable |
|---|---|---|
| Chat 9, "paste ten lines fast: one 'too quickly' line" | GM accounts skip the flood limit (`docs/gameplay/chat-system.md`, "Access level GameMaster (2) and above skip the bucket"), and the lab accounts `lab`, `lab2`, `lab3` are access level 2. HP-C1 pastes ten lines anyway and records that none is refused | a non-GM account on the colo (decision D4: deferred to Session 7; the flood-limit clause is not graded in this pass) |
| Chat 10, `.mute B 1` | needs a second player (dropped by the request) | two accounts |
| Chat 7, DND and Ignore halves | need a second player | |
| Mail 1, "B logs in later"; Mail 3, "retry with full bags" | need an offline recipient who logs in, and 100+ bag fills | a second character; a fill recipe |
| Pets U8, the KillCount objective | a mission with a kill counter must be assigned; the only seeded chains with `OnEntityDeath -> IncrementCounter` are in the Cellblock/Castle missions, which this pass does not run. The XP and credit half is run | decision D6 |
| Pets U6, the pet info window half | `PetAbilityWin` opens from a menu in this client. A follow-up could open it with `PetMod.onTogglePetInfo()` (Lua) and click `Pet_StanceIcon_1..3` | a small follow-up packet |
| Pets U10, the second character half | dropped by the request. HP-P6 checks the server rows for an orphan instead | |
| Pets U15 | known broken (K12). Recorded only | n/a |
| Deployables DP-U5, behind a wall | needs a spot behind an occluder inside the 5 m range and a camera that can click it. The hub room has no placed-and-checked occluder data (see `docs/content/debug-hub.md`, "Placement is unchecked") | a Sonnet exploration task (decision D7) |
| Deployables DP-U7 | needs a second player | |
| Every "only a human can check" item | the looker is a pre-screen. Its answers are recorded as `PASS(looker)`; the row stays `NEEDS_HUMAN` for the look-and-feel part until the owner confirms | the owner's own session |
| Not in the request, so not planned | Pets U2, U11, U13; Chat 8, 9b, 11-13; Mail 3b (needs crafting items); the Gate Mail Clerk (K2) | |

## Facts the packets rest on

All checked in the repo on 2026-10-12; nothing below was run live.

| Fact | Value | Source |
|---|---|---|
| Lab accounts | `lab` (id 10), `lab2` (11), `lab3` (12): `accesslevel` 2 = GM, `test_account` true | `db/sgw/Accounts/Seed/account.sql`. Round 1 used the same accounts for GM-only commands (`.bank`, `/gmgiveitem`, `.gotolocation DebugArea`) on fresh characters, so no extra rights step exists |
| Instances | `default` = `lab` (client 1), `p2` = `lab2`, `p3` = `lab3`; one lease each | [live-research-lab.md, Parallel clients](../../guides/live-research-lab.md#parallel-clients-up-to-five) |
| Characters (new on every deploy) | `Hubmail` on `default`, `Hubpets` on `p2`, `Hubdeploy` on `p3`: SGU Soldier, male, first name `Uat` (3+ letters, letters only) | round 1 made `Bankone`/`Labtwo`/`Labthree` the same way |
| Creation call | `lab_ensure_in_world {lease_id, character:"<Last>", create:{alignment:"sgu", archetype:"Soldier", gender:"male", first:"Uat"}}` | [lab-driver.md](../../../.claude/agents/lab-driver.md), round 1 BANK-C1 |
| Where a new SGU character appears | SGC_W1, world 58; entity introductions are held 16 s, so wait 17 s | round 1 README, Setup |
| Where the packets work | Mail and chat: SGC_W1, no travel. Pets and deployables: the Debug Area plaza, `.gotolocation DebugArea 252.0 7.0 -923.0` (12 m of clear floor), world 1300 (GM only; 20-40 s to load) | `docs/content/debug-area.md`, round 1 AT-C1 used `.gotolocation DebugArea 257 7.2 -928` |
| Why not the stasis-room hub | The hub NPCs these rows would use are broken (Gate Mail Clerk, Dialog NPC: K2) or not needed (the pet trainer is U2, out of scope). The room's placement and navmesh are unchecked and cramped, and it has no hostile mobs. If a hub run is wanted: respawner `(-334.23, 73.47, -228.03)`, `.gotolocation Castle_CellBlock -334.23 73.47 -228.03`, NPC coordinates in `docs/content/debug-hub.md` | decision D8 |
| Ordinary backpack item for `.mail item` | `2893` Health Slappack TC1: `container_sets {1,17}`, stack 10, flags 36224. Round 1 used `5254` Steel Core, a crafting component | `db/resources/Items/Seed/items.sql` |
| Mission-only item | `10` Gopher Head: `container_sets {2}` (Mission bag), flags 34816. `/gmgiveitem 10 1` may drop it in the Main bag (round-1 finding P5); the server must still refuse to mail it | same file, unified-uat.md row 3a |
| Hostile mob | template `74` "Rat" (AMBRat, level 1, faction 10, pistol fallback ability 592). `.spawnrandom 74 5 5 <n>` creates `n` of them on a ring and answers with their ids. The Debug Area gallery also has a passive one at spawn 13307 `(99, 23.06, -592)` | `docs/content/debug-area.md`, `docs/commands.md` |
| Pet kill target for XP | the same Rat; Straegis pet ability `221` Energy Shock does the damage | `docs/gameplay/pet-system.md` |
| Death site (U9, DP-U6) | four level-31 NID Operatives at about x 466-469, z -911 to -919: `.gotolocation DebugArea 458.0 12.5 -915.0`, they kill a level-1 character in about 10 s; respawners 130 and 131 | `docs/content/debug-area.md`, "Death and respawn test" |
| Ability ids | summons 2826 (Straegis), 1643 (Jaffa), 1644 (Lo'taur), 1645 (Prime): warmup 6 s, cooldown 5 s; 2824 Holy Warrior; 2839 To The Death; 2852 Heed Our Calling (passive); 1654 Prime: Focus Degeneration (does nothing, refused); 1012 Deployable: Microwave Emitter (ground target, range 5 m, warmup 2 s, cooldown 30 s, 30 pulses, radius 10 m); 221 Energy Shock; 592 Pistol Shot | `db/resources/Abilities/Seed/abilities.sql` |
| Mail window | `Access_MailButton` toggles it; `GateMailInboxWin`, `GateMailReadWin`, `GateMailCreateWin`; `GateMailInbox_TakeAttachmentsButton`, `GateMailRead_PayCODButton`, `GateMailRead_ReturnMessageButton`, `GateMailCreate_SendButton`; Lua `mailGetHeaderInfo`, `mailTakeItem`, `mailPayCOD`, `mailSetItemAttachment`; a client COD take is blocked client-side, so the server guard is probed with `mailTakeItem(id,-1,-1)` (N3) | the client's `Content/UI/Core/GateMail/GateMail.lua` (read from the local client copy) |
| Chat internals | `ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input)` runs what Enter runs; `ChatMod.stickyCommand` remembers the last channel; `client_chat_log` returns `channel`, `colour`, `tabs`, `seq` | `ChatWindow.lua`, `crates/lab/src/server/ui/mod.rs` |
| Pet internals | pet unit ids `Unit.Pet1..4`; windows `PetContainer1Win`, `PetContainer1_StanceButton1..3`; `usePetAbility`, `changePetStance` | `Content/UI/Core/Pet/*.lua` |

### GM commands, checked against the code

All exist in the console registry (`crates/cell-console/src/cell/console/registry/commands/`) and in [commands.md](../../commands.md#dev-console--commands):

| Command | Syntax used here | Where |
|---|---|---|
| `.pet` | `summon <templateId or abilityId>`, `dismiss`, `stance <0-2>`, `info`, `list` | `pet.rs` |
| `.giveability` | `<abilityId>`, saves to the character | `abilities.rs` |
| `.mail` | `[to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]` | `social.rs` |
| `.mailbox`, `.mail_expire` | `.mailbox [name]`, `.mail_expire <mailId>` | `social.rs` |
| `.announce` | `[space] <text>` | `social.rs` |
| `.location` | no arguments reports the selected NPC's or object's position; the registry gives it `Target::Spawnable`, so with nothing selected it answers with an error line (still a GM feedback line, which is what Chat 10 colours check) | `travel.rs` |
| `.spawnrandom`, `.spawn`, `.aggro`, `.gotolocation`, `.bug` | `.spawnrandom <templateId> <xRange> <zRange> [count]`; `.aggro on\|off`; `.gotolocation <World> x y z`; `.bug <note>` | spawn, travel, meta |
| `/gmgivecash`, `/gmgiveitem`, `/gmsetgodmode`, `/gmsendgmshout` | `<n>`; `<id> <qty>`; `<0\|1>`; `<global 0\|1> <text>` | `docs/commands.md` |

## Preflight (the coordinator, before the first packet)

1. **Kill the screensaver, then hold the display awake.** `Get-Process -Name Ribbons -ErrorAction SilentlyContinue | Stop-Process` (round 1 lost a start to `Ribbons.scr`). Check `Get-Process | Where-Object { $_.Path -like '*.scr' }` is empty. `lab_ensure_in_world` fails with `D3DERR_NOTAVAILABLE` while the Ribbons.scr screensaver runs (its timeout on this workstation is 600 s), so killing it once is not enough: for the whole run, keep a process alive that calls `SetThreadExecutionState` with `ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED` (0x80000003) so the screensaver and display sleep never start again.
2. **The build lane must be idle.** `python tools/build-lane/lane_stats.py --recent 3` shows nothing running, and `Get-ChildItem $env:LOCALAPPDATA\cimmeria-build\lane` has no `slot.*` folder. Round 1's three failed starts happened under other sessions' cargo jobs. Do not start a build while the lab runs.
3. **The lab is installed and hosts three instances.** `%LOCALAPPDATA%\cimmeria-lab\labd.env` has `CIMMERIA_LAB_INSTANCES=default,p2,p3`; `lab instances status` shows `lab`, `lab2` and `lab3` account files and no running client; the daemon runs the current build (`lab restart` if you changed anything). Reconnect the MCP server with `/mcp` in the coordinator session after a restart, and start the driver agents after that, so their tool lists are current (LH-17 added `client_item_action`, `client_lua_eval`, `client_type_text`, `client_die_and_respawn` to `lab-driver`; D2 added `client_use_ability` and `client_hotbar`; a session that predates it still has the old agent).
4. **The lab points at the colo.** `server_db_query` (lab-server) answers `SELECT now()` from the colo database, and the client's `LoginInternal.lua` has no stale `Local` row (a stale row once sent the lab to a local server while the database read the colo).
5. **Server evidence works.** `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT count() FROM signoz_logs.distributed_logs_v2 WHERE timestamp > now() - INTERVAL 1 HOUR"'` returns a number. The SigNoz MCP returned 401 last round; use it only if it works now. Look at one pets row and one deployables row once to see whether DEBUG rows are exported (the pets and deployables telemetry is mostly DEBUG).
6. **The accounts are GM and the build is the expected one.** `SELECT account_name, accesslevel FROM public.account WHERE account_name IN ('lab','lab2','lab3')` returns 2 for all three; the colo runs `v2026-10-12.3` or later (the latest `service.version` in ClickHouse).
7. **The colo is quiet for HP-C2 only.** `server_sessions` lists only the lab clients before HP-C2 (its `.announce` and `/gmsendgmshout` reach everyone online and open a modal prompt on every client).
8. **Fresh characters.** `SELECT player_name FROM public.sgw_player WHERE player_name ILIKE 'Hub%'` is empty after a deploy; if the three exist (a re-run) they are reused, which is fine, but note that HP-P1 and HP-D1 then skip their WAIT17.
9. **Stagger the starts.** Dispatch the first `default` packet; when its driver is in the world, wait two minutes and dispatch the first `p2` packet; two minutes after that, the first `p3` packet. A client boot takes about 25 s with others running, and a character's first login also loads SGC_W1.
10. **No lab lease is held by anyone else.** `lab_lease_status` without arguments lists all three instances with no holder.

## Pipeline and retry rules

```
dispatch driver (Haiku lab-driver, attempt 1)
   -> report, or no report (ran out of context / crashed)
        -> attempt 2 (successor: RESUME PROBE), attempt 3
             -> still not finished: a Sonnet agent drives the packet (attempt 4)
   -> verifier (Sonnet) grades each row from server evidence
        -> looker (Haiku) answers the screenshot questions
        -> a product bug: the verifier files a GitHub issue with a plan
   -> the coordinator copies the result into the results table
```

- **A packet attempt is finished** when the driver's final report has a block for every row of the packet (each DONE, FAIL, BLOCKED or NOT-RUN with a reason) and says `lease released: yes`. Anything else is not finished: the driver ran out of context, stopped on an infrastructure error, or returned nothing.
- **Attempt 2 and 3** use the same brief file with the `ATTEMPT:` field changed. Each brief starts with a cheap probe (mailbox headers, the pet, the `HPUdone` chat lines, the known abilities) and has per-row completion markers, so the successor skips finished rows. The markers per packet are in [work-packets.md](work-packets.md). A successor takes over the dead driver's lease with `force` only when the lease owner is exactly `hubpass-<packet>` (the brief says how); the lease also expires by itself after 900 s.
- **Do not burn attempts on a recipe gap.** If a driver reports BLOCKED because a recipe does not work (HP-D1's ground cast above all), go straight to a Sonnet agent: a third Haiku attempt would repeat the same failing call.
- **After three failed attempts** a Sonnet agent drives the packet. Brief it with the same file plus the Sonnet addendum in [work-packets.md](work-packets.md#sonnet-takeover): all `cimmeria-lab` tools are allowed (`client_use_ability`, `client_hotbar`, `client_target`, `client_combat_log`), it may find a new recipe, and it records the recipe it found so the briefs can be updated.
- **The verifier grades what was done.** If attempts together covered a row, the verifier grades the combination; each report goes in its brief. A row nobody reached is `NOT-RUN` and stays in the status table.
- **Re-runs.** A row graded `FAIL` because of a lab or driver fault (not a product fault) is re-run once by a new driver with a corrected brief; the verifier says which in `RESUME HINT`.

## The looker

For each screenshot a verifier brief names, the verifier starts a Haiku agent (Agent tool, `subagent_type: general-purpose`, `model: haiku`) with: `Read the image file <PATH>. Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible.` followed by the numbered questions in the brief. Screenshots are saved by the lab at `%LOCALAPPDATA%\cimmeria-lab\screenshots\` and cost the driver nothing but the path. The looker never sees the driver's report. A NO or UNSURE leaves a row at best `PARTIAL (visual)`; a black capture is reported as such.

## Filing a bug

The verifier does this (it has `gh`); the coordinator only links the number into the results table.

1. **Decide what it is.** (a) A product bug or (b) a doc mismatch: file. (c) A lab, harness or driver problem (a wrong click, a missing recipe): not a product bug; comment on the lab-harness issue [#1660](https://github.com/SandboxServers/Cimmeria/issues/1660) if it is a tool gap, otherwise only report it. (d) Already in the [known issues](../../guides/unified-uat.md#current-known-issues) K1-K33: do not file.
2. **Search first.** `gh issue list --search "<words>" --state all`; comment on an existing issue rather than filing a duplicate.
3. **Investigate before filing.** Read the code path the row exercises and the doc that describes it; put the ClickHouse rows (with times), the driver's quote, the screenshot paths and what `docs/` already says in the body.
4. **Write the body to a file and file it from PowerShell:** `gh issue create --title "<system>: <what is wrong> (hub pass UAT <packet>)" --label bug --body-file <file>`. The body follows the ticket contract in [issue-tracker.md](../../agents/issue-tracker.md#ticket-body-contract): `## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type` (from [TESTING.md](../../../TESTING.md), with the bug shape the guard must fail on), `## Docs to update` (rows of [doc-update-map.md](../../agents/doc-update-map.md)), `## Client impact`, `## Domain advisor`, `## Needs a human for`, plus an `## Implementation plan`: files and functions, order of edits, and the failing test. Mention `@Cadacious`.
5. **Labels.** `bug`, and `needs-triage` only if that label exists yet ([triage-labels.md](../../agents/triage-labels.md): only `wontfix` existed on 2026-09-19). Never `ready-for-agent`: only a maintainer applies it. Anything whose acceptance test is in-game UAT is `ready-for-human` by that file's rules; say so in the body instead of labelling.
6. **Per root cause, one issue.** Several failing rows with one cause share an issue.

## Escalation to Opus

Opus is for the cases below and nothing else. A Sonnet verifier or takeover agent that thinks it needs Opus says which case in its report; the coordinator decides.

1. **A finding needs reverse engineering or an architecture decision.** For example a row fails because the client expects a different wire layout, a new opcode, a client patch or a wire-crypto change (CLAUDE.md: those need a maintainer decision first), and the evidence in `docs/protocol/` and the RE findings disagree. Opus writes the proposal; a human decides.
2. **A server-authority hole.** A row shows the server accepting something a hostile client could abuse (an item leaving the bag on a refused mail, a cooldown bypass, a pet order on another player's pet). Opus runs the adversarial review; the verifier files the issue first.
3. **The same infrastructure failure stops two lanes for more than an hour** (client crashes, lease or daemon trouble, the colo down) and a Sonnet agent could not diagnose it.
4. **Sonnet and the verifier disagree** about whether a failure is a product bug or a lab fault, the answer decides whether a seed or a wire constant changes, and one more Sonnet look would not settle it.

Never for: re-driving a packet, finding a driving recipe, reading logs, grading a row, writing a plan for a bug in server code (Sonnet does these).

## Decisions

| # | Decision | Status | Why / what it blocks |
|---|---|---|---|
| D1 | Three characters, one per section and instance: `Hubmail` (`default`, mail and chat), `Hubpets` (`p2`), `Hubdeploy` (`p3`); SGU Soldier, male, first name `Uat` | Adopted | Round 1's recipe; lets the three lanes run in parallel without sharing a client |
| D2 | Add `client_use_ability` (and `client_hotbar`) to the `lab-driver` tool list so casts and pet orders run at N1 | **Decided yes, 2026-10-13, by the owner** | Today the driver casts with stock Lua (`useAbility`, `usePetAbility`: N3), so every cast row grades at best `NATIVE_SHORTFALL`. The tool costs about 1k tokens of schema per turn. Without it the pets and deployables rows can still prove the server behaviour. Effect: both tools are in `.claude/agents/lab-driver.md`; the pet and deployable briefs cast with `client_use_ability` (hotbar key or click, N1; `place:true` is N3 setup only), press pet-bar and stance buttons with `client_ui_sequence` clicks (the tool does not drive the separate pet bar), and keep stock Lua only as a labelled N3 fallback. The verify briefs expect N1 and PASS-eligible. A session that predates the agent edit needs a restart to see the tools |
| D3 | Tier policy: a GM or slash command submitted by `ChatMod.onTextAccepted` (CHATX) counts as typed (G) when the command itself is the step; the cast, stance click, take, pay and return rows need N1 for `PASS` | Adopted | Matches round 1 ("typed") and the vocabulary in automated-uat.md; the real keys are used for `/say hi` (9a) where typing is the point |
| D4 | A non-GM account on the colo, to test the chat flood limit ("too quickly") and the non-GM refusal of GM commands | **Deferred, 2026-10-13, by the owner** (to Session 7 / a non-GM account) | The lab accounts are GM and exempt. Effect: Chat 9's flood-limit refusal is not graded in this pass. HP-C1 may still paste the ten lines when that is cheap and records the result as not graded |
| D5 | HP-C2 runs alone, with no other lane driving, and only when `server_sessions` shows no real tester online | Adopted (a scheduling rule) | `.announce` and `/gmsendgmshout` open a modal prompt on every online client |
| D6 | The KillCount half of Pets U8 needs a seeded mission with a kill counter that a Soldier can hold; none is wired for this pass | **XP half only, decided 2026-10-13, by the owner** | Effect: U8 grades XP and kill credit; the KillCount half is PARTIAL / not run |
| D7 | DP-U5's "behind a wall" case is a Sonnet exploration task after HP-D2: find a spot inside 5 m of an occluder, then click it with a top-down camera | Proposed | The hub room's occluder data is unchecked, and a click behind a wall needs a camera above it |
| D8 | Pets and deployables use the Debug Area plaza, not the stasis-room hub | Adopted | Hostile mobs on demand, 12 m of clear floor, no travel between rows; the hub NPCs these rows would use are broken or not needed |
| D9 | The looker's answers are a pre-screen. A visual-only row (the model on the ground, tab colours, the pet bar) stays `NEEDS_HUMAN` for the owner until a person confirms it | Adopted | automated-uat.md has `NEEDS_HUMAN` for exactly this |
| D10 | A self-send through the compose window may be refused; if so HP-M4 retries once with `Hubpets` as the recipient | Adopted | No doc says a player may mail themselves; `.mail` to yourself is the documented solo fallback |
| D11 | U15 is recorded, never graded; the row is closed by K12 | Adopted | Known broken |
| D12 | Chat lines for GM commands go through `ChatMod.onTextAccepted` in one `client_batch` (many lines, one call) instead of three calls per line | Adopted | Cuts a packet's calls by half; the real keys are proven in round 1 and used where typing is the test |

## Packet status

Statuses: Ready, Driving, Driven (report in), Verifying, Verified, Done (results table filled), Blocked.

| Packet | Instance | Rows | Est. calls | Depends on | Status |
|---|---|---|---|---|---|
| HP-M1 | default | Mail 1, 2, 3 | 23 | none | Verified 2026-10-13 (attempt 1) |
| HP-M2 | default | Mail 4 | 24 | HP-M1 (the character) | Done 2026-10-13: pay half (attempt 1) + takes (probe 07:58Z); attempts 1-2 blocked by covering windows (#1676) |
| HP-M3 | default | Mail 5, 6 | 21 | HP-M1 | Verified 2026-10-13 (attempt 1) |
| HP-M4 | default | Mail 1 (window), 2 (window), 3a | 25 | HP-M1 | Verified 2026-10-13 (attempt 1) |
| HP-C1 | default | Chat 9, 9a, 7, flood probe | 19 | HP-M1 | Verified 2026-10-13 (attempt 1): Chat 7 PASS, Chat 9 PARTIAL (visual), Chat 9a FAIL (#1678 emote twice); lab gap #1679 |
| HP-C2 | default | Chat 9c, 10 | 23 | HP-M1; a quiet colo (D5) | Ready |
| HP-P1 | p2 | U1, U2b, U3 | 25 | none | Verified 2026-10-13 (attempt 1): U2b, U3 PASS; U1 PARTIAL, pet bar not shown (#1674); lab-harness note on #1660 |
| HP-P2 | p2 | U4, U6, U12 | 22 | HP-P1 | Verified 2026-10-13 (attempt 1): U12 PASS; U4 PARTIAL (leash not run, rerun `briefs/HP-P2-u4.md`); U6 PARTIAL (pet bar blocked by #1674) |
| HP-P3 | p2 | U5, U7, U8 | 26 | HP-P1 | Verified 2026-10-13 (attempt 1): U5 not exercised (Rat chose Hubdeploy on the shared plaza), U7/U8 BLOCKED (Hubpets died to two Rats at 07:58:46Z); setup faults, no product bug; rerun `briefs/HP-P3-r2.md` only while the deployables lane (p3) is idle |
| HP-P4 | p2 | U14, U15, U16 | 27 | HP-P1 | Ready |
| HP-P5 | p2 | U17, U18, U19, U20 | 27 | HP-P1 | Ready |
| HP-P6 | p2 | U9, U10, U2b relog | 18 | HP-P1; run last on p2 | Ready |
| HP-D1 | p3 | DP-U1, U2, U4 | 22 | none; may end BLOCKED | Verified 2026-10-13 (attempt 1): ground cast proven; U1 and U4 server-PASS, U2 shot void (pet in frame), retake in HP-D2 8.2 |
| HP-D2 | p3 | DP-U3, U5, U8, DP-U2 retake | 28 | HP-D1 proved the ground cast; p2 pets lane idle (shared plaza) | Ready (brief fixed 2026-10-13) |
| HP-D3 | p3 | DP-U6 | 26 | HP-D1; p2 pets lane idle (shared plaza, death site) | Ready (brief fixed 2026-10-13) |

## Results

Fill from the verifier reports. Result vocabulary: PASS, NATIVE_SHORTFALL, PARTIAL, FAIL, BLOCKED, NOT-RUN, NEEDS_HUMAN ([automated-uat.md](../../guides/automated-uat.md#results)). Tier: the lowest native level the step used (N1 real input, N2 slash command standing in, N3 stock Lua, G GM command). `Issue` links the GitHub issue a FAIL produced.

| Row | Packet | Result | Tier | Evidence (time, scope, event) | Issue |
|---|---|---|---|---|---|
| Mail 1 | HP-M1, HP-M4 | PARTIAL (visual); bad-name half through the compose window PASS (see HP-M4 findings) | G (`.mail`); mailbox opened by Lua (N3, setup) | 07:44:45 and 07:44:57Z mail `mail.gm_action` + `mail.system_sent` + `mail.notified` (header_pushed) for mail 1, 2; DB has both; looker: inbox open, HPUM1r1 listed, no New marker visible. `recipient_open_mail` is a mail count, not a window flag | |
| Mail 2 | HP-M1, HP-M4 | PARTIAL (visual) | N1 (Take click; mail selected by Lua = setup) | 07:45:11Z `mail.cash_taken` cash 100, naquadah 0 to 100; DB naquadah 100, mail cash 0, no WARN/ERROR; looker saw an "Att:" label, not a coin icon, on the entry (NEEDS_HUMAN) | |
| Mail 3 | HP-M1 | PASS | N1 (Take click; mail selected by Lua = setup) | 07:45:28Z `mail.item_taken` type 2893, container 1, slot 1; one 2893 row in `sgw_inventory`; `sgw_gate_mail_item` empty; no WARN/ERROR | |
| Mail 3a | HP-M4 | PASS | N1 (Send click); item attached by Lua `mailSetItemAttachment` (N3, setup; the stock step is a drag) | 07:53:19Z DEBUG `mail.attachment_refused` reason `item_no_carried_bag` item_id 10308, then WARN `mail.send_refused` result `MAILRESULT_ItemNotAvailable`; the COD retry at 07:53:34Z the same. DB: no HPUM4r3a/b mail, no `sgw_gate_mail_item` for type 10, Gopher Head still in container 1, naquadah 775 unchanged, no `cash_debited`. The server DID receive the attachment (`mail.send_decoded` item_id 10308, qty 1), so the "ItemDragContainer hidden" read was only a stale widget read. Looker: compose window open, no icon visible in slot, no red text (NEEDS_HUMAN) | [#1675](https://github.com/SandboxServers/Cimmeria/issues/1675) (P8: the item sat in Main) |
| Mail 4 | HP-M2 | PASS (history: attempt 1 pay half only; attempts 1-2 Take clicks blocked by GateMailReadWin/GateMailCreateWin covering the inbox, lab issue #1676; probe 07:58Z used GateMailCreateWin:hide() and GateMailReadWin:hide() as setup, then inbox Take clicks) | N1 (Pay click, inbox Take x2); forced mailTakeItem N3 probe of the guard; hide() calls are setup | 07:47:24Z WARN mail.op_refused take_item cod_unpaid; 07:47:32Z mail.cod_paid price 200, naquadah 600 to 400, payment_mail_id 6; 07:58:09Z mail.item_taken mail 5 item 10295 type 2893 to container 1 slot 3 (Main slot 4); 07:58:20Z mail.cash_taken mail 6 cash 200, naquadah 775 to 975; DB: escrow rows for mail 5/6 gone, naquadah 975, mail 5 cod_paid true; no WARN/ERROR in 07:57:30-07:58:50Z beyond the expected 07:47 refusal. Order refusal, cod_paid, item_taken held. Looker: not run (client-read and server rows sufficient) | none new (#1676 already filed) |
| Mail 5 | HP-M3 | PASS | N1 (Return click and prompt Yes click; mail opened by Lua = setup) | 07:50:18Z `mail.returned` mail 7, player 71, cod_cancelled=200 (a number, not a bool), item 10301 (type 2893); DB: returned true, cash 0, COD flag clear, escrow row kept; no second `mail.returned`, no `mail.op_refused`. The second Return click sent nothing: the stock client disables `GateMailRead_ReturnMessageButton` unless COD and cash > 0 (`GateMail.lua` ~594-606), and the returned mail has COD 0, so the clause is client-guard, client-read only. Looker: detail window open, prompt "Are you sure you want to return this message?" with Yes/No; Return and Pay shown as unlabeled icons (UNSURE) | |
| Mail 6 | HP-M3 | PASS | G (`.mail`, `.mailbox`, `.mail_expire`) | 07:50:56Z `mail.expired` mail 8 path returned/source gm, 07:50:58Z mail 9 path deleted, 07:51:01Z mail 7 path quarantined reason already_returned (WARN); three `mail.gm_action` mail_expire and two `mailbox`; DB: 8 returned with no COD price and escrow kept, 9 gone, 7 quarantined true; second `.mailbox` quarantined 1 | |
| Chat 7 (solo) | HP-C1 | PASS | G (typed `/tell`, CHATX; client-read for the lines) | 07:56:58Z DEBUG `chat.tell_refused` reason `self` (target player 71); 07:57:00Z `chat.tell_refused` reason `not_online` target Nobodyqqzz; client lines "You cannot send a tell to yourself." and "Player Nobodyqqzz is not online." (seq 144346/144347); squad: 07:57:02Z INFO `squad.chat` rejected `not_in_squad` + "You are not in a squad." (not graded). No WARN/ERROR | |
| Chat 9 | HP-C1 | PARTIAL (visual) | client-read only; relog real (N/A tier) | Relog was a real login: 07:56:00Z `session.end`, 07:56:01Z char list, 07:56:05Z playCharacter, 07:56:14Z `session.start` and onClientReady (the welcome is sent in that bundle; no per-send log row). Screenshot 07:56:34Z shows "Welcome to Stargate Worlds. Your player id is: 7." on the Info tab, no popup, no "joined channel" line. Colour: looker saw white and yellow lines, none sky blue (NEEDS_HUMAN: docs say sky blue). Auto-joins: 3 `chat.channel_joined` at the FIRST login 07:44:21Z only; at the relog the server logged `channel_left` x3 (`session_end`, 07:56:00Z) and no new `channel_joined`, because the same client process never re-sent `chatJoin` (expected behaviour of a relog, not a graded defect). Driver's chat read missed the welcome and gave no colour/tabs: lab gap | [#1679](https://github.com/SandboxServers/Cimmeria/issues/1679) (lab) |
| Chat 9a | HP-C1 | FAIL (emote shown twice) | N1 `/say` (driver report); emote and yell via CHATX (G) | One `sendPlayerCommunication` + one `chat.spatial_broadcast` (witness_count 0) per line at 07:56:45 (say, ch 0), 07:56:49 (emote, ch 1), 07:56:50 (yell, ch 2); no WARN/ERROR in chat/rate_limit. Client: say once (143960), emote twice (143961 flags 0, 143962 flags 1), yell once (143963 flags 1). The server sends the sender one echo per emote (flags 1), so the flags-0 copy is the client's own local line; `spatial.rs` gates the echo only for say. Looker: two "Hubmail HPUwaves" lines, three colours | [#1678](https://github.com/SandboxServers/Cimmeria/issues/1678) |
| Flood probe (D4, not graded) | HP-C1 | Recorded | G | 07:57:10-11Z ten `sendPlayerCommunication` channel 0 (text_len 9, one 10); zero `rate_limit` rows; account 10 `accesslevel` 2 = `CHAT_EXEMPT_ACCESS_LEVEL`, so the GM skips the chat bucket (`rate_limit/limits.rs`). The real "too quickly" refusal stays NOT RUN (D4) | |
| Chat 9c | HP-C2 | Pending | | | |
| Chat 10 | HP-C2 | Pending | | | |
| U1 | HP-P1 | PARTIAL | G (`.pet summon 350`) | 07:50:59Z pets.lifecycle `summoned` + pets.command, template 350, pet 100447, owner 4; `follow_armed` 07:51:00; no WARN/ERROR; client: pet entity rendered at 2 m, but `PetContainer1Win` and the three stance buttons read invisible, and the 07:51:09Z shot (10 s later) shows no pet panel (looker UNSURE/NO, my read agrees). Window name is right per PetContainer.lua. Server cannot say whether the owner-only lists were sent (no log on success). Pet look: looker graded the wrong figure (an NPC); the dark spiky mesh in the foreground is the pet, NEEDS_HUMAN | [#1674](https://github.com/SandboxServers/Cimmeria/issues/1674) |
| U2b | HP-P1, HP-P6 | PASS (HP-P1 half; relog half in HP-P6) | G (`.giveability 2826` x2) | 07:51:22Z INFO `GmGrantAbility: persisted` ability 2826, subject player 72; 07:51:24Z console.feedback ".giveability: entity 4 already knows 2826"; DB `abilities` = {2826}; client KNOWN "2826" after. Driver saw no Feedback line because the replies arrive on channel null / id 9 (briefs fixed) | |
| U3 | HP-P1 | PASS | N1 (`client_use_ability` cast); the move-cancel was a GM teleport, not walking | 07:51:39Z `summon_launched` warmup 6 s, `summon_fired`+`summoned`(ability 2826)+`summon_spawned` 07:51:45Z (6 s); second cast 07:51:53Z fired 07:51:59Z with `summon_replaced_pet` 100475 and a `despawned` for it, one pet; third cast 07:52:12Z then `summon_interrupted` reason `caster_moved` 07:52:17Z and no `summon_spawned` until the manual `.pet summon` 07:52:30Z (the interrupt is logged at the end of the warmup, not at the move). No WARN/ERROR | |
| U4 | HP-P2 | PARTIAL (follow held; leash NOT-RUN) | G (`.pet summon 350`); walking by `W` hold (N1) | 07:53:56Z pets.ai DEBUG `follow_armed` (pet 100479; again 07:54:54Z pet 100480); no `owner_missing`, no WARN/ERROR in pets.*; client-read pet 2.1 m after the walk. The `W` 8 s run moved 0 m (blocked at (252.61, -908.73)), so no `pets.ai teleported` row. The 07:54:42Z `.gotolocation` made `pets.lifecycle owner_teleported` path `console_travel` (`teleported_with_owner`): a GM jump moves the pet with you and never uses the leash. Rerun: `briefs/HP-P2-u4.md` | |
| U5 | HP-P3 | NOT-RUN in effect (PARTIAL: no Rat attacked the owner while the pet was Defensive; setup collision, not a product bug) | G (`.pet summon`, `.pet stance 1`) | 07:57:26Z `pets.command` `gm_stance_set` defensive (pet 100508); 07:57:28Z `npc_ai.aggro` acquired by Rat 100509 with target Hubdeploy (player 73, 5.0 m), not Hubpets; Hubdeploy shot dead 07:58:14Z (`player.death` killer AMBRat, Pistol Shot, at 252, -923: the same spot as Hubpets); Rat leashed home 07:58:14-16Z. No `pets.ai engaged`, no damage to Hubpets before 07:58:22Z; the driver set stance Passive at 07:58:22Z. Looker `P3-fight`: no creature or pet identified, no target frame, no fight effect | |
| U6 | HP-P2 | PARTIAL (console route PASS; pet-bar clause BLOCKED by #1674) | N3 (`changePetStance`) for the buttons (NATIVE_SHORTFALL at best); G for `.pet stance` | 07:55:06Z and 07:55:18Z wire `petChangeStance` (CM 90) and DEBUG `stance_set` `requested` 1, `source` stance_id, defensive to defensive (no change); the driver's `changePetStance(Unit.Pet1, 3)` produced no inbound CM 90 and no `rejected` row (nothing reached the server). Console: 07:55:19Z `stance_changed` defensive to passive, 07:55:23Z passive to aggressive, 07:55:26Z aggressive to defensive, 07:55:37Z defensive to aggressive; each followed by `gm_inspected`. Looker: no pet bar or highlight visible in any of the three shots (UNSURE x3); the chat in shot A shows the stance as defensive | [#1674](https://github.com/SandboxServers/Cimmeria/issues/1674) |
| U7 | HP-P3 | BLOCKED (selection failed; the player was dead then respawned 39 m away) | none reached the server | No `pets.command` invoked or rejected row at all. Hubpets died 07:58:46Z (Rats 100509 and 100510, Pistol Shot 65 per hit, `player.death`), `pets.lifecycle despawned` reason `owner_dead` path `owner_death`, respawn 07:59:17Z via respawner 130 at (251, 8, -962). Rat 100510 stayed at server (257, 7, -923): 39.46 m from the respawn point, which is exactly the driver's `distance_m`. No position desync. Looker `P3-select-failed`: no Rat, no pet, no target frame, player upright. Rerun: `briefs/HP-P3-r2.md` | |
| U8 | HP-P3 | BLOCKED (no pet: it despawned on the owner death 07:58:46Z) | | 08:01:05Z `pets.command` `gm_refused` reason `no_pet` (".pet stance: you have no pet out"); no `pets.credit`, no Rat death or `target_killed` for a Rat in the window; the Rats were gone from the client list only because they leashed home (07:58:46-48Z) and Hubpets respawned 39 m away. DB: Hubpets level 1 exp 0, so exp 0 is expected (no kill happened). KillCount NOT-RUN (D6) | |
| U9 | HP-P6 | Pending | | | |
| U10 | HP-P6 | Pending | | | |
| U12 | HP-P2 | PASS | G (`.pet info`) | 07:55:41Z DEBUG `gm_inspected` verb info, and four `console.feedback` rows: `.pet info: pet 100480, template 350, owner Hubpets (4), summoned by ability 2826`; `stance aggressive (allowed: passive, defensive, aggressive), AI state Follow`; `abilities [221, 1156], toggled off []`; `distance to owner 2.0 u, last teleport never`. The driver missed the abilities line; the server sent it | |
| U14 | HP-P4 | Pending | | | |
| U15 | HP-P4 | Recorded only (K12) | | | |
| U16 | HP-P4 | Pending | | | |
| U17 | HP-P5 | Pending | | | |
| U18 | HP-P5 | Pending | | | |
| U19 | HP-P5 | Pending | | | |
| U20 | HP-P5 | Pending | | | |
| DP-U1 | HP-D1 | PARTIAL (visual) | N1 (`client_use_ability` key "1" plus a real world click; `place:true` = N3 setup) | 07:53:06Z `deployables.lifecycle` `deploy_launched` (DEBUG) ability 1012, template 400; 07:53:08Z `spawned` (INFO) at (256.20, 6.99, -923.01), 4.2 m from the caster, radius 10, pulses_total 30, entity 100478; 2 s warmup exact; DB abilities of player 73 = [1012]; no `deploy_refused`, no deployables WARN/ERROR; client-read: driver GC.3 listed "Deployable: Microwave Emitter" 100478 at 4.2 m. Looker `D1-warmup`: cast bar UNSURE (shot taken at GC.3, 6 s after the spawn, so it cannot show the warmup) | |
| DP-U2 | HP-D1 | NOT-RUN (no valid shot; retake in HP-D2 8.2) | N1 | The three look shots show the p2 lane's "Summoned Straegis Fighter" pet (a blue crystalline creature) standing at the same spot, not the emitter: both lanes work in the same world 1300 space at the same plaza. Looker answers on `D1-object-front` are about the pet and are void. NEEDS_HUMAN stays open | |
| DP-U3 | HP-D2 | Pending | | | |
| DP-U4 | HP-D1 | PASS | N1 (as DP-U1) | 07:53:38Z INFO `despawned` reason `expired`, `lifetime_secs` 30.0005, `pulses` 30, entity 100478 (spawned 07:53:08Z); 30 DEBUG `deployables.pulse` rows 07:53:09Z-07:53:38Z. Client-read: driver found no match at ~07:53:5x (first absent read ~20 s after the despawn, the +20 s listed read was GC.3 at 07:53:14). Looker `D1-after-30s`: UNSURE (a pet hides the avatar, no device visible), consistent with gone | |
| DP-U5 | HP-D2 | Pending | | | |
| DP-U6 | HP-D3 | Pending | | | |
| DP-U8 | HP-D2 | Pending | | | |

Findings and doc mismatches found by the verifiers go in two more tables here (`P` product findings with issue numbers, `D` doc mismatches with the fix), as in the [round 1 report](../haiku-lab-uat/README.md#product-findings). After the pass, the unified-uat.md Mail, Pets and Deployables sections get the results in the usual way (the campaign's close-out, not per packet: status docs change once per campaign).

Verifier findings, HP-M3 (2026-10-13):

- Lab (not product): `client_chat_log` labels GM feedback (server channel id 9, `CHAN_FEEDBACK`, sky blue Info tab per chat-system.md) as `None`, so `channel:"Feedback"` name filters can match nothing. Briefs HP-C1 and HP-C2 now filter by id `"9"`. Still to fix by the coordinator once not running: HP-M4 (READ macro and steps 1.4, 2.4, 2.5, 3a.5, 3a.7) and the other briefs that use `"channel":"Feedback"` (HP-M2, M3, P1-P6, D1-D3). The unified-uat expectation (sky blue, Info tab) is unchanged; the colour and tab are read from the line, not the name.
- Lab gap: `client_window_read` has no `prompt` kind: [#1673](https://github.com/SandboxServers/Cimmeria/issues/1673).
- Doc nit: Mail 6 in unified-uat expects `.mailbox` to show an escrow count and next expiry; the reply read "N open of 100, 0 archived, K from the system, Q quarantined" with neither.

Verifier findings, HP-M4 (2026-10-13):

- Mail 1, bad name through the compose window: PASS (N1 Send click; fields filled by Lua = setup). 07:52:38Z DEBUG `mail.recipient_failed` reason `unknown_recipient`, WARN `mail.send_refused` reason `no_deliverable_recipients` result `MAILRESULT_NoRecipients`; no HPUM4r1 mail in the DB. The compose window stayed open: stock behaviour (unified-uat.md Known client cosmetics, K14: Send stays grey after a refused send until New or Reply). Looker: compose window open with To/Subject/body filled; Send seen only as a green check icon (UNSURE).
- Mail 2, the real send through the compose window: PASS (N1 Send click). 07:52:57Z INFO `mail.sent` delivered 1 failed 0 mail 10; `mail.cash_debited` postage 25, cash 100, naquadah 900 to 775 (125 total); DB mail 10 cash 100 character 71 (self-send worked, no refusal); `mail.notified` delivery `sent`.
- P8 (product, filed): [#1675](https://github.com/SandboxServers/Cimmeria/issues/1675) `/gmgiveitem` puts a mission-only (`{2}`) item in the Main bag; round 1's P5, confirmed on a second item (cause: `give.rs` requests INV_MAIN and `grant_container({2}, 1)` keeps it). No earlier issue existed for P5.
- Not a defect: the refusal text "That item cannot be carried in a backpack or crafting bag" is correct for the item type (the recipient's take could never place it) and follows from P8; fixing P8 removes the case. The wording is a little confusing for an item already in the backpack but names the real reason.
- Lab (not product): `GateMailCreate_ItemDragContainer:isVisible()` read false after `mailSetItemAttachment`; the widget is shown by an update event (`GateMailMod.onUpdateNewMessageItemAttachment`), so the read was early or the event did not fire for the Lua call. The native attachment state held the item at Send time (server saw item_id 10308), so the refusal came from the server, not the client. Read the widget after a short wait next time.

Verifier findings, HP-P2 (2026-10-13):

- U6 doc/brief mismatch (D): the brief expects bar button 1 = Passive, 2 = Defensive, 3 = Aggressive. The server resolves a requested value as an `EPetStance` id first, then as a 1-based slot (`crates/cell-pets/src/cell/cell_methods/player/pet/stance.rs`, A-07/N6 in `docs/analysis/pets/worknotes/pt-04.md`), so with the full list [passive, defensive, aggressive] bar buttons 1, 2, 3 give Defensive, Aggressive, Aggressive (Passive is unreachable from the small bar). This is a documented known limit, not new; seen on the wire as `requested` 1 -> `source` stance_id -> defensive. The real bar path is still unproven because the bar never shows (#1674). Not filed separately.
- Lab (not product): `changePetStance(Unit.Pet1, 3)` sent no CM 90 at all (no inbound `petChangeStance`, no `rejected` row), so slot 3 -> Aggressive was not exercised on the wire. Cause unknown (client native may drop a value above 2); a follow-up should try it with the real bar once #1674 is fixed.
- U4 leash: a GM teleport cannot exercise it (`on_owner_teleported` places the pet at once, `owner_follow.rs` leash is for a walking owner). A 50 m run along x 252 from z -935 to -885 is checked on the navmesh (one component, floor 7.0 to 7.4); whether the pet falls 40 m behind is unknown. See `briefs/HP-P2-u4.md` verifier notes.
- No WARN/ERROR in pets.* in the window; one movement.speed_warning at 07:54:03Z (warn-only, expected for a run).

Verifier findings, HP-D1 (2026-10-13):

- Ground cast recipe (proven): `client_use_ability {ability_id:1012, place:true, observe_ms:1500}` (key "1", verdict `nothing_observed` is expected), then `client_world_click {point:{x:256,y:7,z:-923}, button:"left", expect:"nothing", settle_ms:3500, rotate_camera:true}`. The server logged `deploy_launched` 07:53:06Z, `spawned` 07:53:08Z at the clicked point, `despawned expired` 07:53:38Z. The click also selected entity 100477 (`target_after`): that is Hubpets' summoned Straegis Fighter (pets lane, `.pet summon 350` at 07:52:30Z), standing on the same plaza. It did not block the placement (the spawn point is the click point). HP-D2 and HP-D3 now use the recipe.
- Design flaw (fixed in the schedule, not the product): the p2 pets lane and the p3 deployables lane share world 1300, space 65552 and the plaza centre. Hubdeploy sees Hubpets' pet (AoI enter at 07:52:30Z), it blocks the camera, and in HP-D2 it would fight the Rats. HP-D2 and HP-D3 run only while the pets lane is idle, and HP-D2 S2 checks for "Summoned" within 40 m.
- Lab, not product: `client_camera` took `pitch_deg`/`yaw_offset_deg` as f64 (`CameraRequest`, `crates/lab/src/supervisor/world/camera.rs`); the refusal `invalid type: string "-15"` means the driver sent quoted strings. Working call: `{"lease_id":L,"face_point":{"x":256.2,"y":7.6,"z":-923.0},"pitch_deg":-15,"zoom_to":250}` with JSON numbers. `face_entity_id` on the deployable fails with "entity 100478 has no pose": `snapshot` leaves `pose` empty when the entity has no actor (actor 0) or the pose read fails, while `find` still lists it as rendered. Whether the emitter has a scene actor is unproven (no valid shot yet), so no issue yet; HP-D2 8.2 takes the shot, and a missing object in it with a "rendered" entity-table row is the evidence to file with. D2 and D3 face a point, never the entity.
- Lab: `.gotolocation` to world 1300 breaks `client_wait_for` (`poll_until` returns `Err` on the first failed poll while the main thread loads the world): [#1677](https://github.com/SandboxServers/Cimmeria/issues/1677). D2 and D3 use fixed `client_batch` waits (2 x 20 s) instead.
- Briefs: server replies arrive on channel `None` (id 9); D2 and D3 READs now use `contains` and no channel. HP-D1's own brief still has the old READ, PLAZA and ground-cast text: use the D2 text if D1 is re-run.
- Unrelated WARN: 07:52:33Z `request_entity_update` `not_in_witness_aoi` cause `entity_gone` for entity 100476 (witness Hubdeploy); before the cast, same family as the closed #849. Not a deployables row.

- Finding (HP-D2, p3 recovery, 2026-10-13): lab_ensure_in_world failed twice with 'player's name did not read for 15 s' on a healthy client in world 1300 (unitName(Unit.Player) empty; Hubdeploy had been killed by a Rat at 07:58Z). lab_logout then hit a bridge dispatch timeout and the client crashed; relogin failed typing into a covered window. lab_client_restart then lab_ensure_in_world recovered it, and .gotolocation put Hubdeploy back on the plaza. No restart happened at 08:02Z (launch was about 07:51Z). Filed as #1680.
