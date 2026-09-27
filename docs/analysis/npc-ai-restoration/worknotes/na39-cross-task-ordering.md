# NA39: cross-task send ordering to a single client channel

**Status:** Complete. Hazard found and fixed at the `Channel` layer, not at
the AoI/witness-fanout layer the task brief suspected. **Scope title:**
does reliable traffic about entity B, sent to client A, leave the server in
the wrong relative order because two different tokio tasks raced?
**Advisor:** aoi-witness-broadcast, with bigworld-engine-advisor's NA38
receive-path work as the direct predecessor. Follows NA37/NA38.

## Recap: what NA34/NA37/NA38 already ruled out

- NA34: `SpaceManager::compute_aoi_changes` is synchronous, no `.await` gap
  an AoI tick could race on the identity-stamp path.
- NA37 round 1: real two-client wire test, both directions, both arrival
  orders — server-side AoI introduction works over a lossless wire.
- NA37 round 2 / NA38: the real SGW client orders its own reliable stream
  (`UnAckedHandler::queueAckForPacket`, `ghidra://SGW.exe@0x0158cba0`) and
  caches early position relays for unknown entity ids
  (`EntityManager+0x30`), so a `CREATE_ENTITY` that arrives after its
  cascade/movement traffic (loss + retransmit, or simply because movement
  rides the unreliable stream) is *not* lost — the client holds it and
  replays once the entity exists. NA38 wired the equivalent in-order gate
  into the server's own receive path (`channel/rx_order.rs`) and the
  harness, so both sides now agree on "what a real client would see."
- NA38's own conclusion named the next candidate explicitly: "a reliable
  message about B reaching A from a task other than the one that sent A's
  create for B." That is this worknote's task.

## 1. Mapping every send-to-a-client path

Every path that puts a Mercury datagram on a client's wire funnels through
exactly two production entry points into `ConnectedClientState`:

- **Legacy/live path** — `ConnectedClientState::next_seq: Arc<AtomicU32>`
  (reliable) / `next_seq_unreliable` (unreliable), consumed by
  `crates/base-session/src/base/helpers/mod.rs`'s `send_to_witness`,
  `send_to_witness_reliable`, and `send_bundle_to_witness_reliable`. Every
  AoI create/cascade (`entered_aoi`), leave (`left_aoi`), movement relay
  (`entity_moved`, unreliable), entity-method call
  (`entity_method_call[_batch]`, `witness_entity_method`), appearance
  refresh, vendor/trade/mail/crafting/inventory replies, teleport, gate
  travel, and contact-list/progression fan-outs go through one of these
  three helpers. This is the *only* live path — confirmed by grep: no
  production caller of `Channel::send_packet` exists outside tests.
- **`Channel::register_sent_packet`** — called by `shadow_register_reliable_send`
  right after `transport.send_to(...).await` succeeds, to mirror the
  encrypted bytes into the per-session `Channel`'s `tx_window` for
  retransmit/ACK bookkeeping. `Channel::send_packet` (which assigns from
  `Channel::next_tx_seq` under the same lock as insertion) is dead in
  production — "used only by tests and unmigrated paths" per its own
  doc comment, confirmed by grep.

For each helper: **sequence reservation and the socket write are not
atomic.** `send_to_witness_reliable`
(`crates/base-session/src/base/helpers/mod.rs:593`) locks `connected`,
`fetch_add`s `next_seq`, and **releases the lock** before building the
packet and calling `transport.send_to(&packet, addr).await`. Only after
that succeeds does it call `shadow_register_reliable_send`, which takes a
*different* lock (`ConnectedClientState::channel: Mutex<Channel>`) to run
`register_sent_packet`. Between "reserve seq N" and "register seq N in the
Channel", the task can be suspended at the `.await`, and any other task
holding a handle to the same `ConnectedClientState` can run.

## 2. Which tasks can race on the same witness's channel

Base runs two always-on tasks that are genuinely concurrent with each
other (`crates/base/src/base/service.rs::start`):

- `run_connect_loop` — the UDP recv loop. One task, one datagram at a time
  (`handle_datagram(...).await` inside `loop { transport.recv_from(...).await }`,
  no per-packet spawn), but it drives *every* client-request handler:
  vendor, trade, mail, crafting, inventory/appearance, gate travel,
  teleport, login, world_entry.
- The `cell_rx` drain — also one task, draining `CellToBaseMsg` strictly
  FIFO (`while let Some(msg) = cell_rx.recv().await { handle_cell_message(msg, ...).await; }`,
  fully awaited per message, no per-message spawn). This is what carries
  Cell-originated AoI create/cascade/entity-method traffic.

