# DW-22-E: dot social, organization, squad, bank and Black Market evidence

This is the source contract for DW-22, not live-client acceptance. The inventory is the exact 25 `Spec` names in `crates/cell-console/src/cell/console/registry/commands/{social,org,org_create,squad,bank,black_market}.rs`; headings add the typed `.` prefix. A disabled palette action still exists in the raw GM console. No lab lease or client was used.

## Shared execution and verification rules

`crates/cell-console/src/cell/console/chat.rs` gates the dot channel on server-side GameMaster access. `dispatch.rs::handle_console_command` splits on whitespace, checks registry count, and audits only the name/count, not raw arguments. All 25 entries have `Target::None`: a current selection is neither required nor an authority claim. Names, organization IDs and auction/mail IDs are arguments. The parser has no quoting or escape convention; free text is reconstructed with single spaces where the handler says so. A Cell-to-Base enqueue is not a committed result. In particular, `crates/base-methods/src/base/world_entry/methods/mail/gm`, `.../black_market/gm`, `base-session/.../mutes/gm`, `base-session/.../bank_dump`, and `base-session/.../bank_expand` do not re-read a current GM access level for their forwarded operation. The organization creation and organization handlers do re-read the current GM session; this does not make a later feedback line an identity-fenced receipt. Audit, history, traces and screenshots should not retain private mail subjects, mute reasons, account identity or vault contents as form input. `.announce` explicitly logs its public text in `chat.gm_broadcast`.

The existing `server_db_query` is a read-only, row-capped SQL tool; compare exact IDs and fields before and after a Base result, and reject truncated/unknown results. `server_log_tail` supplies correlated outcome events, not a state read. `.org_info`, `.org_list`, `.mailbox`, `.bankdump`, `.bm_list`, `.squad_info`, and `.duel_status` are their own limited display reads. `server_entity_get` does not expose mail, mute, auction, treasury, membership or vault contents. A successful GM feedback line may be lost or delivered to a reused entity after a commit; neither a line nor an accepted-command audit alone proves the state. The SQL recipes below name existing tables/fields, not a new tool or a permission to query arbitrary private data. `docs/commands.md` (GM command table) describes operator syntax; handlers and tests below govern behavior.

Private read results still use entity-ID-addressed feedback. A later holder of that ID must not receive a previous GM's result; the availability decisions below require original-session delivery fences for such reads.

## Social, duel and mail

### .announce

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/social.rs::{parse_announce,announce}`; `crates/cell-console/src/cell/console/gm/shout.rs::broadcast`; `crates/cell-console/src/cell/console/social.rs::tests::announce_parse_defaults_to_global` and `announce_parse_space_keyword_selects_space_scope`; `docs/commands.md`.
- **Arguments:** At least one text word in the handler, although registry minimum is zero so a bare command gets its own usage refusal. Optional first word `space` (case insensitive) scopes to the current space; remaining words join with single spaces. `gm::shout::broadcast` validates nonempty chat text, at most 255 UTF-16 units and forbidden characters.
- **Target/scope:** GM speaker; all online players for default global, or players in the caller's current space instance. No selected target.
- **Preconditions and authority:** Cell GM gate, live caller/space, valid chat text. Global form forwards to Base; space form queues one `EntityMethodCall` per current player.
- **Effects and impact:** Public server-channel announcement; recipients can act on it. The accepted `chat.gm_broadcast` row includes the full public text and scope. No persistent game-state mutation.
- **Verification:** `server_log_tail` for `chat.gm_broadcast`, `chat.gm_broadcast_send_failed` and recipient counts; sampled recipient client chat or packet tap is needed to prove delivery. Accepted audit is only pre-send.
- **Compensation:** None: a corrective announcement cannot erase a delivered line.
- **Availability:** Runnable with public-text preview, explicit global/space scope and no secret-entry affordance; delivery remains unverified without recipient evidence.

### .duel_status

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/duel.rs::{parse_duel_status,resolve_subject,status_line,duel_status}`; `crates/cell-console/src/cell/console/tests/ss_u2_duel.rs`; `docs/commands.md`.
- **Arguments:** Zero or one case-sensitive online character name; no quote parsing. Extra words fail registry count.
- **Target/scope:** GM's own player by default, otherwise named online player's `player_id` in the Cell duel registry; listing may reveal the opponent.
- **Preconditions and authority:** Cell GM gate; caller or resolved subject needs `player_id`; ambiguous/missing name refuses.
- **Effects and impact:** Read-only live challenge/countdown/duel status, ID, stage, opponent and remaining time; no persistent write. Player names are sensitive in audit/history.
- **Verification:** `.duel_status [name]` feedback is the actual live registry read; `duel.gm_status` log identifies the subject. No independent `server_entity_get` duel registry field; time-sensitive state can change immediately.
- **Compensation:** None needed.
- **Availability:** Disabled for the palette until the Cell-to-Base feedback is fenced to the original GM session; a time-stamped answer alone cannot prevent a recycled entity ID receiving the opponent's identity.

