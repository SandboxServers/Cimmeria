# CS-02L: b2-lite minigame callback trust seal

> **Type:** Worknote. **Audience:** Castle orchestrator, CS-MGF1/2 and CS-02c1/c2 workers, minigame-systems-advisor.
> **Base:** `da2b82c709bb28a8d374f535b032f838ec0a743f` (code read at that SHA).

## Authority review (server-authority-enforcer)

### Verdict: `change` (accept b2-lite with five contract edits; none is a reject reason)

b2-lite does replace the full CS-02b2 for Castle callbacks 1372 and 1375, provided the five edits below land. The residual risk after the edits is the validator of the game itself, which is the minigame advisor's question.

### What the client can assert, and what the server knows

| Client-supplied | Server authority | Where |
|---|---|---|
| ticket string (in the launch URL) | one-use receipt, minted only when the minigame server deposits a terminal result for the registered session | `crates/minigame/src/minigame/session_result.rs:98-164` (identity claim only; `player_id`, `session_instance_id`, `on_victory_chains` copied from the registry; `terminal_reported` blocks a second deposit; `redeem_result` removes the receipt) |
| `victory` SFS command | server-validated for `Livewire` only; every other name is `client_declared` | `crates/minigame/src/minigame/games/mod.rs:36-41`, `livewire/mod.rs:358` |
| `on_victory_chains` | not client data. Copied from the `start_minigame` action params at launch | `cell_dispatch/minigame.rs:111`, `executor/dispatch.rs:177-197` |
| `player_id` | Base-minted at launch from the Cell's own `start_minigame` request | `cell_dispatch/minigame.rs:96-111`; Cell side `crates/cell/src/cell/service/base_messages/player_init/mod.rs:199` sets `entity.player_id` from Base player_init, not the wire |

Spoof path from client data to `on_victory_chains`: **none found**. The client holds a ticket and plays a game; the chain list and the owner are fixed at launch. Cross-player redemption is closed (ticket is the receipt key and is issued to one address). Replay is closed (`terminal_reported` plus remove-on-redeem). A duplicate session per entity is refused at register.

### The six points in the packet

1. **Base b0/b1 recheck (`minigame.rs:244-284`).** Verified: `snapshot_recipient(entity_id, player_id)` filtered by `session_instance_id`, checked before and after the close. It cannot cover the hop that follows: the `BaseToCellMsg` goes over a channel and the Cell resolves `entity_id` afresh. Entity ids are recycled from a free list, so the entity the Cell finds may be another live player (see agent-memory `exploit_entity_id_recycling`). b2-lite's `player_id` equality closes exactly this hop. It is needed, not redundant.
2. **Same-player re-entry.** Safe. Same `player_id` means the same character's own mission state; the Cell cannot see the Base session instance, which is acceptable because the step guard bounds what a relogged character can advance to its own active step.
3. **Guarded path evaluates world and step at fire time.** Verified in `direct_invoke.rs:19-72`: fresh `ExecutionContext`, `populate_world_context`, `populate_mission_context` from the live entity, then `resolve_event_filtered`. The Cell is a single task owning `SpaceManager` (agent-memory `reference_content_chain_authority_invariants`), so evaluation and execution are atomic for `delay_ms = 0`. Unpopulated step state reads as `not_active`, so a missing state fails closed.
4. **`entity_missing` stops firing with player 0.** Correct and strictly safer: today the legacy loop runs `fire_chain_by_id(.., player 0)` against a cell that holds no such entity.
5. **No legacy callback chain has conditions.** Taken as stated for the ten ids in the contract; it is only true until someone adds a condition. See edit 3 (it is also why a seed guard is required).
6. **`delay_ms` and staleness.** The guarded path evaluates conditions once, at fire time. A delayed action is queued (`executor/mod.rs:96-120`) and runs later with no re-check, so a `delay_ms > 0` on the advancing action breaks the step-gate single-grant property. The contract's "delay_ms = 0 on every action" is a convention only. See edit 3.

### Contract edits required (exact)

