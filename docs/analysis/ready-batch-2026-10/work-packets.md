# ready-batch-2026-10: work packets

Ledger: [README.md](README.md). Resume card: [RESUME.md](RESUME.md).

## Dispatch rules

- One worktree per packet. `ops` creates it with `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`. The branch and name are in each packet. The worktree is `.claude/worktrees/<name>`.
- Dispatch each wave's independent packets in one message, in the background. The brief gives the worktree path, this file's packet section, the report path and the return line.
- Report path: `<worktree>\.claude\reports\ready-batch-2026-10\<packet>-coder.md` (the review-fixer writes `<packet>-review.md`). Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- After each wave, one `packet-review-fixer` reviews the whole wave. Advisor passes are listed per packet.
- Waves:
  - **Wave 1:** P01-P06. Six small packets with no shared files.
  - **Wave 2:** P07, P08, P09, and P10 once P06 has merged.
  - **Wave 3:** P11 once P09 has merged, then P12 (close-out).

## Rules (every packet; the coder reads this section first)

A packet section and this section together make the whole brief. packet-coder does not read CLAUDE.md.

- **R1 Lane only.** Never type `cargo` directly; a hook refuses it. Run everything with `pwsh tools/build-lane/lane.ps1 cargo <subcommand> -p <crate> ...`, from the worktree root. Live-DB tests run with `pwsh tools/build-lane/live-db-test.ps1 <test-name substring>`. Read the lane's summary and failures file; don't rerun just to see output.
- **R2 Check order, per crate.** Run these in this order:
  1. `lane.ps1 cargo fmt -p <crate>`
  2. `lane.ps1 cargo clippy -p <crate> --all-targets -- -D warnings`
  3. `lane.ps1 cargo nextest run -p <crate>`

  At most 5 fix rounds per check. If you are still red after that, return `failed` with the failures-file path.
- **R3 Tests fail on revert.** Each new test must fail when the change is reverted. The report says how you know: revert the change locally, run the test, see it red, then restore. Name the TESTING.md type you used.
- **R4 File caps.** A Rust file has a soft cap of 500 lines and a hard cap of 700. Never push a file past 700. If a packet says "move" or "split", do it. Put new tests in a new sibling file rather than growing a long test file.
- **R1b Workspace check.** Where a packet asks for a workspace check, run this exact command, which takes every slot:

  ```powershell
  pwsh tools/build-lane/lane.ps1 --exclusive cargo check --workspace --all-targets --exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab
  ```

- **R5 Docs are CRLF.** Files under `docs/` use CRLF line endings. After editing one, run this in the PowerShell tool:

  ```powershell
  $p = '<abs path>'; $t = [IO.File]::ReadAllText($p) -replace "\r?\n", "`r`n"; [IO.File]::WriteAllText($p, $t)
  ```

  Then check the result: `(Get-Content -Raw $p) -match "[^\r]\n"` must be False. Never hand-edit a `<!-- gen:NAME -->` block. Don't touch `docs/gap-analysis*`, `docs/project-status.md` or `docs/guides/unified-uat.md` (close-out owns them).
- **R6 Git.** No `git stash`, no push, no `git worktree prune`, no rebase. Commit locally with the packet's subject, plus the attribution lines the brief gives. Then stop.
- **R7 Scope.** Edit only the files the packet names. If you need another file, return `blocked` and say why. Don't change seeds, opcodes or wire bytes unless the packet says so.
- **R8 Shell.** PowerShell only, with absolute paths. No bash.
- **R9 Sound rule (binding).** Never run anything that can make this PC play a sound: GUI or Electron apps, the game client, e2e or Playwright runs, or anything that raises a Windows dialog. If a step would need one, return `blocked`.
- **R10 Context.** If you get near the end of your context, write `<worktree>\docs\analysis\ready-batch-2026-10\worknotes\<packet>.md` (what is done, what is left, the last lane result). Then return `blocked` with the reason `context`.

## Contract (the shapes packets build against)

- P02: `pub fn encode(&self, buf: &mut BytesMut) -> cimmeria_common::Result<()>` on `cimmeria_mercury::bundle::Bundle`.
- P03: `pub(super) struct OutOfRange;` and `pub(super) fn opt_i32(row: &DbActionRow, key: &str) -> Result<Option<i32>, OutOfRange>` in `crates/content-engine/src/loader/action_params.rs`.
- P06 (P10 uses these):
  - `pub const SPEED_WINDOW: Duration`, `pub const MIN_SPEED_DISTANCE: f32`
  - `pub fn observe_speed(&mut self, entity_id: u32, distance: f32, dt_secs: f32, top_speed: f32) -> Option<SpeedSample>` on `MovementValidator`
  - The returned `SpeedSample` holds the **window sums**: `distance` is the summed distance, `dt_secs` the summed dt, `implied_speed` is `sum_d / max(sum_dt, SPEED_WINDOW)`.
- P08: `grant_container(&[2], INV_MAIN) == Some(2)`. Every other existing case keeps its result. As a result, `trade_destination(&[2], 1 | 15) == None`, so the trade is refused (D-08).
- P09 (P11 uses these), all in `cimmeria_minigame` `minigame::session`:
  - `pub async fn try_admit(&self, ip: IpAddr) -> Result<AdmissionGuard, AdmissionRefusal>` on `SessionRegistry`
  - `pub struct AdmissionGuard` (releases its slot on Drop)
  - `pub struct AdmissionRefusal { pub reason: &'static str, pub pending_sessions: usize }`
  - The `reason` is one of `"unexpected_peer"`, `"unexpected_peer_with_sessions"` or `"no_free_ticket"`.

---

# Wave 1

## P01: escape the auth SOAP response XML (#447)

- **Implementer:** packet-coder (Haiku). **Crate:** `cimmeria-auth`.
- **Branch:** `ready-batch-2026-10/p01-auth-xml-escape`. **Worktree:** `rb10-p01`.
- **Commit:** `fix(auth): escape attribute values in the SOAP response XML (#447)`
- **Supersedes** the stale PR #604 (it targets `crates/services/src/auth/handlers.rs`, which is gone). Don't read or reuse it.

**File:** `crates/auth/src/auth/soap_response.rs` (124 lines).

**Change:**

1. Add `fn attr(s: &str) -> std::borrow::Cow<'_, str> { quick_xml::escape::escape(s) }`. quick-xml 0.42 is already a normal dependency of this crate. `escape` turns `& < > " '` into entities.
2. Split the two `Response` wrappers so the XML can be tested:
   - `pub(super) fn login_error_xml(msg: &str) -> String`
   - `pub(super) fn select_error_xml(msg: &str) -> String`

   `login_error` and `select_error` keep their signatures and wrap these.
3. Escape every interpolated **string**: `msg` (both error builders), `s.name` (`ServerName`), `session_key`, `ticket` and `shard.host` (`IP`). Leave `account_id` and `port` as they are; they are integers.