### .duel_end

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/duel.rs::{duel_end,resolve_subject}`; `crates/cell-world/src/cell/duel/gm.rs::gm_end`; `crates/cell-console/src/cell/console/tests/ss_u2_duel.rs`.
- **Arguments:** One required case-sensitive online character name; registry minimum zero routes a bare call to the specific usage refusal.
- **Target/scope:** Named player's challenge or duel and its other participant, possibly two different player sessions.
- **Preconditions and authority:** Cell GM gate, resolvable `player_id`, active challenge/countdown/duel; `nothing_to_end` refuses. Subject is resolved by name at execution.
- **Effects and impact:** Removes challenge/countdown or ends engaged duel; clears PvP flags/combat pair/effects and sends abort notice to both players still in world. No pair cooldown. Cross-player, combat-affecting transient mutation.
- **Verification:** `duel.gm_ended` with duel ID/stage and `.duel_end` line after `gm_end`; subsequent `.duel_status` for both players. Notification delivery and all combat side effects need packet/log evidence, not status alone.
- **Compensation:** None; re-challenging is a new duel and does not restore removed effects or combat state.
- **Availability:** Disabled until a joined-session, two-player before/after receipt can prove the abort and its notifications; no automatic inverse.

### .mute

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/social.rs::{parse_mute,mute}`; `crates/base-session/src/base/mutes/gm.rs::apply_gm_mute`; `crates/cell-console/src/cell/console/social.rs::tests::mute_parse_bounds_minutes_and_reason`; `crates/cell-console/src/cell/console/tests/ss_c3_mute.rs`.
- **Arguments:** Name, unsigned whole minutes 1–10080, optional free-text reason joined with spaces; reason follows 255-unit chat-text validation. Bare form has handler usage. Typed name is one token.
- **Target/scope:** Named online non-GM player; Base mute table keyed by their `player_id` survives logout/relog. Reason, target name and account identity are sensitive.
- **Preconditions and authority:** Cell GM gate; Base resolves valid, unique online name and refuses a GM target. Base bounds duration but does not re-read GM authority/current session identity from `ChatCellToBase::Mute`.
- **Effects and impact:** Replaces any existing mute with a new expiry and prevents chat/tells; GM and target receive lines; memory-only until server restart.
- **Verification:** `chat.gm_mute` outcome has subject ID, duration and `previous_remaining_secs`; GM/subject feedback is sent after table write. No existing read-only mute-table tool; a refused chat attempt could corroborate but changes behavior and is not an independent before/after read.
- **Compensation:** `.unmute` removes the entire current mute, including a preexisting one; it is not an inverse. Prior duration cannot be restored exactly after elapsed time.
- **Availability:** Disabled until Base revalidates immutable GM session and a read-only subject mute-state receipt exists; redact reason in form audit.

### .unmute

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/social.rs::{parse_unmute,unmute}`; `crates/base-session/src/base/mutes/gm.rs::apply_gm_unmute`; `crates/cell-console/src/cell/console/social.rs::tests::unmute_parse_takes_exactly_one_name`.
- **Arguments:** Exactly one online character name token. Bare form routes to its usage refusal.
- **Target/scope:** Named player's Base mute-table entry; cross-player.
- **Preconditions and authority:** Cell GM gate; Base name lookup must resolve an online player. Base does not re-read GM authority/current actor binding.
- **Effects and impact:** Removes active mute if present and tells GM/target; may expose identity and prior moderation state.
- **Verification:** `chat.gm_unmute` and feedback; no read-only mute-table snapshot, so loss of the entry is not independently proven. A no-mute refusal changes nothing.
- **Compensation:** Manually issue a new mute if justified; original remaining duration and moderator attribution are lost. No exact inverse.
- **Availability:** Disabled until Base actor fence and mute-state readback are added; no auto-undo.

### .mail

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/mail.rs::{parse_mail,mail}`; `crates/base-methods/src/base/world_entry/methods/mail/gm/mod.rs::{gm_send,write_gm_send}`; `crates/cell-console/src/cell/console/tests/ss_u1_mail.rs`; `crates/base-methods/src/base/world_entry/methods/mail/tests/gm_live.rs`.
- **Arguments:** Optional option words first, in any order: `to <one-token name>`, `cash <i32 0..MAX>`, `item <positive i32 typeId> [positive i32 qty]`, `cod <positive i32>`, then free-text subject (default `GM test mail`). COD requires item and forbids gift cash. Base `system::validate` adds mail/item/text validation; numeric `item` quantity is parsed only when the next token is an `i32`.
- **Target/scope:** GM sender; self by default or named offline/online recipient. System gift mints cash/item without sender ID; COD mints item in escrow and uses GM character as sender so payment returns to them. Recipient, subject and COD are sensitive.
- **Preconditions and authority:** Cell GM gate and actor `player_id`; Base checks GM row exists, recipient resolution, COD shape and DB validation, but does not re-read current GM access or prove actor session still owns carried player/entity IDs.
- **Effects and impact:** Transaction commits `sgw_gate_mail` and optional escrow item; cash/item are created, not debited. Mailbox cap/postage skipped. Online recipient notified after commit. Cross-player economic impact; COD later transfers recipient payment to GM.
- **Verification:** Base `mail.gm_action` and `Mail <id> sent` follow commit. Query `sgw_gate_mail.mail_id,character_id,sender_id,subject,cash,flags` and `sgw_gate_mail_item.mail_id,item_id,type_id,stack_size` by returned ID; `.mailbox` is aggregate only. Notification is a separate delivery, not proven by DB row. Do not log/display subject outside restricted readback.
- **Compensation:** None proven. `.mail_expire` may return/delete/quarantine depending on current state; it cannot unmint redeemed cash/items or undo COD payment. Manual DB/economic reconciliation only.
- **Availability:** Disabled until Base checks immutable GM actor/session and recipient-bound receipt, with amount/type limits and redacted audit; no send/undo shortcut.

