# Social Systems: Mail, Chat and Dueling

> Type: how-to. Audience: the Claude Code coordinator, packet workers and the owner.
> Updated: 2026-09-27. Companions: [audit](audit.md), [work packets](work-packets.md), [session resume](handoffs/session-resume.md), [organizations ledger](../organizations/README.md), [documentation index](../../readme.md).

## Purpose

This campaign finishes three player-to-player systems whose client UI ships complete and whose server side is a stub:

- **Mail (GateMail).** Reading, archiving and deleting work. Sending, attachments, taking, cash on delivery (COD) and return are stubs that tell the dispatcher "handled", so the client's Send button does nothing.
- **Chat.** Say, emote and yell work. A tell is never delivered, nothing enforces the Ignore list, nothing limits flooding, there is no GM broadcast, and the channel ids disagree with `enumerations.xml`.
- **Dueling, 1v1.** Nothing exists on the server. The client ships the challenge prompt, the countdown and every wire method.

The 2009 Python server never implemented any of the write paths: the mutating mail methods are `print` stubs, the duel methods are `pass`, and `SGWDuelMarker.py` is an empty class. Only tells have a working legacy reference (`deprecated/python/base/Chat.py`). Every design choice below that the client does not fix is therefore project policy, and the docs must say so.

It supersedes the phase plans in issues [#72](https://github.com/SandboxServers/Cimmeria/issues/72) (mail) and [#569](https://github.com/SandboxServers/Cimmeria/issues/569) (duels), and folds in the security criteria from the closed audits [#466](https://github.com/SandboxServers/Cimmeria/issues/466) ([CAT-G](../../security-audit/2026-05-31-server-authority/findings/CAT-G-mail.md), all eight), #471 ([CAT-L](../../security-audit/2026-05-31-server-authority/findings/CAT-L-chat-contact.md) L-01, L-03, L-06 and the `chatIgnore` part of L-07) and [#472](https://github.com/SandboxServers/Cimmeria/issues/472) ([CAT-M](../../security-audit/2026-05-31-server-authority/findings/CAT-M-org-squad-duel.md) M-12 to M-15). Issue #70 is already closed as superseded by #569.

In scope:

- mail send (plain, cash, one item, COD), take cash, take item, pay COD, return, the new-mail notification and expiry;
- tells, Ignore enforcement, flood limiting, a channel allowlist, GM broadcast, basic GM moderation (mute), and feedback for every Communicator method the server does not implement;
- 1v1 duels: challenge, response, countdown, the PvP flag, the combat gate, and every end path.

Out of scope:

- **Squad duels** (`aSquadDuel = 1`). They are rejected with feedback.
- **User-created channels** (`chatJoin` of a new name, ids 12 and up) and channel operator commands (`chatOp`, `chatMute`, `chatKick`, `chatBan`, `chatPassword`). They get a "not supported" feedback line, nothing more.
- **Petitions** (`petition`, `announcePetition`): feedback only.
- **Mail to the vault, Team or Command aliases** (`MAIL_ToVault` to `MAIL_ToCommandRank7`). Vault aliases belong to the Bank campaign; organization aliases need the organizations campaign's membership API. Both are rejected with a result code until then (D-SS07).
- **Team, squad, command and officer chat.** The organizations campaign owns them (ORG-04, ORG-09).
- **Any client patch.** Every feature here uses wire methods the shipped client already handles.

## What was found

Against `main` @ `09a880ba`. The [audit](audit.md) has the file:line evidence for each row.

| Area | State on `main` | Packets |
|---|---|---|
| Mail read side | Headers, body, archive and delete work. The header query ignores `bArchive`, the read-time `UPDATE` is not scoped to the owner (CAT-G-07), and `ToText` carries the reader's name (CAT-G-08). | SS-M1 |
| Mail write side | CM 44, 47, 49, 50 and 51 are stubs. `sendMailResult` (79) is never built. `MailOp` has four variants. | SS-M1 to SS-M3 |
| Attachments | The schema has one `item_id` and no quantity. Every inventory query selects all of a character's rows, so an escrowed item cannot stay in `sgw_inventory`. `MessageAttachment` is defined in `alias.xml`, but the server always sends an empty array. | SS-M2 |
| New mail, expiry | `onNewMail` is a server-internal cell method, not a client method; nothing tells an online recipient. There is no expiry column or sweep. | SS-M4 |
| Tells | Forwarded to the cell like speech, which answers "Speaking on channel 9 is not supported yet!". The base has no name-to-session index. | SS-00, SS-C1 |
| Ignore | The contact list's Ignore list (flags 301) is stored and synced, but nothing reads it. `chatIgnore` (0xC5) is not dispatched. Unmerged PR #585 (pre-split) implemented it with AoI hiding. | SS-C1 |
| Flood and channel trust | No rate limiter for any player action; no length cap at the base; any channel byte reaches the cell. | SS-00, SS-C3 |
| GM broadcast | The client ships `/gmshout` and cell method 222 `sendGMShout`; nothing handles it. | SS-C2 |
| Channel ids | Server 7, tell 9 and splash 10 in code; 8, 10 and 11 in `enumerations.xml`. The organizations campaign owns the fix (D-ORG14). | SS-C4 (with ORG-09) |
| Duels | CM 102 and 103 are log-only stubs. Base method 0xD9 `sendDuelChallenge` lands in the unhandled catch-all. No duel state, no marker, no PvP flag after world entry. | SS-D1 to SS-D3 |
| Duel entity set (resolved by SS-E1) | AoI sends `onDuelEntitiesRemove` (152) for interactable NPCs. SS-E1 D-Q5 showed that 151, 152 and 153 only edit a client-side set that the interactability check never reads, so sending 151 at duel start and 153 at duel end is safe (D-SS25 superseded). The `aoi.rs` comment has 152's direction backwards. | SS-D2 (fix the comment) |
| Hostility gate | Four sites reject player-on-player damage (issue #569 lists three; AT-10 added the warmup site). | SS-D2 |
| In-game test surface | None for any of the three systems. | SS-U1 to SS-U3 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-SS01 | **APPROVED** (owner, 2026-09-27 kickoff) | Autonomous run: one worktree and one test database per worker, squash-merge each PR on green CI (rebase and re-test first when its CI predates `main`), and `/release` on the last merged PR. | Kickoff instructions. |
| D-SS02 | APPROVED (owner, 2026-09-27) | **Postage is enforced.** A send with cash or an item attached costs 25 naquadah on top of the attached cash; a text-only mail is free. The fee is a sink (destroyed), debited in the send transaction, and a sender who cannot cover cash plus postage gets `MAILRESULT_NotEnoughCash` (4). COD mail pays postage too, because it carries an item. | `GateMailMod.attachmentCost = 25` (`GateMail.lua:10`), and the cash spinner's maximum is `getCash() - attachmentCost` (`:479-494`), so the client already assumes the fee comes out of the sender's balance on top of the cash. Not enforcing it makes the client's cost box a lie. |
| D-SS03 | APPROVED (owner, 2026-09-27) | **The 100-message mailbox cap is enforced** on player sends: a recipient with 100 or more messages that are neither archived nor quarantined (`NOT archived AND NOT quarantined`) is listed in `FailedRecipients`. Server-generated mail (returns, COD payments, expiry returns) ignores the cap, so money and items are never lost to it. | The client warns at 90 and pins "100% Full" at 100 (`GateMail.lua:40-68`). Whether the 2009 server enforced it is unknowable; enforcing it matches what the UI promises, and exempting system mail keeps escrow safe. SS-E1 checks whether the client's count includes the archive. |
| D-SS04 | APPROVED (owner, 2026-09-27) | **Expiry.** Mail gets an `expires_at`. The TTL is the client's own constant if SS-E1 finds one (the Expires column is computed client-side), otherwise 30 days. Archived mail never expires (the client shows "Never"). On expiry, the sweep takes one of three terminal paths, each in one transaction with its escrow row. (1) **Not yet returned, still carrying an item, gift cash or unpaid COD:** returned to its sender once (the SS-M3 return path); an unpaid COD is cancelled, with the COD flag cleared **and the COD amount zeroed**, so the price never becomes gift cash. (2) **Nothing attached:** deleted. (3) **Already returned, still holding an item or gift cash:** **quarantined, never deleted**. The mail row and its `sgw_gate_mail_item` escrow row are kept, marked `quarantined`, excluded from the mailbox list and the D-SS03 cap, and logged at WARN, and a GM recovers it by id. No escrow row is ever orphaned. | Issue #72 asks for either a sweep or a non-expiring decision. The client displays a countdown, so the server must match it or the UI lies (`GateMail.lua` `ExpiresText`; there is no expiry field in `MessageHeader`, `alias.xml:89-101`). Auto-return of unpaid COD is the standard escape hatch and reuses the SS-M3 return path. |
| D-SS05 | PROPOSED | **Recipients.** A send with any attachment (cash, item or COD) must have exactly one recipient, else `MAILRESULT_AttachmentsAndMultipleRecipients` (3). A text-only mail may have up to **10** distinct recipients after de-duplication; more is rejected whole with `MAILRESULT_NoRecipients` (1) and feedback. Unknown names go in `FailedRecipients`, and the rest are delivered. | The result code (value 3, `enumerations.xml:1071`) confirms the one-recipient rule. The 10 cap is CAT-G-01's remediation; nothing in the client bounds the list. |
| D-SS06 | PROPOSED | **Atomic debit.** Every send is one base transaction: lock the sender's and **every recipient's** `sgw_player` rows with `SELECT … FOR UPDATE` in **ascending `player_id` order** (a fixed order, so two concurrent sends cannot deadlock), then count each recipient's mail that is neither archived nor quarantined, under that lock, for the D-SS03 cap, re-check and debit cash plus postage, move the item into escrow, insert every mail row, commit. Any failure rolls everything back and answers with a result code. Nothing is debited optimistically and refunded later. | CAT-G-01 and issue #72. A refund-on-failure design has a window in which a crash loses or duplicates value. Trade already works this way (`trade/execute/mod.rs:1-26`). |
| D-SS07 | PROPOSED | **Vault and organization aliases are deferred.** A send with any `MAIL_To*` bit set in `RecipientFlags` is rejected: vault bits with `FailedRecipientFlags` set to the vault bits and `MAILRESULT_NoRecipients`, organization bits the same way, plus a feedback line naming the reason. The Bank campaign (cimmeria-97) owns `MAILRESULT_VaultButNoItem`, `VaultPlusCash` and `SentToVault`; organization mass mail is a follow-up once ORG-07 exposes membership. | Scope. The mail send path exposes one seam (`resolve_recipient_flags`) so the Bank campaign replaces one function. |
| D-SS08 | PROPOSED | **Item escrow leaves `sgw_inventory`.** On send, the attached item row (or the split-off quantity) moves into a new `sgw_gate_mail_item` table keyed by `mail_id`, keeping every instance column (type, stack, durability, charges, flags, bound, ammo). On take, it moves back into the recipient's `sgw_inventory` in one transaction. Bound items and mission items (container 2) cannot be mailed (`MAILRESULT_ItemNotAvailable`, 2). `database-persistence` settles the table shape (for example `INHERITS (sgw_inventory_base)`) in SS-M2. | Both copies of `INVENTORY_ITEM_SELECT` select every row for a `character_id` (audit A-14), so an escrowed row left in `sgw_inventory` would reappear in someone's bags. |
| D-SS09 | PROPOSED | **COD rules.** COD needs an item attached and a positive cash amount. The amount is read from the stored row, never from the client. Paying debits the recipient, clears the COD flag and the amount in the same transaction, and delivers the payment to the sender as a new server mail carrying the cash. The item is then taken with an ordinary take-item. Cash can never be taken from a COD mail. | CAT-G-05 and issue #72 ("sender receives payment via mail"). Paying by mail works while the sender is offline. The take rules follow the client's own coupling in `onTakeAttachments` (`GateMail.lua:362-382`). |
| D-SS10 | PROPOSED | **Return.** A player may return any non-archived mail from a player sender that has not already been returned. The mail is re-addressed to the stored `sender_id` (never `sender_name`) with its item and any gift cash; a COD amount is cleared. A returned mail cannot be returned again, and system mail (no `sender_id`) cannot be returned. | CAT-G-06. The client enables Return only for unpaid COD, but the wire method accepts any id, so the server needs its own rule; this one cannot loop and cannot redirect. |
| D-SS11 | PROPOSED | **New-mail notification.** When a mail is delivered to an online recipient, the base sends them a feedback line ("You have new mail from X.") and, if SS-E1 shows it is harmless with the window closed, a fresh `onMailHeaderInfo`. `onNewMail` and `notifyPlayersOfNewMail` stay unused. | The def has no client method for new mail (`SGWMailManager.def`: `onNewMail` is in `CellMethods`), so the client can only learn from a header push or a chat line. |
| D-SS12 | PROPOSED | **Text rules.** Mail subject 1-128 UTF-16 units (the column is `varchar(128)`), body up to 1,000, chat text up to 255, **the only cap** (SS-E1 C-Q3: the client's chat input has no length limit). Every field uses the organizations campaign's D-ORG10 rejection rules (bounded decode, no lone surrogates, C0/C1, bidi or zero-width characters; newline allowed in a mail body). Violations are rejected with feedback, not truncated. | CAT-G-01, CAT-L-01. One implementation shared with ORG-01, not a second filter. |
| D-SS13 | PROPOSED | **Name resolution.** A tell, mail recipient or duel target resolves by exact name first, then case-insensitively if exactly one character matches. Mail resolves against `sgw_player` (offline recipients are valid); tells and duels resolve against online sessions only. | `sgw_player.player_name` is `UNIQUE` but case-sensitive (`_primary_keys.sql:55`). Players type names in any case; an ambiguous case-fold is refused rather than guessed. |
| D-SS14 | PROPOSED | **Rate limits.** Per-player token buckets, enforced from day one: **chat** (every player channel, tells included) burst 5, refill 1 per second; **mail send** burst 3, refill 1 per 10 seconds. An over-limit action is dropped with one feedback line ("You are sending messages too quickly."), itself limited to one per 5 seconds, and a `rate_limit.exceeded` event with the category. GameMaster and above are exempt from the chat bucket only. | CAT-L-01 and the proposal in `server-infrastructure-proposals.md:74-97` (which suggests 5 per second, burst 10). Five lines a second is still a flood to the reader, so the sustained rate here is one a second. The proposal's warn-only phase is skipped because no human types faster than this; SigNoz tells us if that is wrong. |
| D-SS15 | APPROVED (owner, 2026-09-27) | **Ignore.** The contact list's Ignore list (flags 301) is the only source; `chatIgnore` (0xC5) edits that list. If A ignores B: B's tells to A are not delivered and B gets "A is not accepting your messages."; B's say, emote and yell are not sent to A; B's duel challenges and mail to A are refused the same way. It is **one-directional** and covers chat, duel challenges and mail only: nobody is hidden from anyone's AoI. | CAT-L-01 and `docs/gameplay/contact-list.md:170`. PR #585's symmetric AoI hiding is a Cimmeria addition that changes gameplay (an ignored attacker or duel partner would vanish), so it is not adopted; its caches and tests are a salvage source. |
| D-SS16 | APPROVED (owner, 2026-09-27) | **GM broadcast.** Access level GameMaster (2) or higher, read from the server's session, may broadcast. The native `/gmshout` sends cell method 222 `sendGMShout(isGlobal, Text)`; `isGlobal = 1` reaches every connected player, `0` the GM's space. A `.announce [space] <text>` console command does the same for GMs whose client lacks the slash binding. Delivery is `onPlayerCommunication` on the server channel with the GM speaker flag and the GM's name. Every broadcast logs `chat.gm_broadcast` with the actor. | CAT-L-06. The client ships `Event_SlashCmd_GMShout`, and the GM command convention is native binding first, `.` console second (`project_gm_commands_native_console`). |
| D-SS17 | PROPOSED | **Channel ids follow D-ORG14.** This campaign does not change `DEFAULT_CHAT_CHANNELS` or the `CHAN_*` constants itself. Tells are routed on whatever byte the client really sends for `/tell`, as ORG-E1 Q5 (or SS-E1, if it gets there first) establishes; the constant change lands once, in ORG-09 or in a PR both coordinators agree on. | The organizations campaign owns the drift (D-ORG14, its audit A-40). Two campaigns editing the same constants would conflict. |
| D-SS18 | PROPOSED | **Duel timers.** A challenge expires after **30 seconds** without an answer (provisional, from #569). After an accept, a **5-second** countdown shown through the client's duel timer, then the duel is engaged. Both are provisional until SS-E1 finds what drives `Event_UI_DuelTimerStart` and any duration constant. | D.1 is open. `ETimerUpdateType.DuelTimer = 14` (`enumerations.xml:714`) suggests the countdown rides `onTimerUpdate`; SS-E1 confirms. |
| D-SS19 | PROPOSED | **Duel range.** A challenge needs the target within **20 units** in the same space (text 877). The arena is a **40-unit** radius around the midpoint at the start; a duelist outside it for more than 5 seconds loses with `EDUEL_DEFEAT_Range` (4). Provisional until D.2. | Text 877 ("You are not close enough to send a duel request") proves a challenge range exists; no value has been recovered. `MAX_INTERACT_DISTANCE = 5` is too short for a challenge. |
| D-SS20 | APPROVED (owner, 2026-09-27) | **Non-lethal end.** Duel-partner damage (ability or effect) that would bring a duelist to 0 HP clamps them at 1 HP and ends the duel with `EDUEL_DEFEAT_Health` (1): no death, corpse, loot, XP, respawn or death penalty. Damage from anything else stays lethal; a duelist killed by a third party loses the duel and dies normally. Health is not restored at the end. Both duelists leave combat with each other. | D.6 is open, but the seeded texts have "You won the duel" (879) and no loss or death string, which fits a non-lethal end. A lethal end would route duel deaths through the loot path (the CAT-M exploit shape). |
| D-SS21 | PROPOSED | **Challenge rate limit.** A challenger bucket of burst 2, refill 1 per 15 seconds; one open challenge per challenger and one per target at a time ("You are already involved in a duel", 873); after a decline or timeout the same challenger cannot challenge the same target again for 60 seconds. | CAT-M-12 (g). The challenge prompt is modal on the target's screen, so spam is griefing. |
| D-SS22 | APPROVED (owner, 2026-09-27) | **No duel rewards.** Nothing is awarded or recorded: no XP, cash, items, rating or stats table. The winner gets text 879, the loser a feedback line. | No reward evidence anywhere (no column, text or wire field). Rewards would make the duel path an exploit target. |
| D-SS23 | PROPOSED, revisit in SS-D2 | **PvP flag.** SS-E1 D-Q4 found a dedicated `pvpFlag` CELL_PUBLIC property and `setPvPFlag`/`startPvPTimer` cell methods on `SGWPlayer.def`. `CELL_PUBLIC` is ghosting-only in SGW, so that is an unresolved possibility, not proof of a client fanout. **Everything in this row about setting and fanning out `GENERICPROPERTY_PvPFlag` is provisional** until SS-D2 traces which vehicle the client actually receives. What is not provisional: the combat gate reads the server's duel registry, never any flag. `GENERICPROPERTY_PvPFlag` (4) is set to 1 on both duelists when the duel is engaged and cleared to 0 on every end path, sent to each duelist and their witnesses. It is **presentation only**: the combat gate reads the server's duel registry (the pair), never the flag. | D.7 is open. Whatever the client does with the flag, a flag-based gate would let a stuck flag make a player attackable by everyone. SS-E1 reports what the client does with it. |
| D-SS24 | PROPOSED | **Duel state lives in a cell `DuelRegistry`**, not in a spawned entity. The type-6 `SGWDuelMarker` entity is created only if SS-E1 shows the client needs one (a boundary visual or `onDuelEntities*` references it). | Both legacy marker files are empty classes, `duelDetectorID` is always 0, and `Duel.layout` has no boundary UI. A registry keyed by `player_id` has no entity lifecycle to leak. |
| D-SS25 | SUPERSEDED (SS-E1, 2026-09-27) | **Lifted: sending 151 at duel start and 153 at duel end is safe.** SS-E1 D-Q5 decompiled all three: they insert into, erase from and clear a set at `GamePlayer+0x16c`, then force an interaction recompute, and the interactability decision never reads that set. `aoi.rs:203-211` has the direction backwards (152 erases); SS-D2 corrects the comment. Original row: **No `onDuelEntities*` send until SS-E1 answers.** The server must not send 151 (`Set`) or 153 (`Clear`), and must not change how it uses 152, until SS-E1 has decompiled all three handlers and the claim at `aoi.rs:203-211`. | AoI uses 152 to make NPCs interactable (audit A-41). A duel end that clears that set could make every NPC unclickable. |
| D-SS26 | PROPOSED | **Moderation basics.** GM `.mute <name> <minutes>` and `.unmute <name>` (GameMaster and above) block a player's chat and tells, with feedback to the muted player on each attempt. Mutes are held on the base session by `player_id` with an expiry and do not survive a restart. Every Communicator base method the server does not implement (0xC6-0xCE) answers with one "not supported yet" feedback line instead of the silent catch-all. | CAT-L-07 to L-09 are out of scope, but the project rule says every press gets visible feedback. Persisted mutes are a later decision. |

PROPOSED rows are adopted at their defaults under D-SS01 unless the owner objects. A change is recorded as a new row, never by editing an old one.

## Owner decisions

Answered by the owner on 2026-09-27. Every answer matched the recommendation:

1. **D-SS02:** yes, the 25-naquadah postage is charged when cash or an item is attached.
2. **D-SS03:** yes, the 100-message cap is enforced for player mail; server-generated mail is exempt.
3. **D-SS04:** the client's TTL, else 30 days. Anything still attached returns to the sender once, and archived mail never expires.
4. **D-SS15:** Ignore blocks chat, duel challenges and mail, one way. Nobody is hidden from anyone's AoI.
5. **D-SS16:** GameMaster and above may broadcast, through native `/gmshout` or `.announce`.
6. **D-SS20 and D-SS22:** duels are non-lethal (clamped at 1 HP) with no rewards.

The other PROPOSED rows proceed as written unless evidence or the owner overturns them.

## Autonomy

The owner authorized an autonomous run (D-SS01):

- workers run in isolated worktrees, one test database each;
- the coordinator squash-merges each PR once CI is green and the named advisors have reviewed it;
- the coordinator puts `/release` on the last merged PR (SS-99).

The coordinator stops and asks only when evidence contradicts an APPROVED row, when a packet needs a client patch, a new opcode or a wire-crypto change (project rule), or when another campaign's coordinator disagrees about a contended file.

## Coordinator launch prompt

You are the coordinator for the social-systems campaign (mail, chat, 1v1 duels). Implement [work-packets.md](work-packets.md) as small reviewed PRs.

1. Record `git rev-parse origin/main` and check that the audit's file references still hold. Check `~/.claude/sessions/*.json` for a live peer holding `social/*` branches; if one exists, message it and stand down.
2. Write the worker rules file `%TEMP%\cimmeria-castle\SS-WORKER-RULES.md` from `ORG-WORKER-RULES.md`, changing the ledger path, the branch prefix (`social/`), and the id block (templates **390-399**, spawns **490-499**).
3. Create each worker's worktree with `bash tools/build-lane/mk-worktree.sh social/<packet>-<slug> ss-<packet>`. Give the worker its packet, the contract section, the decisions and audit rows it cites, and the rules file.
4. `rust-gameserver-dev` writes. `social-systems-engineer` advises on every packet's design. `server-authority-enforcer` reviews every packet that handles a client-supplied name, id, amount, container, slot or channel. `database-persistence` reviews SS-M1 to SS-M4. `combat-systems-advisor` reviews SS-D2 and SS-D3. `aoi-witness-broadcast` reviews the PvP flag fanout and any duel entity. `testing-validation-engineer` reviews the SS-M2, SS-M3 and SS-D3 test plans. `game-archaeology-specialist` runs SS-E1.
5. Squash-merge on green CI. Merge contended files in the order [work-packets.md](work-packets.md) gives.
6. Before SS-C1 dispatches, ask the organizations coordinator (cimmeria-fa) whether ORG-E1 Q5 (the `/tell` channel byte) has landed. Before SS-M2, tell the Bank coordinator (cimmeria-97) about the `resolve_recipient_flags` seam. Before SS-U3 seeds anything, agree positions with the stasis-hub owner.
7. When blocked, leave `handoffs/<packet>.md` with the exact next action, and keep `handoffs/session-resume.md` current.

## Other campaigns

| Campaign | Coordinator | What we share |
|---|---|---|
| Organizations ([ledger](../organizations/README.md)) | cimmeria-fa | The `EChannel` drift (D-ORG14, its audit A-40, ORG-E1 Q5); team, squad, command and officer chat (ORG-04, ORG-09); the D-ORG10 text rules (ORG-01); `destroy_client_entities` (ORG-06). ORG-04 and ORG-09 promise "the existing flood limit": SS-00 is where it comes from. |
| Bank / Vault | cimmeria-97 | Vault mail aliases and the three vault result codes (D-SS07); the inventory move path. |
| Crafting | cimmeria-af | The inventory move path, if crafting changes it. |
| Pets | cimmeria-b5 | The hostility gates, if pets can be attacked or attack players (SS-D2). |
| Stasis debug hub (#846) | stasishub | Hub slots in the Castle_CellBlock stasis room (SS-U3). |

## UAT milestone

The owner runs [SS-UAT](work-packets.md#ss-uat-owner-uat-colo-after-the-release) on the colo after the release. Mail and GM broadcast work with one client. Tells, Ignore and duels need two clients (two accounts); each step lists a solo fallback where one exists. The coordinator reads SigNoz afterwards for the `mail`, `chat`, `duel` and `rate_limit` targets.

## Where confidence is low

- Which byte the client sends for `/tell`, and whether it hardcodes any `EChannel` value (ORG-E1 Q5, SS-E1 C-Q1). Tells cannot ship until this is known.
- What `onDuelEntitiesSet`, `Remove` and `Clear` do in the client, and whether the AoI code's use of 152 is right (SS-E1 D-Q5). The duel start and end sequence depends on it.
- Whether the client computes `ExpiresHours` from a constant, and its value (SS-E1 M-Q3).
- How the client parses the To field into `Recipients` and `RecipientFlags`, and whether `ItemId` is an item instance id or a type id (SS-E1 M-Q2). The escrow key depends on it.
- Every number in D-SS02 to D-SS05, D-SS12, D-SS14 and D-SS18 to D-SS21 is project policy, not recovered data, and the docs must say so.
