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
| Chat 9, "paste ten lines fast: one 'too quickly' line" | GM accounts skip the flood limit (`docs/gameplay/chat-system.md`, "Access level GameMaster (2) and above skip the bucket"), and the lab accounts `lab`, `lab2`, `lab3` are access level 2. HP-C1 pastes ten lines anyway and records that none is refused | a non-GM account on the colo (decision D4) |
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

1. **Kill the screensaver.** `Get-Process -Name Ribbons -ErrorAction SilentlyContinue | Stop-Process` (the lab's D3D preflight refuses to start a client while a screensaver runs; round 1 lost a start to `Ribbons.scr`). Check `Get-Process | Where-Object { $_.Path -like '*.scr' }` is empty.
2. **The build lane must be idle.** `python tools/build-lane/lane_stats.py --recent 3` shows nothing running, and `Get-ChildItem $env:LOCALAPPDATA\cimmeria-build\lane` has no `slot.*` folder. Round 1's three failed starts happened under other sessions' cargo jobs. Do not start a build while the lab runs.
3. **The lab is installed and hosts three instances.** `%LOCALAPPDATA%\cimmeria-lab\labd.env` has `CIMMERIA_LAB_INSTANCES=default,p2,p3`; `lab instances status` shows `lab`, `lab2` and `lab3` account files and no running client; the daemon runs the current build (`lab restart` if you changed anything). Reconnect the MCP server with `/mcp` in the coordinator session after a restart, and start the driver agents after that, so their tool lists are current (LH-17 added `client_item_action`, `client_lua_eval`, `client_type_text`, `client_die_and_respawn` to `lab-driver`; a session that predates it still has the old agent).
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
| D2 | Add `client_use_ability` (and `client_hotbar`) to the `lab-driver` tool list so casts and pet orders run at N1 | **Owner decision** | Today the driver casts with stock Lua (`useAbility`, `usePetAbility`: N3), so every cast row grades at best `NATIVE_SHORTFALL`. The tool costs about 1k tokens of schema per turn. Without it the pets and deployables rows can still prove the server behaviour. Recommended: yes, then re-run the NATIVE_SHORTFALL rows |
| D3 | Tier policy: a GM or slash command submitted by `ChatMod.onTextAccepted` (CHATX) counts as typed (G) when the command itself is the step; the cast, stance click, take, pay and return rows need N1 for `PASS` | Adopted | Matches round 1 ("typed") and the vocabulary in automated-uat.md; the real keys are used for `/say hi` (9a) where typing is the point |
| D4 | A non-GM account on the colo, to test the chat flood limit ("too quickly") and the non-GM refusal of GM commands | **Owner decision** | The lab accounts are GM and exempt. Needed for Chat 9's last clause |
| D5 | HP-C2 runs alone, with no other lane driving, and only when `server_sessions` shows no real tester online | Adopted (a scheduling rule) | `.announce` and `/gmsendgmshout` open a modal prompt on every online client |
| D6 | The KillCount half of Pets U8 needs a seeded mission with a kill counter that a Soldier can hold; none is wired for this pass | **Owner decision** (or skip) | Without it U8 stays PARTIAL |
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
| HP-M1 | default | Mail 1, 2, 3 | 23 | none | Ready |
| HP-M2 | default | Mail 4 | 24 | HP-M1 (the character) | Ready |
| HP-M3 | default | Mail 5, 6 | 21 | HP-M1 | Ready |
| HP-M4 | default | Mail 1 (window), 2 (window), 3a | 25 | HP-M1 | Ready |
| HP-C1 | default | Chat 9, 9a, 7, flood probe | 19 | HP-M1 | Ready |
| HP-C2 | default | Chat 9c, 10 | 23 | HP-M1; a quiet colo (D5) | Ready |
| HP-P1 | p2 | U1, U2b, U3 | 22 | none | Ready |
| HP-P2 | p2 | U4, U6, U12 | 22 | HP-P1 | Ready |
| HP-P3 | p2 | U5, U7, U8 | 24 | HP-P1 | Ready |
| HP-P4 | p2 | U14, U15, U16 | 23 | HP-P1 | Ready |
| HP-P5 | p2 | U17, U18, U19, U20 | 20 | HP-P1 | Ready |
| HP-P6 | p2 | U9, U10, U2b relog | 18 | HP-P1; run last on p2 | Ready |
| HP-D1 | p3 | DP-U1, U2, U4 | 22 | none; may end BLOCKED | Ready |
| HP-D2 | p3 | DP-U3, U5, U8 | 24 | HP-D1 proves the ground cast | Ready after HP-D1 |
| HP-D3 | p3 | DP-U6 | 24 | HP-D1 | Ready after HP-D1 |

## Results

Fill from the verifier reports. Result vocabulary: PASS, NATIVE_SHORTFALL, PARTIAL, FAIL, BLOCKED, NOT-RUN, NEEDS_HUMAN ([automated-uat.md](../../guides/automated-uat.md#results)). Tier: the lowest native level the step used (N1 real input, N2 slash command standing in, N3 stock Lua, G GM command). `Issue` links the GitHub issue a FAIL produced.

| Row | Packet | Result | Tier | Evidence (time, scope, event) | Issue |
|---|---|---|---|---|---|
| Mail 1 | HP-M1, HP-M4 | Pending | | | |
| Mail 2 | HP-M1, HP-M4 | Pending | | | |
| Mail 3 | HP-M1 | Pending | | | |
| Mail 3a | HP-M4 | Pending | | | |
| Mail 4 | HP-M2 | Pending | | | |
| Mail 5 | HP-M3 | Pending | | | |
| Mail 6 | HP-M3 | Pending | | | |
| Chat 7 (solo) | HP-C1 | Pending | | | |
| Chat 9 | HP-C1 | Pending | | | |
| Chat 9a | HP-C1 | Pending | | | |
| Chat 9c | HP-C2 | Pending | | | |
| Chat 10 | HP-C2 | Pending | | | |
| U1 | HP-P1 | Pending | | | |
| U2b | HP-P1, HP-P6 | Pending | | | |
| U3 | HP-P1 | Pending | | | |
| U4 | HP-P2 | Pending | | | |
| U5 | HP-P3 | Pending | | | |
| U6 | HP-P2 | Pending | | | |
| U7 | HP-P3 | Pending | | | |
| U8 | HP-P3 | Pending | | | |
| U9 | HP-P6 | Pending | | | |
| U10 | HP-P6 | Pending | | | |
| U12 | HP-P2 | Pending | | | |
| U14 | HP-P4 | Pending | | | |
| U15 | HP-P4 | Recorded only (K12) | | | |
| U16 | HP-P4 | Pending | | | |
| U17 | HP-P5 | Pending | | | |
| U18 | HP-P5 | Pending | | | |
| U19 | HP-P5 | Pending | | | |
| U20 | HP-P5 | Pending | | | |
| DP-U1 | HP-D1 | Pending | | | |
| DP-U2 | HP-D1 | Pending | | | |
| DP-U3 | HP-D2 | Pending | | | |
| DP-U4 | HP-D1 | Pending | | | |
| DP-U5 | HP-D2 | Pending | | | |
| DP-U6 | HP-D3 | Pending | | | |
| DP-U8 | HP-D2 | Pending | | | |

Findings and doc mismatches found by the verifiers go in two more tables here (`P` product findings with issue numbers, `D` doc mismatches with the fix), as in the [round 1 report](../haiku-lab-uat/README.md#product-findings). After the pass, the unified-uat.md Mail, Pets and Deployables sections get the results in the usual way (the campaign's close-out, not per packet: status docs change once per campaign).
