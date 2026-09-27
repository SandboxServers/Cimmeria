# Social Systems: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27 (close-out, SS-99). Companions: [decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md), [gap analysis §21, §24, §27](../../../gap-analysis.md).

## State: complete, awaiting the owner's UAT

Every implementation packet is merged. SS-99, the close-out, is on `social/99-close-out` for the coordinator to open as a PR and put `/release` on (D-SS01). Branches were `social/*`; the id block is entity templates **390-399** and spawns **490-499** (only template 390 and spawn 490, the Gate Mail Clerk, are used).

| Packet | Status | PR |
|---|---|---|
| Plan | Integrated | #873 |
| SS-E1 client evidence | Integrated | #875 |
| SS-00 shared infrastructure | Integrated | #880 |
| `chat.rs` split | Integrated | #885 |
| SS-C2 GM broadcast | Integrated | #887 |
| SS-D1 challenge and response | Integrated | #888 |
| SS-C1 tells and Ignore | Integrated | #893 |
| SS-M1 plain send | Integrated | #894 |
| Ledger update | Integrated | #907 |
| SS-U2 `sparbot` and duel GM tools | Integrated | #910 |
| SS-D2 PvP flag and harm gate | Integrated | #911 |
| SS-M2 attachments and escrow | Integrated | #912 |
| SS-D3 end paths | Integrated | #924 |
| SS-C3 allowlist, mutes, feedback | Integrated | #925 |
| SS-M3 take, COD, return | Integrated | #926 |
| SS-U1 system mail and GM mail tools | Integrated | #929 |
| SS-M4 notification and expiry | Integrated | #933 |
| SS-U3 Gate Mail Clerk | Integrated | #934 |
| SS-C4 channel ids | Integrated | #937 |
| SS-99 close-out | Review | (coordinator opens it) |

Where the work stands, by line:

