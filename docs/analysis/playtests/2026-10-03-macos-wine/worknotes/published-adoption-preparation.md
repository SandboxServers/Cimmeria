# Published-client adoption reference preparation

> **Type:** Reference and implementation handoff
> **Audience:** Desktop launcher implementers
> **Date:** 2026-10-04
> **Branch:** `launcher/published-adoption`
> **Worktree:** `.claude/worktrees/launcher-published-adoption`
> **Base:** `3be16f5c7`

## Native contract

`adoption::start_preview_wine(state, request, HelperResource)` prepares the
reference through the existing `WineSeedExtractor` / `SeedExtraction` adapter.
The native resource binds the helper and pinned runtime identities; renderer
input cannot select either executable. Seed and ordered patch inputs remain
local authenticated artifacts. There is no new downloader or extractor.
`start_preview` retains the native ZIP path for fixtures/native composition.
Windows adoption still fails closed pending its separate filesystem contract.

Before reference extraction, a durable `PreparationRecord`, independent owner
lease and nonterminal **Adopt** operation retain private stage/cache ownership.
The record's `InstallIntent`-shaped descriptor is only an adapter identity: it
creates no Install operation, installed intent file, receipt or permanent owner.
The preview UUID is its operation ID. Another preview, Install or launcher
updater cannot run beside it. Initial admission and explicit recovery/abandonment
check updater idleness under the state mutex.

The source lock and reviewed Preview remain native-owned. Confirmation validates
that retained preparation, finishes it and admits the distinct copy Adopt work
under one state mutex. Whole-file raw/prepared comparison, source revalidation,
normalization consent and unchanged imported JSON remain in force. The copied
installation records the selected backend identity.

`list_preparations` discovers retained records, including orphaned handoffs.
`inspect_preparation(state, id)` returns their native descriptor.
`abandon_preparation(state, id, revision)` is a **blocking native-worker API**.
It acquires the independent lease, checks the exact saved directory identity,
rejects links/hardlinks/special files, and removes only that private reference.
Wine cleanup first proves host absence and stops the exact owned prefix with the
existing recovery machinery. A durable cleanup checkpoint allows explicit retry
after partial deletion or a terminal-journal failure. Missing directories without
that checkpoint are refused. Runtime/prefix/helper evidence remains retained.

A known completed preview dropped without confirmation removes its private
reference and cancels preparation. A helper preparation/extraction failure is
conservatively retained as reconciliation; it is never automatically replayed.
If Drop cannot acquire the state mutex, the record stays retained for explicit
cleanup rather than recursively taking the mutex. A crash before any preparation
record exists can leave an empty temporary directory; no extractor has been
admitted then, and cleanup never guesses ownership from a name.

ZIP preflight now accepts ordinary explicit directory entries and rejects case
collisions in implicit ancestor directories before extraction. This corrects the
older policy that rejected a standard trailing directory slash. RAR/MakeCAB
preflight is the coordinator's shared unpack change, not a duplicate extractor.

## Validation and boundaries

Tests use disk-backed signed fixtures and the real production native adapters.
No DB, server, telemetry, user's game/prefix or graphics settings were accessed.
No frontend changed, so no frontend JS/visual UAT is claimed.

Executed from this worktree through the build lane:

```sh
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/engine/Cargo.toml --lib storage::adoption
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/engine/Cargo.toml --lib
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/engine/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path crates/launcher/desktop/engine/Cargo.toml --all -- --check
```

The focused suite passes **23 tests**, with **2 ignored platform fixtures**.
The full-suite final count and regression proof are recorded below at handoff.

The two ignored fixtures were separately run with `CIMMERIA_WINE_HELPER`, its
independently supplied build SHA (`CIMMERIA_WINE_HELPER_SHA256`), and a read-only
cached runtime source (`CIMMERIA_WINE_RUNTIME_TREE`). Each copied that runtime to
its own state root and created its own headless prefix:

```sh
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/engine/Cargo.toml --lib storage::adoption::preparation_tests::retained_wine_reference -- --ignored --test-threads=1
```

**2 passed:** a signed stored-RAR fixture and a signed RAR containing a valid
uncompressed cabinet plus installer INF. Both go through the Windows-native
helper, cabinet.dll where applicable, source whole-file comparison, confirmation
and durable copy publication. Both assert unchanged source snapshots. The cabinet
is independently constructed fixture bytes, not a full published MakeCAB set.
The helper SHA was
`d0c89fad444cb4dc6478f1db8a5e62bc54d5696ee2a84a63d740bf3a5b92c6a3`.
That helper predates the coordinator's archive-preflight change: the rebuilt
Windows helper must pass its own extraction/preflight gate before user exposure.

Ordinary tests cover mutual exclusion, retained owner locking, helper no-replay,
active-host refusal, reopen cleanup, orphan discovery, stale revisions, foreign
directory replacement, nested links and missing-cleanup-evidence refusal. They
supplement the existing source/consent/publication/recovery matrix.

Not covered: the full published multi-cabinet seed, native Windows adoption,
actual game execution, power loss, hostile same-user ancestor races, effective
imported configuration, owner/current-release schema separation or UI adoption.
The original copy-admission destination orphan limitation remains separate from
this reference-preparation cleanup. No claim of complete migration parity.

## Integration ownership

The feature commit owns adoption files and this worknote/project memory.
The separate integration commit extends `storage/extraction_work`, exposes a
bound helper-record read and factors the existing Wine recovery gate for
reference descriptors. Keep the coordinator's early updater guards. No archive
helper protocol or extraction algorithm is introduced by this packet.

Coordinator-owned follow-up: link this worknote from the campaign ledger/shared
indexes and update the desktop migration contract/guide with these native APIs;
then integrate owner/current-release separation and effective settings/UI.