### .mailbox

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/mail.rs::mailbox`; `crates/base-methods/src/base/world_entry/methods/mail/gm/mailbox.rs::{gm_mailbox,MAILBOX_SUMMARY_SQL,mailbox_lines}`; `crates/base-methods/src/base/world_entry/methods/mail/tests/gm_live.rs`.
- **Arguments:** Optional single character name; no selection. Base resolves stored name by exact/unique case-insensitive rules.
- **Target/scope:** GM mailbox by default or another offline/online player's mailbox; reveals private aggregate mail/escrow state, not message subjects.
- **Preconditions and authority:** Cell GM gate and actor `player_id`; DB required. Base has no current-GM session recheck in this forwarded read.
- **Effects and impact:** Read-only counts of open, archived, system and quarantined mail, escrow items/gift cash/COD and next expiry; sensitive recipient financial state.
- **Verification:** Its three Base feedback lines and `mail.gm_action action=mailbox`; exact DB query is `MAILBOX_SUMMARY_SQL` on `sgw_gate_mail` left joined to `sgw_gate_mail_item`, keyed by resolved `character_id`. Aggregate may change with claims/sweep.
- **Compensation:** None needed; restrict output retention.
- **Availability:** Disabled for palette cross-player use until Base rechecks current GM session and bound subject; own-mail mode can be separated later.

### .mail_expire

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/social.rs::SPECS`; `crates/cell-console/src/cell/console/mail.rs::{parse_mail_expire,mail_expire}`; `crates/base-methods/src/base/world_entry/methods/mail/gm/expire.rs::gm_expire`; `crates/base-methods/src/base/world_entry/methods/mail/expiry`; `crates/base-methods/src/base/world_entry/methods/mail/tests/gm_live.rs`.
- **Arguments:** One positive signed-`i32` mail ID; bare form gets specific usage line. No mailbox owner argument.
- **Target/scope:** Any player's mail identified globally by `mail_id`; sender and recipient may both be affected.
- **Preconditions and authority:** Cell GM gate; Base conditionally marks only nonarchived, nonquarantined mail due, but does not recheck GM session authority. Missing/archived/quarantined IDs refuse.
- **Effects and impact:** First updates `expires_at`, then runs sweep path: return attachments, delete empty mail, or quarantine; if settlement fails the mail remains due for later sweep. Cross-player item/cash impact.
- **Verification:** `mail.gm_action action=mail_expire` records path when known; feedback distinguishes return/delete/quarantine/due. Query exact `sgw_gate_mail` and `sgw_gate_mail_item` row plus any returned mail/escrow after sweep; a missing row alone does not prove attachment disposition. `server_log_tail` expiry path evidence is required.
- **Compensation:** None: restoring `expires_at` does not reverse a return, deletion, claim or payout. Manual reconciliation only.
- **Availability:** Disabled until Base actor authorization, exact mail-owner preview and path/recipient readback exist; no automatic inverse.

## Organizations