- **Mail** does everything the client's mail window asks for, on the server: text to up to 10 recipients, gift cash, an item or COD to one recipient with 25 naquadah postage and the item in escrow, take, pay, return, a new-mail line, and a 30-day expiry that returns, deletes or quarantines. Gap analysis: 15 NT, 2 KM (quarantine release, vault and organization aliases).
- **Chat** has tells, Ignore, a flood limit, text rules, a channel allowlist, GM broadcast and GM mutes, and its channel ids now match the client's. Gap analysis: 7 NT, 1 IM, 3 KM. Team, command and officer chat belong to the organizations campaign.
- **Duels** are 1v1, non-lethal and reward-free, with every end path. Gap analysis: 5 IM, 1 KM (the marker entity, not needed).
- **Tests:** 392 net new, 5.3% of the workspace ([inventory](../../../testing/inventory/README.md#social-systems-campaign-2026-09-27)). The type 11 two-client tests (`two_client_tell`, `two_client_mail_cod`, `duel_two_duelists_and_a_spectator`, `sparbot_duel`) do not run in CI.

Nothing in the campaign has been run in the real client.

## Owner UAT

The checklist is [SS-UAT in work-packets.md](../work-packets.md#ss-uat-owner-uat-colo-after-the-release). It is kept in one place so it cannot drift; this is what you need before you start:

- two accounts, A and B, each with a character in the Castle_CellBlock stasis room (world 12);
- GM rights on A for the solo fallbacks in steps 2 to 4 and for steps 6 and 10;
- a third account, C, only for the spectator in step 11;
- for a solo duel, `sparbot` needs its own account and, on the colo, the colo's `--auth-url` (question Q-g below).

**The Gate Mail Clerk's dialog id is 60104.** SS-U3 seeded it as 100104, but client dialog ids must be 65535 or less: ids from 100100 up crashed the colo client on map load. The pets hotfix PR #938 renumbers every Cimmeria-authored dialog below 65536, and the clerk's becomes 60104. Until #938 merges, `main` still carries 100104 in `db/resources/`, in `docs/content/debug-hub.md` and in the clerk's tests; the UAT assumes #938 has shipped.

## Owner questions

Current behaviour is the default for each until the owner answers.

| ID | Question | Current behaviour | Background |
|---|---|---|---|
| Q-a | Should a deleted recipient's escrowed mail items and COD go back to the sender? | Deleted with the character: `sgw_gate_mail_item` cascades from `sgw_gate_mail`, which cascades from the character. | Keeping character deletion unblocked was the reason (SS-M2). A return needs a trigger or a mail-back step in the delete path. |
| Q-b | A COD whose sender was deleted becomes a free take. Keep that policy? | Implemented (SS-M3): paying cancels the COD, charges nothing, and the item becomes an ordinary take. | Without it the item is stranded: nobody can be paid and nobody returned to. |
| Q-c | Cap archived mail that holds items or cash at 100? | No cap. Archived mail is exempt from the 100-message cap and never expires, so mailing yourself and archiving is unlimited storage at 25 naquadah per item. | Recommended by SS-M4, its reviewer and the Bank campaign: it undercuts the per-player vault sizes (BV-01), and the archive header list is read with no `LIMIT`. Server-side only, no client patch. |
| Q-d | Key mutes by account instead of character? | Per character (`player_id`), in memory, cleared by a server restart. | An alt escapes a mute today. |
| Q-e | Should a mute block mail too? | It does not. A mute covers chat and tells only (D-SS26). | Mail has its own flood limit. |
| Q-f | May pets join duels? | No. A pet cannot harm its owner's duel opponent or be harmed by them (`combat::player_may_attack_pve`). | Pets campaign question; the guard `duel_opponent_cannot_harm_partner_pet` pins the default. |
| Q-g | Provide a colo account for `sparbot`? | The bot has only run against the in-process test server. | Needed for step 11's solo fallback on the colo. |
| Q-h | Apply bind-on-acquire at grant? | Never applied (#914), so a reward meant to be bound can be mailed or traded. | Mail refuses bound items correctly; the flag is just never set. |
| Q-i | Confirm the provisional duel numbers? | 30 s to answer a challenge, 5 s countdown, 20-unit challenge range, 40-unit arena with a 5 s grace, 60 s pair cooldown, 10-minute limit. | Project policy (D-SS18, D-SS19, D-SS21); no values were recovered from the client. |
| Q-j | Add a GM `.mail_release <id> [to <name>]` for quarantined mail? | Quarantined mail and its escrow row are kept, and `.mailbox` counts them, but nothing releases them. | D-SS04 promises GM recovery by id. The command must take `claim::lock_mail`'s lock order and restore through the take SQL. |
| Q-k | Patch two client-only cosmetics, or accept them? | The unit-frame PvP indicator does not refresh live (a typo in the client's Lua); after a refused mail send, the Send button stays greyed until New or Reply. | Both need a client patch, so they need a maintainer decision (project rule). The server sends the right data in both cases. |
| Q-l | Keep `sgw_player_content_cooldown` as the general per-player content cooldown table? | Added by SS-U3 for the Gate Mail Clerk: `(player_id, cooldown_key, last_used_at)`, `ON DELETE CASCADE`, claimed in the mail's own transaction. | A new `sgw` table, the campaign's only schema addition outside `db/sgw/Mail/`. |

The squad-duel text (874 once squads exist, SS-D1) and a result text other than 878 for the safety ends (SS-D2) are minor wording questions with no current owner impact.

## Follow-up issues

Filed during the campaign, all open:

- [#906](https://github.com/SandboxServers/Cimmeria/issues/906): instant-cast player abilities can hit a target in another space (no same-space check at fire).
- [#913](https://github.com/SandboxServers/Cimmeria/issues/913): the trade row-lock order can deadlock against gate mail.
- [#914](https://github.com/SandboxServers/Cimmeria/issues/914): `BIND_ON_ACQUIRE` is never applied at grant (Q-h).
- [#928](https://github.com/SandboxServers/Cimmeria/issues/928): vendor buyback takes row locks before the inventory advisory lock.

Issues #72 (mail) and #569 (duels) are covered by the campaign. The coordinator closes them with a pointer to this ledger.

## Things a resuming session must not miss

- **Two findings docs still say the old thing.** `docs/reverse-engineering/findings/organization-restoration.md:241-243` says the Rust channel ids "must change" (SS-C4 changed them), and `chat-wire-formats.md` does not yet record that `onChatJoined`'s `ChannelID` is a display id (the channel minus 12). SS-C4 left both to their RE owners.
- **The Black Market** must call `sent.notify(...)` after committing a `send_system_mail(_tx)` (BM-02b), so an online recipient is told. Unclaimed payouts expire after 30 days into quarantine, not loss.
- **The Bank campaign** owns the vault mail aliases and result codes (D-SS07), through the one seam `send::resolve_recipient_flags`.
- **The organizations campaign** owns team, command and officer chat and organization mail aliases. It knows that no channel is registered at login and that `DEFAULT_CHAT_CHANNELS` is gone.
- **Mail rows from before SS-M4** have `expires_at` NULL and never expire. The colo database is rebuilt from the seed on every deploy, and the seed has no mail, so none exist there.
- **Worktrees.** Retire each `ss-*` worktree, its `sgw_ss-*` database and its branch now that its PR has merged (remove the `external` junction non-recursively first).

## Other campaigns

- **Organizations** (cimmeria-fa, `docs/analysis/organizations/`): team, command and officer chat; organization mail aliases.
- **Bank / Vault** (cimmeria-97): vault mail aliases (D-SS07) and the mail-as-storage question (Q-c).
- **Crafting** (cimmeria-af): the crafting bag (15) is a mail source and a take destination (SS-M4).
- **Pets** (cimmeria-b5): pets in duels (Q-f); the dialog renumbering hotfix #938.
- **Black Market** (cimmeria-e3): BM-02b moves its payouts onto the system-mail writer.
