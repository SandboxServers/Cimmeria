# Desktop launcher engine scaffold

> **Type:** Reference
> **Audience:** Launcher contributors
> **Last updated:** 2026-10-04
> **Companions:** [Implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md), [build rules](../../../CLAUDE.md), [test policy](../../../TESTING.md)

This standalone Cargo workspace contains `cimmeria-launcher-engine`, the
platform-independent operation owner for the planned Tauri launcher. It has no
executable, installer, Tauri command adapter or Effect integration yet. It does
not perform installation, repair, removal or game launch.

## Contract

`engine/src/operations.rs` owns a versioned snapshot with a revision and one
current operation. Each operation carries an ID, kind, intent digest and state.

- A matching retry of the current operation returns its snapshot without
  admitting work again. Reusing its ID with a different kind or digest fails.
  Other IDs must supply the current revision. Keep the original request revision
  on transport retries; this is not a history of every previously used ID.
- A nonterminal operation retains ownership. Cancellation records
  `cancel_requested`; only native observation establishes a terminal outcome.
  Actual completion may win a cancellation race.
- Restoring interrupted work records `reconciliation_required`, without resuming
  it. An authoritative filesystem/process inspector must resolve ownership.
- Changes call `Journal::commit` before replacing the published snapshot.
  Failed commits leave the previous in-memory state intact.
- Restoration rejects unsupported schema versions. Revisions stay within
  JavaScript's exact integer range; serialized structs reject unknown fields.

The `Journal` trait requires atomic replacement and durable commits. There is
**no concrete file journal yet**; tests use an in-memory store. Exclusive process
ownership, command serialization, canonical intent validation and digest
calculation remain adapter responsibilities. Future IPC must keep `observe`
and `reconcile` native-only. Terminal success will mean the operation's defined
native result, never inferred login or gameplay readiness.

## Validation

Run from the repository root. Use native Windows for Windows validation; the
user-authorized Mac launcher implementation can validate this portable engine
natively on macOS. All compilation uses the pinned toolchain and build lane:

```bash
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml
bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml -- --check
```

On 2026-10-04, nine tests, clippy and formatting passed on macOS. Tests cover
retries, intent conflict, stale requests, cancellation ownership/races,
immutability of terminal results, commit failure, unresolved restart recovery,
mutex-serialized admission, schema/revision limits and selected JSON tags.
The concurrency guard covers an external mutex, not cross-process exclusion.
Root workspace tests do not cover this nested workspace; an explicit manifest
invocation is required. Windows-native checks have not run.

## Next integration gates

Implement and test the file journal and process lock, including failed writes
and recovery; validate native intents; prove worker dispatch occurs only after
successful persistence; and connect authoritative inspection and Effect services.
Add complete DTO/transition and nested-workspace CI coverage. The existing egui
worker still uses its prior command contract; none of its ownership/preparation
logic has been extracted by this scaffold.

No frontend changed in this packet, so JS REPL and visual UAT do not apply to
these Rust changes. Each subsequent frontend packet still requires both.
The implementation plan's real install/launch, telemetry, platform adapters,
signing, human game UAT and final self-contained startup gates remain open.
