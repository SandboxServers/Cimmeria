# Wave 2: launch, modifiers and the movement hold

> Type: how-to. Part of the [minigames work packets](../work-packets.md); the contract (C1-C7) and the dispatch rules live there.
> Filled at gate G1 (2026-10-19) from [spec-launch-flow.md](../spec-launch-flow.md) (MG-R7), [spec-conversebasichumanoid.md](../spec-conversebasichumanoid.md) § 7 (MG-R4) and the README decisions D-MG17 to D-MG20. Every value below is final; nothing is left for the coder to choose.

## How wave 2 ships

- Five packets: MG-22, MG-21, MG-24 (one chain) and MG-63a, MG-63b (a second chain). MG-23, MG-25 and MG-26 are Done as not applicable (D-MG17); see the end of this file.
- **Ships into the integration branch.** Each packet gets its own worktree off `minigames/integration`; ops merges it back with `merge-noff.ps1` once its review loop says `ship`. No PR per packet: the campaign ships one PR at the end (owner directive). A chained packet's worktree is created **after** its predecessor has merged into `minigames/integration`, so nothing stacks.
- **Merge order:** MG-22, MG-63a, MG-21, MG-24, MG-63b. MG-63b waits for MG-24 because both add a row to `cell-effect-scripts/src/cell/effects/registry.rs`.
- **Can start now:** MG-22 and MG-63a (disjoint files). Wave 3's setup slices may run beside them: they touch only their own game folders.
- **Extra review passes:** MG-22 and MG-24: server-authority-enforcer. MG-21: bigworld-engine-advisor. MG-63a: movement-teleport-advisor and server-authority-enforcer. MG-63b: combat-systems-advisor.

## Rules for every wave-2 packet (inline, C1)

1. PowerShell only, never Bash. Start every command with `Set-Location <wt>;`. Use absolute paths with `[IO.File]`.
2. Cargo only through `pwsh tools/build-lane/lane.ps1` (`cargo fmt` may run directly), in the foreground. Live-DB tests only through `pwsh tools/build-lane/live-db-test.ps1 <filter>`.
3. Edit only the files the packet names. If another file must change, stop and return `blocked` with the reason.
4. Files: 500 lines soft, 700 hard. New tests go in new files. `crates/cell-world/src/cell/space_manager/mod.rs` is at 650 lines: add at most 4 lines there.
5. Every behaviour change has a test that fails when the change is reverted. The report says why it fails.
6. Logs follow the repo shape: a WARN carries `entity_id`, `entity_name` (`space_mgr.entity_label(id)`), `player_id`, `player_name` where the player is known, and a `reason = "<snake_case>"` field.
7. Docs are CRLF. Never hand-edit a `<!-- gen:NAME -->` block.
8. No `git stash`, `git worktree prune`, push or PR. One local commit with the packet's subject and the brief's attribution lines.
9. Seeds only in `db/resources/`; never `db/scripts/*.sql`. No lab, client or launcher.

Lane check order, per crate the packet lists (several `-p` may share a call):

```powershell
Set-Location <wt>; cargo fmt -p <package>
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p <package> --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p <package>
```

## Contract excerpt (C3, as merged by MG-02 and MG-06)

```rust
// crates/wire/src/cell/messages/cell_to_base.rs
CellToBaseMsg::StartMinigame {
    entity_id: u32, player_id: i32, game_name: String, difficulty: u32,
    on_victory_chains: Vec<i64>, on_failure_chains: Vec<i64>,
    player_level: u32, intelligence: u32, tech_competency: u32, abilities_mask: u32,
}
// crates/cell-world/src/cell/minigame_hold.rs (MG-06)
pub async fn hold_for_minigame(entity_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) -> bool;
pub async fn release_minigame_hold(entity_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) -> bool;
// cimmeria_wire::state_field::{BSF_PLAYING_MINIGAME (bit 4), BSF_MOVEMENT_LOCK (bit 6)}
```

Result codes: 0 canceled, 1 victory, 2 failure, 3 interrupted or not started, 4 defeated. Any code releases the hold (MG-06).