1. **Accessor placement and cap.** `crates/content-engine/src/chain/mod.rs` is **537 lines at BASE**, not 491, so "stay at or under 505" cannot hold and 541 would exceed the soft cap. Put `chain_has_conditions` in the existing sibling `chain/step_anchors.rs` (an `impl ChainEngine` block that already iterates `chains_by_trigger`), or any new sibling under `chain/`; leave `mod.rs` unchanged. Replace the contract line "491 lines; stay at or under 505" with "put the accessor in a sibling module; `mod.rs` (537) must not grow".
2. **Check the player once, before the loop, and name it.** In `handle_minigame_result` on a victory: resolve `entity`, then refuse the whole message with WARN `reason="entity_missing"` or `reason="player_mismatch"` (fields: `entity_id`, `expected_player_id`, `actual_player_id`, `chain_ids`, no names needed under `nt:id-only` for the missing case). The Base-supplied `player_id` is passed to the legacy path; the guarded path keeps its own `is_player` and non-zero check as defence in depth.
3. **Seed guard is mandatory, not an assumption.** CS-02c1 (seeds) and CS-02c2 (replay) must each include a live-DB test that **fails if reverted**, asserting for chains 1372 and 1375: (a) `engine.chain_has_conditions(id)` is true; (b) conditions include `world 8` and the exact `step_status <mission> <step> active`; (c) every `content_actions.delay_ms` is 0; (d) the chain has no `content_triggers` row (synthetic `__direct_invoke_<id>` only); (e) the starter chains 1371 and 1374 emit `start_minigame` with `minigame_type == "Livewire"` and `on_victory_chains` equal to `[1372]` / `[1375]` (pattern: `chain_replay_tests/livewire_pairs.rs:169`). Without (a), a lost condition row silently demotes a Castle callback to the unguarded legacy path; without (e), a placeholder game name would make the victory client-declared.
4. **Silent miss on the guarded path.** `direct_invoke.rs:62-64` returns with no row when `resolved.actions.is_empty()`. Add an INFO row `reason="guarded_callback_not_matched"` (fields `chain_id`, `entity_id`) so a stale step, wrong world, or a conditioned chain that also has a real trigger (it would never match the synthetic event) is visible from telemetry alone. Allowlist for CS-MGF2 must add `crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs`.
5. **CS-MGF2 tests (each must fail when its fix is reverted):**
   - victory, entity missing: nothing fires (today it fires);
   - victory, `entity.player_id` differs from the message: nothing fires;
   - victory, `player_id == 0` in the message: nothing fires;
   - conditioned chain, step not active: nothing fires; conditioned chain, step active: fires once; a second identical message then does nothing (single grant);
   - unconditioned legacy chain still fires for the matching player (guards the "unchanged" claim);
   - Base forwards `receipt.player_id` (extend `minigame_result_tests.rs:98-121`; the current assertion `matches!(.., on_victory_chains == vec![100, 200])` must also pin `player_id`).

### Residuals (not blockers for b2-lite)

- `fire_chain_by_id` (legacy path) uses `get_chain_actions`, which ignores `chain.enabled`; a disabled unconditioned callback still fires. Pre-existing, not Castle (1372/1375 take the guarded path, which honours `enabled`). Worth a follow-up.
- Same-player relog between launch and Cell receipt is not distinguishable in the Cell (no session instance). Bounded by the step guard.
- `RESULT_RECEIPT_TTL` is 180 s; a receipt that expires before Base redeems yields no callback (fails closed).

### For the minigame-systems-advisor (still to decide)

1. Is Livewire's server-side validation (`livewire/mod.rs` around line 358, `session_loop.rs:340/460`) strong enough that a client cannot declare victory without solving the board? This is the only thing standing between a modified client and chains 1372/1375, because everything downstream is server-authoritative.
2. Which game name and difficulty the 1520/1521 starters should use, and whether Castle's dispenser and tank flows should use Livewire at all or a `client_declared` game (if the latter, b2-lite alone does not make the callback trustworthy; the verdict above assumes Livewire).
3. Whether `RESULT_VICTORY` for a session whose `connected` flag dropped, or a cancel racing the deposit, can produce a victory receipt after the client closed the window (`session_result.rs:119-121` refuses both; confirm against the SFS lifecycle).
4. Whether 180 s receipt TTL and the 4096 cap suit Castle.

