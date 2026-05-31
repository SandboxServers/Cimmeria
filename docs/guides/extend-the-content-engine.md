---
title: How to extend the content engine (quickstart)
type: how-to
audience: engineers (first content-engine extension)
last_updated: 2026-05-27
companion_docs:
  - ../content/content-engine.md
  - ../content/extending-the-engine.md
  - ../content/proposed-extensions.md
  - ../../TESTING.md
---

# How to extend the content engine — quickstart

This is the **1-page entry point** to extending the data-driven content engine. The full reference is [`docs/content/extending-the-engine.md`](../content/extending-the-engine.md) — read that for the detailed walkthrough with a worked example. Use this page to orient yourself and pick the right extension shape before you dive in.

If you've never touched the content engine before, **first read** [`docs/content/content-engine.md`](../content/content-engine.md) (the runtime reference — vocabulary, schema, lifecycle). This guide assumes you know what `Trigger`, `Condition`, `Action`, `ExecutionContext`, and the bridge are.

---

## What the content engine is, in two sentences

A runtime that interprets database rows as game logic. Instead of writing Python (or Rust) for every "when X, if Y, do Z" pattern, you express it as `trigger / condition / action` rows in PostgreSQL and the engine runs it.

That's roughly 60% of the original game's hand-written scripts and 100% of the auto-generated mission steps. Most non-combat content lives here.

---

## Pick the extension shape

The engine extends along three axes. Pick the one that matches your need:

| You want to… | Add a… | Where the work goes |
|---|---|---|
| React to a new gameplay event (player did X for the first time, NPC arrived, timer fired) | **Trigger** | `crates/content-engine/src/triggers.rs` + a populator that fires the event |
| Gate an existing chain on new state (player has buff X, faction standing, mission repeats) | **Condition** | `crates/content-engine/src/conditions.rs` + a context-key populator |
| Cause a new effect when a chain fires (set a flag, grant currency, trigger UI, open a vendor) | **Action** | `crates/content-engine/src/actions.rs` + an executor arm |

**Spanning multiple axes?** That's fine, but list each as a separate task. A "gate on item count" feature might need both a condition and a context populator that writes `item_<id>_count`.

---

## The four-file pattern

Every extension touches the same four file families:

1. **Variant declaration** — the enum that names the new shape ([`crates/content-engine/src/actions.rs`](../../crates/content-engine/src/actions.rs), `triggers.rs`, `conditions.rs`).
2. **Loader arm** — parses a DB row into the variant ([`crates/content-engine/src/loader.rs`](../../crates/content-engine/src/loader.rs)).
3. **Executor / evaluator arm** — does the work (actions) or returns a bool (conditions) or fires the event (triggers). For actions, this is in [`crates/services/src/cell/content/executor.rs`](../../crates/services/src/cell/content/executor.rs).
4. **Tests** — unit tests next to the executor + a chain-replay test in [`chain_replay_tests.rs`](../../crates/services/src/cell/content/chain_replay_tests.rs).
5. **Seed SQL or migration** — [`db/resources/Content/Seed/`](../../db/resources/Content/Seed/) for new seed content, or [`db/scripts/`](../../db/scripts/) for a runtime migration on existing databases. See [`write-a-database-migration.md`](write-a-database-migration.md).

---

## The mental model

Once you have the four-file pattern, here's the data flow at runtime:

```text
DB row (chain_actions, chain_conditions, chain_triggers)
    │
    ▼
loader.rs::convert_action(row) → Option<Action>
    │ (boundary validation here — i32 range, enum parsing, etc.)
    ▼
Action variant stored on a Chain
    │
    ▼ (trigger fires for this chain)
    ▼
executor.rs match arm for that variant → does the work
    │ (looks up entities, mutates state, sends Mercury messages)
    ▼
Side effects in-game
```

**Boundary validation goes in the loader.** Out-of-range values produce a `warn!` and `return None` (drop the action). Don't propagate raw DB values into the variant — that defers the failure to the executor, where it's much harder to log usefully.

---

## Don't break what's there

Three rules that trip people up:

1. **`Option<T>` for forward-compat.** Use it for any field a future seed might omit. The chain replay test will pin existing behaviour, but the loader must tolerate older rows.
2. **List fields use `Vec<T>` typed by sub-action.** See `StartMinigame.on_victory_chains` for the canonical pattern.
3. **Idempotency.** Actions can re-run if the chain is re-triggered. If your action grants currency or items, it must guard against double-application (most existing actions check a flag on `ExecutionContext` or use the `chain_completions` table — see existing examples).

---

## When something doesn't fit

Some "I want to extend this" requests are actually a wire-format feature, not a content-engine feature. Quick triage:

- "When the player opens a vendor, run my code" — content engine. Add an `OnVendorOpen` trigger.
- "Add a new message the client sends" — wire protocol. See [`add-a-message-handler.md`](add-a-message-handler.md).
- "Send a new server-to-client message" — wire protocol. Same.
- "When a mob takes damage, fire my logic" — content engine. Existing `OnEntityDamaged` (or add a new trigger if the shape doesn't fit).
- "Persist a new column on the player" — schema change. See [`write-a-database-migration.md`](write-a-database-migration.md).

If your feature is "trigger / condition / action" shaped, it's content-engine work. If it's "the client sends bytes, we decode them and reply with bytes," it's wire-protocol work.

---

## Now read the full guide

The detailed walkthrough — with the `Action::ChangeStat` worked example, the boundary-validation patterns, the test recipes — is in [`docs/content/extending-the-engine.md`](../content/extending-the-engine.md). Open it now.

---

## See also

- [`../content/content-engine.md`](../content/content-engine.md) — the runtime reference (vocabulary, schema, lifecycle, observability).
- [`../content/extending-the-engine.md`](../content/extending-the-engine.md) — the **full** how-to with worked example.
- [`../content/proposed-extensions.md`](../content/proposed-extensions.md) — justified extensions on the roadmap; check before proposing a new one.
- [`add-a-message-handler.md`](add-a-message-handler.md) — for wire-protocol work that *isn't* content-engine shaped.
- [`write-a-database-migration.md`](write-a-database-migration.md) — when you need to seed new content rows.
- [`../../TESTING.md`](../../TESTING.md) → "Chain-replay" type — the test shape that pins chain behaviour against regressions.