### .org_disband

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/org.rs::SPECS`; `crates/cell-console/src/cell/console/org.rs::disband`; `crates/base-session/src/base/organization/handlers/disband.rs::gm_disband`; `crates/cell-console/src/cell/console/tests/org06_disband.rs`; `crates/base-session/src/base/organization/handlers/tests/disband.rs`.
- **Arguments:** Required signed-`i32` organization ID; `parse_i32` does not require positive, so Base decides missing ID.
- **Target/scope:** Entire Team or Command and every member; no selected entity. Org name/member list and leader identity are sensitive.
- **Preconditions and authority:** Base checks current session's entity **and** player ID and GameMaster access; DB lock checks organization exists and vault is empty. No member permission required.
- **Effects and impact:** DB transaction disbands organization; afterward fans `onOrganizationLeft(Disbanded)` to online members. Vault-empty protects items, but membership/treasury/history impact is broad.
- **Verification:** Base `org.disband` and `org.gm_action` `disbanded` after commit, GM line with member/told counts; `server_db_query` exact `sgw_organizations.org_id`, `sgw_organization_members.org_id` and treasury/log rows before/after. Fanout count is not proof every client applied it.
- **Compensation:** None. Recreating an org cannot restore original IDs, roles, roster, history and notifications.
- **Availability:** Disabled: destructive cross-player operation with no inverse; require full member/treasury preview and commit/fanout receipt before exposing a form.

### .org_join

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/org.rs::SPECS`; `crates/cell-console/src/cell/console/org.rs::join`; `crates/base-session/src/base/organization/handlers/gm.rs::{gm_join,join_locked}`; `crates/cell-console/src/cell/console/tests/org07_join_rank.rs`; `crates/base-session/src/base/organization/handlers/tests/gm.rs`.
- **Arguments:** Required signed-`i32` org ID and optional one-token online player name; absent name means GM self.
- **Target/scope:** Named online player's membership in a Team/Command; organization and existing members receive join announcement.
- **Preconditions and authority:** Base binds current GM session by player+entity and re-reads GameMaster access. Target name must resolve uniquely online; one-per-type membership rule; memberless organization grants Leader, otherwise type entry rank.
- **Effects and impact:** Transaction adds membership and rank; after commit announces join and sends GM line. Cross-player persistent effect and possibly Leader authority.
- **Verification:** `org.gm_join`/`org.gm_action` and `org_join: added ...` after commit; query exact `(org_id,player_id,org_type,rank)` in `sgw_organization_members`, then `.org_info <name>`. Client state push is separate.
- **Compensation:** A manual kick may be possible but is a new permissioned action; no exact inverse for join notices, Leader recovery or intervening org actions.
- **Availability:** Disabled until explicit immutable target confirmation and joined-session receipt prevent acting on the wrong online player; no automatic inverse.

### .org_rank

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/org.rs::SPECS`; `crates/cell-console/src/cell/console/org.rs::rank`; `crates/base-session/src/base/organization/handlers/gm.rs::{gm_rank,rank_locked,org_of_member}`; `crates/cell-console/src/cell/console/tests/org07_join_rank.rs`; `crates/base-session/src/base/organization/handlers/tests/gm.rs`.
- **Arguments:** Player name token, unsigned `u8` rank (Cell accepts 0–255), optional signed-`i32` org ID. Without ID, Base refuses if player is in both types. Base permits only type-valid non-Leader rank and refuses changing a Leader.
- **Target/scope:** Named online or offline member in one organization; their permissions and officer-note visibility change.
- **Preconditions and authority:** Base GM session/player/entity check and DB transaction lock; target membership, valid rank, changed rank. No ordinary member rank authority check for GM.
- **Effects and impact:** Commits rank and associated note sync; sends rank update to members and GM line stating old/new. Persistent cross-player authority change.
- **Verification:** Base `org.gm_rank`/`org.gm_action`, `rank_changed`, GM line; query exact `sgw_organization_members.org_id,player_id,rank` before/after and `.org_info <player>`; fanout is separate.
- **Compensation:** Manually restore captured old rank only if member and policy still match; notices and note access during the interval remain. No automatic inverse.
- **Availability:** Disabled until exact org/member preview and post-commit readback are wired; rank text alone must not count as proof.

### .org_info

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/org.rs::SPECS`; `crates/cell-console/src/cell/console/org.rs::info`; `crates/base-session/src/base/organization/handlers/gm_inspect.rs::gm_info`; `crates/cell-console/src/cell/console/tests/org10_gm_suite.rs`.
- **Arguments:** Optional one-token player name; absent means GM's own character. Base resolves offline names exactly first, then uniquely case-insensitive.
- **Target/scope:** One player's Team/Command memberships, rank and permission mask; sensitive member identity and role.
- **Preconditions and authority:** Base binds current GM player/entity session and re-reads access before DB awaits; ambiguous/missing name refuses. It does not recheck the original session before sending the private result by entity ID.
- **Effects and impact:** Read-only membership list; no DB mutation or org state push.
- **Verification:** Base feedback header and per-membership rows, `org.gm_action action=gm_org_info` count; independently query `sgw_organization_members` by resolved player ID. Results can age.
- **Compensation:** None needed; redact or restrict retained cross-player result.
- **Availability:** Disabled until each private result is fenced to the original GM session after the DB read; result-retention controls alone cannot prevent delivery to a recycled entity ID.

