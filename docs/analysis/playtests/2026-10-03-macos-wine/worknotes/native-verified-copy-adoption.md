# Native verified-copy adoption: bounded ZIP phase

> **Type:** Reference and implementation handoff
> **Audience:** Desktop launcher implementers
> **Date:** 2026-10-04
> **Base:** `ba4b20b6bc97b8cacbdffa78acb6c199f881965f`
> **Branch:** `launcher/native-adoption`
> **Worktree:** `.claude/worktrees/launcher-native-adoption`

## Status and evidence boundary

This phase implements a native, source-preserving verified copy from **already
cached authenticated ZIP artifacts**. It is deliberately not migration parity:
the published RAR/MakeCAB seed, Wine reference extraction, effective launch
configuration, frontend, and game-content Update still need their own phases.
The existing Play admission remains blocked for adoption provenance. Nothing in
this packet launches a game, uploads telemetry, downloads a release, or changes a
server. Historical patch claims never authenticate installed content.

Implementation: `crates/launcher/desktop/engine/src/storage/adoption/`.
The fixture suite exercises real signed ZIP extraction and disk publication,
not a synthetic successful Install receipt.

## Native API and renderer boundary

`adoption::start_preview(Arc<Mutex<DesktopState>>, PreviewRequest)` returns a
retained blocking `PreviewWorker` with cancellation, extraction progress, and a
one-shot `Result<Preview, Error>`. `PreviewRequest` contains the previously
imported source digest, native-selected destination, operation/preferences
revisions, a `VerifiedRelease`, and native cached seed/ordered patch paths.
These native input types must never be deserialized from renderer JSON.

`Preview::report()` returns the complete bounded per-file report, preview UUID,
source and destination, signed-body digest, imported identity, unchanged requested
configuration, and explicit unavailable-telemetry/user-data status. The opaque
Preview retains the source launcher lock and private reconstructed reference.
The host retains this object under its handle; the renderer only returns that
handle and closed choices. Dropping it before confirmation removes its private
reference and releases the source lock without claiming a destination.

`start_confirmation(Preview, work_id, preview_handle, Choices)` consumes that
exact preview once. Choices are `normalize_managed_files`,
`accept_unavailable_game_telemetry`, and `old_game_closed`. Missing/modified files
and known setup transformations require normalization consent. Requested game
opt-in requires explicit acceptance that transport is unavailable. A game-close
assertion does not prove unrelated writers are idle: confirmation rechecks file
identities and whole-file bytes. The retained worker continues if the observer
is dropped. Cancellation is a request; only an observed worker result confirms
it. Do not redispatch after a lost reply.

`inspect(&DesktopState, work_id)`, `recover(&mut DesktopState, work_id, revision)`,
and `abandon(&mut DesktopState, work_id, revision)` expose durable reconciliation.
Use retained blocking workers for recovery too. Missing Staged evidence is an
error, not permission to copy again. Abandon marks a pre-promotion operation
cancelled and deliberately retains its quarantined destination bytes. Safe
artifact cleanup remains a follow-up; it never recursively deletes a guessed
path. Synchronous `preview`/`confirm` exist for native composition and tests; do
not invoke them on the command/UI thread.

## Verification and preservation policy

Policy version 1 accepts ASCII relative paths, at most 200,000 inventory entries,
64 levels, and bounded 32 MiB persisted plans/reports. It rejects links, hardlinks,
special files, case collisions, unsafe ZIP names, duplicate ZIP entries, file/
directory conflicts, and unsupported archives. Canonical source/destination/state
roots cannot overlap. The destination must be absent at confirmation. Existing
empty directories are not taken over.

Every artifact is hashed against its authenticated manifest size/SHA before
private extraction. Patches use manifest order and the existing `patch_dest`
root resolver and unpack/patchset algorithms. Reconstruction snapshots raw
release bytes, applies the existing stock-case/login/ASLR setup with imported
ordered login servers, and derives a final per-file reference. Comparison checks
whole raw/prepared file hashes; unexplained executable/Lua differences remain
modified. Supported full, Working-root, and flat source layouts map into the
separate desktop game layout; simultaneous layouts fail closed.

Matching and raw-known-transform bytes are copied from source through read-only
handles into freshly created files. Missing/modified bytes come from reference
only with consent. Unknown files, game-local settings, cache and mods remain in
the source. The synthesized ledger describes only the reconstructed copy; the
original ledger is an extra file, never reused. Final content is rehashed against
the reference. Exact legacy config/identity/ledger JSON stays in the original
migration archive and remains unchanged on disk.

Only macOS implements this phase's filesystem identity and exclusive rename
policy. Other hosts fail closed. The code rejects observed replacement and
foreign trees; it does not yet provide fully descriptor-relative traversal for
an actively hostile same-user process racing every ancestor lookup. Windows
reparse/locking/no-clobber behavior and power-loss durability remain unvalidated.

## Publication and provenance

The durable plan binds consent, source snapshot digest, imported record/digest,
legacy install UUID, a distinct desktop owner UUID, work UUID, release digest,
setup policy version, full report, stage/destination inode identities, immutable
InstallIntent-compatible layout, and before/after preferences. Ownership is
create-new; the journal operation is **Adopt**, never Install.