Plus assorted `tokio::spawn`ed fire-and-forget tasks that each grab their
own clone of `connected`/`entity_to_addr`/`transport` and call the same
helpers independently: contact-list presence fan-out (login/logout),
progression's GainLevel contact-list fan-out, the first-login cinematic,
gate-travel's post-crossing tasks, the logout response, and
`world_entry_db`'s writes.

**Any two of these targeting the same witness's `ConnectedClientState` at
overlapping times can interleave their reserve→send→register sequence in
either order.** Concretely: a player standing in a busy area (AoI
create/cascade/entity-method traffic flowing to them via `cell_rx`) who
*simultaneously* does anything that gets a synchronous reliable reply on
their own client (open a vendor window, move an inventory item, chat) —
handled by `run_connect_loop` — races their own channel. This is not a
rare edge case; it is routine play.

## 3. What is *not* the hazard (ruled out, with evidence)

- **CellService itself never races with itself.** `run_cell_loop`
  (`crates/cell/src/cell/service/message_loop.rs`) is a single task; a
  `tokio::select!` between shutdown, `BaseToCellMsg` receipt, and the
  100 ms tick. Each branch runs to completion before the loop re-selects
  — confirmed no internal `tokio::spawn` exists inside the
  message-processing loop under any `cell-*` crate (`crates/cell`,
  `crates/cell-world`, `crates/cell-combat`, `crates/cell-console`,
  `crates/cell-content`, `crates/cell-cover`, `crates/cell-interactions`,
  `crates/cell-methods`) — the only `tokio::spawn` in that family is
  `crates/cell/src/cell/service/startup.rs`'s single top-level launch of
  `run_cell_loop` itself, which is the "one task" referred to above, not
  an internal race. `compute_aoi_changes_for_player` (used by
  `handle_connect_entity`) is synchronous; each resulting `CellToBaseMsg`
  is `tx.send(event).await`'d in a plain `for` loop, so enqueue order into
  `cell_to_base_tx` is exactly program order.
- **Base's `cell_rx` drain never races with itself.** Confirmed above —
  one task, fully awaited per message. So `entered_aoi`'s own
  create-then-cascade ordering (`crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi.rs:84`)
  is safe: cascade's `send_to_witness_reliable` call is not even invoked
  until create's entire call (including its `.await`) returns, which is
  ordinary sequential-code guarantee, not a lock. No other task can make
  create's `fetch_add` happen after cascade's.