### .org_list

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/org.rs::SPECS`; `crates/cell-console/src/cell/console/org.rs::list`; `crates/base-session/src/base/organization/handlers/gm_inspect.rs::{gm_list,ORG_LIST_MAX}`; `crates/cell-console/src/cell/console/tests/org10_gm_suite.rs`.
- **Arguments:** None.
- **Target/scope:** Every Team/Command; lists up to 50 oldest IDs, names, member counts and leader names. No selected entity.
- **Preconditions and authority:** Base binds GM current player/entity session and re-reads GameMaster access before the DB await; DB required. It does not recheck the original session before sending leader names by entity ID.
- **Effects and impact:** Read-only global organization metadata; leader names are sensitive.
- **Verification:** `org_list` feedback and `org.gm_action action=gm_org_list` shown count; `server_db_query` `sgw_organizations` plus membership count. A `more than 50` marker means feedback is incomplete.
- **Compensation:** None needed.
- **Availability:** Disabled until each private result is fenced to the original GM session after the DB read; retain the 50-row display cap and truncation label.

### .org_set_perms

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/org.rs::SPECS`; `crates/cell-console/src/cell/console/org.rs::{set_perms,parse_mask}`; `crates/base-session/src/base/organization/handlers/gm_perms.rs::gm_set_perms`; `crates/cell-console/src/cell/console/tests/org10_gm_suite.rs`; `crates/base-session/src/base/organization/handlers/tests/gm_suite.rs`.
- **Arguments:** Signed-`i32` org ID, unsigned `u8` rank, unsigned `u32` decimal or `0x` hex mask. Base rejects absent rank/Leader and clamps bits outside type editor; unchanged result refuses.
- **Target/scope:** Rank policy for all current/future members of one organization; OfficerNotes access may change.
- **Preconditions and authority:** Base current GM session/player/entity check, organization lock, type-valid editable rank and DB.
- **Effects and impact:** Transaction changes rank permission mask, then broadcasts rank table and optional officer-note sync. Persistent organization-wide authority change.
- **Verification:** `org.gm_action action=gm_org_set_perms`, `permissions_changed` with from/to/ignored bits and GM line; query exact `sgw_organization_ranks` row's permission mask before/after. Requested wire mask may differ from stored mask.
- **Compensation:** Restore captured stored mask manually under current policy; any access exercised before restore remains. No safe automatic inverse.
- **Availability:** Disabled until policy preview shows clamped effective mask and member impact, with post-commit exact readback.

### .org_create

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/org_create.rs::SPECS`; `crates/cell-console/src/cell/console/org_create.rs::org_create`; `crates/base-session/src/base/organization/creation/handler.rs::{handle_gm_create,found_and_answer}`; `crates/cell-console/src/cell/console/tests/org05_org_create.rs`; `crates/base-session/src/base/organization/creation/tests/found.rs`.
- **Arguments:** `team|command` (case-insensitive) plus one or more name words joined with spaces; registry minimum two. Base name validation applies; creation cost is from its organization rules, currently zero per `docs/commands.md`.
- **Target/scope:** GM's own character becomes Leader of a new Team/Command; name may be visible to others.
- **Preconditions and authority:** Cell GM gate and loaded character; Base binds current player/entity session and re-reads GameMaster access. Name uniqueness/text, one-per-type rule, funding and DB transaction apply.
- **Effects and impact:** Creates organization, Leader membership and any founding cash debit; after commit sends creation result, organization state and GM line. Persistent ID/name/treasury footprint; UI push failure can occur after commit.
- **Verification:** `org.gm_action action=gm_org_create` with org ID and cash result, `onOrganizationCreationResult` plus line; query exact `sgw_organizations.org_id,org_type,name` and Leader `sgw_organization_members`, and caller `sgw_player.naquadah` before/after. Window/push is separate.
- **Compensation:** `.org_disband` is destructive and vault-sensitive, not an exact inverse; creation announcements and cost/ID cannot be rolled back automatically.
- **Availability:** Disabled until validated name/cost preview and committed-ID receipt/readback are joined; do not suggest disband as undo.

## Squads

### .squad_invite

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/squad.rs::SPECS`; `crates/cell-console/src/cell/console/squad.rs::{dispatch,invite}`; `crates/cell-interactions/src/cell/organization/squad/gm.rs::gm_invite`; `crates/cell-console/src/cell/console/tests/org04_squad.rs`.
- **Arguments:** One online player name token.
- **Target/scope:** GM invites named player to GM's squad through ordinary `/squadinvite` path; target sees an invitation and may accept.
- **Preconditions and authority:** Cell GM gate repeated in handler; GM loaded player; normal invite path resolves target and enforces squad rules/limits.
- **Effects and impact:** Creates pending cross-player invitation and possible later squad join; transient Cell state and notifications. Target name/account are sensitive.
- **Verification:** `org.gm_action action=gm_squad_invite`, `squad.invite` and GM/recipient feedback; `.squad_info` only proves membership after acceptance, not pending invite. No read-only pending-invite tool.
- **Compensation:** Pending invite may expire/cancel through ordinary mechanics, but no proven GM exact inverse or notification retraction.
- **Availability:** Disabled until target-bound invite receipt and pending-state readback exist.

