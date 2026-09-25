# First-login cinematic AoI hold

> **Last updated**: 2026-09-19
> **Audience**: Engineers touching world entry, the deferred-AoI buffer, the
> first-login cinematic, or the invisible-static-NPC defect (#582)
> **Type**: Architecture decision (experimental — this is a hypothesis with a
> test attached, not a confirmed fix)
> **Owner**: AoI / entity lifecycle
> **Companions**: [player-ghost-aoi-cascade.md](player-ghost-aoi-cascade.md)
> (what an introduction actually carries, and the emit-time join that makes a
> long hold safe), [mercury-bundle.md](mercury-bundle.md) (the two-bundle
> shape the flush emits), [observability.md](observability.md) (the
> `aoi.cinematic_hold` and `aoi.create_emit` targets),
> [../gap-analysis.md](../gap-analysis.md) §8,
> [../analysis/castle-cellblock-rebuild/uat-guide.md](../analysis/castle-cellblock-rebuild/uat-guide.md)
> (how to test it in game)

## Status

**Accepted — experimental.** Shipped as a mitigation for #582 and as the
cheapest available experiment on the cinematic hypothesis. Not validated in
game. If the corpse still goes missing with the hold in place, the hypothesis
is wrong and the decision should be revisited, not patched.

## TL;DR

On a character's **first** login the server now holds entity introductions off
the wire until the intro movie is over — immediately when the player presses
Esc, otherwise 16 seconds after `onClientReady`. NPCs and other players appear
as the movie ends instead of arriving while it plays. Nothing changes for a
returning character.

The reason is a client-side drop that survives a perfectly delivered packet.

## Context

### The defect

A first-login player in Castle_CellBlock never sees the GuardBody corpse until
they relog. The defect has been open since **June 2026** — the 2026-06-19/20
colo repro — and has outlived two hypotheses. PR #582 did not introduce it; it
added the `aoi.create_emit` / `aoi.create_send_failed` seams to localise it.

The 2026-09-19 colo repro (SigNoz; server restart 22:24:36Z; account
`lomiada1`) is the clean one. First-login character `qwddd` (player 71) never
saw the guard corpse — a `class_id 0` static mesh — although the Frost corpse
5.8 m away, a `class_id 1` entity, rendered fine. The player walked over the
spawn point twice, relaunched, made a new character (player 72), and found the
corpse within three seconds. A screenshot confirms no mesh and no nameplate at
the spawn point.

Three things came out of that session, and they are what this decision rests
on:

1. **The introduction was sent, both times.** `aoi::entered_aoi` puts the
   CREATE_ENTITY + UPDATE_AVATAR pair and the `createOnClient` cascade on the
   wire as two reliable packets per entity. The server logged identical
   `aoi.entity_enter` events for the corpse in the bad session and the good
   one, and no `aoi.create_send_failed` in either.
2. **Mercury delivered it.** In the bad session's first 75 seconds exactly one
   reliable packet was retransmitted — seq 229, once, 17 s after the create
   burst. Every create packet was therefore ACKed by the client before its
   RTO. This retires the June "Mercury-level loss or coalescing" suspect: the
   drop is *inside the client, after delivery*.
3. **The seam built to localise it was invisible.** The `aoi.create_emit`
   DEBUG event added by #582 never reached SigNoz, because the OTLP
   `EnvFilter` in `crates/server/src/logging.rs` (now [`crates/server/src/logging/filters.rs`](../../crates/server/src/logging/filters.rs))
   named only `aoi.entity_enter=debug,aoi.entity_leave=debug`, and an unnamed
   custom target inherits the leading `info`.

### The lead

One thing differed between the bad session and the good one: in the bad
session the `Cine-SGWLogo` cinematic ran its full length (the 20 s
appearance-spam guard completed all 200 resends); in the good session the
client sent `cancelMovie` after 1.5 s. In both sessions the creates went out in
the same instant as `onPlayMovie`.

That is **n = 1**. Treat it as a lead, not a finding.

The mechanism it suggests is one we already know exists. The cinematic-exit
`CollectGarbage` reclaims the player's own appearance asset — that is #288,
the "dev cube", healed by the appearance spam in
[`world_entry_appearance/cinematic.rs`](../../crates/services/src/base/world_entry_appearance/cinematic.rs).
The working theory is that the same GC also reclaims a static mesh whose entity
was created mid-movie, and that nothing re-sends it. A `class_id 1` entity
apparently survives, or heals itself; a `class_id 0` static mesh has no heal
path at all.

## Decision

Hold entity introductions for the duration of the first-login movie, then
flush them.

### Where the hold starts

[`world_entry_appearance/cinematic_aoi_hold/`](../../crates/services/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs)
owns the hold. `begin` runs inside `handle_on_client_ready`'s
`pending_client_ready` take, in the **same critical section**, when
`first_login != 0`.

That placement is the load-bearing part. Taking `pending_client_ready` is what
opens the pre-ready AoI gate, and the cell answers `ConnectEntity` with
`EnteredAoI` while the handler is still awaiting its DB reads. A hold armed
any later — next to `send_cinematic`, say, which reads naturally — lets those
creates reach a client that is about to play a fullscreen movie, which is
precisely the shape being guarded against.

### What is held, and what is not

While `ConnectedClientState.cinematic_aoi_hold` is set:

| Message | During the hold | Why |
|---|---|---|
| `EnteredAoI`, `LeftAoI` | Buffered (`deferred_aoi::should_hold_entity_traffic`) | The introductions themselves, and the removals that must stay ordered behind them |
| `EntityMoved` | Dropped | Unreliable and short-lived; the client has no such entity yet, and the next post-flush frame supersedes it. Same reasoning as the pre-ready gate |
| `WitnessEntityMethod`, `EntityInvisible` | Buffered (`deferred_aoi::cinematic_hold_active`) | **New.** The pre-`onClientReady` gate leaves these ungated. A method for a held entity would otherwise overtake its create, reach a client that has no such entity, and be dropped for good |
| `EntityMethodCall` on the player's own entity | **Not held** — flushed at `onClientReady` | Mission, dialog and hotbar traffic must not be parked behind a 13-second movie. `flush_deferred_self_methods` drains exactly these and leaves the rest buffered |

All of it uses the existing deferred-AoI buffer, so the hold inherits its cap
(`MAX_DEFERRED_AOI_MSGS`) and its WARN-on-overflow behaviour for free.

### Where the hold ends

Whichever comes first:

- **`cancelMovie`** — the client's Esc or Lua stop. `handle_cancel_movie`
  calls `release_on_cancel` *after* the appearance resend, because the
  appearance is what the client needs first.
- **`HOLD_DURATION` = 16 s** — the timeout task armed at `onClientReady`. The
  client sends nothing when a movie ends on its own, so a timer is the only
  available signal. The deadline is measured from the moment the hold
  *began* (`sleep_until(hold.started + HOLD_DURATION)`), not from when the
  task was armed: the handler arms it only after its DB reads and cell sends,
  and a slow dependency there must shorten the remaining wait, not stretch
  the hold past the movie.

16 s is `Cine-SGWLogo`'s 13.10 s (314 frames @ 23.976 fps) plus room for the
exit GC. It is not an arbitrary safety margin: in the repro the player's first
input came 16.4 s after ready, so NPCs arriving at 16 s land *behind* the intro
dialog rather than popping into a room the player has already started reading.

Four invariants in the release and flush path are worth knowing before you
touch it:

- **Exactly one task releases a hold.** `cancelMovie` and the timeout can both
  fire at the 16-second boundary. The first to arrive claims the hold
  (`CinematicAoiHold::releasing`) under the `connected` lock and the second
  returns at once. Without the claim, the second task finds the buffer
  momentarily empty while the first is still awaiting its sends, lifts the
  hold, and live traffic overtakes the in-flight creates.
- **An entity's leave and re-entry keep their order.** The flush bundles
  introductions ahead of everything else, which is only safe while no entity
  both leaves and enters inside one dispatch — and over 16 seconds a patrolling
  NPC does exactly that. `deferred_aoi_lifecycle::lifecycle_segments` cancels
  an `EnteredAoI(X)` against the `LeftAoI(X)` that undoes it (along with
  anything said about X in between — the client never needs to hear about an
  entity that came and went while it was not listening), and starts a new
  dispatch segment at an `EnteredAoI(X)` that follows a `LeftAoI(X)`, so the
  leave reaches the client before the re-introduction. The world-entry burst
  is introductions only, stays one segment, and keeps its two-bundle packet
  budget.
- **The hold lifts only when a flush leaves the buffer empty under the same
  lock.** Messages that arrive while a flush is awaiting its sends buffer
  behind it and go out on the next pass. Without this, a live `LeftAoI(X)`
  could overtake the buffered `EnteredAoI(X)` and leave a ghost on the client.
- **A stale timeout cannot release a later hold.** Each hold carries a token;
  the timeout task releases only its own. The case to picture is a player who
  Escs the movie, backs out to character select, creates another new character
  and enters the world again — all inside the first hold's 16 seconds. The
  first hold's timer is still armed when the second hold starts, and without
  the token it would release it early.

### Where the flush lives now

The flush code moved out of the over-cap `cell_dispatch/aoi.rs` into
[`cell_dispatch/deferred_flush.rs`](../../crates/services/src/base/world_entry/cell_dispatch/deferred_flush.rs).
`flush_deferred_aoi` now takes a `trigger: &'static str` — `on_client_ready`
or `cinematic_hold_release` — and the info line reads "Flushing deferred-AoI
buffer" with a `trigger` field. `flush_deferred_self_methods` is the
partial-drain sibling. The two-bundle batching shape is unchanged; only its
address moved.

### What you can see in SigNoz

- **`aoi.cinematic_hold`** (INFO): `event = "hold_started"` with `witness_id`,
  `token`, `hold_ms`; `event = "hold_released"` with `reason`
  (`cancel_movie` | `timeout`), `flushed`, `held_ms`.
- **`aoi.create_emit`** (DEBUG) now actually exports. `OTEL_FILTER` is a named
  constant in `logging/filters.rs` and includes `aoi.create_emit=debug`, pinned by the
  unit test `otel_filter_exports_the_debug_level_aoi_seams`. This is worth
  more than the hold itself: whatever the next repro shows, the per-entity
  emit seam will be in it.

## Alternatives considered

**Re-introduce `class_id 0` entities after the movie.** Send the creates as
normal, then re-send introductions for static meshes once the cinematic ends.
Narrower blast radius, and it does not delay anything the player can see. But
it assumes the client tolerates a second CREATE_ENTITY for a live entity,
which is unverified, and it only covers the entity class we happened to notice.
Still the natural **next** step if the hold does not work.

**Make the packet "more reliable".** Retransmit harder, bundle differently,
send twice. Ruled out by the repro: the packets were reliable, they were ACKed
first try, and only one retransmit occurred in 75 seconds. There is nothing
left to harden on the delivery side.

**Do nothing, and instrument only.** Tempting, since the `OTEL_FILTER` fix is
independently valuable and the hypothesis is n = 1. Rejected because the hold
*is* the experiment: it changes exactly one variable (were the entities created
during the movie?) and the answer is visible on the next playtest either way.
Instrumenting without changing anything buys another repro and no new
information.

## Consequences

- **Player-visible.** On a character's first login, NPCs and other players
  appear when the intro movie ends — immediately on Esc, otherwise 16 s after
  `onClientReady` (the client's "map loaded, send me entities" signal, which
  is also when the movie starts). Returning characters are unaffected.
- **The deferred window gets longer.** The emit-time identity join described in
  [player-ghost-aoi-cascade.md](player-ghost-aoi-cascade.md) now has to survive
  up to 16 seconds rather than a few. It already does — the join reads the
  observee's session when the packet is composed — but any future change that
  moves work to buffer time inherits a much wider staleness window.
- **Two gates now guard the same buffer.** `should_defer` (pre-`onClientReady`)
  and `should_hold_entity_traffic` / `cinematic_hold_active` (the hold) are
  deliberately different shapes, because the hold must *not* park player-self
  method calls. Widening one without the other is the easy mistake here.
- **The 16 s is tied to one asset.** `HOLD_DURATION` is sized for
  `Cine-SGWLogo`. Any future first-login cinematic of a different length needs
  this reconsidered, ideally by deriving it from the BIK header the way
  `cinematic.rs` documents.
- **If the theory is wrong, players waited for nothing.** The cost of being
  wrong is 16 seconds of empty room on one login per character, and a revert.

## Confidence

**Low to medium.** The delivery evidence is strong and well-sourced: the
creates were sent, ACKed, and not retransmitted, so the drop is client-side
and after delivery. That part is not in doubt.

The *cinematic* attribution is a single differential observation across two
sessions, supported by an analogous known mechanism (#288) rather than by
direct evidence that `CollectGarbage` reclaims a foreign static mesh. Nobody
has watched the client's entity list across the movie boundary.

The next step if the hold does not fix it is either re-introducing `class_id 0`
entities after the movie, or querying the client's entity list directly
through the [live research lab](live-research-lab.md). Either way
`aoi.create_emit` will be in the trace this time.
