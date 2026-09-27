---
title: Organizations UAT — Squads, Teams and Commands
type: how-to
audience: the owner and playtesters running the organizations acceptance test on the colo, as GM
last_updated: 2026-09-27
companion_docs:
  - ../gameplay/organization-system.md
  - ../gameplay/group-system.md
  - ../commands.md
  - ../architecture/observability.md
  - ../content/debug-hub.md
  - ../analysis/organizations/work-packets.md
---

# Organizations UAT — Squads, Teams and Commands

This guide runs the owner's acceptance test for the organizations campaign
(ledger step ORG-UAT in
[work-packets.md](../analysis/organizations/work-packets.md#org-uat-owner-two-client-uat-colo)).
It covers squads (ORG-03, ORG-04), founding a Team or Command at a registrar
(ORG-05), login restore and presence (ORG-06), invite, kick and rank change
(ORG-07), the MOTD, notes and rank editor (ORG-08), Team, Command and officer
chat (ORG-09), and the GM suite (ORG-10).

Each step gives the two-client script, a one-client fallback that uses GM
commands in place of the second player, what to watch for on screen, the log
rows the step writes, and the SigNoz query that shows them. If a step fails,
the query tells you which path ran and why it refused, without a repro.

Vault and treasury tests belong to the Bank campaign and are not here.

## Before you start

You need:

- Two accounts, **A** and **B**, each with a character. A is a GameMaster
  (access level 2 or more). B can be an ordinary player; the fallback steps
  run everything from A.
- For the one-client fallback, a third character that is logged in somewhere
  (a second client window, or a character you park online), called **S**
  below. `.org_join` and `.squad_join` need the other character online.
- Both characters in the stasis room of `Castle_CellBlock` (world 12), where
  new characters wake up. The Team and Command registrars stand in the debug
  hub there; see [debug-hub.md](../content/debug-hub.md).
- SigNoz open on the Logs explorer, time range covering the session.

Write down each character's player id before you start. `.org_info <name>`
prints it (`character <id>`), and every query below filters on it.

At any oddity, type `.bug <what you saw>`. It writes a `playtest.bookmark`
row with your position, your target and every entity nearby, so the moment
can be found again:

```text
service.name = 'cimmeria-server' AND scope_name = 'playtest.bookmark'
```

## How to read a step back from SigNoz

Every query in this guide starts with the same two filters. Type them once
and add the step's own terms:

```text
service.name = 'cimmeria-server' AND scope_name IN ('squad', 'org')
```

Squads log on `squad`; Teams, Commands and every GM organization command log
on `org`. Every player action ends in exactly one INFO row whose `event` is
the action (`squad.invite`, `org.kick`, ...), with `outcome` (`ok` or
`rejected`) and, on a refusal, a `reason`. The row carries the actor's
`account_id` and `player_id`, and the second player's `target_account_id` and
`target_player_id` when there is one. State changes are DEBUG rows named for
the change (`member_joined`, `rank_changed`, `permissions_changed`, ...) with
the values before and after. Text is logged as its length (`text_units`,
`from_units`, `to_units`), never the text itself.

Three queries answer most questions:

| To see | Add to the filter |
|---|---|
| Everything one player did | `AND player_id = <id>`, sorted by time |
| Why something was refused | `AND outcome = 'rejected'`, grouped by `event`, `reason` |
| Everything a GM did | `AND event = 'org.gm_action'` |

DEBUG rows only reach SigNoz when the server's `OTEL_FILTER` keeps DEBUG for
`org` and `squad`; if a transition row below is missing but the INFO row is
there, check the filter before suspecting the code.

Event names for ORG-08 (texts and the rank editor) and ORG-09 (Team, Command
and officer chat) come from their packet text, written while those packets
were in flight. If a query returns nothing, check the `org` / `squad` row of
the [observability.md](../architecture/observability.md) catalog for the
names as merged.

## Step 1 — Squad invite

**Two clients.** A types `/squadinvite B`. B sees the invite and accepts.

**One client.** A types `.squad_join S`, which puts A into S's squad (or
founds one S leads) with no handshake. `.squad_info` lists it.

**Watch for.** Both squad frames show the other member. B's accept closes
the prompt. A refused invite (B already in a squad, A sent too many) gives A
a line on the feedback channel, never silence.

**Log rows.**

| Row | Level | What it tells you |
|---|---|---|
| `squad.invite` | INFO | A's invite: `outcome`, `request_id`, B as target |
| `invite_created` | DEBUG | the pending invite and its `request_id` |
| `squad.invite_response` | INFO | B's answer; the target is A |
| `invite_consumed`, `squad_created`, `member_joined` | DEBUG | the squad forming |
| `org.gm_action` with `action = gm_squad_join` | INFO | the fallback's join |

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name IN ('squad', 'org')
  AND event IN ('squad.invite', 'squad.invite_response', 'invite_created', 'invite_consumed', 'squad_created', 'member_joined')
  AND (player_id = <A> OR player_id = <B>)
```

A refusal: `AND event = 'squad.invite' AND outcome = 'rejected'`, then read
`reason` (`already_in_squad`, `squad_full`, `ignored`, `rate_limited`,
`invite_limit`, `target_in_transition`, ...).

## Step 2 — Squad chat and loot mode

**Two clients.** Both type on the squad channel. B, who is not the leader,
changes the loot mode from the squad menu. Then A changes it.

**One client.** A types on the squad channel with S in the squad; S's
client, if you have it open, shows the line. A as leader changes the loot
mode. For the non-leader refusal, use `.squad_join` so that S leads and A is
a member, then change the loot mode as A.

**Watch for.**

- **Each squad line shows once in each chat window, not twice.** The server
  sends the line to every member, the speaker included (as the 2009 server
  did), exactly once each. If the speaker sees their own line twice, the
  client also echoes it locally; if a listener sees it twice, the server sent
  it twice. Report either with `.bug`: the server count below tells you
  which side it is.
- B's loot-mode change snaps the menu back and B gets a line saying only the
  leader can change it. A's change reaches both frames.

**Log rows.** `squad.chat` (INFO, `recipients` = members reached, the
speaker's own copy not counted, `text_units`); `squad.loot_mode` (INFO,
`rejected` with `reason = not_leader` for B, `ok` for A); `loot_mode_changed`
(DEBUG, `from`, `to`).

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'squad'
  AND event IN ('squad.chat', 'squad.loot_mode', 'loot_mode_changed')
  AND player_id IN (<A>, <B>)
```

With two members, `recipients = 1` on every `squad.chat` row (the speaker's
own copy is sent but not counted). One row per line typed, and a doubled line
on the speaker's screen, is a client-side echo; two rows for one line typed is
a server bug.

## Step 3 — Squad across worlds

**Two clients.** B gates to another world. A and B both chat on the squad
channel while B is away and after B arrives.

**One client.** A gates away (for example `.gotolocation` to another world)
and chats back to S.

**Watch for.** The squad survives the trip: B's squad frame still shows A
after arrival. Squad chat reaches B once B has arrived. A line sent while B
is mid-transit may miss B; that is a known carried gap, so note it with
`.bug` rather than failing the step.

**Log rows.** `squad.world_entry_replay` (DEBUG) when B's squad state is
replayed on arrival; `squad.chat` rows with `recipients`; on a line that
missed B, `recipients` is one lower than the squad size minus one.

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'squad'
  AND (event = 'squad.world_entry_replay' OR event = 'squad.chat')
  AND squad_id = <squad id from step 1>
```

## Step 4 — Squad leave

**Two clients.** A leaves the squad. B becomes leader. B leaves, and the
squad is gone.

**One client.** With A and S in a squad, A leaves; S becomes leader. Then
`.squad_info S` shows S's squad, or that S has none after S leaves.

**Watch for.** B's frame shows B as leader after A leaves. After B leaves,
neither frame shows a squad.

**Log rows.** `squad.leave` (INFO) for each leave; `member_left` (DEBUG,
`reason = requested`); `leader_changed` (DEBUG, `from_player_id`,
`to_player_id`); `disbanded` (DEBUG) when the last member goes.

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'squad'
  AND event IN ('squad.leave', 'member_left', 'leader_changed', 'disbanded')
  AND squad_id = <squad id>
```

## Step 5 — Create a Command at the registrar

**Two clients.** A walks to the **Command** registrar in the stasis-room
debug hub (template 331, spawn 431; both registrars are named "Organization
Registrar", the Command one wears armour) and right-clicks it. A names the
Command in the dialog and sees the Command window with A as Leader. A second
try with the same name, on B or on another character, is refused visibly.

**One client.** `.org_create command <name>` founds a Command you lead with
no registrar. Use it only if the registrar itself fails, and file the
registrar failure with `.bug`.

**Watch for.** This step checks client behaviour nobody has seen yet:

- Whether right-clicking the registrar opens the naming window at all. If
  nothing happens, look for an `org.registrar_open` row: none means the
  client never sent the interact; a `rejected` row says why the server
  refused (`not_eligible` if A is already in a Command, `too_far`).
- What the naming window does on a refused name (`CreateCommandWin` with
  result 0): stay open for another try, or close. Note which. The refusal
  line on the feedback channel arrives either way.
- After success, the Command window opens with A as Leader, the name, and
  a one-member roster with A online.

**Log rows.**

| Row | Level | What it tells you |
|---|---|---|
| `org.registrar_open` | INFO | the click: `ok`, or `not_eligible` / `too_far` (`distance`) / `rate_limited` |
| `pending_creation_created`, `pending_creation_consumed`, `pending_creation_attempt_charged`, `pending_creation_expired` | DEBUG | the offer's life: five minutes, three names |
| `org.create` | INFO | the name: `ok` with the new `org_id` and `name_units`, or `name_taken`, `text_invalid`, `already_in_org_type`, `no_pending_creation`, ... |
| `org.state_push` | INFO | the founder's Command window: `source = push`, `new_member = true` |
| `org.gm_action` with `action = gm_org_create` | INFO | the fallback |

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event IN ('org.registrar_open', 'org.create', 'pending_creation_created', 'pending_creation_consumed', 'pending_creation_attempt_charged', 'pending_creation_expired', 'org.state_push')
  AND player_id = <A>
```

The duplicate: `AND event = 'org.create' AND reason = 'name_taken'`. A
duplicate name costs nothing.

## Step 6 — Invite into the Command

**Two clients.** A invites B from the Command window. B accepts. Both
rosters show both members.

**One client.** `.org_join <orgId> S` adds S at the entry rank (Command
Initiate, rank 1). `.org_list` shows the member count go to 2.

**Watch for.** B gets an invite prompt naming the Command. After the
accept, B's Command window opens with the full state, and A's roster gains
B, marked online. Whether the client's invite menu sends `organizationInvite`
with the org id or `organizationInviteByType` with type 2 is not confirmed;
both work, and the `org.invite` row does not say which.

**Log rows.** `org.invite` (INFO, `request_id`, B as target, `actor_rank`);
`invite_created` (DEBUG); `org.invite_response` (INFO, `after = joined` or
`declined`); `invite_consumed`, `member_joined` (DEBUG, `via =
invite_response`, `rank`); `org.state_push` for B. The fallback writes
`org.gm_join` and its `org.gm_action` twin (`action = gm_org_join`).

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event IN ('org.invite', 'invite_created', 'org.invite_response', 'invite_consumed', 'member_joined')
  AND org_id = <orgId>
```

To follow one invite end to end: `AND request_id = <request_id>`.

## Step 7 — Rank

**Two clients.** A promotes B to Officer (rank 6). B tries to promote A and
is refused. B, as Officer, invites and then kicks a third character (S, or a
character A added with `.org_join`).

**One client.** `.org_rank S 6 <orgId>` makes S an Officer. For the refusal,
check with `.org_info S` that S's rank lacks `Promote` (bit `0x4`). The
invite and kick by an Officer need a second client; the GM equivalent is
`.org_join` and nothing kicks for you, so mark that part untested.

**Watch for.** Both rosters show B's new rank. B's promote of A is refused
with a line on the feedback channel. The third character's invite and kick
work, and the kicked character's Command window closes.

**Log rows.** `org.rank_change` (INFO, `actor_rank`, `target_rank`,
`to_rank`; a refusal's `reason` is one of `missing_permission`,
`rank_too_low`, `leader_not_assignable`, `rank_not_in_type`, `self_target`);
`rank_changed` (DEBUG, `from_rank`, `to_rank`); `org.invite` and `org.kick`
(INFO) for B's actions; `member_left` (DEBUG, `reason = kicked`). The kick
also sends the cell `OrgMembershipEnded` (the Bank's vault hook; DEBUG
`org.membership_ended` on the cell).

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event IN ('org.rank_change', 'rank_changed', 'org.invite', 'org.kick', 'member_left')
  AND org_id = <orgId>
```

## Step 8 — Texts and the rank editor

This step exercises ORG-08. Its event names come from the packet.

**Two clients.** A sets the MOTD, A's own roster note, an officer note on B,
and renames a rank. B sees each change. Then take `OfficerNotes` (bit `0x40`)
away from B's rank in the rank editor: B's view hides officer notes until B
holds the bit again.

**One client.** Set the texts as A with S in the Command. Toggle S's rank's
officer-note bit with the GM command instead of the editor:

1. `.org_info S` prints S's rank and mask, for example `rank 6, permissions
   0x0150572`.
2. `.org_set_perms <orgId> 6 0x0150532` clears `0x40`. The reply gives the
   old and new mask, and names any bits it ignored because the Command
   editor does not show them.
3. `.org_info S` again to confirm.

`.org_set_perms` refuses the Leader rank (`leader_row_pinned`), a rank the
type does not use, and an edit that changes nothing.

**Watch for.** Each text appears for B without a relog. Officer notes
disappear from B's roster when the bit goes. The rank editor's change reaches
every online member's window.

**Log rows.** `org.set_text` (INFO, `field` = `motd`, `note` or
`officer_note`, `from_units`, `to_units`, `target_player_id` for an officer
note); `org.set_rank_name`; `org.set_rank_permissions` (INFO, `rank`,
`from_mask`, `to_mask`, `wire_mask`); `text_changed` and
`permissions_changed` (DEBUG). The GM fallback writes `org.gm_action` with
`action = gm_org_set_perms` and DEBUG `permissions_changed` with `via = gm`
and `ignored_bits`.

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event IN ('org.set_text', 'org.set_rank_name', 'org.set_rank_permissions', 'text_changed', 'permissions_changed', 'org.gm_action')
  AND org_id = <orgId>
```

## Step 9 — Command and officer chat

This step exercises ORG-09. Its event names come from the packet.

**Two clients.** Both chat on the Command channel and on the officer
channel; each line reaches the other. Take `OfficerChat` (bit `0x100`) away
from B's rank; B's next officer line is refused with a line on the feedback
channel.

**One client.** Chat as A with S in the Command. Clear `0x100` on S's rank
with `.org_set_perms` as in step 8, then have S (second window) speak on the
officer channel, or read the refusal row if S cannot.

**Watch for.** Each line shows once. The officer refusal is visible to the
speaker. Nobody outside the Command sees the lines.

**Log rows.** `org.chat` (INFO, `channel` = 5 for Command, 6 for officer,
3 for Team; `recipients`; `text_units`; a refusal's `reason`, for example
`missing_permission` for officer chat without the bit, `rate_limited`,
`text_invalid`).

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event = 'org.chat' AND org_id = <orgId>
```

## Step 10 — Relog

**Two clients.** Both log out and back in. Then A logs out while B watches,
and logs back in.

**One client.** A logs out and back in; then `/ReloadOrganizations` (the
native GM console command, `gmReloadOrganizations`) re-sends the same state
without a relog.

**Watch for.**

- After login the Command window comes back with the name, MOTD, rank names
  and roster, and the members who are online show as online.
- B sees A go offline when A logs out (A's roster row stays, marked offline)
  and online again when A logs back in.
- `/ReloadOrganizations` gives the same window as a fresh login and a line
  saying how many organizations it re-sent. It tells nobody else anything.
  If the window was already right, nothing visible changes.

**Log rows.** `org.login_restore` (INFO, `org_count`, `pushed`) per login;
one `org.state_push` (INFO, `source = login_restore`, `roster_size`,
`online_members`) per organization; `member_online` and `member_offline`
(INFO, `recipients`, and for offline the `disconnect_reason`). The reload
writes `org.gm_action` with `action = gm_reload_organizations` and `count`,
and one `org.state_push` with `source = push` per organization.

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event IN ('org.login_restore', 'org.state_push', 'member_online', 'member_offline')
  AND player_id = <A>
```

The reload: `AND event = 'org.gm_action' AND action = 'gm_reload_organizations'`.

## Step 11 — Team

**Two clients.** Repeat steps 5 and 6 at the **Team** registrar (template
330, spawn 430, the one in the SGC uniform). A now holds a Team and a
Command at once. A second Team for A, or for B, is refused.

**One client.** `.org_create team <name>`, then `.org_join <teamId> S`.
`.org_info` shows A in both organizations.

**Watch for.** Both windows work side by side. A Team uses only ranks 2
(Member), 3 (Senior Member) and 8 (Leader), and its rank editor shows 12
permission bits where the Command's shows 14.

**Log rows and query.** As steps 5 and 6, with `org_type = 'team'`. The
one-per-type refusal is `reason = already_in_org_type` on `org.create`,
`org.invite` or `org.invite_response`, or `not_eligible` on
`org.registrar_open`.

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND org_type = 'team' AND player_id IN (<A>, <B>)
```

## Step 12 — Leave and disband

**Two clients.** A tries to leave the Command while B is still in it and is
refused. B leaves. Then A leaves, and the Command is gone: `.org_list` no
longer shows it.

**One client.** As above with S in B's place. `.org_disband <orgId>` is the
GM's way to remove an organization outright; it is refused while the vault
holds anything.

**Watch for.** A's refused leave gives a line explaining the leader cannot
leave while others remain. B's leave closes B's window and removes B from
A's roster. A's leave closes A's window.

**Log rows.** `org.leave` (INFO; `rejected` with `reason =
leader_cannot_leave` for A's first try; `ok` with `after = left`, or `after =
disbanded` for the last member); `member_left` (DEBUG, `reason =
requested`); `disbanded` (DEBUG, `reason = last_member_left`, or `gm` for
`.org_disband`).

**Query.**

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event IN ('org.leave', 'member_left', 'disbanded', 'org.disband')
  AND org_id = <orgId>
```

## The GM commands

Every GM organization command is refused to a player who is not a
GameMaster, twice: on the cell, which runs only a GM's `.` line, and on the
base, which re-reads the access level itself. Each one, refused or not,
writes exactly one `org.gm_action` row with `action`, `outcome`, the GM's ids
and the target's.

| Command | Use it for | `action` |
|---|---|---|
| `.squad_invite <name>` / `.squad_join <name>` / `.squad_info [name]` | build and inspect a squad alone | `gm_squad_invite` / `gm_squad_join` / `gm_squad_info` |
| `.org_create <team\|command> <name>` | found an organization without a registrar | `gm_org_create` |
| `.org_join <orgId> [player]` | add an online character at the entry rank | `gm_org_join` |
| `.org_rank <player> <rank> [orgId]` | set a member's rank (never Leader) | `gm_org_rank` |
| `.org_set_perms <orgId> <rank> <mask>` | set a rank's permissions, clamped to the editor's bits | `gm_org_set_perms` |
| `.org_info [player]` | a character's organizations, ranks and masks | `gm_org_info` |
| `.org_list` | every organization, its size and leader | `gm_org_list` |
| `.org_disband <orgId>` | remove an organization | `disband` |
| `/ReloadOrganizations` | re-send your own organization windows | `gm_reload_organizations` |

```text
service.name = 'cimmeria-server' AND scope_name = 'org'
  AND event = 'org.gm_action' AND player_id = <A>
```

Permission bits, for reading `.org_info` masks and writing
`.org_set_perms` ones: `Invite 0x2`, `Promote 0x4`, `Demote 0x8`,
`Eject 0x10`, `RosterNotes 0x20`, `OfficerNotes 0x40`, `RankNames 0x80`,
`OfficerChat 0x100`, `EmailLists 0x200`, `MOTD 0x400`, `DepositBank
0x10000`, `WithdrawBank 0x20000`, `DepositCash 0x40000`, `WithdrawCash
0x80000`, `ViewBankLogs 0x100000`, `AlterPerms 0x800000`. The full list is in
[organization-system.md](../gameplay/organization-system.md).

## When something is wrong

| Symptom | Query |
|---|---|
| A press did nothing | `AND player_id = <id>` around the time: a `rejected` row names the `reason`; no row at all means the client sent nothing, or the call did not decode (`severity_text = 'WARN' AND event LIKE '%malformed'`) |
| A member was not told | `AND event IN ('org.send_failed', 'org.broadcast_failed', 'squad.send_failed')` (`what`, `reason`) |
| A forwarded call went astray | `AND event IN ('org.forward', 'org.actor_mismatch')` (`route`, `method_index`) |
| A window is out of date | `/ReloadOrganizations`, then compare the `org.state_push` row's `roster_size` with `.org_info` |