## MG-22: shared launch helper, `abilityBitfield` from the player

- Agent: packet-coder · Host: windows · Wave 2 · Depends on: MG-02, MG-06 (merged) · Status Ready
- Worktree `mg-22` · Branch `minigames/mg-22-modifiers` · Commit `MG-22: one minigame launch helper; abilityBitfield comes from the player`
- **Decision (D-MG18).** `techcomp` stays 1: the original took it from the triggering instrument item, and no instrument path exists (M9). The ability mask is built at launch from the player's known abilities, then filtered per game, so a game only ever sees the bits it implements. Until a port flips `PORTED`, its filter is 0.
- **Files:**
  1. New `crates/cell-world/src/cell/minigame_launch/mod.rs` (about 150 lines). Register it in `crates/cell-world/src/cell/mod.rs` with `pub mod minigame_launch;` beside `pub mod minigame_hold;` (line 20).

     ```rust
     /// Where a launch came from; a log field, never logic.
     #[derive(Debug, Clone, PartialEq, Eq)]
     pub struct MinigameLaunchRequest {
         pub entity_id: u32,
         pub player_id: i32,
         pub game_name: String,
         pub difficulty: u32,
         pub on_victory_chains: Vec<i64>,
         pub on_failure_chains: Vec<i64>,
         pub source: &'static str, // "content_chain", "start_dialog" or "effect_script"
     }
     /// techcomp sent to every game (D-MG18; README M9).
     pub const MINIGAME_TECH_COMPETENCY: u32 = 1;
     /// Send `CellToBaseMsg::StartMinigame` and take the hold. True when sent.
     pub async fn launch_minigame(req: MinigameLaunchRequest, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) -> bool;
     ```

     Body: move the logic of the `Action::StartMinigame` arm of `crates/cell-content/src/cell/content/executor/dispatch.rs` (lines 177-243) here unchanged in behaviour: read `player_level` (`e.level`) and `intelligence` (`e.stats.get(cimmeria_entity::stats::INTELLIGENCE).map_or(0, |s| s.cur.max(0) as u32)`), default `(1, 0)` when the entity is gone; send; on `Err` log the existing WARN (add `source`) and return false; otherwise call `crate::cell::minigame_hold::hold_for_minigame(...)` and return true. New: `tech_competency: MINIGAME_TECH_COMPETENCY` and `abilities_mask: modifiers::abilities_mask(entity)` (0 when the entity is gone).
  2. New `crates/cell-world/src/cell/minigame_launch/modifiers.rs` (about 80 lines):

     ```rust
     /// Bit n of abilityBitfield and the abilities that grant it
     /// (spec-launch-flow § 4 for 0-14, spec-conversebasichumanoid § 7 for 15-18).
     pub const ABILITY_BITS: &[(u32, &[i32])] = &[
         (0, &[677, 690, 698]), (1, &[680, 691]), (2, &[683, 692]), (3, &[685, 693, 699]),
         (4, &[678]), (6, &[684]), (7, &[687, 700]), (8, &[688, 704]),
         (9, &[681, 689, 701]), (10, &[694, 702]), (11, &[695, 703]), (12, &[679, 696]),
         (13, &[682, 697]), (14, &[686]),
         (15, &[778]), (16, &[779]), (17, &[792]), (18, &[793]),
     ];
     pub fn abilities_mask_from(knows: impl Fn(i32) -> bool) -> u32;   // OR of 1 << bit
     pub fn abilities_mask(entity: &CellEntity) -> u32;                // knows = |id| entity.<abilities>.has_ability(id)
     ```

     Bit 5 has no ability. `has_ability(i32)` is at `crates/entity/src/abilities/manager.rs:138`; Grep the `CellEntity` field that holds the manager.
  3. `crates/cell-content/src/cell/content/executor/dispatch.rs`: the `StartMinigame` arm keeps its INFO log (with `chain_id` and `chain_name`), builds a `MinigameLaunchRequest { source: "content_chain", .. }` and calls `cimmeria_cell_world::cell::minigame_launch::launch_minigame(req, tx, space_mgr).await`. No other logic stays in the arm.
  4. `crates/minigame/src/minigame/games/kind.rs` (87 lines): add

     ```rust
     /// The abilityBitfield bits this game implements; 0 until it is ported.
     pub fn ability_bits(self) -> u32 {
         if self.validation() != Validation::Server { return 0; }
         match self {
             Self::Alignment | Self::GoauldCrystals => 0x7FFF,   // cheat bits 0-14
             Self::ConverseBasicHumanoid => 0x78000,              // Trump bits 15-18
             _ => 0,                                              // Livewire cheats deferred (MG-LW1 delta 11)
         }
     }
     ```

     Derive or add `PartialEq` on `Validation` if it lacks it.
  5. `crates/minigame/src/minigame/games/mod.rs`: `pub fn ability_bits(game_name: &str) -> u32`, via `GameKind::from_name`, 0 for an unknown name.
  6. `crates/minigame/src/minigame/session.rs`, first line of `SessionRegistry::register` (about line 132):

     ```rust
     let abilities_mask = abilities_mask & crate::minigame::games::ability_bits(&game_name);
     ```

