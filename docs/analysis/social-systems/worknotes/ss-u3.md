# SS-U3 Worknotes

> Type: reference. Audience: social-systems coordinator; the crafting coordinator (placement check).
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [ss-u1.md](ss-u1.md), [ss-u2.md](ss-u2.md).

## Contract

> **Update 2026-09-27: dialog 60104 is quarantined.** Its cooked-data override
> moved from `DIALOG_OVERRIDES` to `QUARANTINED_DIALOG_OVERRIDES` and is not
> served. The debug-hub dialogs, pushed as overrides, coincide with a client
> crash on entry into Castle_CellBlock, and the bad field is not yet known.
> The clerk, chains 7010-7011 and the seed rows stay, but the clerk shows no
> dialog. `.mail` is the mail test meanwhile. The override tests named below
> now check the quarantined definition and that it is not served. See
> [debug-hub.md](../../../content/debug-hub.md#dialog-npc-template-302-airman-lance).

- **Packet:** SS-U3, hub additions and UAT docs: the `send_system_mail` content action, the Gate Mail Clerk (template 390, spawn 490), the debug-hub doc, and a runnable SS-UAT.
- **Decisions in force:** D-SS03 (server mail ignores the cap), D-SS10 (no `sender_id` means not returnable), the owner's plain-English preference, "every button press gets visible feedback on the first press", telemetry first-class. Coordinator changes to the worker rules: no Copilot reviews; no edits to `gap-analysis.md`, `project-status.md`, test counts or the crate graph.
- **Base:** written on `origin/main` @ `91919e909` (mail M1-M3, SS-U1 `send_system_mail`, chat C1-C3, duels D1-D3, SS-U2 sparbot), then rebased onto `1c6bed3cc` (BV-04 #931 debug-hub Banker, ORG-04 #922, PT-13 #930). Rebase conflicts were additive (a `CellToBaseMsg` variant and its dispatch arm, the hub seed rows, the hub count in `debug_hub_dispatch_tests`, `debug-hub.md`), except the clerk's position, which moved past the Banker (below). Branch `social/u3-hub-uat-docs`, worktree `.claude/worktrees/ss-u3`.
- **Owned paths (new):**
  - `crates/wire/src/cell/messages/content_mail_cell_to_base.rs`: `ContentSystemMail`, `ContentMailCooldown`
  - `crates/content-engine/src/loader/action_mail.rs`, `loader/tests/send_system_mail_conversion.rs`
  - `crates/cell-content/src/cell/content/executor/mail.rs`, `executor/tests/mail.rs`, `chain_replay_tests/debug_hub_mail_clerk.rs`
  - `crates/base-methods/src/base/world_entry/methods/mail/content.rs`, `mail/tests/content_live.rs`
  - `crates/cell-catalog/src/cell/spawner/tests/live_db_mail_clerk.rs`
  - `db/sgw/Players/Tables/sgw_player_content_cooldown.sql`
  - this file
- **Edited:** `CellToBaseMsg` (one variant), `Action` (one variant), `loader/action.rs` (one arm), `executor/mod.rs` (one arm), base `cell_dispatch/mod.rs` (one arm), `mail/mod.rs` (`mod content`), `mail/tests/{mod.rs, packets.rs}` (`Client::content`), `db/database.sql` and `db/sgw/_foreign_keys.sql` (the new table), the seeds (`entity_templates.sql`, `spawnlist.sql`, `dialogs.sql`, `dialog_screens.sql`, `dialog_screen_buttons.sql`, `debug_hub_chains.sql`), `DIALOG_OVERRIDES`, the override agreement test and the regenerated-id pin in `resources`, `interact_tag_linter.rs` (allowlist 7010), `live_db_debug_hub.rs` (placement), `debug_hub_dispatch_tests.rs` (click the clerk). Docs: `docs/content/debug-hub.md`, `docs/content/content-engine.md`, `docs/guides/extend-the-content-engine.md`, `docs/gameplay/mail-system.md`, `docs/architecture/observability.md`.
- **Read set:** `SS-WORKER-RULES.md`; work-packets (SS-U3, SS-UAT, contended files); `ss-u1.md`; `docs/content/debug-hub.md`; `docs/content/content-engine.md` §3 and §9; `.github/instructions/content-chains.instructions.md`; `docs/guides/extend-the-content-engine.md`; `docs/architecture/wireclient.md` (sparbot); `origin/craft/cr11-debug-hub-stations` (debug-hub.md "Crafting corner", `live_db_crafting_hub.rs`, `live_db_seed_sequences.rs`, the seed footers); `mail/{mod.rs, gm.rs, system/mod.rs}`; the executor, loader and chain-replay modules; `base/dispatch/tell.rs` (self-tell refusal); `docs/commands.md`.

## Design decisions

- **The base does the work.** `cell-content` cannot depend on `base-methods` (the base track sits above the cell track), and the writer and the cooldown table are base-side. So the executor sends one `CellToBaseMsg::ContentSystemMail` and the base runs one transaction. This is the `GrantItem` / `StartMinigame` shape, not a round trip: nothing later in the chain waits on the result.
- **A durable cooldown table, not a check against `sgw_gate_mail`.** A player can take the attachments and delete the mail (the delete guard allows it once cash and item are gone), so "no clerk mail in the last 10 minutes" would reset every time. An in-memory map would reset on a server restart. `sgw_player_content_cooldown (player_id, cooldown_key, last_used_at)` with `ON DELETE CASCADE` from `sgw_player` holds across all three. It is a schema addition in `db/sgw/`, not a `db/scripts/` migration; the colo database is rebuilt from `db/database.sql` on deploy (as for SS-M3's `returned` / `cod_paid` columns). **Owner-visible decision:** a new `sgw` table. If the owner prefers none, the fallback is a check on `sgw_gate_mail` with the delete loophole documented.
- **Lock order and atomicity.** The recipient's `sgw_player` row `FOR UPDATE`, then the claim (`INSERT ... ON CONFLICT DO UPDATE ... WHERE last_used_at <= now - secs`, `rows_affected == 1`), then `send_system_mail_tx` (which re-locks the same row), then commit. The claim and the mail commit together, so a refused mail (unknown item, stack too big) leaves the old claim, and two simultaneous presses write one mail.
- **Key and scope.** The key is `send_system_mail/<chain_id>`, derived by the executor, so each authored mail has its own window and an author cannot share one by accident. The cooldown is per character (mail is per character); an account with several characters gets one mail per character.
- **Feedback on every press.** The base answers each firing with one `SYSTEM` feedback line: "Gate Mail Clerk sent you mail 12 with 50 naquadah and 5 x Health Slappack TC1. Open your mail to take them.", or "Gate Mail Clerk has already sent you mail. You can ask again in 7 minutes." (minutes rounded up), or "could not send your mail (...)". No `onNewMail` yet (SS-M4), so the line is the only live signal.
- **Loader rejects, never defaults.** `sender` and `subject` are required one-line 1-128 characters, `body` at most 1,000, `cash` 0 to `i32::MAX`, `qty >= 1` and only with `item_id`, `cooldown_secs >= 1`. A bad row is dropped with a warn naming the chain, the `npc_bark` rule: otherwise every press would be refused for an authoring mistake.
- **Clerk identity.** No shipped moniker says "Mail Clerk" and new `texts.sql` ids never render, so the clerk shows "Sgt. Harriman" (26715) with template 58's Walter Harriman look and speaker 843. The dialog text and the mail's sender say "Gate Mail Clerk". Talk cursor `INT_NonAStoryMissionAvaliable` (134217728), the hub dialog NPC's bit; without a bit the client never sends the click.
- **Contents.** 5 x item 2893 Health Slappack TC1 (max stack 10, so a take can merge with the crate's drops) and 50 naquadah. The packet said "a stack"; 5 is a half stack, chosen so both a fresh slot and a merge are testable. Deviation from plain intent: none beyond that choice.
- **Ids.** Template 390 and spawn 490 (391-399 / 491-499 stay reserved). Chains 7010-7011 in the hub's 7001-7099 range; dialog 60104, screen 200005, button row 200001 (the next free ids after the hub's; no in-flight branch uses them, checked across every unmerged remote branch). The seed footers for `dialogs`, `dialog_screens` and `dialog_screen_buttons` were raised to the new maxima. The `spawnlist` footer is already MAX-based on `main` and was not touched. The `entity_templates` footer on `main` is still the fixed `304`; CR-11 (#909) replaces it with the MAX-based form with a 399 floor, and this branch does not touch it, so the merge keeps CR-11's line.

## Clerk placement (for the crafting coordinator)

Spawn 490 at **(-324.11, 73.472, -227.84)**, heading **-1.5123** (faces the room centre (-333.03, -227.32), yaw = atan2(dx, dz)).

Room: point set 2032 `Castle_Cellblock.Region1`, corners A(-347.17, -230.14) B(-327.67, -240.70) C(-318.89, -224.51) D(-338.39, -213.94). The hub line runs 3 units in from the A-B wall at 3-unit spacing: 400, 401, 402, (empty), 403, 404, 450. The only open slot on it is the one at (-335.99, -232.78), 5.07 units from the respawner, which the hub leaves empty on purpose ("the nearest NPC is 5.5 units from where a new character appears"). The pet trainer took the last slot before B, CR-11's crafting corner takes the D-A wall, and BV-04's Banker (spawn 470, merged as #931 while this packet was in flight) the middle of the B-C wall at 9.2 units along it. So the clerk stands on the B-C wall past the Banker, towards C:

| Measure | Value |
|---|---|
| In from the B-C wall | 3.0 |
| Along the B-C wall from B | 13.0 (the wall is 18.4 long) |
| To the Banker (470) | 3.80 |
| To the C-D exit wall | 5.42 |
| To the pet trainer (450) | 10.05 |
| To the respawner | 10.12 |
| To the nearest CR-11 spawn | 412 at (-337.96, -219.44), 16.2 units away |

A first draft stood at 6.5 units along the B-C wall; after rebasing onto BV-04 that was 2.7 units from the Banker (the 2.5 guard passes, but the bodies nearly touch), so it moved past the Banker.

The extended guard (`debug_hub_spawns_sit_inside_the_stasis_room`) now checks each hub spawn against every spawn inside Region1, so CR-11's spawns are covered on merge. CR-11's own guard (`crafting_hub_spawns_sit_inside_the_stasis_room`) and BV-04's `live_db_debug_banker.rs` already check against every in-room spawn, so they cover 490 too. Alternatives: the empty A-B slot passes the guard (5.07 >= 5.0) but crowds the wake-up spot; between the pet trainer and the Banker (6.1 along B-C) leaves only about 3.1 units to each.

As with the rest of the hub, the position is derived, not checked in the client (no navmesh or occluder data for this room). UAT: stand at the respawner and confirm the clerk stands on the floor, clear of the wall and the pods.

## Telemetry (debuggable from SigNoz alone)

| Event | Target | Level | Fields |
|---|---|---|---|
| `content.send_system_mail` `outcome=requested` | `content` | INFO (cell) | `entity_id`, `account_id`, `player_id`, `chain_id`, `sender_name`, `cash`, `type_id`, `quantity`, `cooldown_secs` |
| `content.send_system_mail` `outcome=sent` | `content` | INFO (base, after commit) | the ids, `mail_id`, `item_id`, `type_id`, `stack_size`, `cash`, `cooldown_key`, `cooldown_secs` |
| `mail.system_sent` | `mail` | INFO (writer) | SS-U1's row: `target_player_id`, `mail_id`, `item_id`, `recipient_open_mail`, `over_cap` |
| `content.send_system_mail` `reason=...` | `content` | WARN | the ids, `chain_id`, `reason` (`cooldown` \| `no_player` \| `base_channel_closed` \| `no_db_pool` \| a `mail.system_refused` reason), `cooldown_key`, `last_used_at`, `remaining_secs`, `error` |

No new target (`content` ships at INFO; the one DEBUG line, a feedback line dropped because the player logged out, is on `mail`). The target-scan guard passes.

SigNoz (Logs):

- every clerk press by a player: `attributes.event = 'content.send_system_mail' AND attributes.player_id = <id>`;
- presses refused by the cooldown: `... AND attributes.reason = 'cooldown'`, with `remaining_secs`;
- the mail a press produced: take `mail_id` from the `outcome = 'sent'` row, then `scope_name = 'mail' AND attributes.mail_id = <id>` for `mail.system_sent`, `mail.cash_taken`, `mail.item_taken`.

## Tests

- **Loader (unit):** `convert_send_system_mail_full_row`, `convert_send_system_mail_defaults`, `convert_send_system_mail_rejects_bad_params` (14 shapes).
- **Executor, type 12:** `send_system_mail_refuses_a_non_player_entity`, `send_system_mail_warns_when_cell_to_base_channel_closed`.
- **Chain replay, type 6 (live DB):** `mail_clerk_click_opens_dialog_60104_as_the_clerk`, `mail_clerk_button_sends_exactly_one_mail` (the packet's replay test: exactly one `ContentSystemMail`, full struct equality, and the trigger negative is proven non-vacuous).
- **Base, live DB (type 3, 5, 12):** `content_mail_sends_once_then_refuses_inside_the_cooldown` (the packet's cooldown test: one mail and escrow row, the feedback lines, `mail.system_sent`, the WARN with `reason=cooldown` and the ids), `content_mail_cooldown_outlives_the_mail_and_expires_on_time` (delete the mail, 599 s refused with 1 s left, 600 s sends), `content_mail_claim_rolls_back_with_a_refused_mail`, `content_mail_concurrent_presses_write_one_mail`, `content_mail_refusals_answer_the_player` (missing player, no pool), `content_mail_wait_text_rounds_up`.
- **Seed guards (live DB):** `debug_hub_spawns_sit_inside_the_stasis_room` (extended), `mail_clerk_template_carries_its_role_fields`, `mail_clerk_dialog_has_one_screen_and_one_button`, `debug_hub_npcs_answer_a_click_with_their_own_interaction` (clicks the clerk; the hub count is now 7).
- **Non-DB seed/override:** the debug-hub override agreement tests cover 60104; `apply_dialog_overrides_regenerates_existing_and_inserts_new` pins 60104 and its button.

Sentinels: accounts and players `0x7300_5300`-`0x7300_5331`, entities `0x7300_5390`-`0x7300_5394`, `0x7300_53FE` (a missing player), `0x7300_53FF` (a missing item type). Cleanup by exact account id; the cooldown rows cascade with the player.

## Commands run

All from the worktree root, through the lane.

| Command | Result |
|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-cell-content -p cimmeria-cell-catalog -p cimmeria-resources -p cimmeria-content-engine -p cimmeria-base-methods -p cimmeria-base-world-entry --all-targets` | exit 0 |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-content-engine -p cimmeria-cell-content -p cimmeria-cell-catalog -p cimmeria-cell-methods -p cimmeria-resources -p cimmeria-base-methods -p cimmeria-base-world-entry --all-targets -- -D warnings` | exit 0 |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | exit 0 |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-content-engine -p cimmeria-resources -p cimmeria-wire` | 635 passed (first run 1 failed: the regenerated-id pin, fixed) |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-cell-content -p cimmeria-cell-catalog -p cimmeria-base-methods -p cimmeria-base-world-entry` | 1,365 passed, 0 skipped |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-server logging` | 55 passed, 5 skipped (the crate's ignored tests); first run failed on a DEBUG row on `content`, moved to `mail` |
| `bash tools/build-lane/live-db-test.sh ::` (reloads `sgw_ss_u3`) | 4,841 run, 4,841 passed, 0 skipped |
| `bash tools/build-lane/live-db-test.sh content_mail mail_clerk debug_hub pet_trainer spawnlist_sequence` | 32 run; the first run failed the 4 hub dispatch tests (count of hub NPCs, extended to the clerk), then 22/22 on the rerun of `debug_hub mail_clerk` |
| `npx markdownlint-cli2` on the touched docs | no finding on the lines this packet touched |
| After the rebase onto `1c6bed3cc`: `cargo fmt --all -- --check`; clippy on the 8 crates above | exit 0; exit 0 |
| After the rebase: `cargo nextest run -p cimmeria-wire -p cimmeria-content-engine -p cimmeria-cell-content -p cimmeria-cell-catalog -p cimmeria-cell-methods -p cimmeria-resources -p cimmeria-base-methods -p cimmeria-base-world-entry` | 2,344 passed, 0 skipped |
| After the rebase: `bash tools/build-lane/live-db-test.sh ::` | 4,891 run, 4,891 passed, 0 skipped |
| After the rebase: `cargo nextest run -p cimmeria-server logging` | 55 passed, 5 skipped |

## Regression proof

Committed first (`0046a6f61`, `f4855a05f`, `1bf896fcd`, the shas before the rebase; the proofs ran with the clerk at its first position), then each mutation applied by script, the named guard run through the lane (seed mutations through `live-db-test.sh`, which reloads the database), the file restored with `git checkout HEAD -- <file>` and touched. `git status` clean after the last restore.

| Mutation | Guard | Result |
|---|---|---|
| Spawn 490 moved onto the pet trainer | `debug_hub_spawns_sit_inside_the_stasis_room` | FAILED |
| Same seed, with the pre-SS-U3 guard file | `debug_hub_spawns_sit_inside_the_stasis_room` | passed (so the extension is what catches it) |
| Chain 7011's `send_system_mail` row deleted | `mail_clerk_button_sends_exactly_one_mail` | FAILED |
| Executor arm removed (falls to the catch-all) | `mail_clerk_button_sends_exactly_one_mail` | FAILED |
| Template 390 `interaction_type` 0 | `mail_clerk_template_carries_its_role_fields` | FAILED |
| Claim upsert without its window `WHERE` | `content_mail_sends_once_then_refuses_inside_the_cooldown`, `..._outlives_the_mail_...`, `..._concurrent_presses_...` | FAILED (3) |
| Window boundary `<=` to `<` | `content_mail_cooldown_outlives_the_mail_and_expires_on_time` | FAILED |
| Claim committed in its own transaction before the mail | `content_mail_claim_rolls_back_with_a_refused_mail` | FAILED |
| Cell `no_player` check off | `send_system_mail_refuses_a_non_player_entity` | FAILED |

## Known gaps

1. **No `onNewMail`** (SS-M4): an online player learns of the mail from the feedback line, and sees it when the mailbox is next opened.
2. **Not checked in the client:** the clerk's position, the Harriman composite in the stasis room, and that dialog 60104 draws its button (the override is generated the same way as 60100's).
3. **One cooldown per chain, not per NPC or per item.** Two chains that should share a limit need a `cooldown_key` param, which this packet did not add (no second user yet).
4. **SS-UAT step 6 cannot run** until SS-M4: `.mail_expire` is still a refusal.

## Out-of-scope findings

- `.bug <note>` (used at the top of SS-UAT) exists (`cell-console` `registry/commands/meta.rs`) but has no row in `docs/commands.md`.
- `entity_templates_template_id_seq` on `main` is still `setval(..., 304)` although templates 350-360 exist; CR-11 fixes it.

## Close-out edits for SS-99

This packet did not edit `docs/gap-analysis.md`, `docs/project-status.md`, test counts or the crate graph.

- `gap-analysis.md` §24 (Mail): add "content-engine mail action with a per-player cooldown (`send_system_mail`, SS-U3); the debug hub's Gate Mail Clerk uses it".
- `project-status.md` Mail row: add "hub Gate Mail Clerk (SS-U3)".
- Test inventory: +15 tests (3 loader, 2 executor, 2 chain replay, 6 base live-DB, 2 clerk seed guards), plus assertions added to 4 existing tests; below the 5% threshold.
- Crate graph: no new dependency edge.
- Schema: one new `sgw` table, `sgw_player_content_cooldown`.

## Integration edits for the coordinator

1. **Placement:** confirm spawn 490 with the crafting coordinator (cimmeria-23); coordinates and reasoning above. If CR-11 merges first, rebase: `entity_templates.sql` and `spawnlist.sql` conflict only in adjacent inserts; keep CR-11's MAX-based footers.
2. **SS-UAT in `work-packets.md`** is runnable as written, with these edits:
   - Add a "Before you start" line: two accounts with a character each in the Castle_CellBlock stasis room (world 12, where new characters wake up); account A needs GM rights for steps 2-4 solo, 6 and 10; a third account C only for step 11.
   - Step 3, solo: "the Gate Mail Clerk (Sgt. Harriman, stasis room, right-hand wall past the pet trainer): Send me a mail, then take the 50 naquadah and 5 slappacks. Press again: you are told to wait about 10 minutes."
   - Step 6: mark "needs SS-M4" (`.mail_expire` is refused until then).
   - Step 7, solo: "a tell to your own name is refused, a tell to an offline name tells you so".
   - Step 11, solo: sparbot needs a second account and, on the colo, the colo's `--auth-url` ([wireclient.md](../../../architecture/wireclient.md#sparbot-a-duel-partner-for-solo-testing)).
   - Step 12: the Gate Mail Clerk is a handy NPC for the D-SS25 right-click check.
   - Top: link `.bug` to its row once `commands.md` has one (finding above).
   - Add SigNoz queries per line, starting with the mail ones in "Telemetry" above.
3. `work-packets.md` SS-U3 section: acceptance tests are the ones listed above; the scope grew by one `sgw` table.