None of this changes the `change` verdict; if the advisor answers (1) negatively, b2-lite is insufficient for any Castle minigame callback and CS-MGF1/2 become BlockedDecision.

## Minigame verdict (minigame-systems-advisor)

All paths below are under `crates/minigame/src/minigame/` at BASE `da2b82c70`. Nothing here was run; every claim is from reading code. A claim I could not close is marked UNRESOLVED.

### Verdict: `change`. b2-lite stands for Castle callbacks 1372 and 1375, with the trust claim restated and four edits. It is not a reject, with one condition the orchestrator must confirm (see 1).

### 1. Can a modified client declare victory on Livewire?

**It cannot declare it. It can script it.** Those are different, and the contract must say which one b2-lite buys.

- **No client-declared path.** `games/livewire/mod.rs:193-385` has no `victory` arm. A forged `victory` extension request falls to the `_` arm (`:373-384`), which logs `reason="unknown_command"` and returns nothing. The only `victory` arm in the tree is the placeholder's (`games/placeholder.rs:39-45`). Victory is produced in exactly one place, `livewire/mod.rs:357-363`, when `goal_cut == goal_total`. `session_loop.rs:340` and `:460` are the only consumers of `GameOutput::Victory`.
- **What `goal_cut` counts.** Each `processmove` is checked for: game started (`:258`), wire name exists (`:272-287`), wire not already cut (`:289-300`), and a legal prefix (`:306-349`). `g` wires add to `goal_cut` (`:307-309`). The wire set is generated server-side (`livewire/setup.rs:201-344`), so the client cannot invent wires or cut one twice.
- **What it does not check.**
  - Goal wire names are deterministic: `g1..gN`, with N = `goals` for the tier (`setup.rs:258-269`; tier goals 2/4/4/6 at `:30-95`).
  - There is no ordering, geometry, obstacle-cover, minimum-elapsed-time or rate check on a `g` cut. A `g` cut does not even test `playfield_active` (`:311`, only `p` wires do).
  - `opendoor` (`:195-210`) is client-triggered and only sets `game_started`.
  - A client that has a valid ticket can therefore send `opendoor`, then `processmove g1..gN`, and win in a few milliseconds without solving the board. Nothing in `crates/minigame` forbids it, and no test pins either behaviour (no test in `games/livewire/tests.rs` or the server tests asserts that a forged `victory` is ignored or that goal cuts need a minimum time).
- **The tools prove the bot path is cheap anyway.** The lab ships `client_minigame_play` and `lab_livewire_plan`; a real-UI bot wins the same way. No server-side check on this wire protocol would stop a bot that plays the real SWF.
- **Why this does not block b2-lite.** The claim b2-lite needs is: "a victory callback fires only for the ticket holder, once, because the server counted the goal cuts, not because the client said so." That holds (rows above, plus `session_result.rs:98-164`). The residual is "a cheating player skips the puzzle", and its blast radius is that player's own mission step (step-guarded, self-only). It is identical for the ten legacy Livewire callbacks already in the seeds (`castle_cellblock_chains.sql:198`, `castle_701_chains.sql:381`, and so on).
- **Condition.** If D-CA12 means "skill-proof", then this is a reject and CS-MGF1/2 become BlockedDecision. I read D-CA12 as "player-bound and server-counted" because the original SGW also left goal-cut ordering to the SWF, so I am not blocking. **Open orchestrator condition (unrecorded until confirmed):** the orchestrator must confirm that reading and record one line in RESUME.md. Until that line exists, both verdicts (authority and minigame) are conditional on it.

### 2. Game name and difficulty for the 1520/1521 starters (1371, 1374)