### .squad_join

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/squad.rs::SPECS`; `crates/cell-console/src/cell/console/squad.rs::{dispatch,join}`; `crates/cell-interactions/src/cell/organization/squad/gm.rs::gm_join`; `crates/cell-console/src/cell/console/tests/org04_squad.rs`.
- **Arguments:** One online host name token; ambiguity, absent host or in-transition host refuses.
- **Target/scope:** GM joins host's squad without invite; if host has none, creates one with host as leader. Both players and existing squad receive join fanout.
- **Preconditions and authority:** Cell GM gate repeated; GM and host must be current player actors; `SquadRegistry::force_join` enforces membership/size rules but skips invite/leader handshake.
- **Effects and impact:** Transient cross-player squad creation/join, including host leadership if created; client fanout/GM line after mutation.
- **Verification:** `org.gm_action action=gm_squad_join` with squad ID; `.squad_info <host>` and `.squad_info` read both members/leader. Notification delivery remains separate.
- **Compensation:** Leaving/disbanding later is a new action, not exact inverse of creation or notices.
- **Availability:** Disabled until explicit host identity and before/after squad receipt are bound; no auto-undo.

### .squad_info

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/squad.rs::SPECS`; `crates/cell-console/src/cell/console/squad.rs::{dispatch,info,info_lines}`; `crates/cell-console/src/cell/console/tests/org04_squad.rs`.
- **Arguments:** Optional one-token online player name; absent means GM.
- **Target/scope:** Subject's live squad: ID, member names, leader/member, level, entity ID or in-transit state, loot rule. Sensitive player roster.
- **Preconditions and authority:** Cell GM gate repeated; named subject must resolve online and have loaded player ID/name.
- **Effects and impact:** Read-only Cell squad registry display; no persistent write.
- **Verification:** Its own feedback lines and `org.gm_action action=gm_squad_info`; no independent DB row because squads are live Cell state.
- **Compensation:** None needed; restrict roster retention.
- **Availability:** Disabled until the Cell-to-Base feedback is fenced to the original GM session; a recycled entity ID can otherwise receive a private roster despite sensitive-result retention controls.

## Bank and treasury

### .bank

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/bank.rs::SPECS`; `crates/cell-console/src/cell/console/bank.rs::open`; `crates/cell-interactions/src/cell/interactions/bank/open.rs::open_vault_gm`; `crates/cell-console/src/cell/console/tests/bv02_bank.rs`.
- **Arguments:** None.
- **Target/scope:** GM's own personal vault session in current space; no Banker needed and no selected target.
- **Preconditions and authority:** Cell GM gate; caller entity and space must exist. Non-GM path gives explicit refusal.
- **Effects and impact:** Sets transient `vault_session` with `banker_id=None` and sends `onVaultOpen`; later bank moves skip Banker proximity until any interact closes session. Does not itself move items or cash.
- **Verification:** `vault_session_opened` and `bank: personal vault opened` feedback; client `onVaultOpen`/window or subsequent guarded bank access needed to prove it reached client. No `server_entity_get` vault-session field.
- **Compensation:** Interact closes the session; no item/cash undo needed for opening alone.
- **Availability:** Runnable with clear proximity-bypass scope and no claim that vault contents changed.

### .bankdump

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/bank.rs::SPECS`; `crates/cell-console/src/cell/console/bank.rs::dump`; `crates/base-session/src/base/bank_dump/mod.rs::{load_vault_dump,run_gm_dump,dump_lines}`; `crates/cell-console/src/cell/console/tests/bv04_bankdump.rs`.
- **Arguments:** Optional exact, case-sensitive one-token character name; absent means GM's own `player_id`.
- **Target/scope:** Personal vault container 17 of self or any offline/online character; item IDs/types/stack and player identity are sensitive.
- **Preconditions and authority:** Cell GM gate; Base needs DB but does not recheck current GM session in `handle_gm_dump`. Named target need not be online.
- **Effects and impact:** Read-only DB listing, bank slot size and item rows. At most 100 item lines; summary counts all rows and marks those beyond declared slots.
- **Verification:** `gm_action action=bankdump result=ok` with target/count/slots, feedback item rows; `server_db_query` `sgw_player.bank_slots` and `sgw_inventory` where `character_id=target,container_id=17`, comparing `item_id,type_id,slot_id,stack_size`. Truncated display is explicit.
- **Compensation:** None needed; do not put another player's vault data in unrestricted audit/history.
- **Availability:** Disabled for combined self/other form until Base rechecks GM actor and target-bound private read; self-only can be separated.