After final validation and directory sync, a Staged checkpoint is durable before
macOS `RENAME_EXCL` promotion. Recovery accepts only the captured stage inode in
exactly one of the stage/game slots, verifies every file against the derived
reference digest, and reverifies retained signed release bytes. It then publishes
content receipt, schema-2 installed-content provenance, preference CAS, Published
checkpoint and terminal success. No receipt can bypass an incomplete adoption
checkpoint or nonterminal operation. Preferences changed outside the saved
before/after transition block recovery. Recovery never re-downloads or re-copies.

An interruption before a Staged checkpoint retains a quarantined destination.
An interruption during initial ownership/plan admission can leave an orphaned
create-new destination with no admitted operation; this phase never infers
permission to delete it from missing records. Such early-orphan inspection and
cleanup are still needed before exposing the journey to users.

Legacy identity and config remain distinct from desktop deletion ownership.
Uninstall consumes only the new desktop root. The original copy is preserved;
this is not an automatic rollback system.

## Integration checklist

The feature commit owns only adoption modules, this worknote, and unique project
memory. A separate integration commit declares/re-exports adoption, adds
OperationKind::Adopt, and extends installed-content schema/provenance. The
coordinator owns shared docs/indexes and integration with other workers.

Before integration is usable with the updater worker, insert
`state.ensure_updater_idle()?` at the start of adoption's `idle` admission guard
and before `recover`/`abandon` mutations. That method does not exist at the base;
this packet does not copy the updater implementation. Retain the native guard
before reference preparation or destination creation, not only in the shell.

The current installed-content read-only admission view deliberately returns Busy
for adopted content; this gates existing Play until effective-settings/runtime
integration supplies a separate verified contract. Keep that gate until patch
disablement, remapped overrides, legacy telemetry identity/consent and runtime
backend consumption have fixture coverage. Exposing a UI button is not enough.

The local artifact API has no downloader. A future native producer must bind
cache artifact paths to the selected verified release; renderer URLs/hash claims
are never acceptable. Custom manifest URLs currently block rather than silently
switching to the built-in catalog. DLL overrides block pending explicit policy.

## Enabling the actual published client next

Reuse `install::SeedExtractor` / `SeedExtraction` from
`crates/launcher/src/install_seed.rs`. Downloads and hash checks already support
`install::install_all_with_seed_extractor`; `mac_wine::WineSeedExtractor::prepare`
retains helper/runtime/prefix ownership through `storage::extraction_work` and
tracks uncertain helper exits. Repair preparation demonstrates a retained
handoff. Do not start a new generic extraction framework.

The missing seam has two pieces:

1. Extend `ExtractionWork` to recognize an adoption-reference operation and its
   private stage/cache, carrying the immutable Wine backend identities. A preview
   has no admitted mutating operation today, so this needs its own retained native
   preparation record rather than faking an Install intent or changing permanent
   ownership while a helper runs.
2. Before RAR/CAB extraction, enumerate and reject unsafe/duplicate/case-colliding
   archive paths, cabinet outputs and cross-cabinet conflicts. Preserve raw output
   before client setup so whole-file raw/prepared comparisons remain possible.
   Reuse patch ordering/root/setup algorithms after authenticated seed extraction.
   Propagate uncertain helper exit as reconciliation and retain stage/cache/prefix.

Then prove the same fixture publication/recovery matrix through the real helper
adapter. Native Windows and packaged Mac/Wine tests are separate gates. Do not
label the current ZIP fixture path as working published-client adoption.

## Validation

Run through the build lane, from this worktree:

```sh
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/engine/Cargo.toml --lib storage::adoption
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/engine/Cargo.toml --lib
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/engine/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path crates/launcher/desktop/engine/Cargo.toml --all -- --check
```

The final worker handoff records exact executed counts. Tests cover actual signed
patch order/root reconstruction, artifact mismatch, unsafe archives, forged
historical ledger, raw ASLR and arbitrary executable edits, missing executable,
extras, source edits after preview, legacy lock retention, links/hardlinks,
ambiguous layout, consent, each post-staging publication checkpoint, missing
checkpoint, foreign game slot, stale preferences, Play gating, and uninstall
preserving the original source.

Not covered: RAR/CAB helper adoption, game execution/login, game telemetry,
network download integration, live servers, frontend/JS REPL/visual UAT,
Windows filesystems, real power loss, deterministic disk-full injection, or
comprehensive hostile ancestor-replacement races. No frontend changed.

### Executed native checks

- Adoption fixture filter: **16 passed**, zero failed.
- Full engine library suite: **341 passed, 15 ignored**, zero failed.
- Engine `clippy --all-targets -- -D warnings`: passed.
- Desktop workspace `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- Regression proof: after committing, removed only the source hardlink rejection;
  `hardlinks_and_ambiguous_layouts_fail_closed` failed at its hardlink assertion.
  Restored the committed implementation and reran the full engine library suite
  successfully. The deliberately failed run is not an outstanding defect.

### Permanent ownership / future Update limitation

The new owner UUID is independent of the legacy identity and work UUID, and
adoption provenance is explicit. However, the compatibility owner marker remains
an `InstallIntent` containing the original release digest. This phase therefore
does **not** finish the permanent-owner/current-release schema split required by
assignment 3. Before implementing Update, introduce a permanent owner record
whose identity does not change with releases, and separately authenticated current
release receipts, then adapt Repair/Uninstall/runtime consumers with compatibility
coverage. Do not mutate the saved InstallIntent in place or call same-release
Repair an Update.
