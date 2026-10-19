# Wave 2: launch and modifiers

> Type: how-to. Part of the [minigames work packets](../work-packets.md); the contract (C1-C7) and the dispatch rules live there.
> These packets depend on [spec-launch-flow.md](../spec-launch-flow.md) (MG-R7). The architect confirms or corrects each `<from MG-R7>` value at gate G1, before dispatch. A packet whose value still reads `<from MG-R7>` is not dispatchable.

Ships one PR per packet. Merge order: MG-22, MG-23 (stacked on MG-22), MG-24, MG-21, MG-25, MG-26. MG-21 and MG-24 both edit `crates/cell-content/src/cell/content/executor/dispatch.rs`. Whichever merges second rebases onto the first, and the wave reviewer checks that the shared launch helper is not written twice.

**Rules (inline, C1):** PowerShell only, with `Set-Location <wt>;` on every command. Cargo runs only through `pwsh tools/build-lane/lane.ps1` (except `cargo fmt`), in the foreground. Edit only the named files. Files stay under the 500/700-line caps, with new tests in new files. Every change needs a test that fails on revert. Docs are CRLF. No `git stash`, prune or push; make one local commit with the brief's attribution. Seeds go in `db/resources/` only, never `db/scripts`. No lab.

## MG-21: pre-game dialog and cell methods 24 and 30