- **Game name: exactly `Livewire`** (case-sensitive). The factory (`games/mod.rs:11-12`) and `validation()` (`:36-41`) both match that string only. Any other name, including `Hack`, `Bypass`, `Activate`, `Analyze`, `Converse`, an unknown typo, or `Alignment`/`GoauldCrystals` (commented out at `:13-15`), runs `PlaceholderGame`, whose own `victory` command is trusted (`placeholder.rs:39-45`). For those, b2-lite makes the callback player-bound but not trustworthy. A typo such as `LiveWire` silently demotes to client-declared with only a WARN (`games/mod.rs:19-26`). This is why the authority note's guard (e) (`minigame_type == "Livewire"`) is mandatory; I add that it must compare the exact string, not a case-insensitive match.
- **Difficulty: use 1 for both, set explicitly in the params.** Evidence:
  - The dispatcher defaults to 1 when the seed omits it (`cimmeria-cell-content .../executor/dispatch.rs:194-196`), and the legacy starters I grepped (1233, 1292, 1024, 1041, 1060, 1356, 7004, 13004) omit it, so they run at tier 1.
  - Valid tiers are 1-4. The loader accepts 1-5 (original assert, see agent-memory `difficulty-ranges`), but `LivewireGame::new` clamps to 1..=4 (`games/livewire/tests.rs:119-139`), and the table has four entries (`setup.rs:30`). Never author 5: it silently becomes tier 4.
  - `tech_competency` is still the hard-coded 1 (agent-memory `chain-wiring-and-gaps`), so tier 1 means 2 goals and the most generous countdown (`countdown_rate 20`, `setup.rs:31-46`).
  - The original difficulty of the dispenser and tank games is UNRESOLVED. The Castle starters are new authoring (R2 table: "1371 dispenser to Livewire", "1374 tank to Livewire"), and CS-00Ra/Rb own the original-evidence question. If they find a different tier, that wins; otherwise 1.
- **Do not use a Castle-specific "harder" tier to compensate for the weak win check.** More goals only add more `g` names for a script to cut.

### 3. Can a cancel or disconnect racing the deposit still produce a victory receipt?

**No, with one correct ordering that is not a leak.** Everything below happens under one registry mutex.

- **Cancel before deposit.** `SessionRegistry::cancel` removes the session, flips `cancel`, and clears the entity's receipts (`session.rs:488-497`). `deposit_result_decision` then finds no session, or `*session.cancel.borrow()`, and returns `Refused` (`session_result.rs:104-124`). `report_terminal` maps that to `EndReason::PlayerCanceled` (`server/terminal_outcome.rs:46-48`), not victory.
- **Cancel after deposit, before Base redeems.** `invalidate_entity_receipts` (`session_result.rs:47-56`, called at `session.rs:482, 495`) deletes the unredeemed receipt, so Base's `redeem_result` returns `None` (`session_result.rs:155-162`). No callback fires.
- **Cancel after Base redeemed.** The win was deposited first, so it is a legitimate victory that happened before the cancel; this is the one ordering where the player closing the window "too late" still gets credit. Acceptable: the board really was won.
- **Disconnect (FIN/RST) before the final cut is read.** The read arm yields `Closed`/`Io` (`session_loop.rs:404-417`), no `Victory`, `aborted()` and `RESULT_CANCELED` follow (`:509-546`). The cancel arm is `biased` first (`:309-315`) and the post-loop recheck (`:506-508`) catches a FIN that beat the close RPC.
- **Disconnect after the final cut.** The cut bytes precede the FIN in the stream, so the victory output is produced first. The receipt deliberately outlives normal teardown (`session_result.rs:1`, `remove_if_ticket` at `session.rs:554-581` does not touch receipts), so Base can still redeem it.
- **`connected` and `terminal_reported` guards.** `!session.connected` and `session.terminal_reported` are in the same refusal condition (`session_result.rs:111-119`), so a second deposit for one ticket is refused. A second connection for the same ticket is refused at claim (`session.rs:392-396`).
- **Not proven.** I did not read Base's side of the close RPC (the path that calls `cancel`/`cancel_if_ticket`), so whether every client close reaches the registry before the deposit is UNRESOLVED here. That is the authority note's Base b0/b1 recheck, not a minigame defect.

