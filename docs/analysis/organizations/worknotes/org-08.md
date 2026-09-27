# ORG-08 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [org-02.md](org-02.md), [org-06.md](org-06.md), [org-07.md](org-07.md).

## Contract

- **Packet:** ORG-08, MOTD, member and officer notes, and the rank editor: CM 13-17 for Team and Command ids, fanout [45], [46], [47], [49], [50].
- **Decisions in force:** D-ORG04 (ORG-LOCK), D-ORG08 (the `Leader` row pinned), D-ORG09 (1), (2), (3), (5), (6), D-ORG10 and D-ORG23 (text), D-ORG22 (the permission-edit rule, `OrgPermission::apply_edit`).
- **Coordinator notes applied:** the ORG-07 "not available yet" base arms for CM 13-17 are replaced; CM 10 and CM 19 are untouched. [47] goes only to `OfficerNotes` holders. The `Leader` row is refused for every permission edit. No path touches `sgw_player` while it holds the organization. Feedback goes through `fanout::feedback`, never a literal channel byte. No edits to `gap-analysis.md`, `project-status.md` or test counts.
- **Base:** `origin/main` @ `2b30ec1ba`, rebased onto `1f1a019d8` (ORG-09, #951) and then onto `be3e77228` (ORG-10, #952). Before the second rebase the branch was squashed into one commit, so the conflicts were resolved once, by hand, and compiled before continuing:
  - `handlers/{mod.rs,tests/mod.rs}`: module lists, the sentinel doc, and the `Fixture` constructors.
  - `telemetry.rs` and `answer.rs`: `OrgReject` variants.
  - `org_dispatch.rs`: imports.
  - `org_arms.rs`: two new tests.
  - `organization-system.md` and the one `observability.md` row: main's text was kept and the ORG-08 segment added.
- **ORG-10 switch (coordinator decision, 2026-09-27).** ORG-10 landed first. This PR moves its `.org_set_perms` (`handlers/gm_perms.rs`) onto the ORG-08 path. See design decision 9. Branch `org/08-motd-notes-ranks`, worktree `.claude/worktrees/org-08`, test DB `sgw_org_08`.
- **Sentinels:** `0x7000_5800..=0x7000_59FF` (`Fixture::org08`, 32 blocks of 16, 19 used; names `Org08P<n>`, organizations `Org08 ...`), as the coordinator assigned in the final map: ORG-09 `0x7000_5600..=0x7000_56FF`, ORG-10 `0x7000_5A00..=0x7000_5BFF`. Self-picks of `5600` and then `5A00` were withdrawn.

## Owned paths

- New in `crates/base-session/src/base/organization/handlers/`:
  - `texts.rs`: CM 13-15 through `handle_set_text` and `TextEdit`.
  - `rank_editor.rs`: CM 16 and 17, plus `rank_permissions_locked`, `announce_perm_edit` and `PermEdit`.
  - `officer_notes.rs`: holders, `NoteSync`, and the syncs by rank and by member.
  - `order.rs`: `org_order_guard`.
  - `edit_row.rs`: `EditRow`, the outcome row.
- New tests in `handlers/tests/`: `texts.rs`, `rank_editor.rs`, `officer_notes.rs` and `org08_support.rs`. `tests/mod.rs` gains `Fixture::org08`.
- Edited in `handlers/`:
  - `mod.rs`: modules and re-exports.
  - `telemetry.rs`: `OrgReject::{OwnRank, ChangesUnheldBits, InvalidText}`. The `Leader`-row refusal reuses ORG-10's `LeaderRowPinned` (`leader_row_pinned`); ORG-08's own `LeaderPinned` was dropped in the rebase.
  - `gm_perms.rs` (ORG-10): `.org_set_perms` now calls `rank_permissions_locked` and `announce_perm_edit` under `org_order_guard`.
  - `answer.rs`: their lines, `invalid_text`, and the `not_available` doc.
  - `push.rs`: officer notes are blanked in the login push.
  - `rank.rs` and `gm.rs` (ORG-07): the order guard and the officer-note sync on a rank move. `gm_rank` resolves a missing org id before it takes the guard (`org_of_member`).
- Base world entry, `cell_dispatch/org_dispatch.rs`: arms for CM 13-17. `tests_dispatch_arms/org_arms.rs`: the unserved test now uses CM 10, and `texts_and_rank_editor_reach_their_handlers` is new.
- Docs: `docs/gameplay/organization-system.md` (status, the CM 13-17 rows, the new "MOTD, notes and the rank editor (ORG-08)" section), `docs/architecture/observability.md` (the `org` / `squad` row), `docs/protocol/client-method-dispatch-table.md` (the organization block note), and this worknote.

## Read set

- From `work-packets.md`: the ORG-08 packet, the contract, ORG-API, CAT-M rows M-07, M-08 and M-10, and the telemetry section.
- From the README: D-ORG08, 09, 10, 22 and 23.
- The worknotes for ORG-01 (`apply_edit`), ORG-02, ORG-06 and ORG-07.
- In `base/organization/`: `api.rs` ("Lock order", `OrgAccess`), `persistence/{texts.rs,loads.rs,error.rs,mod.rs}`, and `handlers/{rank.rs,gm.rs,targets.rs,broadcast.rs,push.rs,fanout.rs,answer.rs,telemetry.rs}` with its tests.
- Also: `entity/organization/{permissions.rs,org_text.rs,types.rs}`, the wire builders 45-50 and the CM 13-17 decoders, `org_dispatch.rs` and `org_arms.rs`.

## Design decisions

1. **Two handler files, one row type.** `texts.rs` holds CM 13-15 (one `handle_set_text` over `TextEdit`). `rank_editor.rs` holds CM 16-17. `EditRow` is its own struct, not new fields on ORG-07's `ActionRow`, so ORG-09 and ORG-10 edits to `telemetry.rs` stay small. The only `telemetry.rs` change is four `OrgReject` variants at the end.
2. **Check order.** Text rules run first, before the lock (the check is pure). Then membership, rank in type, the `Leader` row (CM 16), the bit, own rank, and strictly-below. D-ORG09 (2) applies to CM 16 and 17 as well as to officer notes: without it an Officer granted `AlterPerms` could strip `Eject` and `Demote` from the rank above (authority review). Own rank is reported as `own_rank` before `rank_too_low`, even though (2) implies it.
3. **Officer-note target.** `targets::member_by_name` (ORG-07) resolves the name among that organization's member rows only, inside the transaction. The name echoed in [47] is the stored one, not the case the client typed.
4. **The permission edit is one function.** `rank_permissions_locked(tx, &OrgAccess, rank, wire_mask, &mut EditRow) -> PermEdit`, followed by `announce_perm_edit(ctx, &PermEdit)`.
   - It reads the old mask under the lock, runs `apply_edit` with the actor's mask, and writes with ORG-02's `set_rank_permissions`, whose signature is unchanged. It `debug_assert`s that the mask `set_rank_permissions` returns equals the pre-read.
   - A GM `OrgAccess::system` (`Leader`, every bit) passes the authority checks but still meets the rank-in-type rule, the `Leader` pin and the `editable_for` clamp.
5. **Unchanged edits.** The same text or the same mask is `ok` with `after = unchanged`: no write and no fanout, but the line is still sent (the first-press rule).
6. **Officer-note visibility (CAT-M-10, reviewer M1-M3).**
   - [47] recipients are the holders read inside the edit's transaction, not taken from `broadcast_to_org(required)`, whose rank read is post-commit and unlocked.
   - The login push blanks every officer note for a recipient whose rank lacks `OfficerNotes` in the push's own rank read, or has no rank row (fail closed). Before this, every member received every officer note at login: an adjacent CAT-M-10 leak in ORG-06's push, fixed here.
   - A mask edit that moves `OfficerNotes` sends that rank's online members one [47] per stored note, after the [49]: the text on a grant, "" on a revoke. A rank change between ranks that differ in the bit does the same for the moved member, after the [40]. The data is computed under the lock and sent after the commit, from the shared ORG-07 path (`announce_rank`, used by 0xD2 and `.org_rank`).
7. **Order guard (`order.rs`).** This is an in-process async mutex per `org_id`, taken before `pool.begin()` and held until the last send. CM 13-17, the rank change and `.org_rank` hold it, so post-commit sends go out in commit order. A revoke's blank [47] and an officer note's text can no longer arrive in the wrong order.
   - **No deadlock.** The guard is always taken before the transaction, and nothing waits on it while holding a database lock.
   - **Map entries.** One entry per organization edited; entries are not reclaimed.
   - **ORG-10's `.org_set_perms` must take it too** (see below).
8. **The Leader cannot rename rank 8.** D-ORG09 (2) and (3) refuse it for everyone. Open question below.
9. **`.org_set_perms` on the shared path.** `gm_set_perms` takes `org_order_guard(org_id)` before its transaction. Inside it, `edit_locked` builds `OrgAccess::system`, works out ORG-10's `ignored` bits (`wire & !editable_for(type)`), and calls `rank_permissions_locked(tx, &access, rank, wire, &mut EditRow)`.
   - An unchanged result is still refused as `permissions_unchanged` (ORG-10's GM contract; a member's CM 16 gets `ok` / `unchanged` instead).
   - After the commit, `announce_perm_edit` sends the [49] from the table read under the lock, then the officer-note sync. ORG-10's post-commit `load_ranks` re-read and its `org.broadcast_failed` (`what = rank_update`) WARN are gone.
   - ORG-10's GM-facing behaviour is unchanged: the `org.gm_action` row, DEBUG `permissions_changed` with `ignored_bits`, the lines and the reasons. Its `gm_suite` tests pass as they were. `rank_permissions_locked`, `announce_perm_edit` and `EditRow` stay `pub(super)` in `handlers`, which is all `gm_perms.rs` needs.

## Advisor review

`server-authority-enforcer` (design, read-only).

**Applied:**

- M1: holders read under the lock, and the order guard.
- M2: the visibility sync on the shared rank-change path, and a reusable locked permission function.
- M3: fail-closed blanking in the push.
- S2 (partly): the no-op skips.
- S3: canonical names.
- S4: one `old`.

**Not applied, and why:**

- S1: the login push reads with no shared snapshot. That is ORG-06's code, recorded as a gap.
- S2: no rate limit on CM 13 and 14. The same text is a no-op, and every write needs a bit. Recorded as a gap.
- S5: an owner question.
- S6: rank-name impersonation, recorded as a gap.

## Telemetry added

| Event | Level | SigNoz filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|---|
| `org.set_text` (span and row: `field`, `from_units`, `to_units`, `target_player_id` / `target_account_id` / `target_rank` for an officer note, `actor_rank`, `after`) | INFO | `event = 'org.set_text' AND org_id = <id>` |
| `org.set_rank_permissions` (`rank`, `from_mask`, `to_mask`, `wire_mask`, `unheld_mask` on refusal) | INFO | `event = 'org.set_rank_permissions' AND org_id = <id>` |
| `org.set_rank_name` (`rank`, `from_units`, `to_units`) | INFO | `event = 'org.set_rank_name' AND org_id = <id>` |
| `rank_permissions_changed` (`from_mask`, `to_mask`, `officer_notes_sync`) | DEBUG | `event = 'rank_permissions_changed' AND org_id = <id>` |
| `org.broadcast` with `what = officer_note` (`holders`, `recipients`) | DEBUG | `event = 'org.broadcast' AND what = 'officer_note'` |
| `org.officer_note_sync` (`show`, `notes`, `members`, `recipients`) | DEBUG | `event = 'org.officer_note_sync' AND org_id = <id>` |
| `org.send_failed` (`what` = `officer_note` \| `officer_note_sync`), `org.action_failed` (`reason = db_error`) | WARN | `severity_text = 'WARN' AND event IN ('org.send_failed','org.action_failed')` |

Reasons: `not_member`, `missing_permission`, `rank_not_in_type`, `leader_row_pinned`, `own_rank`, `rank_too_low`, `changes_unheld_bits`, `target_not_member`, `target_ambiguous`, `self_target`, the `TextReject` reasons, `no_db` and `db_error`. The counter is `org_actions_total{action = set_text | set_rank_permissions | set_rank_name, outcome, reason}`. The persistence layer's own DEBUG `set_text` and `set_rank_permissions` rows (ORG-02) still fire underneath. The set is also in the `observability.md` row.

## Commands run

All run from the worktree root through the lane (`target=B:\targets/org-08`). Each live-DB run reloads `sgw_org_08`. No live-DB test self-skipped.

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo check -p cimmeria-base-session --all-targets`, `-p cimmeria-base-world-entry` | 0 | |
| `lane.sh cargo nextest run -p cimmeria-base-world-entry -p cimmeria-base-session --lib --no-fail-fast` (pre-rebase) | 0 | 719 passed |
| `live-db-test.sh organization:: org_arms --no-fail-fast` (pre-rebase) | 0 | 249 passed |
| `live-db-test.sh edits_wait_for_the_org_order_guard` | 0 | 1 passed |
| `python org08_mutations.py` (the 15 proofs below) | 0 | every guard failed, exit 100; `git status` clean of code afterwards |
| `git rebase origin/main` (onto `3684fa7eb`) | 0 | no conflicts |
| `git rebase origin/main` (onto `1f1a019d8`, ORG-09 #951) | 1, then 0 | conflicts in module lists and docs, kept both sides |
| `lane.sh cargo fmt --all -- --check` | 0 | |
| `lane.sh cargo clippy -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-base -p cimmeria-services --all-targets -- -D warnings` | 0 | |
| `lane.sh cargo nextest run -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-base -p cimmeria-cell-methods -p cimmeria-wire --lib --no-fail-fast` | 0 | 1518 passed |
| `live-db-test.sh "::"` (the whole tier, after the first rebase) | 0 | 5246 passed, 0 skipped |
| after the second rebase: `fmt --check`, the same clippy, `nextest` (adds `-p cimmeria-cell-console`) | 0, 0, 0 | 1908 passed |
| `live-db-test.sh "::"` (after the second rebase) | 0 | 5255 passed, 0 skipped |
| squash, then `git rebase origin/main` (onto `be3e77228`, ORG-10 #952) | 1, then 0 | the conflicts in "Base" above, resolved by hand; `cargo check` before continuing |
| after the third rebase and the `.org_set_perms` switch: `fmt --check`, `clippy -p` base-session, base-world-entry, base, cell-console, services `--all-targets -D warnings` | 0, 0 | |
| `lane.sh cargo nextest run` (base-session, base-world-entry, base, cell-methods, wire, cell-console) `--lib` | 0 | 1927 passed |
| `live-db-test.sh "::"` (after the third rebase) | 0 | 5274 passed, 0 skipped |

## Regression proof

The script is `org08_mutations.py` in the scratchpad. It applies each mutation alone to the committed tree and runs its guard, either `live-db-test.sh <tests> --no-fail-fast` or `lane.sh cargo nextest run -p <crate> --lib`. It then restores with `git checkout HEAD -- <files>` and `touch`.

| Id | Mutation | Exit | Failed |
|---|---|---|---|
| M1 | texts: the per-method bit check disabled | 100 | `motd_rejects_without_perm`, `note_updates_own_note_and_fans_out` |
| M2 | `member_by_name`: the organization filter dropped | 100 | `officer_note_rejects_target_outside_org` |
| M3 | officer note: the actor-above-target check removed | 100 | `officer_note_rejects_higher_rank_target` |
| M4 | [47] to every member (the holder filter removed) | 100 | `officer_note_fanout_filtered_by_permission` |
| M5 | text rules skipped in both handlers and in `set_text` | 100 | `text_rejects_bidi_and_zero_width`, `set_rank_name_rejects_over_cap` |
| M6 | `apply_edit` given `ALL` instead of the actor's mask | 100 | `set_perms_rejects_grant_of_unheld_bit` |
| M7 | own-rank refusal and rank comparison removed | 100 | `set_perms_rejects_own_rank`, `set_perms_needs_alter_perms_a_lower_rank_and_a_rank_in_type` |
| M8 | the `Leader`-row refusal removed | 100 | `set_perms_rejects_leader_row` |
| M9 | CM 17 without `RankNames` | 100 | `set_rank_name_rejects_without_perm` |
| M10 | login push sends officer notes to every rank | 100 | `login_push_hides_officer_notes_without_the_permission` (unit) |
| M11 | mask edit: no officer-note sync | 100 | `revoking_officer_notes_blanks_them_for_that_rank` |
| M12 | rank change: no officer-note sync | 100 | `rank_change_out_of_officer_notes_blanks_them` |
| M13 | order guard not taken by CM 15, CM 16 and the rank change | 100 | `edits_wait_for_the_org_order_guard` |
| M14 | dispatch: CM 13 falls through to "not available yet" | 100 | `texts_and_rank_editor_reach_their_handlers` (unit, base-world-entry) |
| M15 | an unchanged text is written and fanned out again | 100 | `motd_updates_and_fans_out` |
| M16 | `.org_set_perms` back to ORG-10's fanout ([49] only, no `announce_perm_edit`) | 100 | `gm_set_perms_moving_officer_notes_sends_the_note_sync` (ORG-10's `gm_set_perms_clamps_to_the_editor_bits_and_fans_out` still passed) |
| M17 | `.org_set_perms` without the order guard | 100 | `edits_wait_for_the_org_order_guard` |

The mutations ran with the sentinel base at `0x7000_5600`. The later moves (to `0x7000_5A00`, then `0x7000_5800`) change no logic, and the whole tier ran green after each.

## Test catalogue

- **`base-session` `handlers::tests` (live-DB, types 3, 8 and 12):**
  - **`texts.rs`:**
    - The CAT-M-10 guards: `motd_rejects_without_perm`, `officer_note_rejects_target_outside_org`, `officer_note_rejects_higher_rank_target`, `text_rejects_bidi_and_zero_width`.
    - Also: `motd_updates_and_fans_out`, `note_updates_own_note_and_fans_out`, `officer_note_fanout_filtered_by_permission`.
  - **`rank_editor.rs`:**
    - The CAT-M-07 guards: `set_perms_rejects_grant_of_unheld_bit`, `set_perms_rejects_own_rank`, `set_perms_rejects_leader_row`.
    - The CAT-M-08 guards: `set_rank_name_rejects_without_perm`, `set_rank_name_rejects_over_cap`.
    - Also: `set_perms_needs_alter_perms_a_lower_rank_and_a_rank_in_type`, `set_perms_updates_and_fans_out`, `set_rank_name_updates_and_fans_out`.
  - **`officer_notes.rs`:** `gm_set_perms_moving_officer_notes_sends_the_note_sync` (the ORG-10 switch), `revoking_officer_notes_blanks_them_for_that_rank`, `rank_change_out_of_officer_notes_blanks_them`, `an_unsendable_officer_note_sync_warns_with_reason` (type 12), `edits_wait_for_the_org_order_guard` (which also covers `.org_set_perms`), and `login_push_hides_officer_notes_without_the_permission` (unit).
- **`base-session` `handlers::order::tests`:** `one_holder_per_organization` (unit).
- **`base-world-entry` `org_arms.rs`:** `texts_and_rank_editor_reach_their_handlers` (new) and `unserved_calls_from_a_live_session_are_answered` (now CM 10).

Every CAT-M name for M-07, M-08 and M-10 exists as written.

## Known gaps

- **No real-client run.** Three things are untested on a client:
  - Whether the client's editor sends CM 16 with the full editable mask. The server assumes it does; D-ORG22 keeps the hidden bits either way.
  - How the client applies [49] and [50]. The server sends the whole table, which is safe whether the client replaces or merges.
  - Whether an empty [47] clears a note's display.

  ORG-UAT step 8 watches all three.
- **Login-push snapshot (reviewer S1).** `push_org_state` reads memberships, ranks and roster as three separate pool reads. A revoke that commits between them can let one push show notes from a rank read taken just before it. The order guard does not cover the push. The fix would be one `FOR SHARE` or REPEATABLE READ read in ORG-06's push.
- **No rate limit on CM 13 and 14 (reviewer S2).** Every write needs a bit, and an unchanged text is a no-op, but a member can still loop distinct notes: one transaction and one [46] fanout per call.
- **Rank-name impersonation (reviewer S6).** A Senior Officer can rename Member to "Leader". This was not requested and needs a policy decision.
- **The Leader cannot rename rank 8**, although the client's Command editor offers it (worknote org-01). The refusal has feedback.
- **A kicked or left member keeps officer notes** on their client until they close the window. Nothing is sent to un-show them; the membership itself is gone.
- **Order-guard map entries are never reclaimed** (a few bytes per edited organization).

## Integration edits for the coordinator

- **ORG-10 (`.org_set_perms`):** done in this PR (design decision 9). A future permission editor must go through `rank_permissions_locked` + `announce_perm_edit` under `org_order_guard`, never `persistence::set_rank_permissions` directly.
- **Ledger:**
  - Record the sentinel range `0x7000_5800..=0x7000_59FF` (ORG-09 `5600`, ORG-10 `5A00`, per the coordinator's map).
  - Record that ORG-10's `.org_set_perms` now runs on the ORG-08 path. `docs/guides/organizations-uat.md` step 8's log rows were corrected to the events that exist (`set_text` and `set_rank_permissions` from persistence, `rank_permissions_changed`, `org.officer_note_sync`); it had named `text_changed`, which is never logged.
  - Mark ORG-08's fanout as delivered.
  - The officer-note push fix touches ORG-06's `push.rs`.
  - ORG-07's `rank.rs` and `gm.rs` now take the order guard and send the officer-note sync.
- **ORG-09.** Officer chat must read `OfficerChat` under the lock (org-07's note). Nothing in ORG-08 caches permissions.
- **Contended files touched:** `org_dispatch.rs` (five new arms before the strike-team arm) and `handlers/telemetry.rs` (four variants at the end of `OrgReject`). No `OrgCellToBase` or `OrgBaseToCell` variants were added.