- Agent: rust-gameserver-dev · Host: windows · Wave 2 · Depends on: MG-R7, MG-06, G1 · Reviewer pass: plus bigworld-engine-advisor
- Worktree `mg-21` · Branch `minigames/mg-21-start-dialog` · Commit `MG-21: optional pre-game minigame dialog and startMinigame/minigameStartCancel`
- **Goal.** The original flow: `start_minigame` → client method 53 `onStartMinigameDialog(<from MG-R7>)` → the player clicks → cell method 24 `startMinigame(<from MG-R7>)` launches, or cell method 30 `minigameStartCancel` closes it with 54 `onStartMinigameDialogClose`. It ships behind a server switch whose default is **off** (D-MG11). With the switch off, behaviour is unchanged: the launch is direct.
- **Files:**
  - The cell config. Add `minigame_start_dialog: bool` (default false) next to the other gameplay switches (Grep the cell's config struct for an existing `bool` gameplay switch). Document it in `config/` the way its neighbours are documented.
  - The executor's `StartMinigame` arm. When the switch is on, store a `PendingMinigameStart { game_name, difficulty, on_victory_chains, on_failure_chains, expires_at }` on the player's cell entity (one slot, overwritten by a newer start) and send method 53. Do not set the minigame flags yet.
  - `crates/cell-methods/src/cell/cell_methods/minigame.rs`:
    - `startMinigame` (line 70) takes the pending start and runs the same launch path as the direct launch (MG-02 and MG-06). Factor that path into one helper that the executor and this method both call. With no pending start, log a WARN and do nothing.
    - `minigameStartCancel` (line 133) clears the pending start and sends method 54.
  - Expiry. A pending start older than `<from MG-R7, else 60 s>` is dropped at the next `startMinigame`, with a WARN.
- **Tests (type 1):**
  - switch off: one `StartMinigame` sends `CellToBaseMsg::StartMinigame` directly;
  - switch on: no base message, method 53 sent, then `startMinigame` sends it once;
  - a second `startMinigame` sends nothing;
  - cancel sends 54 and nothing to the base;
  - an expired pending start is refused.

  On revert, the stubs log UNIMPLEMENTED and no launch happens.
- **Lanes:** C2 for `cimmeria-cell-content`, `cimmeria-cell-methods` and `cimmeria-cell`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the launch-flow section; the config reference row for the switch. Doc-update-map rows "Game systems" and "Config keys".

## MG-22: `techcomp` and `abilityBitfield` from the player

- Agent: packet-coder · Host: windows · Wave 2 · Depends on: MG-R7, MG-02, G1
- Worktree `mg-22` · Branch `minigames/mg-22-modifiers` · Commit `MG-22: minigame techcomp and abilityBitfield come from the player`
- **Inputs (from MG-R7, confirmed at G1):**
  - techcomp source: `<from MG-R7>`, range `<from MG-R7>`;
  - bit map: bit `<n>` = ability `<id>` (`<name>`), one row per bit, `<from MG-R7>`.
- **Files:**
  - New `crates/cell-content/src/cell/content/executor/minigame_modifiers.rs`:
    - `pub(crate) const ABILITY_BITS: &[(u32 /*bit*/, u32 /*ability id*/)] = &[...]`, from the table above;
    - `pub(crate) fn abilities_mask(known: impl Fn(u32) -> bool) -> u32`;
    - `pub(crate) fn tech_competency(<inputs per MG-R7>) -> u32`.
  - The executor's `StartMinigame` arm: replace the `// MG-22` literals with these calls. Use the player's known-abilities accessor (Grep `has_ability` or `known_abilities` in `crates/entity`).
- **Tests (type 1),** in new `minigame_modifiers_tests.rs`:
  - a player knowing exactly the abilities of bits 0 and 3 gives `0b1001`;
  - no abilities gives 0;
  - the techcomp mapping on its boundaries.

  On revert, the executor sends 1 and 0 again, and a dispatch test asserting the mask fails.
- **Lanes:** C2 for `cimmeria-cell-content`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the `joinOK` params table: `techcomp` and `abilityBitfield` sources.

## MG-23: item cheats `CA0`-`CA4`, `instcc`, `consUsed`

- Agent: rust-gameserver-dev · Host: windows · Wave 2 · Depends on: MG-R7, MG-22, G1 · Reviewer pass: plus server-authority-enforcer and items-systems-advisor
- Worktree `mg-23` · Branch `minigames/mg-23-item-cheats` · Commit `MG-23: minigame item cheats come from inventory and are consumed server-side`
- **Gate.** If MG-R7 found no item ids for the cheat slots (README M8), G1 marks this packet Done as not-applicable, and MG-72 records it.
- **Inputs:** slot `CA<n>` = item id `<from MG-R7>`; `instcc` = `<from MG-R7>`; consumed on `<from MG-R7: activation | game start>`.
- **Design (D-MG14).**
  - At launch, the base reads the player's inventory counts for the mapped items (base inventory owns items; no cell trip). It passes `item_cheats: [u32; 5]` and `instcc: u32` into `MinigameSession`, and `joinOK` carries them.
  - When a game reports a cheat activation (the per-game `itemCheatActivate` message, routed by a new `GameOutput::ConsumeItem { item_id }`), the session loop asks base inventory to remove one item.
  - The game applies the cheat only after the removal succeeds. A refused removal sends the game's "cheat refused" reply (`<from the game spec>`) and logs a WARN.
  - Never trust a client count.
- **Files:**
  - `crates/minigame/src/minigame/{game.rs,session.rs,server/session_loop.rs}`, plus a new `server/item_cheats.rs` for the consume round-trip;
  - the base launch path in `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs`.
  - If the inventory removal needs a new base-internal API, put it in a new file beside the existing inventory remove helpers, and name it in the report.
- **Tests:**
  - type 1: `joinOK` carries the counts from a stubbed inventory;
  - type 1: a consume request removes exactly one item and only then yields the game's cheat output;
  - type 1: an empty slot refuses, with a WARN.

  On revert, the counts are zero and no item is removed.
- **Lanes:** C2 for `cimmeria-minigame` and `cimmeria-base-world-entry`. Add `live-db-test.ps1 minigame_item_cheat` if the removal touches the DB path.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the item-cheats subsection.

## MG-24: `StartMinigame` effect script

- Agent: rust-gameserver-dev · Host: windows · Wave 2 · Depends on: MG-R7, MG-02, G1 · Reviewer pass: plus combat-systems-advisor
- Worktree `mg-24` · Branch `minigames/mg-24-effect-script` · Commit `MG-24: StartMinigame effect script launches a minigame from an ability`
- **Contract (D-MG8).**
  - Script name `StartMinigame`.
  - Effect NVP params (read through `EffectDef::param_i32` and the string-param accessor): `minigame_type` (`<from MG-R7>`), `difficulty` (default 1), `on_victory_chains` and `on_failure_chains` (chain-id lists, if the NVP format allows lists; otherwise one id each, `on_victory_chain` and `on_failure_chain`).
  - Only a player caster launches. The game runs on the **caster** unless MG-R7 says the target plays.
- **Design.** `EffectContext` has no channel to the base (`crates/cell-world/src/cell/effects/mod.rs:75`).
  - Add a small launch queue on `SpaceManager`: `pending_minigame_launches: Vec<MinigameLaunchRequest>`, in a new file in `cell-world`.
  - The script pushes one request.
  - The cell service drains the queue after effect application, through the shared launch helper (the same function the executor's `StartMinigame` arm uses, flags and lock included). If MG-21 has merged, reuse its helper; otherwise extract it here, and MG-21 reuses this one.
  - A launch while bit 4 (`BSF_PLAYING_MINIGAME`) is set is refused with a WARN.
- **Files:**
  - new `crates/cell-effect-scripts/src/cell/effects/start_minigame.rs` and its `EFFECT_SCRIPTS` row in `registry.rs`;
  - the queue file in `crates/cell-world`;
  - the drain call in the cell service's effect path (Grep where `effect_scripts` results are applied);
  - the helper extraction in `crates/cell-content/src/cell/content/executor/`.
  - No seed changes: MG-62 and MG-63 set `script_name` on the effects.
- **Tests:**
  - type 1: the script pushes one request with the effect's params;
  - type 1: an NPC caster pushes none;
  - type 1: the drain sends `CellToBaseMsg::StartMinigame` and sets the flags;
  - type 1: a second cast while playing is refused.

  The registry's seeded-names test stays green, because no seed names the script yet. On revert, no request is queued.
- **Lanes:** C2 for `cimmeria-cell-effect-scripts`, `cimmeria-cell-content` and `cimmeria-cell`, and `cimmeria-cell-world` (`crates/cell-world`).
- **Docs:** the effect-script catalog under `docs/gameplay/` (Grep `EFFECT_SCRIPTS` or `script_name` in `docs/`), and [minigame-system.md](../../../gameplay/minigame-system.md) § ability launch.

## MG-25: SWF PRNG port (only if G1 says so)

- Agent: packet-coder · Host: windows · Wave 2 · Depends on: MG-R6, G1
- Worktree `mg-25` · Branch `minigames/mg-25-swf-prng` · Commit `MG-25: port the minigame SWF PRNG for client-reproduced state`
- **Gate (D-MG12).** Dispatch only if `spec-protocol-common.md` question 3 says the client derives state from `seed`. Otherwise G1 marks this packet Done as not-applicable.
- **Files:**
  - new `crates/minigame/src/minigame/swf_rng.rs`, `pub struct SwfRng`, holding the exact algorithm from the spec (G1 inlines it here);
  - new `swf_rng_tests.rs`.
- **Test (type 1):** the first 20 outputs for seeds 1 and 0xDEADBEEF match the vectors in the spec. G1 copies them here. On revert the module is gone.
- **Lanes:** C2 for `cimmeria-minigame`.
- **Docs:** [minigame-wire-formats.md](../../../reverse-engineering/findings/minigame-wire-formats.md), a seed-semantics line.

## MG-26: shell alias routing (only under D-MG3 B)

- Agent: packet-coder · Host: windows · Wave 2 · Depends on: MG-03, D-MG3 = B · **BlockedDecision**
- Worktree `mg-26` · Branch `minigames/mg-26-shell-alias` · Commit `MG-26: route <shell> to the server-validated <game> rules`
- **Shape.** In `games/kind.rs`, add `fn rules_for(self) -> GameKind`, which returns the aliased game for each shell the owner names (for example `Bypass` → `Alignment`), and itself otherwise. `create()` and `validation()` use `rules_for()`. The alias follows the target's `PORTED` flag.
- **Test (type 1):** `validation("Bypass")` is `"server"` once Alignment is ported (use a `#[cfg(test)]` override or test against the real flag after MG-AL3), and `create` builds the target game. On revert, Bypass stays client-declared.
- **Lanes:** C2 for `cimmeria-minigame`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the game list.