### 4. Do the 180 s receipt TTL and the 4096 cap suit Castle?

**Yes; no change.**

- **TTL.** The receipt is minted at the instant the game ends, not at launch (`session_result.rs:129-142`, `issued_at: now`), and the `MinigameResult` message is sent in the same call (`server/result_dispatch.rs:57-61`). The clock therefore covers only the channel hop to Base, normally milliseconds. Play length is irrelevant, so long Castle playthroughs are not at risk. Expiry is lazy (`expire_receipts` runs only on deposit/redeem, `:63-80`) and fails closed: an expired or evicted receipt means no callback, with a WARN `receipt_expired` or `receipt_capacity`.
- **Cap.** 4096 unredeemed receipts at once would need thousands of victories not yet redeemed; the eviction (`:143-152`) drops the oldest with a WARN. No Castle-specific load changes that.
- **Residual.** A delivered `MinigameResult` that Base never redeems leaves a receipt until the next deposit or redeem sweeps it; that is harmless but not visible from telemetry. Not a Castle matter.

### Edits to the contract

1. **Restate the trust claim** in the b2-lite text: "Castle callbacks fire only for the ticket holder, once, after the server counted all `g` goal wires on a `Livewire` board. This is not proof of skill: the win check has no ordering, timing or geometry rule, so a scripted client can win instantly. Residual R-LW1, accepted, same as the ten legacy callbacks."
2. **Starters must be `minigame_type == "Livewire"` (exact string) with `difficulty` 1 set explicitly.** Fold into the authority note's guard (e): assert the string and `difficulty == 1` in the CS-02c1/c2 live-DB tests, so a typo that demotes to the placeholder fails the test.
3. **Optional hardening packet, not a gate:** a minimum elapsed time (or at most one `g` cut per tick) between `opendoor` and `Victory` in `livewire/mod.rs:357`, with a test that a burst of `g` cuts is refused. This raises the bar from "instant" to "needs a timed bot". File it as a follow-up for `rust-gameserver-dev`; do not add it to R2.
4. **Pin the forged-`victory` behaviour** with a Livewire test (send `victory` before and after `opendoor`; expect no `GameOutput::Victory`), in the same follow-up. No packet in R2 needs it.

No edit is needed for the receipt TTL or the cap.

**Final verdict: change(restate the trust claim as server-counted-not-skill-proof with residual R-LW1; require exact `Livewire` plus explicit difficulty 1 in the starters and their seed guard; file the min-time hardening and forged-victory test as non-gating follow-ups; TTL 180 s and cap 4096 unchanged)**


## Edits pending in r2-packets.md (outside this allowlist)

These edits are not yet written into the overlay. CS-MGF1/2 and the c1/c2 packets must not dispatch until they are (the gate says "change, with its edits applied here"):

- Authority edit 1: replace "491 lines; stay at or under 505" in the contract and CS-MGF1 with "put the accessor in a sibling module; `mod.rs` (537) must not grow".
- Authority edit 2: pick one WARN field set. This note uses `expected_player_id`, `actual_player_id`, `chain_ids`; MGF2 text uses `held_player_id`.
- Authority edit 3: add CS-02c1 and CS-02c2 seed-guard tests (a)-(e), with the exact `"Livewire"` string and `difficulty == 1` from minigame edit 2.
- Authority edit 4: add `crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs` to the CS-MGF2 allowlist and the INFO row `reason="guarded_callback_not_matched"` to its change list.
- Authority edit 5: add `player_id == 0` and the "second identical message does nothing" case to the CS-MGF2 tests.
- Minigame edit 1: add the restated trust claim and residual R-LW1 to the b2-lite contract excerpt.
- Open orchestrator condition above (D-CA12 reading, one line in RESUME.md).