### .bankexpand

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/bank.rs::SPECS`; `crates/cell-console/src/cell/console/bank.rs::expand`; `crates/cell-interactions/src/cell/interactions/bank/expand.rs::gm_expand_vault`; `crates/base-session/src/base/bank_expand/mod.rs::{handle_expand,gm_offer}`; `crates/base-session/src/base/bank_expand/gm_tests.rs`.
- **Arguments:** None; buys exactly one +10 step at current seeded `resources.bank_expansion_price` (not a caller-supplied amount), up to 100 slots.
- **Target/scope:** GM's own personal vault and `sgw_player.naquadah` balance.
- **Preconditions and authority:** Cell GM gate; live personal vault session from `.bank` or Banker access; Base validates vault verdict, current size, price and funds, but forwarded `Expand` has no Base GM access recheck.
- **Effects and impact:** Keyed DB purchase increases `sgw_player.bank_slots` and debits own cash; after commit sends bag size, cash and paid line. Persistent, no item moved.
- **Verification:** Base `bank.expand_purchase` `expand` event after commit or `expand_rejected`; exact `sgw_player.player_id,bank_slots,naquadah` before/after plus `resources.bank_expansion_price.to_slots,price_naquadah`. A Cell enqueue or open session is not purchase proof.
- **Compensation:** No shrink/refund inverse in this command family; manual adjustment would need separately authorized reconciliation.
- **Availability:** Disabled until actor-bound Base authorization, price/size confirmation and exact post-commit balance receipt exist.

### .orgvaultexpand

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/bank.rs::SPECS`; `crates/cell-console/src/cell/console/bank.rs::{org_expand,parse_org_expand_args}`; `crates/base-methods/src/base/world_entry/methods/inventory/org_vault/expand.rs::handle_org_vault_expand`; `crates/cell-console/src/cell/console/tests/bv09_orgvaultexpand.rs`; `crates/base-methods/src/base/world_entry/methods/inventory/org_vault/tests/expand`.
- **Arguments:** Optional `team|command` and signed-`i16` `from_slots`, either order. Default Team; no size quotes only. Purchase requires exact current size. Team grows by +10 to 100, price from `resources.bank_expansion_price`; Command fixed at 100/refused.
- **Target/scope:** Organization where GM is a member of requested type; Team Leader only for purchase. Debits Team treasury, affects every member's vault capacity.
- **Preconditions and authority:** Cell GM gate, loaded player; Base locks player and organization and checks current Leader rank, size, price and funds. The Base path uses carried player/entity IDs and Leader check, but does not separately re-read GM access.
- **Effects and impact:** Quote is read-only; purchase transaction increments `sgw_organizations.vault_slots`, decrements `cash`, writes `sgw_organization_cash_log` direction `vault_expansion`; after commit broadcasts cash and sends bag info/line. Replay size charges nothing.
- **Verification:** Quote feedback has size/price/treasury. Purchase `bank.expand` and `org_cash_transfer` after commit; query exact `sgw_organizations.org_id,cash,vault_slots`, `sgw_organization_cash_log.org_id,direction,amount` and seed price. Fanout is separate from DB proof.
- **Compensation:** No exact inverse: shrinking capacity could strand items; treasury credit is a separate action, not undo.
- **Availability:** Disabled for purchase until Base actor fence and org-bound quote/purchase receipt; a separately typed quote-only read may be offered later.

## Black Market