- **Tests:**
  - type 1, new `crates/cell-world/src/cell/minigame_launch/tests.rs` (`#[cfg(test)] mod tests;` in `mod.rs`): `abilities_mask_from` for exactly 677 and 685 gives `0b1001`; for nothing gives 0; for 778, 779, 792 and 793 gives `0x78000`; for 690 alone gives bit 0 (a second ability on the same bit).
  - type 1, same file, a `#[tokio::test]` with the `mgr_with_player()` fixture copied from `minigame_hold.rs` tests: give the player ability 677 (Grep the manager for its learn/add method), call `launch_minigame`, receive `CellToBaseMsg::StartMinigame` and assert `abilities_mask == 1`, `tech_competency == 1`, `source`-independent fields equal the request, and bit 4 set afterwards. Reverting step 1's mask gives 0 and fails.
  - type 1, `crates/minigame/src/minigame/games/kind_tests.rs`: every kind's `ability_bits()` is 0 today (no port is flipped), including Livewire; `games::ability_bits("Nope")` is 0. A register test beside the existing session tests (new file `session_mask_tests.rs` if `session_tests.rs` is over 650 lines): registering `Livewire` with mask `0x7FFF` stores 0. Revert step 6 and it stores `0x7FFF`.
  - The existing chain-replay `livewire_pairs` tests still pass (the arm's message is unchanged apart from the mask).
- **Lanes:** `cimmeria-cell-world`, `cimmeria-cell-content`, `cimmeria-minigame`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the `joinOK` params table: `techcomp` is 1 (no instrument source), `abilityBitfield` is the known-ability mask filtered by the game's `ability_bits`. Doc-update-map row "Game systems".

## MG-63a: the server refuses movement while a minigame is held

- Agent: rust-gameserver-dev · Host: windows · Wave 2 · Depends on: MG-06 (merged) · Status Ready · Extra passes: movement-teleport-advisor, server-authority-enforcer
- Worktree `mg-63a` · Branch `minigames/mg-63a-movement-gate` · Commit `MG-63a: the cell rejects player movement while a minigame holds it`
- **Why.** MG-06 sets bit 4 and the counted movement lock, so an honest client stops. A modified client can still walk. The server must refuse it, as it refuses an off-navmesh move.
- **Files:**
  1. `crates/entity/src/movement_validation/mod.rs:93`: add `MovementReject::MovementLocked`, with a doc line. Add its `"movement_locked"` arm to every exhaustive match: `crates/cell/src/cell/service/base_messages/movement.rs:26`, `crates/cell-world/src/cell/space_manager/movement_telemetry/reject.rs` (about line 153) and `crates/cell-world/src/cell/space_manager/tests/movement_validation/telemetry_reject.rs:80`. Let the compiler find any other.
  2. `crates/cell-world/src/cell/space_manager/client_move.rs` is 716 lines, over the cap. First move `resolve_recovery_position` (line 620 to its end) unchanged into a new sibling `client_move_recovery.rs` (`impl SpaceManager` block, registered beside `client_move` in the parent `mod.rs`). Then add the gate right after the GM bypass (lines 255-257, `if movement_unrestricted { return self.accept(...) }`) and before the Layer 1 bounds check (259-262):

     ```rust
     if let Some(anchor) = self.movement_validator.hold_anchor(entity_id, held && !is_gm, last_valid) {
         if let Some(reason) = movement_lock::check(position, anchor) {
         return self.reject_outcome(entity_id, reason, last_valid, space_id, bounds, is_gm);
         }
     }
     ```

     Use the local names the function already has; read them, do not invent.
  3. New `crates/cell-world/src/cell/space_manager/movement_lock.rs` (about 60 lines):

     ```rust
     /// Horizontal slack (m) for float noise and the client's last step.
     pub(crate) const HELD_SLACK_M: f32 = 0.5;
     /// The entity is held in place by the server: a minigame (bit 4) or a movement-only hold (the Prison Boot, MG-63b).
     pub(crate) fn movement_held(e: &CellEntity) -> bool { e.state_field & BSF_PLAYING_MINIGAME != 0 || e.holds_movement_only_entry() }
     /// `Some(MovementLocked)` when the x/z displacement of `new_pos` from the hold `anchor` exceeds the slack.
     /// The caller decides whether the entity is held and exempts game masters.
     pub(crate) fn check(new_pos: [f32; 3], anchor: [f32; 3]) -> Option<MovementReject>;
     ```

     Only x and z count: a held player may still fall or turn. A stun's movement lock (bit 6 alone) is **not** gated here: stuns, knockbacks and roots keep their current paths.
- **Tests (type 1),** new `crates/cell-world/src/cell/space_manager/tests/movement_validation/held.rs` (register it in that folder's `mod.rs`), using the fixtures beside it: held and standing still (or within 0.5 m) is accepted; held and moving 2 m is rejected with `MovementLocked` and the position stays at `last_valid`; a GM is not gated; a stun lock alone (bit 6 without bit 4) moving 2 m is accepted. Reverting the gate makes the second test pass the move and fail.
- **Lanes:** `cimmeria-entity`, `cimmeria-cell-world`, `cimmeria-cell`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md) § the minigame hold: "the server rejects horizontal movement while bit 4 is set (`reason=movement_locked`)". [movement-validation.md](../../../architecture/movement-validation.md) and the reject-reason list in [observability-target-catalog.md](../../../architecture/observability-target-catalog.md): add `movement_locked`. Doc-update-map row "Game systems".

## MG-21: optional pre-game dialog and cell methods 24 and 30

- Agent: rust-gameserver-dev · Host: windows · Wave 2 · Depends on: MG-22 merged into `minigames/integration` · Status BlockedDependency · Extra pass: bigworld-engine-advisor
- Worktree `mg-21` · Branch `minigames/mg-21-start-dialog` · Commit `MG-21: optional pre-game minigame dialog with startMinigame and minigameStartCancel`
- **Decision (D-MG19, D-MG11).** The original flow, behind a process-wide switch whose default is **off**: launch → client 53 `onStartMinigameDialog` → Play sends cell 24 `startMinigame`, Cancel sends cell 30 `minigameStartCancel`. With the switch off nothing changes: launches are direct. Two recorded deviations from the original: the hold flags are set at launch, not when the dialog opens (an unanswered dialog must not lock a player), and expiry is checked lazily when cell 24 arrives.
- **Wire (spec-launch-flow § 1, Confirmed).** Client 53 args, in order: `WSTRING Name` = `req.game_name`; `WSTRING Difficulty` = `format!("Difficulty {}", req.difficulty)`; `INT32 TCLevel` = `MINIGAME_TECH_COMPETENCY`; `WSTRING Verb` = `req.game_name`; `INT32 ArchetypeBitfield` = `0xFFFF`; `UINT8 CanPlay` = 1; `UINT8 CanCall` = 0; `UINT8 CanSpectate` = 0. Strings through `cimmeria_wire::mercury::write_wstring(&mut buf, s)`, integers little-endian. Client 54 has no args. Indices: `cimmeria_wire::cell::client_methods::minigame::{ON_START_MINIGAME_DIALOG, ON_START_MINIGAME_DIALOG_CLOSE}` (53, 54). Send both as `CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }`, as `minigame_hold.rs::send_state` does. The client answers a server-sent 54 with cell 30, so a cell 30 with no offer is a silent no-op.
- **Files:**
  1. New `crates/entity/src/minigame_feature.rs` (about 70 lines), the shape of `ammo_feature.rs`: `START_DIALOG_FLAG = "minigame.start_dialog"`, `START_DIALOG_ENV = "CIMMERIA_MINIGAME_START_DIALOG"`, default false, an invalid value gives false with a WARN, `start_dialog()`, `set_start_dialog(bool)`, `init_start_dialog_from_env()`. Reuse `crate::ammo_feature::parse_flag`. `pub mod minigame_feature;` in `crates/entity/src/lib.rs` beside `ammo_feature`. In `crates/server/src/main.rs`, call `init_start_dialog_from_env()` beside the ammo call (line 159) and add the variable's row to the module-doc environment table (the `CIMMERIA_AMMO_FINITE_SPECIAL` row is near line 51).
  2. New `crates/cell-world/src/cell/minigame_launch/state.rs` (about 140 lines):

     ```rust
     #[derive(Debug, Default)]
     pub struct MinigameLaunchState {
         pub offers: HashMap<u32, PendingOffer>,       // by entity_id; a newer offer replaces the old
         pub queue: Vec<MinigameLaunchRequest>,        // MG-24 fills it
     }
     #[derive(Debug, Clone)]
     pub struct PendingOffer { pub req: MinigameLaunchRequest, pub expires_at: Instant }
     pub const OFFER_TTL: Duration = Duration::from_secs(60);
     /// Off: launch_minigame. On: store the offer (now + OFFER_TTL) and send client 53.
     pub async fn start_or_offer(req: MinigameLaunchRequest, start_dialog: bool, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) -> bool;
     /// Cell 24. Live offer: launch it with source "start_dialog". Missing or expired: WARN
     /// (reason "no_offer" or "offer_expired") and send 54. Bit 4 already set: WARN "already_playing", send 54.
     pub async fn accept_offer(entity_id: u32, now: Instant, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) -> bool;
     /// Cell 30. Remove the offer; INFO when one existed. No 54, no chains, no base message.
     pub fn cancel_offer(entity_id: u32, space_mgr: &mut SpaceManager) -> bool;
     ```

     `pub mod state;` and `pub use state::*;` in `minigame_launch/mod.rs`.
  3. `crates/cell-world/src/cell/space_manager/mod.rs` (650 lines; at most 4 added): field `pub minigame_launch: crate::cell::minigame_launch::MinigameLaunchState,` with a one-line doc, and `minigame_launch: Default::default(),` in `SpaceManager::new` (line 585).
  4. `crates/cell-content/src/cell/content/executor/dispatch.rs`: the arm calls `start_or_offer(req, cimmeria_entity::minigame_feature::start_dialog(), tx, space_mgr)` instead of `launch_minigame`.
  5. `crates/cell-methods/src/cell/cell_methods/minigame.rs`: the `START` arm (lines 65-73) calls `accept_offer(entity_id, Instant::now(), tx, space_mgr).await`; the `START_CANCEL` arm (128-136) calls `cancel_offer(entity_id, space_mgr)`. Keep the arms' `entity_id`/`entity_name` logging style; do not use the shared `mgr` reborrow inside these two arms.
- **Tests (type 1),** new `crates/cell-world/src/cell/minigame_launch/state_tests.rs`, each passing `start_dialog` explicitly (never the global):
  - off: `start_or_offer` sends one `StartMinigame` and no 53;
  - on: it sends one 53 with the exact arg bytes above, no `StartMinigame`, and bit 4 is still clear; then `accept_offer` sends one `StartMinigame` and sets bit 4; a second `accept_offer` sends one 54 and no `StartMinigame`;
  - `cancel_offer` after an offer removes it and sends nothing; with no offer it returns false and sends nothing;
  - `accept_offer` with `now` 61 s after the offer sends 54 and no `StartMinigame`.
  - `crates/entity` unit tests for `minigame_feature`: unset gives false, `"1"` gives true, `"bogus"` gives false.

  Revert the `START` arm and the stub logs UNIMPLEMENTED: the accept test, which calls the helper, still passes, so also add one type 1 test in `crates/cell-methods` (new `minigame_start_tests.rs` if needed) that drives `dispatch(.., START, ..)` after an offer and asserts the `StartMinigame` message.
- **Lanes:** `cimmeria-entity`, `cimmeria-cell-world`, `cimmeria-cell-content`, `cimmeria-cell-methods`; then `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-server --all-targets -- -D warnings`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the launch-flow section (dialog, 24, 30, 60 s, the two deviations); the variable's reference is its row in the `crates/server/src/main.rs` module-doc table (file 1; write it in the shape of the `CIMMERIA_AMMO_FINITE_SPECIAL` row: values, default off, WARN on an invalid value, read once at startup). Doc-update-map rows "Game systems" and "Config keys".

## MG-24: `StartMinigame` effect script

- Agent: rust-gameserver-dev · Host: windows · Wave 2 · Depends on: MG-21 merged into `minigames/integration` · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-24` · Branch `minigames/mg-24-effect-script` · Commit `MG-24: StartMinigame effect script launches a minigame from an ability`
- **Decision (D-MG20, D-MG8).** Script name `StartMinigame` (the registry's PascalCase convention; spec-launch-flow wrote `start_minigame`). The **caster** plays. Effect scripts cannot await the base channel, so the script queues a request and the cell drains the queue after effects resolve, through `start_or_offer`, so the dialog switch applies to ability launches too.
- **Files:**
  1. New `crates/cell-effect-scripts/src/cell/effects/start_minigame.rs` (about 120 lines). `pub struct StartMinigame;` with `impl EffectScript` `on_apply` only:
     - `game = ctx.effect.params.get("minigame_type")`. Missing or not a name `cimmeria_minigame` knows: WARN `reason = "unknown_minigame_type"` with `effect_id`/`effect_name`/`ability_id`/`ability_name`, and return. If `cimmeria-cell-effect-scripts` does not depend on `cimmeria-minigame`, check against the content engine's list instead (Grep `MINIGAME` in `crates/content-engine/src/`); do not add a crate dependency.
     - `difficulty = ctx.effect.param_i32("difficulty")` (`crates/entity/src/abilities/defs.rs:335`), default 1, clamped to 1-5.
     - Refusals, each a WARN and no request: the caster is not a player (`ctx.row_ids().player_id` is `None`, `reason = "npc_caster"`); the caster already has bit 4 (`reason = "already_playing"`).
     - Otherwise push `MinigameLaunchRequest { entity_id: ctx.source_id, player_id, game_name, difficulty, on_victory_chains: vec![], on_failure_chains: vec![], source: "effect_script" }` to `ctx.space_mgr.minigame_launch.queue`, and log INFO `event = "minigame_launch_queued"`.
  2. `crates/cell-effect-scripts/src/cell/effects/registry.rs`: the row `("StartMinigame", &start_minigame::StartMinigame),` and the `mod` line, in the same style as `("Stun", &crowd_control::Stun),`.
  3. `crates/cell-world/src/cell/minigame_launch/state.rs`: `pub async fn drain_launch_queue(tx, space_mgr)`: take the queue (`std::mem::take`) and call `start_or_offer(req, cimmeria_entity::minigame_feature::start_dialog(), tx, space_mgr)` for each.
  4. `crates/cell-combat/src/cell/effects/stat_buffs/mod.rs` (314 lines): call `cimmeria_cell_world::cell::minigame_launch::drain_launch_queue(tx, space_mgr).await` in `stat_buff_tick_at` right after `super::interrupt::resolve_all_interrupts(tx, space_mgr).await;` (line 91, before any early return), and at the end of `flush_stat_buff_timers` (lines 121-130) after `resolve_interrupts_for`.
  - No seed changes: MG-62 sets `script_name` on effects 1643 and 2813.
- **Tests (type 1),** new `crates/cell-effect-scripts/src/cell/effects/start_minigame_tests.rs`, built like the crowd-control tests beside it: a player caster with `minigame_type=ConverseBasicHumanoid`, `difficulty=2` queues one request with those values and `source == "effect_script"`; an NPC caster queues none; a caster with bit 4 set queues none; an unknown `minigame_type` queues none; difficulty 9 clamps to 5. One more in `crates/cell-world/src/cell/minigame_launch/state_tests.rs`: two queued requests, `drain_launch_queue` with the switch off sends two `StartMinigame` and empties the queue. The registry's seeded-names test stays green (no seed names the script yet). Revert the registry row and the first test cannot resolve the script.
- **Lanes:** `cimmeria-cell-effect-scripts`, `cimmeria-cell-world`, `cimmeria-cell-combat`.
- **Docs:** [abilities-and-effects-system.md](../../../architecture/abilities-and-effects-system.md), the effect-script list (Grep `crowd_control` there): a `StartMinigame` row with its two params; [minigame-system.md](../../../gameplay/minigame-system.md) § ability launch.

## MG-63b: Prison Boot holds the wearer until its minigame is won

- Agent: rust-gameserver-dev · Host: windows · Wave 2 · Depends on: MG-63a and MG-24 merged into `minigames/integration` · Status BlockedDependency · Extra pass: combat-systems-advisor
- Worktree `mg-63b` · Branch `minigames/mg-63b-prison-boot` · Commit `MG-63b: Prison Boot 1597 holds the wearer until the Livewire escape`
- **What the seeds already do (spec-launch-flow § 7).** Chain 1023 server-casts ability 1597 (effect 1939) on load while mission 689 is open. Using item 3438 starts Livewire (chain 1024); on victory chain 1025 casts 1598, whose effect 1942 "removes the MISSION_Cellblock_PrisonBoot effect". Today 1939 and 1942 have no script, so the boot holds nobody. Chains 1022-1025 stay unchanged.
- **Decision (D-MG17 rulings, MG-63 split).** The boot is a **movement-only** hold: the wearer must still use item 3438 to start the escape game, so the three "incapacitated" gates (abilities, consumables, auto-cycle) must ignore it. A plain `BSF_MOVEMENT_LOCK` ledger entry would block item use, so the entry is marked `movement_only`. Death leaves the entry (effect 1939 has no `EF_ClearOnDeath`), and a respawn's hard reset keeps its hold, so the wearer stays held until the minigame's release; accepted and documented.
- **Files:**
  1. `crates/entity/src/cell_entity/stat_buff.rs`: `pub movement_only: bool` on `TimedEffectSpec` (179-204, `Default` false) and on `TimedEffect` (105-148), copied across where the entry is built (about line 358).
  2. `crates/entity/src/cell_entity/stat_buff_flags.rs` (beside `holds_ledger_flag`, line 76): `holds_incapacitating_flag(mask)` (as `holds_ledger_flag` but skipping `movement_only` entries) and `holds_movement_only_entry()`. Switch the three gate sites from `holds_ledger_flag` to `holds_incapacitating_flag`: `crates/cell-combat/src/cell/abilities/use_ability/incapacitated.rs:36` (Grep the path if it differs), `crates/cell-content/src/cell/content/consumable_use.rs:235`, `crates/cell/src/cell/ticks/auto_cycle.rs:186`.
  3. `crates/cell-world/src/cell/space_manager/movement_lock.rs` (MG-63a): `movement_held` becomes `bit 4 || e.holds_movement_only_entry()`.
  4. New `crates/cell-effect-scripts/src/cell/effects/movement_hold.rs` (about 140 lines):
     - `MovementHold::on_apply`: `apply_timed_effect` with `TimedEffectSpec { state_flags: BSF_MOVEMENT_LOCK, movement_only: true, duration_secs: None, stacking: TimedStacking::PerEffect, effect_id, ability_id, invoker_id: ctx.source_id, effect_flags, moniker_ids, ..Default::default() }` (the Stun model in `crowd_control.rs` around line 120, without the interrupt queue); INFO `event = "movement_hold_applied"`.
     - `MovementRelease::on_apply`: `released = ctx.effect.param_i32("ReleasesEffect")`; `ctx.space_mgr.remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |e| e.effect_id == released)`; an empty removal calls `crate::cell::effects::script_rows::on_remove_found_nothing(ctx, "movement_hold")`. A missing param: WARN `reason = "missing_releases_effect"`.
     - Two registry rows: `("MovementHold", &movement_hold::MovementHold)` and `("MovementRelease", &movement_hold::MovementRelease)`.
  5. Seeds: in `db/resources/Effects/Seed/effects.sql`, `script_name` (the last column, NULL today) becomes `'MovementHold'` on effect 1939 (line 3090) and `'MovementRelease'` on effect 1942 (line 3092). In `db/resources/Effects/Seed/effect_nvps.sql`, after the `-- ability-mechanics generated cc end` line (1640) and before the sequence `setval`, add a hand-authored block:

     ```sql
     -- minigames hand-authored (25500-25509): MG-63b, MG-62
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25500, 1942, 'ReleasesEffect', '1939');
     ```

     Grep `25500` first; if taken, stop and return `blocked`.
- **Tests:**
  - type 1, new `crates/cell-effect-scripts/src/cell/effects/movement_hold_tests.rs`: after `MovementHold`, bit 6 is set, `holds_movement_only_entry()` is true and `holds_incapacitating_flag(BSF_MOVEMENT_LOCK)` is false (the booted player can use items and abilities); `MovementRelease` with `ReleasesEffect=1939` removes it and bit 6 clears; a minigame hold taken and released while booted leaves bit 6 set (the ref count).
  - type 1, in MG-63a's `held.rs`: a booted (movement-only entry) player moving 2 m is rejected with `MovementLocked`.
  - type 3 live-DB, new `crates/cell-effect-scripts/src/cell/effects/movement_hold_seed_live_db_tests.rs` (copy the shape of the registry's seeded-names live-DB test): effects 1939 and 1942 load with those script names, the registry resolves both, and 1942 carries `ReleasesEffect = 1939`. Lane: `pwsh tools/build-lane/live-db-test.ps1 movement_hold_seed`.

  Revert the seed and the live-DB test fails; revert the gate switch and the "can use items" assertion fails.
- **Lanes:** `cimmeria-entity`, `cimmeria-cell-effect-scripts`, `cimmeria-cell-world`, `cimmeria-cell-combat`, `cimmeria-cell-content`, `cimmeria-cell`; then the live-DB filter above.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md) § ability launch: the Prison Boot hold and its release; the [abilities-and-effects-system.md](../../../architecture/abilities-and-effects-system.md) effect-script rows for both scripts; the Castle Cellblock UAT note in `db/resources/Content/Seed/castle_cellblock_chains.sql` comments is **not** edited (seed comments are content). Doc-update-map rows "Game systems" and "Seeds".

## Not applicable (ruled at G1, D-MG17)

- **MG-23 item cheats:** Done, not applicable. The cheat consumables are unobtainable (M8) and no item-to-slot map exists. `instcc` stays -1 and `CA0`-`CA4` stay 0 in every game; every port answers `activateitemcheat` with WARN `reason = "item_cheat_unavailable"` and no reply. D-MG14 is superseded.
- **MG-25 SWF PRNG:** Done, not applicable. No client derives state from `seed` (spec-protocol-common Q3, spec-crystalgame § 3, spec-goauldcrystals § 3).
- **MG-26 shell aliases:** Done, not applicable. D-MG3 is decided A: shells stay client-declared.
