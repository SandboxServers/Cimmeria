# ORG-10 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [org-04.md](org-04.md), [org-05.md](org-05.md), [org-06.md](org-06.md), [org-07.md](org-07.md), [the UAT guide](../../../guides/organizations-uat.md).

## Contract

- **Packet:** ORG-10, the GM suite and the UAT guide.
- **Decisions in force:** D-ORG08 (the Leader row holds every bit), D-ORG09 (6) and D-ORG22 (the mask clamp, `OrgPermission::apply_edit`), D-ORG10 (text caps), D-ORG13 (GM commands on the `.` console, access level re-read on the base, `org.gm_action` with the actor), D-ORG25 (coordinator review in place of Copilot).
- **Base:** `origin/main` @ `2b30ec1ba` (ORG-07 #945). Branch `org/10-gm-uat`, worktree `.claude/worktrees/org-10`, test DB `sgw_org_10`. Rebased before push (see "Commands run").
- **Sentinels:** `0x7000_5A00..=0x7000_5BFF` (32 blocks of 16, names `Org10P<n>`, organizations named `Org10 ...`), `Fixture::org10` in `base-session` `organization/handlers/tests/mod.rs`; blocks 0-5 used. Assigned by the coordinator (an earlier self-claim overlapped ORG-08's block; moved before any push). Cell-console unit fixtures use player 7301 / account 8301 (no database).

## Owned paths

- Wire: `crates/wire/src/cell/messages/org_cell_to_base.rs` (`GmInfo`, `GmList`, `GmSetPerms`, `GmReload`, appended at the end of the enum).
- Base session: `organization/handlers/gm_inspect.rs` (new: `gm_info`, `gm_list`, `gm_reload`), `gm_perms.rs` (new: `gm_set_perms`), `handlers/mod.rs` (re-exports), `telemetry.rs` (`ActionRow::{rank, count, gm_audit}`, `action` on every row, two `OrgReject` variants), `answer.rs` (their lines), `gm.rs` (`gm_audit` on join and rank, `gm_actor` made `pub(super)`), `disband.rs` (the GM row moved to `ActionRow` with `gm_audit`), `api.rs` (`OrgAccess::system`'s GM row renamed `org.gm_access`); tests `handlers/tests/gm_suite.rs` (new), `tests/mod.rs` (`Fixture::org10`), `tests/gm.rs` and `persistence/tests/authority.rs` (the rename).
- Base world entry: `cell_dispatch/org_dispatch.rs` (four arms), `tests_dispatch_arms/org_arms.rs` (one test).
- Cell console: `console/org.rs` (`.org_info`, `.org_list`, `.org_set_perms`, `parse_mask`, the `GmCommand` table and the audit twin on console refusals), `registry/commands/org.rs` (three rows), `dispatch.rs` (three arms), `console/mod.rs` (doc), `gm/organizations.rs` (new: CM 164), `gm/mod.rs` (`GM_RELOAD_ORGANIZATIONS`, the arm), `gm/tests/mod.rs` (index pins), `gm/tests/organizations.rs` (new), `tests/org10_gm_suite.rs` (new), `tests/mod.rs`.
- Docs: `docs/guides/organizations-uat.md` (new), `docs/readme.md` (guides table), `CONTRIBUTING.md` (one paragraph), `docs/gameplay/organization-system.md` (new "GM suite (ORG-10)" section, two corrections for the rename), `docs/commands.md` (three rows), `docs/protocol/cell-method-dispatch-table.md` (164 → DONE), `docs/architecture/observability.md` (`org` / `squad` row), this worknote.

## Read set

The ORG-10 packet, ORG-UAT, the contract, telemetry and carried-gap sections of `work-packets.md`; D-ORG08/09/10/13/22/23; worknotes org-03 to org-07; the ORG-08 and ORG-09 packet text (for the guide's event names); `organization/{api.rs,handlers/*,persistence/{texts,loads}.rs}`; `entity/src/organization/{permissions,types}.rs`; `cell-console/console/{org.rs,org_create.rs,squad.rs,dispatch.rs,gm/*}`; `cell-world/dispatch/gm_gate.rs`; `entities/defs/SGWGmPlayer.def:355`; `docs/protocol/cell-method-dispatch-table.md` (164); `docs/content/debug-hub.md`; `chat/squad.rs` (the speaker's own copy).

## Evidence

- `gmReloadOrganizations` has no arguments and is the 56th exposed `SGWGmPlayer` cell method (`SGWGmPlayer.def:355`, between `gmGotoXYZ` 163 and `gmReloadInventory` 165), so index 164 = 109 + 55. It is inside `requires_gm`'s `index >= SGWGMPLAYER_CELL_METHOD_BASE` range; `enforce_gm_gate` refuses it to a non-GM before the GM dispatch runs.
- Team ranks are 2, 3, 8; Command 1-8 (`OrgRank::for_type`). Editable masks: 12 bits for a Team, those plus `OfficerChat` and `EmailLists` for a Command (`permissions.rs:54-72`). Officer's default mask is `0x150572`, which the guide uses as its worked example.

## Design decisions

1. **`.org_set_perms` reuses the member path's two functions.** `OrgPermission::apply_edit(old, wire, type, access.permissions())` works out the stored mask (with a system actor holding every bit, only the clamp applies), and `persistence::set_rank_permissions` writes it; ORG-08's CM 16 uses the same pair, so nothing is forked. The handler reads the old mask under the org lock, refuses the Leader row (`leader_row_pinned`) and unused ranks before the write (the persistence layer pins both again), and refuses an edit the clamp turns into no change (`permissions_unchanged`) with the ignored bits named. After the commit the full rank table [49] goes to every online member through `broadcast_to_org`. The GM mask is parsed on the cell as decimal or `0x` hex (`u32`); bits above the 26 defined are dropped by `from_bits_truncate`, bits outside the editor are ignored by the clamp, and both are reported to the GM, not refused.
2. **`.org_info` works on offline characters.** It resolves the name in `sgw_player` (exact match first, then a unique case-insensitive one), so a GM can inspect a player who is not logged in. `.org_list` prints at most 50 organizations, oldest first.
3. **`gmReloadOrganizations` re-sends only the caller's own state**: `push_org_state` per membership, the same bundle as world entry, with no presence fanout, since nothing changed for anyone else. A failed push counts as not re-sent; the GM line says how many failed.
4. **The D-ORG13 audit gap, fixed.** Before this packet, `.org_disband`, `.org_join` and `.org_rank` wrote `org.gm_action` only from `OrgAccess::system`, at lock time, with `command` but no result and no target; every refusal before the lock (`not_gm`, a stale session, `target_not_found`, a malformed id on the cell) left no `org.gm_action` at all. `ActionRow` gained `gm_audit`: when set, `ok` / `rejected` write the command's own row and an `org.gm_action` twin from the same data (counted once). The console's own refusals write the twin too (`console/org.rs::refused_row`). `OrgAccess::system`'s GM row is renamed `org.gm_access`, so `event = 'org.gm_action'` is exactly one result row per GM command. The ORG-10 commands, like ORG-04's `.squad_*` and ORG-05's `.org_create`, use `org.gm_action` as their outcome row. The `action` label of the disband twin stays `disband` (ORG-06's counter label) rather than inventing `gm_org_disband`.
5. **Audit of the other rules.** No GM command sets organization text, so D-ORG10 has nothing to bypass (`.org_create`'s name goes through `create_org`'s `org_text::validate`; the `.org_join` / `.org_rank` / `.squad_*` / `.org_info` names are lookups, never stored). The D-ORG09 (6) clamp is only reachable through `.org_set_perms` (and CM 16). Leader-row refusal: `.org_rank` refuses assigning or moving `Leader` (ORG-07), `.org_set_perms` refuses the Leader row. The squad commands already wrote one `org.gm_action` with actor and target (ORG-04). The access level is re-read on the base by character and entity id for every base-side GM command (`gm_session`, ORG-07's fix).
6. **Feedback** goes through `fanout::feedback` on the base and `send_gm_feedback` on the cell; no literal channel byte.

## Telemetry added

| Event | Level | SigNoz filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|---|
| `org.gm_action` for every GM organization command (`action` = `gm_org_info` \| `gm_org_list` \| `gm_org_set_perms` \| `gm_reload_organizations`, plus the new twins `gm_org_join` \| `gm_org_rank` \| `disband`); `outcome`, `reason`, GM and target ids, `org_id`, `rank`, `count` | INFO | `event = 'org.gm_action' AND player_id = <GM>`; refusals `AND outcome = 'rejected'` grouped by `action`, `reason` |
| `org.gm_access` (renamed from the lock-time `org.gm_action`; `command`) | INFO | `event = 'org.gm_access' AND org_id = <id>` |
| `permissions_changed` (`via = gm`, `rank`, `from_mask`, `to_mask`, `wire_mask`, `ignored_bits`) | DEBUG | `event = 'permissions_changed' AND org_id = <id>` |
| `org.broadcast` / `org.broadcast_failed` (`what = rank_update`) for the [49] fanout | DEBUG / WARN | `event LIKE 'org.broadcast%' AND org_id = <id>` |
| `org.state_push` (`source = push`) per organization re-sent by the reload | INFO | `event = 'org.state_push' AND player_id = <GM>` |
| `org.forward_failed` (`kind = gm_reload` and the console's GM kinds, `reason = cell_to_base_closed`) | WARN | `event = 'org.forward_failed'` |
| Spans `org.gm_info`, `org.gm_list`, `org.gm_set_perms`, `org.gm_reload` | INFO | traces: span name |

New reasons: `leader_row_pinned`, `permissions_unchanged`, `mask_invalid` (console); reused: `not_gm`, `no_db`, `db_error`, `target_not_found`, `target_ambiguous`, `no_such_org`, `rank_not_in_type`, `org_id_invalid`, `rank_invalid`, `caller_not_player`, `cell_to_base_closed`. Counter: `org_actions_total{action = gm_org_info | gm_org_list | gm_org_set_perms | gm_reload_organizations, outcome, reason}`. Every `ActionRow` outcome row now also carries `action`.

## Commands run

All from the worktree root through the lane (`target=B:\targets/org-10`). Live-DB runs reload `sgw_org_10`; no live-DB test self-skipped (the tier sets `DATABASE_URL`).

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo check -p cimmeria-base-session -p cimmeria-base-world-entry` / `-p cimmeria-cell-console --all-targets` | 0 | |
| `lane.sh cargo nextest run -p cimmeria-cell-console --lib org10 organizations org07 org06` | 0 | 18 passed |
| `live-db-test.sh organization::handlers organization::persistence --no-fail-fast` | 0 | 87 passed |
| `live-db-test.sh gm_suite` | 0 | 14 passed (6 live-DB, 8 console), none skipped |
| `lane.sh cargo nextest run -p cimmeria-base-world-entry --lib org_arms` | 0 | 9 passed |
| `python org10_mutations.py` (the ten proofs below) | 0 | each guard failed with exit 100; `git status` clean of code after |
| `git rebase origin/main` (onto `6607fbdc2`, BV-05 #947; no conflicts) | 0 | |
| `lane.sh cargo fmt --all -- --check` | 0 | |
| `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell-world --all-targets -- -D warnings` | 101, then 0 | `assertions_on_constants` on the 164 tail pin; made a `const` assertion |
| `lane.sh cargo nextest run` (the five crates above) `--lib --no-fail-fast` | 0 | 1885 passed |
| `live-db-test.sh "::" --no-fail-fast` (the whole live-DB tier, after the rebase) | 0 | 5242 passed, 0 skipped |
| `live-db-test.sh organization:: --no-fail-fast` (after moving the sentinels to the coordinator's `0x7000_5A00` block) | 0 | 226 passed |

No new log target, so the `cimmeria-server` logging parity tests were not re-run. No dependency changed (no hakari run).

## Regression proof

`python org10_mutations.py` (scratchpad): each mutation applied alone to the committed tree (`476201817`, before the rebase), its guards run (`live-db-test.sh <tests> --no-fail-fast` or `lane.sh cargo nextest run -p <crate> --lib <tests>`), then `git checkout HEAD -- <file>` and `touch`. `git status` clean afterwards.

| Id | Mutation | Run | Exit | Failed |
|---|---|---|---|---|
| M1 | `.org_set_perms`: the `apply_edit` clamp bypassed (the GM mask stored as sent) | live | 100 | `gm_set_perms_clamps_to_the_editor_bits_and_fans_out` |
| M2 | `.org_set_perms`: the base access-level check removed | live | 100 | `gm_set_perms_refuses_the_leader_row_and_unused_ranks` |
| M3 | `.org_info` / `.org_list` / reload: the base access-level check removed | live | 100 | `gm_info_lists_every_membership_with_rank_and_mask`, `gm_reload_resends_the_login_push` |
| M4 | `.org_set_perms`: no [49] fanout after the commit | live | 100 | `gm_set_perms_clamps_to_the_editor_bits_and_fans_out` |
| M5 | the `org.gm_action` twin of join, rank and disband disabled (`ActionRow::gm_audit`) | live | 100 | `every_gm_org_command_writes_one_gm_action_row` |
| M6 | console: the `org.gm_action` twin of a console-side refusal disabled | unit (`cimmeria-cell-console`) | 100 | `console_refusals_of_every_gm_org_command_write_one_gm_action_row` |
| M7 | the CM 164 arm removed from the GM dispatch | unit (`cimmeria-cell-console`) | 100 | `gm_reload_organizations_forwards_to_the_base` |
| M8 | CM 164 carved out of the gated `SGWGmPlayer` tail in `requires_gm` | unit (`cimmeria-cell-console`) | 100 | `gm_reload_organizations_is_inside_the_gated_tail` |
| M9 | the `org_set_perms` registry row removed | unit (`cimmeria-cell-console`) | 100 | `org10_commands_are_registered`, `org_set_perms_forwards_to_the_base` |
| M10 | base dispatch: the `GmSetPerms` arm a no-op | unit (`cimmeria-base-world-entry`) | 100 | `org10_gm_variants_reach_their_handlers` |

Every mutation failed exactly its named guards. The proofs ran on the first commit, before the sentinel move and the rebase; neither changes the guarded code.

The Leader-row refusal has two layers (the handler's check and `set_rank_permissions`' `LeaderPinned`); removing the handler's check alone still refuses with `leader_row_pinned` through the persistence layer, so that pair was not mutated separately. ORG-02's `set_rank_permissions` tests pin the persistence layer.

## Test catalogue

- `base-session` `organization::handlers::tests::gm_suite` (live-DB, types 3, 8, 12): `gm_info_lists_every_membership_with_rank_and_mask`, `gm_list_lists_every_organization`, `gm_set_perms_clamps_to_the_editor_bits_and_fans_out`, `gm_set_perms_refuses_the_leader_row_and_unused_ranks`, `gm_reload_resends_the_login_push`, `every_gm_org_command_writes_one_gm_action_row`.
- `base-world-entry` `org_arms`: `org10_gm_variants_reach_their_handlers` (dispatch test for the four new variants).
- `cell-console` `tests::org10_gm_suite`: `org10_commands_are_registered` (registry pin), `org_info_forwards_to_the_base`, `org_list_forwards_to_the_base`, `org_set_perms_forwards_to_the_base` (dispatch tests), `masks_parse_as_decimal_or_hex`, `malformed_org_set_perms_stays_on_the_cell`, `console_refusals_of_every_gm_org_command_write_one_gm_action_row`, `org10_commands_from_a_non_gm_are_not_forwarded`.
- `cell-console` `gm::tests::organizations`: `gm_reload_organizations_is_inside_the_gated_tail` (the 164 pin: the def index, `requires_gm`, and `enforce_gm_gate` refusing a non-GM), `gm_reload_organizations_forwards_to_the_base`, `gm_reload_organizations_without_a_character_stays_on_the_cell`; `gm_indices_match_def_document_order` and `implemented_indices_are_in_gm_tail` extended with 164.
- Updated for the rename: `persistence::tests::authority` (`org.gm_access`), `handlers::tests::gm::gm_join_adds_at_entry_rank_and_keeps_the_type_rule`.

## Known gaps

- **Client behaviour of `/ReloadOrganizations`** is unverified: whether the stock console binding sends CM 164 with no arguments, and whether the client redraws an already-open Command window from a second push. The server side is tested; ORG-UAT step 10 watches for it.
- **`.org_list` stops at 50** organizations and has no paging.
- **`gmReloadOrganizations` cannot clear a stale window** for an organization the GM is no longer in: the push only covers current memberships.
- ORG-08 and ORG-09 event names in the guide come from their packet text; the guide says to check the catalog row if a query comes back empty.
- The disband twin's `action` is `disband`, not `gm_org_disband` (it reuses ORG-06's counter label).

## Integration edits for the coordinator

- **Ledger (work-packets.md):** record `OrgCellToBase::{GmInfo, GmList, GmSetPerms, GmReload}` in § Messages; the rename of `OrgAccess::system`'s GM row to `org.gm_access` (§ ORG-API mentions none, but README/D-ORG13's "logs `org.gm_action` with the actor" now means the handler's result row); the ORG-10 sentinel range `0x7000_5A00..=0x7000_5BFF`; the ORG-UAT SigNoz table's `event = 'login_restore'` / `'gm_action'` should read `org.login_restore` / `org.gm_action` (the guide uses the real names).
- **Merge with ORG-08:** both packets touch `handlers/telemetry.rs` (`OrgReject` variants appended at the end; ORG-08 may add its own Leader-row reason for CM 16 — if it names it differently, pick one and map `LeaderRowPinned` to it), `handlers/mod.rs`, `org_cell_to_base.rs` (variants appended), the observability `org` row (text appended before "Later packets add their decisions here") and `organization-system.md`. ORG-08's CM 16 handler should set nothing in `ActionRow::gm_audit` (members are not GMs).
- **Merge with ORG-09:** only the observability row and `organization-system.md`, both append-only.
- **TESTING.md / inventory counts:** not edited (rule). This packet adds 19 tests (6 live-DB).