### .bm_seed

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/black_market.rs::SPECS`; `crates/cell-console/src/cell/console/black_market.rs::{parse_seed,seed}`; `crates/base-methods/src/base/world_entry/methods/black_market/gm/seed.rs::{gm_seed,insert_listings}`; `crates/base-methods/src/base/world_entry/methods/black_market/seed.rs::ensure_system_seller`; `crates/cell-console/src/cell/console/tests/bm07_black_market.rs`; `crates/base-methods/src/base/world_entry/methods/black_market/gm/tests.rs`.
- **Arguments:** Optional unsigned count 1–60, default 8; Cell rejects zero/text/out-of-range. Base clamps count again, so it is not an independent authority bound.
- **Target/scope:** Reserved system seller, global auction catalog and later buyers; no selected item argument. UAT item specs are selected by `uat_specs`, not user input.
- **Preconditions and authority:** Cell GM gate and loaded player; Base calls `ensure_system_seller` before the listing transaction. That function independently inserts reserved account/character rows, disables an existing matching account, and reads back their identity; conflicts refuse listing. Base does not bind/recheck current GM access for `BmGmActor`.
- **Effects and impact:** Reserved seller insertion or account disable can persist even when the later listing transaction fails. A separate transaction inserts count system listings with new auction IDs; later sale/expiry may mint item and move cash by settlement. Persistent global economy impact.
- **Verification:** Read back reserved `account.account_id,account_name,enabled` and `sgw_player.player_id,account_id,player_name` even after listing refusal. Base `bm.gm_action action=bm_seed` reports count, first and last auction IDs after the listing transaction, but not the complete inserted ID set. Concurrent sequences can interleave, so the range is not a per-listing receipt; a query over it cannot attribute every row to this command. Require a complete committed-ID receipt before verifying each `sgw_auction.sequence_id,seller_id,item_def_id,stack_size,status,expires_at`. `.bm_list` shows only newest ten and cannot prove all 60.
- **Compensation:** No automatic rollback of reserved seller insertion or account disable after listing failure; inspect those rows before any manual correction. `.bm_expire` is a settlement, not undo; active bids and mailed payouts cannot be reversed automatically.
- **Availability:** Disabled until Base GM-session fence, ID-complete receipt and economic impact preview; no bulk auto-undo.

### .bm_expire

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/black_market.rs::SPECS`; `crates/cell-console/src/cell/console/black_market.rs::{parse_expire,expire}`; `crates/base-methods/src/base/world_entry/methods/black_market/gm/expire.rs::gm_expire`; `crates/cell-console/src/cell/console/tests/bm07_black_market.rs`.
- **Arguments:** Positive signed-`i32` auction ID; optional `#` prefix. Missing ID gets specific usage. Other tokens fail registry count.
- **Target/scope:** Any active auction and its seller/bidder; no owner restriction or selected target.
- **Preconditions and authority:** Cell GM gate; Base conditional update requires active status, then runs one sweep pass. Base does not recheck current GM actor. Sold/cancelled/expired/nonexistent IDs refuse.
- **Effects and impact:** Sets expiry no later than now; sweep may sell and mail item/cash, return unsold item, end system listing, quarantine, or retry later. It can settle other due auctions in the same pass. Cross-player irreversible economic effects.
- **Verification:** `bm.gm_action action=bm_expire` outcome and line distinguish sold/expired/quarantined/retry/not-settled; query exact `sgw_auction.sequence_id,status,expires_at,current_bidder,current_bid` and related settlement/mail rows before/after. Feedback about one ID does not prove all side effects of the sweep pass.
- **Compensation:** None; restoring expiry does not reclaim mailed items or paid currency.
- **Availability:** Disabled until GM-session fence and auction/bidder/settlement preview plus complete sweep readback; no inverse.

### .bm_list

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/black_market.rs::SPECS`; `crates/cell-console/src/cell/console/black_market.rs::list`; `crates/base-methods/src/base/world_entry/methods/black_market/gm/list.rs::{gm_list,LIST_LIMIT,format_row}`; `crates/cell-console/src/cell/console/tests/bm07_black_market.rs`.
- **Arguments:** None.
- **Target/scope:** Global active auction catalog, including seller name, item, price, bidder presence and expiry; no selected target.
- **Preconditions and authority:** Cell GM gate and loaded actor; Base needs DB but does not separately recheck current GM access.
- **Effects and impact:** Read-only; returns ten newest active auctions and total count. Seller names and current bids are sensitive operator data.
- **Verification:** Base feedback rows and `bm.gm_action action=bm_list` `total/shown`; query exact `sgw_auction` active `sequence_id,status,expires_at` with `ORDER BY sequence_id DESC LIMIT 10`. More than ten is intentionally truncated.
- **Compensation:** None needed.
- **Availability:** Disabled until Base rechecks current GM authority and fences each private result to the original session after the DB read; retain the ten-row cap and truncation label.

## Source-check samples

- **Currency path:** `.orgvaultexpand` quote supplies current `vault_slots`, seed price and treasury; `expand.rs::expand` locks the Team row and updates `vault_slots` and `cash` in the same transaction, then inserts `CashDirection::VaultExpansion`. `from_slots` mismatch refuses `replay` before debit. The post-commit `org_cash_transfer` row and exact DB balance, not Cell forwarding, establish purchase.
- **Organization path:** `.org_join` `gm_session` checks the current player/entity and access level; `join_locked` uses `OrgAccess::system`, computes entry/Leader rank and commits before fanout. A player already in a same-type organization refuses through `add_joined`; an accepted Cell command is not membership proof.
- **Refusal path:** `.bm_seed 0` is rejected in `black_market.rs::parse_seed` as `invalid_count`, gives `SEED_USAGE`, and sends no `BlackMarketCellToBase` message; `crates/cell-console/src/cell/console/tests/bm07_black_market.rs::refusals_log_the_reason_and_send_nothing_to_the_base` pins that boundary. A Base reservation conflict can refuse even after a valid Cell parse.