- **Base-originated witness fan-out that must reach *other* clients always
  proxies through Cell**, never sends directly:
  `crate::base::helpers::broadcast_to_witnesses` sends
  `BaseToCellMsg::BroadcastToWitnesses` and gets back one
  `CellToBaseMsg::WitnessEntityMethod` per observer via the same serialized
  `cell_rx` path (`crates/base-session/src/base/helpers/witness_broadcast.rs`,
  used by `inventory/appearance.rs`'s equip/holster rebroadcast). The
  direct `send_to_witness_reliable` calls in those same handlers only ever
  target the *requesting* player's own `entity_id` (self-resend), never a
  third party. Teleport is explicitly owner-only, zero witness fan-out —
  witnesses learn from the next AoI tick.
- **The legacy-vs-`Channel::send_packet` dual counter doesn't collide** —
  confirmed no production caller of `send_packet`/`next_tx_seq` exists.
- **`Channel::check_timeouts` (retransmit scan) was never vulnerable** — it
  iterates the entire `tx_window` per-entry (`for entry in
  self.tx_window.iter_mut()`), not front-only, so out-of-order entries
  don't stop it from finding expired ones.
- **Unreliable movement racing ahead of a reliable `CREATE_ENTITY`** is
  real (different stream, no cross-stream ordering) but per NA38 is not a
  bug: the real client caches early position relays for unknown entity ids
  and applies them once the entity exists.

## 4. The confirmed hazard

`Channel::tx_window`'s field doc states the assumption directly: "Entries
are inserted in caller-allocated sequence order (the same order the bytes
went on the wire)." `register_sent_packet`
(`crates/mercury/src/channel/channel_core.rs`) never enforced it — it did
an unconditional `push_back`. Given §2's race, two concurrent reliable
sends to the same witness can register in the *reverse* of their
allocation order (the later-seq send's socket write and lock acquisition
complete first).

`process_acks`'s cumulative-ACK drain is documented and implemented as
front-only: "the tx_window is ordered by sequence (oldest at front), so we
can drain from the front until we hit a sequence beyond the ACK." When
`tx_window` is `[seq=N+2, seq=N, seq=N+1]` (out of order) and the peer acks
only up through `N` (their `create`/`cascade` packets, still catching up),
`process_acks(N)` checks `front = N+2` first: `covered(N+2)` against
`ack_seq=N` is false (`N+2 > N`), so **the loop breaks immediately without
ever popping `N` or `N+1`, even though both were legitimately acked.**

Those entries are now permanently stuck at the *back* of an
otherwise-draining window (the next ack that clears `N+2` from the front
will finally reach them, but until then they occupy TX-window slots and
never get an RTT sample). Repeat this under routine concurrent traffic (as
in §2) and a given client's 32-slot `tx_window` fills with undrained
already-acked ghosts, degrading toward "TX window full" — starving *new*
reliable sends (including future `CREATE_ENTITY`s) to that specific
client, while other clients whose channels didn't happen to hit the race
keep working. **This asymmetry — one channel silently degrades while
others don't — is a strong structural match for the owner's "A sees B, B
doesn't see A" report:** the direction that empties is whichever peer's
channel happened to accumulate the stuck entries, which need not be
symmetric between two players in the same room.

## 5. The fix

`crates/mercury/src/channel/channel_core.rs`:

- Extracted `seq_mod_leq(a, b)` — the same 28-bit modular "is `a` at or
  before `b`" comparator `process_acks`'s `covered` closure already
  computed inline — as a shared free function.
- Added `insert_tx_entry_sorted`, which inserts a `TxEntry` at the position
  that keeps a `VecDeque<TxEntry>` sorted by `packet.sequence` (scanning
  backward from the tail, so the common already-in-order case is O(1)).
- `register_sent_packet` now calls `insert_tx_entry_sorted` for both
  `tx_window` and the `unsent_packets` overflow queue (drained the same
  front-only way by `process_acks`), instead of `push_back`.
- `process_acks`'s `covered` closure now calls `seq_mod_leq` — same math,
  no behavior change there.

This restores the documented invariant regardless of registration order,
without touching sequence allocation (`ConnectedClientState::next_seq`) or
requiring the larger architectural change of unifying allocation and
registration under one lock (which would need `transport.send_to` to
happen while holding `Channel`'s mutex, or a redesign of the send helpers
— out of scope here and worth its own design pass if the follow-up
concurrency test in §6 finds this insufficient in practice).

## 6. Tests (`crates/mercury/src/channel/tests/channel_lifecycle.rs`)

- `register_sent_packet_tolerates_out_of_order_registration` — registers
  seq=2 then seq=1, asserts `tx_window` is `[1, 2]`, acks seq=1 only
  (seq=2 still in flight), asserts seq=1 drains. **Revert-proven:**
  reverting `insert_tx_entry_sorted` back to `push_back` leaves
  `tx_window` as `[2, 1]`; `process_acks(1)`'s front check sees `covered(2)`
  against `ack=1` as false and pops nothing — the test fails.
- `register_sent_packet_sorts_three_out_of_order_registrations` — 2, 3, 1
  in that registration order; asserts sorted `[1, 2, 3]`; a full ack
  drains all three.
- `register_sent_packet_sorts_unsent_queue_overflow` — fills `tx_window`
  to `consts::TX_WINDOW_SIZE`, then registers three out-of-order
  sequences that overflow into `unsent_packets`; asserts that queue is
  sorted too, since `process_acks` drains it the same front-only way.

All three are `Channel`-level unit tests (wire/state-machine correctness),
matching TESTING.md's picker for a channel-state invariant.

## 7. Open questions / follow-ups

- **Not yet proven with a live two-task race.** The tests above pin the
  `Channel`-level invariant directly (registering out of order must still
  drain correctly) rather than proving the OS scheduler actually
  interleaves two real `send_to_witness_reliable` calls in the adversarial
  order under load. A concurrency-flavored integration test
  (`tokio::spawn` two tasks hammering `send_to_witness_reliable` for the
  same witness concurrently, then asserting `tx_window` stays sorted
  end-to-end through a real `ConnectedClientState`) would close this gap.
  Flagging as a follow-up rather than blocking this fix on it, since the
  `Channel`-level bug is real and fixable independent of how often it
  fires in practice.
- **No direct SigNoz corroboration yet.** This mechanism predicts
  `mercury.backpressure` WARN events (`fill_pct >= 50%`) and eventual
  `send_to_witness_reliable`'s "TX window full"-adjacent failures
  clustering on sessions that reported one-way visibility. Worth a SigNoz
  pass (`cimmeria-server` / `cimmeria-network` indexes) correlating those
  signals with reported incidents before calling this fully confirmed
  against the live signal.
- **NA38's other two candidates are still open:** AoI radius/space-id
  mismatch, and a create emitted without its appearance/cascade follow-up
  (using `aoi.create_emit`/`create_send_failed`). This PR only rules out
  (and fixes a real bug found while ruling out) server-side cross-task
  *send ordering* as a cause.
