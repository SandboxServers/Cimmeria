# Ring transport FSM — traps and invariants

`crates/services/src/cell/ring_transport/`. Read before touching ring travel,
and before adding any FSM state that waits on something outside the FSM.

## The starvation mechanism (why unbounded states are expensive here)

`handle_select_destination` refuses any destination whose state is not `Idle`.
A ring parked in a non-terminating state is therefore removed from **every peer
that lists it as a destination** — on an all-to-all mesh (Harset plaza, regions
4-8) one wedged pad is four peers down. So "the FSM reached a bad state" is not
the defect; *destination starvation* is. Any regression guard here must assert
that a later `handle_select_destination` to the freed ring **succeeds**.
Asserting `state == Idle` is a happy-path test.

## Two destroy paths in `SpaceManager`, and they are not interchangeable

`crates/services/src/cell/space_manager/entities.rs`:

- `disconnect_entity(&mut self, entity_id, tx)` — **async, has `tx`**. The real
  client-disconnect route (`handle_disconnect_entity`). Emits `LeftAoI` to every
  observer and scrubs witness sets *immediately*, then calls `destroy_entity`.
  This is where a teardown hook that must dispatch wire effects belongs.
- `destroy_entity(&mut self, entity_id)` — **sync, no `tx`**, ~30 call sites. No
  `LeftAoI`; cleanup is deferred to the next `compute_aoi_changes` diff.

A hook in the sync path cannot dispatch effects. Do **not** flip FSM state there
and queue only the effects: a player who re-triggers inside the gap receives the
*previous* trip's release on top of their new one. Queue the entity id and let
the tick do bookkeeping + state flip + dispatch together.

## The cross-world hand-off destroy trap

`Effect::TeleportCrossWorld` (`dispatch.rs`) calls `space_mgr.destroy_entity()`
**itself**, as a legitimate step, while the destination ring is correctly parked
in `RemoteLoadWait` expecting that traveller. A naive "player gone" hook treats
it as a drop, empties the expectation, fast-paths the destination to `Idle`, and
strands the traveller on arrival.

Nothing on `CellEntity` distinguishes the two: `destination_ring_id` is set in
both. The fix is to make the sync `destroy_entity` reconciliation **source-side
only** (`players`, `send_players`, `reserved_by`) and never touch
`expected_players`. Only the async disconnect path may reconcile the destination.

## Effect ordering on abort: show THEN unlock

`wire_helpers::send_visible` resolves its target set from
`get_witnesses_of(entity_id)` **at call time**. Unlock-then-show opens a window
where the player can move while witnesses still hold them hidden; a witness who
enters AoI in that window never receives the `onVisible(1)` and renders a
permanently invisible avatar. It also matches the happy path
(`remote_warmup_timer_expired` shows, `cooldown_timer_expired` unlocks).

`send_visible` is already witness-correct (targets = witnesses ∪ self; hide is
`EntityInvisible` 0x0B, show is `onVisible(1)`). `update_state_flag`
(`onStateFieldUpdate`) is owner-only, which is correct — witnesses have no
rendering hook for "input suppressed".

## Track trip participants by id, not by count

The Python original passed a bare count (`remoteCountUpdate`) and the first Rust
port copied it. Holding ids (`expected_players`) instead:

- lets a `RemoteLoadWait` abort release passengers who are in flight but not yet
  loaded — including the two early-return paths in `Effect::TeleportPlayer`
  (entity missing from space; cell→base send failed) that leave a locked, hidden
  player in **no set on either ring**, because `warmup_timer_expired` does
  `std::mem::take(&mut self.send_players)`;
- makes a departing passenger a `retain`, not a decrement that can underflow;
- must be removed from `players_loaded` in the **same** pass, or the readiness
  gate `players_loaded.len() == num_remote_players()` becomes unsatisfiable —
  a fresh stall replacing the old one.

Keep the count as a derived accessor so the readiness check does not churn.

## `BSF_MOVEMENT_LOCK` is ref-counted — never raw `|=` / `&= !`

`crates/entity/src/cell_entity/state_flags.rs` — use `set_state_flag` /
`unset_state_flag` and send `onStateFieldUpdate` only on the returned `true`
(the actual bit transition). Other writers are the death path and the `Stun`
effect script. A raw `|=` never bumps the counter, so the next `unset_*` sees
count == 0, takes the no-op branch and leaves the bit **stuck**. Concretely: a
player who dies mid-ring-warmup gets a counted set from the death path, and a
raw clear from the ring frees the corpse.

Same rule for `BSF_DEAD`. Raw ops are fine only for `BSF_CROUCHING` (idempotent
player input) and `BSF_IN_COMBAT` (deduped via `threatened_mobs`).

## Injectable time in `cimmeria-services`

Reuse `cimmeria_mercury::clock::{Clock, SystemClock, system_clock}` — it is
unconditionally public and `Channel` already reads time through it. Mercury's
`TestClock` sits behind the `test-harness` feature, which services does **not**
enable, so tests carry a four-line local struct implementing the same trait.

Put the `Arc<dyn Clock>` on the owning manager rather than threading `now:
Instant` through public entry points when those callers live in other packets'
files. `Arc<dyn Clock>` has no `Debug` bound, so the containing struct needs a
manual `impl Debug` (`finish_non_exhaustive()`).

## Misc

- `ring_regions` (`HashMap<i32, RingRegion>`) and `ring_transporters` are
  **separate maps that can disagree**. A deadline handler that returns early on a
  lookup miss without clearing its timer re-fires every tick forever.
- `RingTransporterManager.regions` is one `HashMap`, so a pair operation cannot
  hold `&mut` on source and peer at once: read what you need into locals, then
  `get_mut` each sequentially.
- Advancing a peer on `state == RecvWait` alone is not enough — require the
  back-pointer `remote_region_id == Some(source)`, or a ring reserved by a third
  source gets dragged into the trip.
- Harset region 8's tag has **two spellings**: `HarsetinRingRightRegion` in
  `ring_transport_regions.sql`, `HarsetRingRight` in the 2009
  `spaces/Harset.py`. The other four agree. Match the source you are reading.
