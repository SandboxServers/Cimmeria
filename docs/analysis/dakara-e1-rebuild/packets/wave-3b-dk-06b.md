# Dakara_E1 wave 3b: DK-06b, the radio dialog verb

> Type: how-to (work packet). Audience: the packet-coder doing DK-06b, and the wave-3b packet-reviewer (read-only; a packet-coder applies its findings).
> Written 2026-10-18 against `main` after #1876. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md). Wave 3b's other packet, DK-10d-B, is in [wave-3.md](wave-3.md#dk-10d-b-1570-step-4903-bratacs-turn-in).

DK-06b was planned for wave 2 and waited on D-DK10. The architect ruled D-DK10 on 2026-10-18 (option B, server-only), so it runs now as wave 3b. This packet carries its own rules; you do not need CLAUDE.md.

## Rules (the wave-2 rules)


1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads your own `sgw_<worktree>` database. Run lane checks in the foreground.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only the files your packet names. Files stay under 700 lines (soft cap 500). Match the surrounding code: comment density, `foo/mod.rs` module style, doc comments on items.
5. Markdown under `docs/` is CRLF; the seed files named here are CRLF too. Keep the line endings of every file you edit: check with `[IO.File]::ReadAllText(<abs>).Contains("`r`n")`, and convert a new file with `[IO.File]::WriteAllText(<abs>, ([IO.File]::ReadAllText(<abs>) -replace "(?<!\r)\n", "`r`n"))`.
6. Seeds change in `db/resources/` only. Never add a `db/scripts/` migration. Every seed row carries an evidence-label comment (`ORIGINAL_DATA`, `PROJECT_FINAL_RECONSTRUCTION`, `RECONSTRUCTION`, `NEW CONTENT`); every spawn sets `respawn_secs`.
7. No new dialog row, no client patch, no cooked-data override (the one exception is DK-10fs's single `MISSION_ADDITIONS` entry, D-DK1 F). Never edit templates 54 or 59 (shared with Harset).
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` (done, left, next step, last `status=` line) and return `blocked` with the reason `context`.

## DK-06b: radio display, the `display_radio_dialog` verb

| Field | Value |
|---|---|
| Role | `packet-coder` (every line of code is given below); escalation `rust-gameserver-dev` |
| Branch / worktree | `dakara/dk-06b-radio-display` / `dk-06b` |
| Commit subject | `feat(content-engine): display_radio_dialog shows a dialog with no speaker entity (DK-06b)` |
| Report | `<worktree>\.claude\reports\dakara\DK-06b-packet-coder.md` |
| Size | About 30k tokens |
| Gate | None. D-DK10 is ruled (architect, 2026-10-18). Dispatch at once; it runs beside the wave-3 review (wave 3b with DK-10d-B, [wave-3.md](wave-3.md#dk-10d-b-1570-step-4903-bratacs-turn-in)). |
| Merge dependency | None. DK-10d-A needs this packet merged. |

**Why.** Step 4905's radio calls, dialogs 5815 (Tau'ri) and 5816 (Jaffa), carry General Hammond's lines (speaker 2609) and no NPC stands near the player. `display_dialog` needs a speaker entity: the monologue set, then `params.target_entity_id`, then the player's `last_interaction_target`, and otherwise a WARN and nothing sent. After a click the pin is the last thing clicked (here an SG-18 remains), so the call would be bound to a corpse. DK-06a ([finding](../../../reverse-engineering/findings/dialog-display-without-npc.md)) found that the client opens the window and shows every screen's text for any `EntityId`, 0 included; the id only chooses the portrait. D-DK10 therefore rules option B: a new verb that always sends `EntityId` 0. It is a new verb, not a `params` flag on `display_dialog`, because the loader drops a `display_dialog` row's params (`crates/content-engine/src/loader/action.rs`, the `"display_dialog"` arm), so a flag would never reach the executor.

**Contract.**

- Seed verb `display_radio_dialog`: `target_id` = the dialog id, `target_key` NULL, params `{}`. A row with a NULL `target_id` is dropped by the loader, as for `display_dialog`.
- Variant `Action::DisplayRadioDialog { dialog_id: i32 }`.
- The executor sends `onDialogDisplay` (client method 105) with `EntityId` 0 through `crate::cell::interactions::send_dialog_display`. That records the dialog as offered, so the #479 gate accepts its close (`dialog_choice`, button -1). It never reads `params` or `last_interaction_target`, never warns about a missing NPC, and keeps `display`'s telemetry (friction watch, player journal, the info line).
- No seed row, no Dakara chain: DK-10d-A uses the verb for step 4905.

**Files.** Edit only these.

| # | File | Change |
|---|---|---|
| 1 | `crates/content-engine/src/actions.rs` | the variant, directly after `DisplayDialog { dialog_id: i32 },` (line 188) |
| 2 | `crates/content-engine/src/loader/action.rs` | the loader arm, directly after the `"display_dialog"` arm (line 20) |
| 3 | `crates/content-engine/src/loader/tests/radio_dialog_conversion.rs` (new) and `loader/tests/mod.rs` | the loader tests; `mod radio_dialog_conversion;` between `npc_bark_conversion` and `send_system_mail_conversion` |
| 4 | `crates/cell-content/src/cell/content/executor/dialog/mod.rs` | move `display`'s tail into `emit`; add `display_radio` |
| 5 | `crates/cell-content/src/cell/content/executor/dispatch.rs` | the dispatch arm, directly after the `Action::DisplayDialog { dialog_id } \| Action::StartDialog { .. }` arm |
| 6 | `crates/cell-content/src/cell/content/executor/tests/radio_dialog.rs` (new) and `executor/tests/mod.rs` | the executor tests; `mod radio_dialog;` between `pets` and `spawn_set`, and one bullet in the `//!` list after `pets` |
| 7 | `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/mod.rs` | one `labels_in` arm, so DK-10d-A's tests read `display_radio_dialog(5816)` instead of `OTHER(..)` |
| 8 | `docs/content/content-engine-vocabulary.md` (CRLF) | one table row and one subsection |

Line counts after the change stay under 700: `actions.rs` about 651, `dispatch.rs` about 623, `dialog/mod.rs` about 595. Do not split any of them.

**1. `actions.rs`**, after `DisplayDialog { dialog_id: i32 },`:

```rust
    /// Display a dialog with no speaker entity: `onDialogDisplay` carries
    /// `EntityId` 0, so the window opens without a portrait. For radio
    /// calls and other remote voices with no NPC near the player (D-DK10,
    /// `docs/reverse-engineering/findings/dialog-display-without-npc.md`).
    DisplayRadioDialog { dialog_id: i32 },
```

**2. `loader/action.rs`**, after the `"display_dialog"` arm:

```rust
        "display_radio_dialog" => Some(Action::DisplayRadioDialog {
            dialog_id: row.target_id?,
        }),
```

**3. `loader/tests/radio_dialog_conversion.rs`** (new, LF like its siblings):

```rust
//! `display_radio_dialog` row → [`Action::DisplayRadioDialog`] (DK-06b,
//! D-DK10). No database.

use super::super::action::convert_action;
use super::super::*;
use crate::actions::Action;

fn radio_row(target_id: Option<i32>) -> DbActionRow {
    DbActionRow {
        chain_id: 8160,
        action_type: "display_radio_dialog".to_string(),
        target_id,
        target_key: None,
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    }
}

/// The Dakara radio row converts to its own variant, not `DisplayDialog`.
#[test]
fn convert_display_radio_dialog_row() {
    assert_eq!(
        convert_action(&radio_row(Some(5816))),
        Some(Action::DisplayRadioDialog { dialog_id: 5816 })
    );
}

/// No dialog id, no action: the row is dropped, as a `display_dialog` row is.
#[test]
fn convert_display_radio_dialog_without_target_is_dropped() {
    assert_eq!(convert_action(&radio_row(None)), None);
}
```

**4. `executor/dialog/mod.rs`.** In `display`, keep everything up to and including `tracing::Span::current().record("npc_entity_id", npc_entity_id);`. Move every line after it (from `// Stuck-player detector` through the `.await;` of `send_dialog_display`) unchanged into the new private `emit` below, and end `display` with the call:

```rust
    tracing::Span::current().record("npc_entity_id", npc_entity_id);
    emit(dialog_id, entity_id, chain_id, npc_entity_id, tx, space_mgr).await;
}
```

Then add, directly after `display`:

```rust
/// The wire `EntityId` of a radio dialog: no entity.
const RADIO_ENTITY_ID: i32 = 0;

/// `Action::DisplayRadioDialog` (D-DK10): show `dialog_id` with no speaker
/// entity. The wire `EntityId` is always 0, whatever NPC the player last
/// clicked. The client opens the window and shows every screen's text; the
/// id only chooses the portrait, so a radio voice gets none (DK-06a,
/// `docs/reverse-engineering/findings/dialog-display-without-npc.md`).
/// Unlike [`display`] it never reads chain params or the interaction pin
/// and never aborts.
#[tracing::instrument(
    name = "dialog.display_radio",
    level = "info",
    skip_all,
    fields(entity_id, dialog_id, chain_id)
)]
pub(super) async fn display_radio(
    dialog_id: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    emit(dialog_id, entity_id, chain_id, RADIO_ENTITY_ID, tx, space_mgr).await;
}

/// The shared tail of [`display`] and [`display_radio`]: the friction
/// watch, the player journal, the info line and the send.
async fn emit(
    dialog_id: i32,
    entity_id: u32,
    chain_id: i64,
    npc_entity_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // the lines moved out of `display`, unchanged
}
```

`space_mgr.entity_names(0)` returns the empty default (DK-06a checked it), so the moved log lines are safe with `npc_entity_id` 0.

**5. `dispatch.rs`**, after the `DisplayDialog | StartDialog` arm. The match ends in an `other =>` no-op, so without this arm the verb would load and then do nothing:

```rust
        Action::DisplayRadioDialog { dialog_id } => {
            dialog::display_radio(dialog_id, entity_id, chain_id, tx, space_mgr).await;
        }
```

**6. `executor/tests/radio_dialog.rs`** (new). The bullet for `mod.rs`'s `//!` list: `//! - [`radio_dialog`]     — `Action::DisplayRadioDialog` (DK-06b): EntityId 0, no NPC needed.`

```rust
//! `Action::DisplayRadioDialog` (DK-06b, D-DK10): a dialog with no speaker
//! entity. The wire `EntityId` of `onDialogDisplay` is 0, with or without an
//! interaction pin, and no NPC is needed.

use super::*;
use crate::test_support::LogCapture;

const PLAYER: u32 = 1;
const RADIO_DIALOG: i32 = 5816;

fn radio() -> ResolvedActions {
    ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            8160,
            Action::DisplayRadioDialog {
                dialog_id: RADIO_DIALOG,
            },
        )],
    }
}

/// The one message sent: `onDialogDisplay` to the player. Returns its
/// `(EntityId, DialogID)`.
fn only_dialog_display(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> (i32, i32) {
    let msg = rx.try_recv().expect("must emit onDialogDisplay");
    assert!(rx.try_recv().is_err(), "exactly one message");
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, PLAYER, "sent to the player");
            assert_eq!(method_index, crate::mercury::method_idx::ON_DIALOG_DISPLAY);
            (
                i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
                i32::from_le_bytes([args[4], args[5], args[6], args[7]]),
            )
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// No NPC and no pin: `display_dialog` would WARN and send nothing. The
/// radio verb sends `EntityId` 0 and the dialog id.
#[tokio::test]
async fn radio_dialog_sends_entity_id_zero_with_no_npc() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(radio(), PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert_eq!(only_dialog_display(&mut rx), (0, RADIO_DIALOG));
    assert!(
        capture
            .find_message(tracing::Level::WARN, "DisplayDialog: no NPC entity id")
            .is_none(),
        "the radio verb needs no NPC and must not warn"
    );
}

/// A player who just clicked an SG-18 remains still holds that pin; the
/// radio call must not borrow it as the speaker.
#[tokio::test]
async fn radio_dialog_ignores_the_interaction_pin() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(0xBEEF);
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(radio(), PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert_eq!(only_dialog_display(&mut rx), (0, RADIO_DIALOG));
}

/// The dialog is recorded as offered, so its close (`dialog_choice`, button
/// -1) passes the #479 gate.
#[tokio::test]
async fn radio_dialog_is_recorded_as_offered() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let (tx, _rx) = mpsc::channel(8);

    execute_actions(radio(), PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .offered_dialogs()
        .contains(&RADIO_DIALOG));
}
```

If a field or helper name differs from the code (for example `find_message` or `get_entity`), use the one the sibling tests in `executor/tests/` and `executor/dialog/tests.rs` use and note it in the report; do not change what is asserted.

**7. `dakara_e1_missions/mod.rs`**, in `labels_in`, directly after the `Action::DisplayDialog` arm:

```rust
                Action::DisplayRadioDialog { dialog_id } => {
                    format!("display_radio_dialog({dialog_id})")
                }
```

**8. `content-engine-vocabulary.md`.** In the "Authorable and executed" table, after the `display_dialog` row, add `| `display_radio_dialog` | `DisplayRadioDialog` | 0 |`. Directly before the `##### `start_minigame` params` heading, add:

```markdown
##### `display_radio_dialog`

Shows a dialog with no speaker entity: `onDialogDisplay` (client method 105) goes out with `EntityId` 0, so the window opens with every screen's text and no portrait. Use it for a radio call or any voice with no NPC near the player, such as Dakara_E1's General Hammond calls (dialogs 5815 and 5816). `display_dialog` cannot do this: with no NPC in scope it binds the last NPC the player clicked, or sends nothing and logs a WARN. `target_id` is the dialog id; `target_key` and `params` are unused. The dialog is recorded as offered, so a `dialog_choice` chain on it fires when the player closes it. Evidence: [dialog-display-without-npc.md](../reverse-engineering/findings/dialog-display-without-npc.md) (DK-06a). Whether the speaker name label is blank with `EntityId` 0 is an open UAT item (Dakara_E1 DK-U17).
```

**Checks**, in order, from the worktree root:

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-content-engine`
2. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
3. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-content-engine --all-targets -- -D warnings`
4. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
5. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine radio_dialog`: 2 tests pass.
6. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content radio_dialog`: 3 tests pass.
7. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content executor::dialog`: the existing `display` tests still pass after the move to `emit`.

No live-DB test: nothing here reads the seed.

**Revert proof.** Remove the dispatch arm: the action falls to the `other =>` no-op, nothing is sent, and `radio_dialog_sends_entity_id_zero_with_no_npc` fails at its `expect`. Route the arm to `dialog::display` instead: the first test gets no message and the second gets `0xBEEF`. Remove the loader arm: the first conversion test fails (correction from the wave-3b review: `convert_display_radio_dialog_without_target_is_dropped` still passes, since an unknown verb also yields `None`; it guards the `target_id.unwrap_or(0)` mutation instead).

**Docs rows owed.** "Game systems, content chains, or content-engine actions": `docs/content/content-engine-vocabulary.md` (file 8). Name it in the report. The Dakara UAT row DK-U17 is in the README already.