**Tests** (unit, in the same file's `mod tests`). Use the payload `const EVIL: &str = r#"x" Injected="1"/><Evil a="&"#;`.

- `login_success_xml_escapes_the_shard_name`: put `EVIL` in the shard name.
- `server_location_xml_escapes_host_key_and_ticket`: put `EVIL` in the host, the session key and the ticket.
- `error_xml_escapes_the_message`: run both `login_error_xml(EVIL)` and `select_error_xml(EVIL)`.

Each test parses the output with `quick_xml::Reader::from_str(&xml)` in a `read_event()` loop until `Event::Eof`. The read-loop shape to copy is in `soap_request.rs`, in the same module. Each test asserts:

- (a) no event is an error;
- (b) no `Start`/`Empty` element is named `Evil`;
- (c) the target attribute, unescaped, equals `EVIL` exactly. Unescape it with the same quick-xml 0.42 attribute API that `soap_request.rs` uses.

**Why they fail on revert:** the raw `"` ends the attribute early, so an `Evil` element appears or the attribute value reads `x`.

**Lane:** R2 with `-p cimmeria-auth`.

**Docs:** none. Say so in the report ("doc-update-map: no row; internal escaping").

## P02: Bundle::encode refuses oversize payloads (#733)

- **Implementer:** packet-coder (Haiku). **Crate:** `cimmeria-mercury`.
- **Branch:** `ready-batch-2026-10/p02-bundle-oversize`. **Worktree:** `rb10-p02`.
- **Commit:** `fix(mercury): Bundle::encode refuses payloads it cannot frame (#733)`

**File:** `crates/mercury/src/bundle.rs` (209 lines).

Today `encode` writes `len = payload.len().min(u16::MAX)` and silently truncates the payload. Decision D-02:

```rust
/// Encode the bundle into `buf`. Refuses (and leaves `buf` untouched) when any
/// payload is 0xFFFF bytes or longer: BigWorld reserves a 0xFFFF length as an
/// escape for a wider length, which this encoder does not implement.
pub fn encode(&self, buf: &mut BytesMut) -> Result<()> {
    if let Some(m) = self.messages.iter().find(|m| m.payload.len() >= u16::MAX as usize) {
        return Err(CimmeriaError::Protocol(format!(
            "bundle message {:#04x}: payload of {} bytes needs the unsupported 0xFFFF length escape",
            m.message_id, m.payload.len()
        )));
    }
    buf.reserve(self.wire_size());
    for msg in &self.messages {
        buf.put_u8(msg.message_id);
        buf.put_u16_le(msg.payload.len() as u16);
        buf.put_slice(&msg.payload);
    }
    Ok(())
}
```

In `decode`, right after `let payload_len = ...`, add a check: if `payload_len == u16::MAX as usize`, return `Err(CimmeriaError::PacketDecode("bundle message: 0xFFFF length escape is not supported".into()))`.

Update the module doc table to say 0xFFFF is refused.

**Callers:** grep `\.encode\(&mut` and `Bundle::` under `crates/`. Today only this file's tests call `Bundle::encode`. Add `.unwrap()` there. If you find a production caller, return `blocked` and name it.

**Tests** (wire-format, type 2, in this file's `mod tests`):

- `encode_refuses_a_0xffff_payload_and_leaves_buf_empty`: a payload of 65535 bytes gives `Err`, and `buf.is_empty()`.
- `encode_frames_a_0xfffe_payload_byte_exact`: a payload of 65534 bytes `0xAB`. Bytes 0..3 are `[id, 0xFE, 0xFF]`, the total length is 3 + 65534, and the last byte is `0xAB`.
- `decode_refuses_the_0xffff_length_escape`: `[0x10, 0xFF, 0xFF]` followed by 65535 zero bytes gives `Err`.

**Why they fail on revert:** the old encode returns `()` and writes a truncated frame; the old decode accepts the 0xFFFF frame.

**Lane:** R2 with `-p cimmeria-mercury`. Callers outside the crate are covered by the grep above. Don't run a workspace check unless the grep finds a caller outside `bundle.rs`.

**Docs:** `docs/protocol/mercury-wire-format.md`, the "What Cimmeria implements" table row for the generic bundle encoder (`Bundle::encode`, about line 252). Say it refuses payloads of 0xFFFF bytes or more instead of clamping. CRLF (R5). Doc-update-map: the protocol / wire-format row.

## P03: action loader stops wrapping out-of-range i32 params (#246 item A)

- **Implementer:** packet-coder (Haiku). **Crate:** `cimmeria-content-engine`.
- **Branch:** `ready-batch-2026-10/p03-action-narrowing`. **Worktree:** `rb10-p03`.
- **Commit:** `fix(content-engine): drop actions whose i32 params are out of range (#246)`

**Files:**

- New `crates/content-engine/src/loader/action_params.rs`. Register it in `crates/content-engine/src/loader/mod.rs` next to the other `action_*` modules (the mod list is at about lines 42-53).
- `crates/content-engine/src/loader/action.rs` (565 lines; it must not grow by more than about 10 lines, R4).
- New `crates/content-engine/src/loader/tests/action_narrowing.rs`, registered in `loader/tests/mod.rs`. Do **not** grow `action_conversion.rs`; it is 591 lines.

**Shape:**

```rust
//! Reading integer action params without silent `as i32` wrapping.
use tracing::warn;
use super::DbActionRow;

/// A param was present but outside `i32`; the caller drops the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OutOfRange;

/// `params[key]` as an `i32`: `Ok(None)` when absent or not an integer,
/// `Err(OutOfRange)` (with a WARN naming the chain) when it does not fit.
pub(super) fn opt_i32(row: &DbActionRow, key: &str) -> Result<Option<i32>, OutOfRange> {
    let Some(v) = row.params.get(key).and_then(|v| v.as_i64()) else { return Ok(None) };
    i32::try_from(v).map(Some).map_err(|_| {
        warn!(
            chain_id = row.chain_id,
            chain_name = cimmeria_names::book().chain(row.chain_id),
            action_type = %row.action_type,
            param = key,
            value = v,
            "action param is out of i32 range; dropping action"
        );
        OutOfRange
    })
}
```

**Sites in `action.rs`.** Replace every `params.get("k").and_then(|v| v.as_i64())...as i32`. The lines are 25, 29, 38, 46, 50, 68, 156, 251, 304, 308, 321, 322, 323, 381, 444, 445, 461, 465 and 477.

- A defaulted param becomes `opt_i32(row, "k").ok()?.unwrap_or(D)`, keeping the same default `D` (0, 1 or 3, as today).
- An optional param becomes `opt_i32(row, "k").ok()?`.
- Keep the existing `change_stat.amount` try_from block (lines 333-348) exactly as it is.
- Leave the `try_from(..).ok()` sites at about lines 524 and 529 alone.

**Test** (unit, table-driven, `action_narrowing.rs`). Build `DbActionRow` the way `tests/action_conversion.rs` does (it has a row constructor at about line 24). Use `serde_json::json!` params.

For each case `(action_type, target_id, target_key, key)`, set `key` to `i32::MAX as i64 + 1` and assert `convert_action(&row).is_none()`. Cover at least these:

| action_type | key(s) |
|---|---|
| add_dialog_set | slot, mission_id |
| add_item | qty, container |
| remove_item | qty |
| set_aggression | level |
| trigger_transporter | regionId |
| qr_combat_damage | stat_id, source_id |
| change_stat | stat_id, min, max |
| increment_counter | amount |
| set_active_slot | bag_id, slot |
| add_dialog | entity_template, mission_id |
| generate_threat | threat_level |

Provide `target_id` or `target_key` where the arm needs it. Add one in-range case per row shape, e.g. `add_item` with qty 5, still gives `Some`.

**Why it fails on revert:** `as i32` wraps `2147483648` to `i32::MIN`, so the action is `Some`.

**Lane:** R2 with `-p cimmeria-content-engine`.

**Docs:** none beyond the module doc comment. Doc-update-map: no row.

## P04: base follow-ups (#246 item B)

- **Implementer:** packet-coder (Haiku). **Crates:** `cimmeria-base`, `cimmeria-base-world-entry`, `cimmeria-base-session`.
- **Branch:** `ready-batch-2026-10/p04-base-followups`. **Worktree:** `rb10-p04`.
- **Commit:** `refactor(base): share the player-id decode, pin outbox tags exhaustively (#246)`

**1. `crates/base/src/base/connect_loop/account_arms.rs`** (366 lines; `mod tests` at about line 320).

The arms for 0xC4 (about line 136), 0xC5 (about line 177) and 0xC6 (about line 195) each decode a little-endian `i32` player id from the front of the payload, falling back to 0. Read all three first and confirm they are the same expression. Replace them with:

```rust
/// The little-endian player id at the front of a character-slot payload;
/// 0 when the payload is shorter than 4 bytes (the arms treat 0 as "none").
fn payload_player_id(payload: &[u8]) -> i32 {
    payload.get(..4).map_or(0, |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
```

If the three arms differ (for example one logs, or uses a different fallback), keep each one's behavior and return `blocked` with the difference.

Unit tests in the existing `mod tests`:

- `payload_player_id(&[])` is 0;
- `payload_player_id(&[1,2,3])` is 0;
- `payload_player_id(&[0x2A,0,0,0,0xFF])` is 42;
- `payload_player_id(&(-5i32).to_le_bytes())` is -5.

**2. `crates/base-world-entry/src/base/world_entry/cell_dispatch/bandolier.rs`.** Delete the history comment at about lines 35-37 ("an earlier draft used `active_bandolier_slot`..."). Keep the schema-column note around it. No test (comment only).

**3. `crates/base-session/src/base/outbox/tests/payload.rs`** (135 lines). The table test `persistence_contract_strings_are_stable_for_all_variants` covers every variant today, but nothing breaks when a new variant is added. Add a test that does:

```rust
/// The persisted tag for each variant, as an exhaustive match: adding a
/// `CellOutboxPayload` variant fails to compile here until its tag is pinned.
fn pinned_tag(p: &CellOutboxPayload) -> &'static str {
    match p {
        CellOutboxPayload::ItemUsed { .. } => "item_used",
        CellOutboxPayload::InventoryItemGranted { .. } => "inventory_item_granted",
        CellOutboxPayload::InventoryItemRemoved { .. } => "inventory_item_removed",
    }
}
```

Then add `every_variant_event_type_matches_its_pinned_tag`. It builds one value of each variant and asserts `v.event_type() == pinned_tag(&v)`, and that `serde_json::to_value(&v)["kind"] == pinned_tag(&v)`. Use no `_` arm.

Test types: unit. The report should say this is a compile-time guard: it fails when a variant is added without a tag, not on revert of a runtime fix. The player-id tests fail if the helper's bounds check is removed.

**Lane:** R2 for each of `-p cimmeria-base`, `-p cimmeria-base-world-entry` and `-p cimmeria-base-session`. Do not run `live-db-test.ps1`; nothing here touches the DB.

**Docs:** none. Doc-update-map: no row.

## P05: mercury and cell-content follow-ups (#246 item C)

- **Implementer:** packet-coder (Haiku). **Crates:** `cimmeria-mercury`, `cimmeria-cell-content`.
- **Branch:** `ready-batch-2026-10/p05-mercury-dialog-followups`. **Worktree:** `rb10-p05`.
- **Commit:** `test(mercury): pin Packet::encode bytes; rename remaining_flags (#246)`

**1. `crates/mercury/src/packet/tests.rs`** (384 lines). Add two wire-format (type 2) tests for `Packet::encode` from `packet/legacy.rs`. `Packet::new(flags: PacketFlags, sequence: u32, body: Bytes)`, and `encode()` calls `build_outgoing(flags, body, seq_if_flag, &[], None)`. The layout is flags, then body, then the sequence footer (u32 LE) only when `FLAG_HAS_SEQUENCE` (0x40) is set.

- `legacy_packet_encode_appends_the_sequence_footer`: `Packet::new(PacketFlags(FLAG_HAS_SEQUENCE), 0x0102_0304, Bytes::from_static(b"ab")).encode()` gives exactly `[0x40, b'a', b'b', 0x04, 0x03, 0x02, 0x01]`.
- `legacy_packet_encode_without_sequence_flag_has_no_footer`: `Packet::new(PacketFlags(0), 7, Bytes::from_static(b"ab")).encode()` gives exactly `[0x00, b'a', b'b']`.

Use the import paths the existing tests in this file use. If `PacketFlags` is built differently (for example `PacketFlags::from_bits`), use that and record it.

**Why they fail on revert:** this pins current behavior, so there is no fix to revert. They fail if the footer order, the endianness or the flag gating changes. Say so in the report.

**2. `crates/cell-content/src/cell/content/executor/dialog/mod.rs`.** Rename the local `removed_flags` to `remaining_flags` at about lines 283 and 312, and at every use in between. Only the name changes. Make sure the result compiles; no new test.

**3. Check only, no edit unless broken:** `crates/cell-content/src/cell/content/chain_replay_tests/mission_638.rs` uses `require_db_or_skip!()` (helper at about line 40; tests at about lines 93, 210 and 286). The macro is in `crates/test-support/src/live_db_gate.rs:124` and `return`s from the enclosing fn.

- Confirm each use sits in the test fn itself, or in a helper whose early return also ends the test.
- If a helper returns early with a value the test then uses, fix it so the test fn gates itself. Record the result in the report either way.

**Lane:**

- R2 with `-p cimmeria-mercury`.
- R2 with `-p cimmeria-cell-content`.
- If you touched `mission_638.rs`, run `pwsh tools/build-lane/live-db-test.ps1 mission_638`.

**Docs:** none. Doc-update-map: no row.

## P06: per-entity speed window in the movement validator (#1249 part A)

- **Implementer:** packet-coder (Haiku). **Crate:** `cimmeria-entity`.
- **Branch:** `ready-batch-2026-10/p06-speed-window`. **Worktree:** `rb10-p06`.
- **Commit:** `feat(entity): sliding speed window for movement warnings (#1249)`
- This packet is **additive**: `check_kinematics` and `KinematicsOutcome.speed_warn` stay as they are (P10 removes them).

**Files:**

- `crates/entity/src/movement_validation/mod.rs`. It is 613 lines; the tests start at line 374.
- New `movement_validation/tests.rs`.
- New `movement_validation/speed_window.rs`.

**Steps:**

1. **Move the tests.** Cut the whole inline `mod tests { ... }` body into `movement_validation/tests.rs`, unchanged, and leave `#[cfg(test)] mod tests;` in its place. Run the tests once (R2 nextest) before step 2 to confirm the move is clean.
2. **Write `speed_window.rs`:**

```rust
//! Per-entity sliding window for the speed warning (#1249): client packets
//! arrive jittered (25 ms then 175 ms), so one packet's distance/dt
//! overstates speed by up to 4x. The window sums distance and dt over at
//! least SPEED_WINDOW of recent packets.
use std::collections::VecDeque;
use std::time::Duration;

/// How much recent movement the speed warning averages over.
pub const SPEED_WINDOW: Duration = Duration::from_millis(250);
/// Moves shorter than this (world units) never warn (standing still,
/// rotating in place, float noise).
pub const MIN_SPEED_DISTANCE: f32 = 0.01;
/// Cap on samples kept, so a flood of zero-dt packets cannot grow the deque.
const MAX_SAMPLES: usize = 32;

#[derive(Debug, Default)]
pub(super) struct SpeedWindow {
    samples: VecDeque<(f32, f32)>, // (distance, dt_secs)
}

impl SpeedWindow {
    /// Add a sample and return (summed distance, summed dt) over the window.
    /// Drops the oldest samples while the rest still cover SPEED_WINDOW.
    pub(super) fn push(&mut self, distance: f32, dt_secs: f32) -> (f32, f32) {
        self.samples.push_back((distance, dt_secs.max(0.0)));
        while self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
        let window = SPEED_WINDOW.as_secs_f32();
        loop {
            let total: f32 = self.samples.iter().map(|s| s.1).sum();
            let Some(&(_, front_dt)) = self.samples.front() else { break };
            if self.samples.len() > 1 && total - front_dt >= window {
                self.samples.pop_front();
            } else {
                break;
            }
        }
        self.samples.iter().fold((0.0, 0.0), |(d, t), s| (d + s.0, t + s.1))
    }
}
```

3. **Wire the window into `MovementValidator` in `mod.rs`:**
   - Add `mod speed_window;` and `pub use speed_window::{MIN_SPEED_DISTANCE, SPEED_WINDOW};`.
   - Add the field `speed_windows: HashMap<u32, speed_window::SpeedWindow>`, with a doc comment, and initialize it in `new()`.
   - Add `self.speed_windows.remove(&entity_id);` to both `note_authorized_teleport` and `forget`.
   - Add the method:

```rust
/// Windowed speed check (#1249). Call once per accepted move that had a
/// time baseline, with the move's distance and server-clock dt. Returns
/// the window's sample when the averaged speed is over
/// `top_speed × SPEED_WARN_TOLERANCE`. Moves under MIN_SPEED_DISTANCE still
/// add their dt (standing still is real time) but never warn.
pub fn observe_speed(&mut self, entity_id: u32, distance: f32, dt_secs: f32, top_speed: f32) -> Option<SpeedSample> {
    let (sum_d, sum_dt) = self.speed_windows.entry(entity_id).or_default().push(distance, dt_secs);
    if distance < MIN_SPEED_DISTANCE {
        return None;
    }
    let implied_speed = sum_d / sum_dt.max(SPEED_WINDOW.as_secs_f32());
    (implied_speed > top_speed * Self::SPEED_WARN_TOLERANCE).then_some(SpeedSample {
        distance: sum_d,
        dt_secs: sum_dt,
        implied_speed,
        top_speed,
    })
}
```

**Tests** (unit). Add them to `tests.rs`, or to a new `tests_window.rs` registered from `mod.rs` if `tests.rs` would go past 500 lines. Use `T = DEFAULT_TOP_SPEED` (8.125).

- `zero_distance_at_zero_dt_never_warns`: 20 calls of `observe_speed(1, 0.0, 0.0, T)`, all `None`.
- `jittered_packets_at_top_speed_do_not_warn`: 40 packets alternating dt 0.025 and 0.175, each covering 0.8125 u. All `None`.
- `bunched_burst_after_a_gap_does_not_warn`: one packet of 0.8125 u at dt 0.4, then 3 packets of 0.8125 u at dt 0.0. All `None`.
- `single_fast_hop_does_not_warn`: one packet of 1.0 u at dt 0.05. `None`.
- `sustained_triple_speed_warns`: packets of 2.4375 u at dt 0.1. Within 4 packets one is `Some`, and its `implied_speed` is greater than `T * 1.5`.
- `teleport_and_forget_clear_the_window`: fill the window with fast samples until it warns, then call `note_authorized_teleport(1, now)`. The next slow sample (0.8125 u at 0.1 s) is `None`. Repeat with `forget(1)`.

**Why they fail on revert:** the methods would not exist and the tests would not compile. The jitter test is #1249's own reproduction; a single-packet ratio warns on the 25 ms packets.

**Lane:** R2 with `-p cimmeria-entity`. `mod.rs` should end up at about 420 lines.

**Docs:** none in this packet. P10 owns the movement docs once the window is wired in. Doc-update-map: no row yet; say so.

---

# Wave 2

## P07: one method-index table (#171)

- **Implementer:** packet-coder (Haiku). If the re-export list fails to compile after 5 fix rounds, escalate to `rust-gameserver-dev`.
- **Crates:** `cimmeria-wire`, `cimmeria-cell-console`.
- **Branch:** `ready-batch-2026-10/p07-method-idx-single-source`. **Worktree:** `rb10-p07`.
- **Commit:** `refactor(wire): method_idx re-exports client_methods; one literal per index (#171)`
- **Decision D-01:** `cell::client_methods` holds the literals, and `mercury::method_idx` keeps only `pub use` re-exports. This is the reverse of the issue body. Wire bytes are unchanged.

**Files:**

- `crates/wire/src/mercury/mod.rs` (657 lines; `pub mod method_idx` is at about lines 205-351)
- `crates/wire/src/cell/client_methods/mod.rs`
- new `crates/wire/src/cell/client_methods/mob.rs`
- `crates/wire/src/mercury/def_conformance/client_methods.rs`
- `crates/cell-console/src/cell/console/net.rs`
- `crates/cell-console/src/cell/console/tests/p38.rs`
- `crates/cell-console/src/cell/console/tests/ss_u2_duel.rs`
- `docs/agents/doc-update-map.md`

**1. New `client_methods/mob.rs`.** Move `ON_AGGRESSION_OVERRIDE_UPDATE: u16 = 27` and `ON_AGGRESSION_OVERRIDE_CLEARED: u16 = 28` here from `method_idx`, keeping their doc comments, including the Ghidra addresses 0x00d31cd0/0x00d31bd0 and NA33. Add a module doc: "SGWMob's own client methods (indices number SGWMob, not SGWPlayer; 27/28 on a player are Communicator)".

In `client_methods/mod.rs`, add `pub mod mob;` and change the module doc, which says pet is the only exception, to name pet **and** mob.

**2. `method_idx` becomes re-exports only.** Delete every `pub const X: u16 = N;` in `pub mod method_idx`. Replace them with `pub use` groups that keep **every existing name**. Grep for callers with `method_idx::` under `crates/`. The mapping:

| From | Names |
|---|---|
| `crate::cell::client_methods::spawnable_entity` | ON_SEQUENCE, INTERACTION_TYPE, ON_ENTITY_FLAGS, ON_ENTITY_PROPERTY, ON_VISIBLE, ON_KISMET_EVENT_SET_UPDATE, ON_ENTITY_TINT, ON_BEING_NAME_ID_UPDATE |
| `::being` | ON_EFFECT_RESULTS, ON_LEVEL_UPDATE, ON_TARGET_UPDATE, ON_BEING_NAME_UPDATE, ON_STATE_FIELD_UPDATE, BEING_APPEARANCE |
| `::combatant` | ON_STAT_UPDATE, ON_STAT_BASE_UPDATE, ON_ARCHETYPE_UPDATE, ON_ALIGNMENT_UPDATE, ON_FACTION_UPDATE |
| `::communicator` | ON_PLAYER_COMMUNICATION, ON_CHAT_JOINED, ON_CHAT_LEFT |
| `::gate_travel` | SETUP_STARGATE_INFO |
| `::inventory` | ON_BAG_INFO, ON_ACTIVE_SLOT_UPDATE, ON_REMOVE_ITEM, ON_UPDATE_ITEM, ON_CASH_CHANGED |
| `::mail` | ON_MAIL_HEADER_INFO, ON_MAIL_HEADER_REMOVE, ON_MAIL_READ, SEND_MAIL_RESULT |
| `::contact_list` | ON_CONTACT_LIST_UPDATE, ON_CONTACT_LIST_DELETE, ON_CONTACT_LIST_ADD_MEMBERS, ON_CONTACT_LIST_REMOVE_MEMBERS, ON_CONTACT_LIST_EVENT |
| `::player` | ON_BEGIN_AID_WAIT, ON_END_AID_WAIT, ON_KNOWN_ABILITIES_UPDATE, ON_TIME_OF_DAY, ON_DIALOG_DISPLAY, ON_TRAINER_OPEN, ON_LOOT_DISPLAY, ON_PLAYER_DATA_LOADED, ON_CLIENT_MAP_LOAD, GIVE_ABILITY, GIVE_XP_FOR_LEVEL, ON_ERROR_CODE, SETUP_WORLD_PARAMETERS, ADD_CLIENT_HINTED_GENERIC_REGION, ON_RESET_MAP_INFO, ON_EXTRA_NAME_UPDATE, ON_EXP_UPDATE, ON_MAX_EXP_UPDATE, ON_RING_TRANSPORTER_LIST, ON_UPDATE_DISCIPLINE, ON_UPDATE_KNOWN_CRAFTS, ON_ABILITY_TREE_INFO, ON_TRADE_STATE, ON_TRADE_RESULTS, ON_PLAY_MOVIE, `CLEAR_CLIENT_HINTED_GENERIC_REGIONS as CLEAR_HINTED_REGIONS` |
| `::mob` | ON_AGGRESSION_OVERRIDE_UPDATE, ON_AGGRESSION_OVERRIDE_CLEARED |

Keep the existing organization re-exports and `LAUNCH_ORGANIZATION_CREATION`.

This table is the architect's reading. **The source wins:**

- Before deleting each literal, check that the target constant exists with the **same value**. Grep `pub const <NAME>: u16` under `crates/wire/src/cell/client_methods/`.
- If a name lives in a different file, re-export it from there.
- If a value differs, or a name has no table constant, stop and return `blocked` with the list. Never change a table value.

Shorten the `method_idx` doc to say it is a compatibility re-export of `cell::client_methods`, and that new code should import from the tables.

**3. Conformance, in `def_conformance/client_methods.rs`:**

- **A** becomes `method_idx_declares_no_literal_constants`. It asserts `block_consts(METHOD_IDX, "pub mod method_idx", &["u16"]).is_empty()`, with a message telling the reader to add the constant to `cell::client_methods` and re-export it. Delete `MOB_ONLY` and `METHOD_IDX_ALIASES`.
- **B:** add `const MOB_FILE: &str = "wire/src/cell/client_methods/mob.rs";` and a third partition for `mob_consts`. Assert `assert_eq!(mob_consts.len(), 2, ...)` and check them with `found.extend(mismatches(&mob_consts, &flatten("SGWMob", Section::Client), 0, &[], ""));`. The completeness and uniqueness checks still run over `player_consts` only.
- **C** becomes `console_files_keep_no_local_client_method_copies`. For each file in `LOCAL_COPIES`, assert `file_consts(file, &["u16"]).is_empty()`. Delete `LOCAL_COPY_ALIASES`. Fix the doc on `LOCAL_COPIES` so it says these files must import from the tables.

**4. cell-console** depends on `cimmeria-wire` directly:

- `net.rs`, about lines 27-31: delete the local `ON_TIMER_UPDATE = 12`, `ON_MAP_INFO = 123` and `ON_CLIENT_CHALLENGE = 142`. Use `use cimmeria_wire::cell::client_methods::{being::ON_TIMER_UPDATE, player::{ON_CLIENT_CHALLENGE, ON_MAP_INFO}};`. Grep the real module of each first.
- `tests/p38.rs:25`: the same, for `ON_TIMER_UPDATE`.
- `tests/ss_u2_duel.rs:286-287`: replace the local `PVP_FLAG = 7` with `spawnable_entity::ON_ENTITY_PROPERTY`, and the local `DUEL_CLEAR = 153` with `player::ON_DUEL_ENTITIES_CLEAR`, at their uses (about lines 328-329). Keep a short comment that the PvP flag rides `onEntityProperty`.

**Why the tests fail on revert:** re-adding any literal to `method_idx`, or any local `u16` copy to the console files, fails test A or C. A wrong mob value fails B.

**Lane:**

- R2 with `-p cimmeria-wire`.
- R2 with `-p cimmeria-cell-console`.
- The R1b workspace check, because `method_idx` has callers in many crates.

**Docs:**

- `docs/agents/doc-update-map.md` row 28 (the `method_idx` constants module in `crates/wire/src/mercury/mod.rs`): name `crates/wire/src/cell/client_methods/` as the place to add a ClientMethod index, with `method_idx` as re-exports only.
- Grep `method_idx` under `docs/protocol/`, and fix any sentence that says the literals live there.
- CRLF (R5). In the report, list the rows you touched.

## P08: mission-only items given into a carried bag land in the mission bag (#1675)

- **Implementer:** packet-coder (Haiku). After the wave review, **items-systems-advisor** does a read-only pass.
- **Crates:** `cimmeria-cell-catalog`, `cimmeria-base-methods`, `cimmeria-cell-console` (a comment only).
- **Branch:** `ready-batch-2026-10/p08-mission-item-grant`. **Worktree:** `rb10-p08`.
- **Commit:** `fix(inventory): grant mission-only items into the mission bag (#1675)`
- **Decision D-03:** fix this in the base-side placement rule, not in the cell. The cell cache knows only the preferred container, so a cell-side fix would send `{3,1,17}` weapons to the bandolier.
- **Decision D-08 (ruling 2026-10-20):** a trade of a mission-only item is refused as untradeable. Trade keeps calling `grant_container`, and `TRADEABLE_CONTAINERS` stays as it is. **Continuation:** the first session finished step 1, uncommitted in rb10-p08. Resume from the worknote `rb10-p08\docs\analysis\ready-batch-2026-10\worknotes\P08.md`. It gives the exact trade edits, tests and docs, and where it differs from this section, the worknote wins.

**1. `crates/cell-catalog/src/item_placement.rs`** (255 lines).

Add:

```rust
/// The mission bag (`INV_Mission`).
pub const INV_MISSION: i32 = 2;
```

next to `INV_MAIN`. Then change the last line of `grant_container` from `Some(first_player_container(container_sets).unwrap_or(requested))` to:

```rust
// An item that lists no carried bag but may sit in the mission bag (a
// mission item given into the main bag, #1675) goes to the mission bag.
Some(
    first_player_container(container_sets)
        .or_else(|| container_sets.contains(&INV_MISSION).then_some(INV_MISSION))
        .unwrap_or(requested),
)
```

Update the doc comments:

- The bullet at about lines 60-63: "an item that lists no carried bag keeps the request" becomes "goes to the mission bag when it lists one, else keeps the request".
- Add `({2}, 1)` gives 2 to the example line.

Tests in this file:

- Rename `carried_request_with_no_carried_bag_listed_is_kept` to `mission_only_item_given_into_a_carried_bag_goes_to_the_mission_bag`. Assert `grant_container(&[2], INV_MAIN) == Some(INV_MISSION)` and `grant_container(&[2], INV_CRAFTING) == Some(INV_MISSION)`.
- Add `carried_request_with_no_carried_or_mission_bag_is_kept`: `grant_container(&[3], INV_MAIN) == Some(INV_MAIN)`.
- Add one more: `grant_container(&[2], 2) == Some(2)`.

**2. Check, and edit only if one goes red:**

- `crates/base-methods/src/base/world_entry/methods/inventory/grant/placement.rs` test `every_grant_target_is_player_accessible_without_a_vault_session`. It includes the shape `&[2]`; `player_accessible(2, NO_SESSION)` must be true.
- If `grant/placement.rs` pins the old result, update that assertion to the new rule and record it.
- Note that `vendor/purchase/placement.rs::purchase_container` does not call `grant_container`, so it is out of scope (see the README follow-ups).

**2b. Trade (D-08; files added to scope):** `trade_destination` (in `trade/execute/placement.rs`) calls `grant_container(sets, source_container)` and keeps only 1 or 15. A `{2}` item now resolves to 2, so the plan fails with `TradeAbort::NoDestination`. Both sides get `Cancelled` and "cannot be traded". This is the intended behavior. Make these changes, with the exact text in the worknote:

- `trade/execute/placement.rs`: doc comments only. `({2}, 1)` now gives `None`; use `({3}, 1)` gives 1 as the keep-source example. No code change.
- `trade/execute/abort.rs`: the `NoDestination` doc and `Display` text now cover a mission-only type as well as a missing `resources.items` row. The label `no_destination` and `refusal_lines` don't change.
- `trade/execute/tests.rs`, `mod destination`:
  - Replace `item_listing_no_carried_bag_keeps_its_source_bag` with `item_listing_no_carried_or_mission_bag_keeps_its_source_bag` (`[3]` from 1 gives 1, `[17]` from 15 gives 15).
  - Add `mission_only_item_is_not_tradeable` (`[2]` from 1 or 15 gives `None`).
  - Add `mission_only_item_offered_from_the_backpack_is_refused_as_untradeable`: a `TradeItemRow` with sets `[2]` in bag 1, run through `plan_destinations`, gives `NoDestination`, and `refusal_lines` gives the untradeable pair.
  - In `destination_is_always_a_tradeable_bag`, swap `&[2]` for `&[3]` and keep the strict assertion.
- Revert check (R3): the two new mission tests go red on the old `grant_container` last line (`Some(1) != None`, and `expect_err` panics on `Ok([1])`). The other two are rule pins.

**3. Live-DB test** (type: live-DB). Append it to `crates/base-methods/src/base/world_entry/methods/inventory/grant/fall_through_tests.rs` (249 lines), reusing that file's helpers (`cleanup`, `insert_account_and_player`, `bags`, `state()`, and the `handle_grant_item(...)` call shape the other tests use).

- Name: `live_db_mission_only_item_given_into_the_main_bag_lands_in_the_mission_bag`.
- Sentinels: `0x7000_C406` (account), `0x7000_C407` (player), `0x7000_C4E3` (entity). Grep the file first to make sure no other test uses them.
- Start with `require_db_or_skip!()`.
- Get the item with `SELECT item_id FROM resources.items WHERE container_sets = '{2}'::integer[] ORDER BY item_id LIMIT 1`. If no such row exists, `panic!` with a message saying the seed has none (do not skip).
- Grant quantity 1 with `requested = 1` (`INV_MAIN`). Assert `bags(...) == vec![(2, <slot>, 1)]`. Match the tuple shape `bags` returns; read it first.
- With `LogCapture` (`crate::test_support`), assert that the INFO event `grant_container_chosen` (target `inventory`) has the field `container_id = 2`. If the event or field is named differently, use the real names and record them.
- Clean up at the start and the end.

**Why it fails on revert:** the item lands in container 1.

**4. `crates/cell-console/src/cell/console/gm/give.rs`, about lines 199-203.** Update the comment by the `GrantItem { container_id: INV_MAIN }` send. It should say the base reroutes items with no carried bag (mission-only items go to the mission bag, #1675). Comment only.

**Lane:**

- R2 with `-p cimmeria-cell-catalog`.
- R2 with `-p cimmeria-base-methods`.
- `pwsh tools/build-lane/live-db-test.ps1 live_db_mission_only_item_given`.
- Then also `pwsh tools/build-lane/live-db-test.ps1 fall_through`, so the neighbors stay green.
- `pwsh tools/build-lane/live-db-test.ps1 trade::tests`, so the live trade suite stays green (D-08).
- `lane.ps1 cargo check -p cimmeria-cell-console`.

No seed change.

**Docs:**

- `docs/gameplay/inventory-system.md`, the grant-placement rules (about lines 134-140): add the mission-bag rule. In the Trade row of the container table (about line 260), add that a mission-only item is refused. CRLF (R5).
- `docs/gameplay/trade-system.md`, lines 135 and 141: a mission-only item is refused, and the keep-source bullet excludes it (D-08).
- `docs/architecture/observability-target-catalog.md`, line 69: extend the `no_destination` description to cover a mission-only `{2}` type. Edit that phrase only.
- Doc-update-map: the gameplay system docs row (52) and the server-side observability row (36).
- Do not touch `unified-uat.md`; P12 owns it.

## P09: per-ticket admission in the minigame session registry (#1119 part A)

- **Implementer:** `rust-gameserver-dev` (Sonnet). The concurrency reasoning is the hard part.
- **Crate:** `cimmeria-minigame`.
- **Branch:** `ready-batch-2026-10/p09-minigame-admission`. **Worktree:** `rb10-p09`.
- **Commit:** `feat(minigame): per-ticket connection admission in SessionRegistry (#1119)`
- This packet is **additive**: the listener (`server/accept.rs`) is unchanged here; P11 switches it over.
- Decisions D-04 and D-05 apply.

**Files:**

- `crates/minigame/src/minigame/session.rs`: 589 lines; budget about +10. Its tests attach with `#[path = "session_tests.rs"]`.
- New `crates/minigame/src/minigame/session_admission.rs`.
- New `crates/minigame/src/minigame/session_admission_tests.rs`.

**Design:**

- **Attach the module.** Add `#[path = "session_admission.rs"] mod admission;` and `pub use admission::{AdmissionGuard, AdmissionRefusal};` inside `session.rs`. As a child module, it can read `SessionRegistryInner`'s private fields.
- **Admission key.** `enum AdmissionKey { Public(IpAddr), NonPublic }`, and `fn admission_key(ip: IpAddr) -> AdmissionKey` canonicalizes first with `to_canonical()`. Non-public means any of:
  - loopback;
  - IPv4 private: 10/8, 172.16/12, 192.168/16;
  - CGNAT 100.64/10;
  - IPv4 link-local 169.254/16;
  - IPv6 ULA fc00::/7;
  - IPv6 link-local fe80::/10.

  All non-public peers share one key. This is the owner's "non-public match" rule: on a LAN or a Docker host, the TCP peer may show a different private address from the one recorded at `start_minigame`.
- **Slots.** A session counts toward its key's slots when its `client_ip` is `Some`, its key matches the peer's, and either:
  - it is `connected`; or
  - it is pending with `created_at.elapsed() < PENDING_SESSION_TTL`.

  Connected sessions count because their connection still holds a guard; that is what makes N tickets admit exactly N.
- **In-flight count.** Add the field `admitted: Arc<std::sync::Mutex<HashMap<AdmissionKey, usize>>>` to `SessionRegistry` (it is `Clone`; the `Arc` is shared), and initialize it in `new()`. It must be a `std` mutex, because the guard's `Drop` cannot await.
- **`try_admit(&self, ip)`:**
  1. Take the tokio `inner` lock, then the std `admitted` lock, in that order. Hold both while counting.
  2. With `slots = 0`: if `inner.sessions` is empty, the reason is `"unexpected_peer"`; otherwise it is `"unexpected_peer_with_sessions"`.
  3. Else, if `admitted[key] >= slots`, the reason is `"no_free_ticket"`.
  4. Else increment `admitted[key]` and return the guard.

  `pending_sessions` in the refusal is the same count `pending_count()` gives (for the INFO row).
- **`AdmissionGuard { key, admitted: Arc<...> }`.** Its `Drop` decrements, and removes the entry at 0. On a poisoned mutex, use `into_inner()` so it never panics in drop.

**Tests** (`session_admission_tests.rs`, attached from `session_admission.rs` with `#[cfg(test)] #[path = "session_admission_tests.rs"] mod tests;`):

- `admission_key_classifies_public_and_non_public`: table-driven over 127.0.0.1, ::1, 10.1.2.3, 172.16.0.1, 172.31.255.255, 172.32.0.1 (public), 192.168.1.1, 100.64.0.1, 100.128.0.1 (public), 169.254.1.1, fd00::1, fe80::1, 8.8.8.8, 2001:4860::8888, and ::ffff:10.0.0.1 (non-public, after canonicalizing).
- `no_sessions_refuses_unexpected_peer`, and `sessions_for_other_peers_refuse_with_sessions`.
- `one_ticket_admits_one_connection`: register one session for 203.0.113.5. The first `try_admit` is `Ok`, the second is `no_free_ticket`. Drop the first guard and the third is `Ok`.
- `three_tickets_admit_three`: three sessions from one public IP admit 3 and refuse the 4th. Mark one session `connected` (through `authenticate_and_claim`, or by setting the field directly in the child-module test) while its guard is held; still only 3 are admitted.
- `non_public_peers_share_the_ticket_pool`: a session registered from 192.168.1.10 admits a peer at 172.17.0.1.
- `expired_pending_sessions_give_no_slot`: set `created_at` back past `PENDING_SESSION_TTL` through the inner lock in the test.
- `guard_drop_frees_the_slot_under_concurrency`: concurrency, type 5. Spawn 16 tasks on one ticket; exactly 1 is admitted (count the `Ok` results while holding the guards).

**Why they fail on revert:** `try_admit` would not exist. The concurrency test fails if the count is checked without holding the lock (a check-then-increment race).

**Lane:** R2 with `-p cimmeria-minigame`.

**Docs:** none here; P11 owns `minigame-system.md`. Record the API in the report.

## P10: wire the speed window into client_move (#1249 part B)

- **Implementer:** `rust-gameserver-dev` (Sonnet). After the wave review, **movement-teleport-advisor** does a read-only pass.
- **Crates:** `cimmeria-entity`, `cimmeria-cell-world`.
- **Depends on:** P06 merged. The worktree is cut from main after that.
- **Branch:** `ready-batch-2026-10/p10-speed-window-seam`. **Worktree:** `rb10-p10`.
- **Commit:** `fix(cell): speed warning uses the per-entity window, not one packet (#1249)`

**Files:**

- `crates/cell-world/src/cell/space_manager/client_move.rs`: **716 lines, over the hard cap.** It must end at or below 700. If the replacement alone doesn't get it there, move the speed-warning block into a new sibling `space_manager/client_move_speed.rs`.
- `crates/entity/src/movement_validation/mod.rs` and its `tests.rs`.
- `crates/cell-world/src/cell/space_manager/tests/movement_validation/kinematics.rs` (360 lines).
- Possibly `tests/movement_validation/onphysics.rs`, at about line 164.

**Change:**

1. In `client_move.rs`, at about lines 394-459, replace the `kin.speed_warn` handling, including the `speed_bunched` DEBUG arm from #1463, with a call to `observe_speed`:
   - only when `kin.reject.is_none()` **and** `prev_sample` is `Some`;
   - with `distance = proposed.distance_to(&last_pos)`, `dt = now.saturating_duration_since(prev).as_secs_f32()` and the same `top_speed` as today.
   - On `Some(sample)`, emit the same WARN: `movement.speed_warning`, target `movement.validation`, with the same field names (distance, dt_secs, implied_speed, top_speed, plus the entity fields it logs today), and increment `movement_validation_warns_total` with reason `"speed"`.
   - The `speed_bunched` DEBUG event goes away, because the window absorbs bunching.
2. Grep `speed_warn` and `speed_bunched` under `crates/` first, and handle every hit. In `movement_validation/mod.rs`, delete `KinematicsOutcome.speed_warn` and its computation in `check_kinematics`; keep the teleport reject. Update the doc comments. Update the entity tests that read `speed_warn` (about 3 sites): delete the ones that only covered single-packet warns, since P06's window tests replace them, and keep the teleport assertions.
3. Rewrite the seam tests in `kinematics.rs` that use `speed_warn` (about lines 141 and 236-352) to drive a **sustained** run:
   - `speed_warn_is_measured_against_the_entitys_own_movement_speed_mod` needs several packets over at least 250 ms.
   - `bunched_packets_do_not_speed_warn` keeps its intent.
   - Add `jittered_packets_at_top_speed_do_not_warn_at_the_seam`, as negative-log type 12: `LogCapture` asserts there is no `movement.speed_warning`.
4. `crates/discord/src/layer/tests.rs:123` mentions the message. Check it; edit it only if it names `speed_bunched`.

**Why they fail on revert:** the jitter seam test warns under the single-packet rule.

**Lane:**

- R2 with `-p cimmeria-entity`.
- R2 with `-p cimmeria-cell-world`.
- `lane.ps1 cargo check -p cimmeria-discord` if you touched it.

**Docs** (CRLF, R5):

- `docs/architecture/movement-validation.md`, about lines 61 and 119: the window rule (D-06: 250 ms, 0.01 u floor, 32 samples), and that `speed_bunched` is gone.
- `docs/architecture/movement-telemetry.md:74`: the sample fields are now window sums.
- `docs/architecture/observability-target-catalog.md:162`.
- `docs/architecture/observability.md:495`: remove `speed_bunched`.
- Doc-update-map: the architecture docs row and the observability catalog row. List them in the report.

---

# Wave 3

## P11: listener admits one connection per ticket (#1119 part B)

- **Implementer:** `rust-gameserver-dev` (Sonnet). After the review, **minigame-systems-advisor** and **server-authority-enforcer** do read-only passes.
- **Crate:** `cimmeria-minigame` (and any crate that builds `ListenerLimits`).
- **Depends on:** P09 merged.
- **Branch:** `ready-batch-2026-10/p11-minigame-listener-admission`. **Worktree:** `rb10-p11`.
- **Commit:** `feat(minigame): replace connection caps with per-ticket admission (#1119)`
- **D-05 removes the #1299 caps.** The orchestrator confirms with the owner before this ships (see the README).

**Files:**

- `crates/minigame/src/minigame/server/accept.rs` (243 lines)
- `server/limits.rs`
- `server/mod.rs`
- `server/listener_tests.rs` (629 lines; must not grow)
- `server/tests.rs` (644 lines; `test_permit` at about line 138; must not grow)
- new `server/admission_tests.rs`
- `crates/minigame/src/minigame/session.rs`: delete `expects_peer` once nothing calls it.

**Change:**

1. In `serve()` in `accept.rs`, replace the `registry.expects_peer(..)` check and the `tracker.try_acquire(..)` call with a single `registry.try_admit(peer.ip()).await`.
   - On `Err(refusal)`, keep the existing INFO refusal rows and their fields (`reason`, `pending_sessions`, and the throttle for `unexpected_peer`). Add `no_free_ticket` at INFO.
   - On `Ok(guard)`, move the guard into the connection task where the permit went today, so it lives exactly as long as the connection.
2. `limits.rs`: delete `max_connections`, `max_connections_per_ip`, `ConnectionTracker` and `ConnectionPermit`. Keep `handshake_timeout` (30 s) and `idle_timeout` (30 min) (D-04). Grep `max_connections` and `ListenerLimits {` under `crates/` and fix every constructor, including outside this crate.
3. Delete the cap tests in `listener_tests.rs` (about lines 243 and 288). Replace `test_permit` in `tests.rs` with a guard obtained from a registry that has one registered session for 127.0.0.1.
4. New `admission_tests.rs`, over a real loopback listener, using the patterns in `listener_tests.rs`:
   - `two_connections_for_one_ticket_admit_one`, concurrency type 5: two simultaneous TCP connects; one gets the handshake, the other is closed before any byte is read.
   - `failed_handshake_frees_the_ticket_slot`: connect, let the handshake time out (use a short `handshake_timeout`), and the next connect is admitted.
   - `household_of_three_tickets_admits_three`.
   - `admission_refusals_log_info_not_warn`, negative-log type 12: `LogCapture` sees `no_free_ticket` at INFO and no WARN.

**Why they fail on revert:** under the caps, 2 connections for one ticket are both admitted (the cap is 8 per IP).

**Lane:**

- R2 with `-p cimmeria-minigame`.
- The R1b workspace check, because `ListenerLimits` is public.

**Docs** (CRLF, R5):

- `docs/gameplay/minigame-system.md`, "### Connection limits" (about line 191; the cap table at about lines 203-204; the unexpected_peer text at about lines 215-217): rename the section to "Connection admission". Describe one connection per valid ticket, the non-public rule and the three reasons. Remove the cap table.
- `docs/security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md`: mark CAT-K-07 and the connection-DoS items resolved by this PR, with the reason.
- `.claude/agent-memory/minigame-systems-advisor/public-port-hardening.md`: update the limits line (project memory is committed with the change).
- Doc-update-map: the gameplay system docs row, plus the security-audit and agent-memory rows if the map has them.

## P12: close-out

- **Implementer:** `documentation-writer`. For the GitHub actions, the orchestrator dispatches `ops`.
- **Depends on:** P01-P11 merged.
- **Branch:** `ready-batch-2026-10/p12-close-out`. **Worktree:** `rb10-p12`.
- **Commit:** `docs: close out ready-batch-2026-10`

**Docs:**

1. `docs/guides/unified-uat.md` (CRLF): add or refresh the owner UAT steps.
   - #1675: `/gmgiveitem <a mission-only item>` lands in the mission bag. Refresh rows 3a and Bank row 8, at about lines 935-949.
   - #1119: two SWF clients on one ticket, where the second is refused; and a household on a LAN with two tickets, where both connect.
   - #1249: a run with jittered packets raises no `movement.speed_warning`.

   The owner runs these steps; no agent launches the client (R9).
2. `docs/gap-analysis*` and `docs/project-status.md`: update them once, only where these issues are listed (grep the issue numbers and topics). If neither lists any of them, record "no change".
3. The ledger README: every row Done or UATPending. Rewrite RESUME.md to say the campaign is closed.

**GitHub, through `ops`:**

- Close #448, citing PR #1051.
- Close PR #604 as superseded by P01's PR.
- Comment on #171 with the follow-up list from the README's out-of-scope section, or file it as a new issue.
- Add a follow-up issue for vendor purchases of mission-only items (`purchase_container`).

**Finish:**

- Write an empty `.claude/state/active-campaign`.
- `ops` posts a board handoff and retires the worktrees with `rm-worktree.ps1 --merged`.
