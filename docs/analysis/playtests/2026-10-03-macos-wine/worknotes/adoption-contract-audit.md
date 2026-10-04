# Existing-user adoption contract audit

Date: 2026-10-04. Read-only evidence at `ba4b20b6b`.
Branch: `analysis/launcher-adoption-contract`.
Worktree: `.claude/worktrees/launcher-adoption-contract`.
No runtime change, compilation, live client access, server or telemetry activity.

## Decision

Implement **verified migration into a separately selected desktop-owned copy**.
Keep the old game tree and old launcher files unchanged. Reuse actual existing
bytes that match authenticated content; this is not a settings-only import or an
unannounced new-install substitute. Present the source and destination separately.
A user who declines copying can retain the current settings-only import and old
launcher. In-place takeover is a later feature: it needs a different deletion and
layout contract and is not the smallest safe path through today's desktop code.

Minimum complete journey: import identity/settings → select separate destination
→ authenticate a chosen release and compare source → review exact differences
and configuration compatibility → confirm copy/normalization → durable verified
publication → prerequisites → Play with preserved effective settings → explicit
signed game-content update preserving identity and a recoverable old desktop tree.
Launcher executable self-update, signing/publishing and original-tree deletion
remain separate. Do not call Repair an update: it deliberately reconstructs the
same retained release.

## Evidence establishing the gap

- `crates/launcher/src/install.rs::adopt_existing_install` checks for an executable
  and missing ledger, then copies the manifest seed hash with `seed_adopted=true`.
  It never verifies installed bytes. `src/state.rs` says this explicitly. Even
  `seed_adopted=false` in an old ledger is not cryptographic file evidence.
- `desktop/engine/src/storage/migration/` preserves exact JSON, identity, ordered
  claims and separate consent; it selects the old game root in preferences but
  creates no desktop content ownership. `desktop/docs/migration.md` accurately
  says settings are archived and not consumed by desktop Play.
- `desktop/engine/src/storage/install_intent/::fresh_destination` refuses a
  nonempty target. `installed_content::verify_installed_identity` requires the
  saved intent, original signed bytes, owner marker and content-ready receipt.
  Its layout is `destination/game`; launch, runtime preparation, Repair and
  Uninstall depend on this. The old root normally contains `Working` directly.
  Therefore the mapping is `old-root/Working/...` → `new-root/game/Working/...`,
  never `old-root/game` invented by relabelling preferences. Preserve any supported
  legacy layout through `install_layout`; ambiguous dual layouts must fail closed.
- `install_worker::content_valid` checks ledger, nonempty executable and containment;
  it is not an exhaustive integrity verifier. Setting `seed_adopted=false` or
  writing a matching ledger does not establish adoption evidence.
- `src/manifest.rs` authenticates artifact hashes, not a per-file installed index.
  Verification must derive the expected tree by extracting authenticated seed and
  patches in signed order, with the correct patch roots, or introduce a separately
  signed file index. Do not hash installed files against an archive hash.
- `src/client_setup/mod.rs` changes stock filename case, login Lua and the PE ASLR
  flag. The original launcher therefore legitimately changes signed extraction
  output. Comparison needs an explicit versioned transformation policy, rather
  than a blanket ignore for executable/Lua files.
- `desktop/engine/src/storage/launch/resource.rs` supports absent client patches;
  shell launch currently requires them. Legacy `enabled=false` and DLL overrides
  cannot silently become the shell default. No imported telemetry identity/consent
  is currently consumed by the desktop game launch path.
- `storage/repair` has retained locks, staged reconstruction, checkpointed promotion
  and backup cleanup. `storage/uninstall` owns the entire destination. Reusing an
  old root as that destination would implicitly grant deletion of unrelated files.
- `OperationKind` contains no Update or Adopt operation at this revision.

Paths above abbreviated `desktop/` and `src/` are rooted at `crates/launcher/`.

## Verification and preservation rules

1. Authenticate and retain original release body/signature with native trust roots.
   Keep imported manifest URL verbatim, but never treat it as a new trust root or
   silently switch it to the built-in catalog. Unsupported custom catalogs get a
   visible compatibility blocker or an explicit reviewed switch.
2. Build a private reference tree from verified artifacts. Derive a bounded,
   deterministic file index (relative path, size, SHA-256, case policy and setup
   policy version). Verify extraction safety, duplicate/case-colliding names and
   all patch roots/order before using the reference. Retained local index is
   derived evidence, not a newly signed publisher assertion.
3. Compare every managed source file to reference variants for exact raw content
   and narrowly defined launcher setup. Generate prepared reference with imported
   login servers; compare full executable bytes after the single known ASLR change,
   not merely a PE header. Reject unexplained executable or Lua differences as
   modifications. Do not apply setup to the source to test it.
4. Classify matched, known-transform, modified, missing and extra paths. Review
   bounded totals plus an inspectable complete report. Never silently enable mod
   code from unknown DLL/Lua files. Copy matching source bytes into private stage;
   use reference bytes for missing/modified managed files only after explicit
   consent. Keep unknown files and originals in the unchanged source, showing that
   they will not become active in the verified desktop copy. Separately migrate
   only explicitly specified user-data paths with tests; until such a policy is
   supplied say that game-local settings/user data remain in the old copy.
5. Hold the actual old executable-adjacent launcher lock throughout source capture
   and source reread. That lock does not prove a game or unrelated editor is idle.
   Require the old game closed, use non-following handles and verify copied bytes
   and source identities; refuse source changes during a reviewed snapshot. No
   hardlinks to source. Reject links/reparse/special files and source/destination
   overlap, including canonical aliases, before copying. Subsequent arbitrary
   mutation of the old copy cannot affect the new independently owned tree.
6. Reverify the completed stage against the derived reference, including explicit
   setup transformations. Its synthesized legacy-compatible ledger describes the
   newly verified/reconstructed copy only; the imported historical ledger and its
   exact bytes remain unchanged and never authorize this receipt.

## Native interface and durable records

Add a dedicated `storage/adoption/` module. Suggested API names are contractual
proposals, not existing methods:

- `preview_adoption(import_digest, native_destination, verified_release, revisions)`
  returns a native-held preview handle and report. Long comparison is retained
  native work with progress/cancellation; it is never done on the command/UI thread.
- `confirm_adoption(work_id, preview_handle, operation_revision,
  preferences_revision, confirmed_choices)` admits exactly the reviewed plan.
  Renderer passes no paths, releases, hash claims or source JSON. Choices are a
  closed enum for normalization and explicitly supported config changes.
- `inspect_adoption`, `cancel_adoption`, `recover_adoption`, `abandon_adoption`
  expose honest native status; lost replies never cause automatic redispatch.

The durable plan binds imported-source digest and legacy install UUID, separate
new desktop ownership UUID, native canonical source/destination, source snapshot
identity, authenticated release digest, derived index/setup version, reviewed
modification policy, effective launch config, backend and exact stage paths.
Historical install identity is not replaced by the desktop operation UUID.
Keep logical user identity, immutable ownership ID and changing release receipt
separate. An explicit schema/provenance variant is required: do not manufacture a
successful first Install record merely to satisfy `installed_content()`.

Use create-new destination ownership and exact operation-owned stages, then a
checkpointed no-clobber promotion. Publish receipt/index, effective preferences
and terminal result with recoverable transitions. No readiness before every
required publication is durable. Capture previous preference revision and use a
compare-and-swap; a changed preference forces review rather than being overwritten.
No source marker/receipt/ledger writes, rename or deletion. Precommit cancellation
leaves the source intact; interrupted stages stay owned and gated. Reopen verifies
retained signed bytes, intent, tree identities and checkpoint combinations before
finishing a publication. Missing paths are not success evidence. Cleanup only
removes exact destination artifacts proven created by this operation.

Uninstall targets the separately created desktop root only. A legacy import by
itself continues to expose no uninstall target. The source remains a fallback;
say "original copy preserved", not "automatic rollback". Subsequent desktop
updates need a distinct Update plan bound to old/new signed release digests,
permanent owner and immutable legacy identity. Stage the new release, present
modification changes, retain the old desktop game as an owned backup through
receipt/terminal durability, and provide explicit checkpoint recovery. Do not
reuse Repair's same-release plan or mutate a permanent InstallIntent in place.
Restoring a previous release is a separate confirmed transition with its signed
receipt and launcher-minimum checks, not a blind backup rename.

## Effective configuration is part of completion

Preserve exact imported record indefinitely; derive a reviewed effective config
for the desktop copy. Consume login-server order/URLs during setup and Play.
Respect patch disablement in shell resource admission (engine already permits
None). An override DLL needs a native-confirmed artifact identity and a supported
policy, or blocks parity; do not silently select the bundled DLL. Cross-platform
paths require explicit native remapping. Preserve telemetry `opted_in`,
`prompt_answered`, auth URL and install identity independently of launcher-summary
consent. `enabled` never becomes opt-in. If the desktop lacks telemetry transport,
display it as unavailable, retaining the requested choice; do not claim complete
telemetry parity. No upload occurs just by importing/adopting. A user may explicitly
accept the displayed unavailable feature, but that is a compatibility exception.

## Exact confirmation copy

"Create a verified desktop copy of your existing game?

Source: {source}
Desktop copy: {destination}/game
Release: {signed release identity}

We will reuse {matched} verified files from your existing game and obtain
{missing_or_changed} files from this authenticated release. The listed modified
files will use release versions in the desktop copy. Extra files and modifications
remain in your original folder and will not be active in the desktop copy. Your
original game, launcher files and historical patch record will stay unchanged.

Your imported identity and the reviewed launch settings will be used for the
desktop copy. Game telemetry: {effective status}. Launcher summaries: {status}.
{explicit compatibility differences}

The desktop launcher will manage and can uninstall only the new copy. Play
requires verification and platform prerequisites to finish. Keep the old launcher
and game closed while copying. This may require {estimated space} additional disk
space and downloads."

Buttons: **Create verified copy**, **Cancel**. Modified-file replacement and any
config exception need visible unchecked consent controls tied to the preview.
If no files are reusable, say so and offer a separately labelled clean-install
choice; do not describe that fallback as successful existing-content adoption.

## Bounded implementation assignments and ownership

1. **Native verified-copy adoption**: new `engine/src/storage/adoption/` modules
   (model, inventory, comparison, worker, commit/recovery and tests), derived
   reference reconstruction seam, explicit installed-content provenance/schema
   and ownership separation. Own adoption-specific docs and unique memory only.
   Requires coordinator integration for `storage/mod.rs`, `lib.rs`, operations,
   `installed_content`, install-worker reusable seam, runtime/launch/repair/uninstall
   consumers. Deliver fixtures through durable publication, no UI button shortcut.
2. **Effective imported settings and desktop flow**: new
   `shell/src/host/adoption/`, `frontend/src/adoption-{view,workflow}` and tests/UAT;
   implement effective config consumption in launch resource resolution and setup.
   Coordinator serializes shared `host.rs`, `main.rs`, contract/app/UI and shell
   launch edits. Include native persistence JS UAT and packaged visual/focus pass.
   This task depends on assignment 1's agreed native DTOs.
3. **Installed game-content Update**: new `storage/update/` with signed old/new
   receipt transitions, staged verification, backup/recovery and explicit rollback
   contract; shell/frontend update surface after native contract. Share reconstruction
   and checkpoint primitives with Repair without changing Repair's meaning. This
   task follows assignment 1's stable permanent identity/release split. Do not
   advertise installed update parity until it passes its own fixture/native gates.

All three are needed for the complete Play/update journey; assignment 1 alone
must not close migration parity. Config unsupported states remain explicit rather
than silently overwriting identity/consent. Coordinator owns shared guide/README,
campaign ledger and index changes; avoid concurrent edits to shared files.

## Required test matrix

- Signed fixture reference: valid/invalid signature, archive digest mismatch,
  missing signing key, wrong patch root/order, renamed same-ID patch, unsupported
  minimum, offline retained evidence and unavailable custom catalog.
- Realistic old root with `Working`, supported flat layout, ambiguous layout,
  `seed_adopted` both values, forged current ledger, duplicate ordered historical
  claims: only actual compared bytes satisfy adoption; old JSON is byte-identical.
- Raw/ASLR/login/case variants; arbitrary executable/Lua edits, missing files,
  unknown DLLs, generated cache/user data and extra directories: exact review and
  policy, preserved source, no unknown executable overlay, final reference match.
- Native-selected paths, overlap/aliases, nested links/reparse points, hardlinks,
  case collisions, special files, source/destination replacement races, disk-full
  and source changes between preview/capture/confirmation/promotion.
- Concurrent legacy lock and desktop owners, game-close limitations, duplicate
  work IDs, stale preferences/operation revisions, preview invalidation, lost
  response, cancellation before commit, uncertain helper exit and no redispatch.
- Fault both sides of every plan/checkpoint/rename/receipt/preferences/terminal
  write, reopen and explicit recovery; source invariant at every fault, retained
  stages/backup, foreign destination refused, no ownership inferred from absence.
- Imported identity/config effective on Play, ordered login servers, patches off,
  unsupported/remapped override, telemetry enabled-without-opted-in, opted-in
  preserved but unavailable transport explicit, summary unchanged, no network send.
- Adoption → reopen → prerequisites → one Play admission/observation → Update to
  another signed fixture → reopen → Repair same current release → Uninstall only
  new destination. Update interruption preserves old game and both signed receipts;
  rollback never bypasses launcher minimum. Real process start is not login proof.
- JS REPL UAT through production Effect + native disk-backed bridge: review/cancel,
  single confirm under duplicate clicks, progress, lost reply/recheck, reopen,
  stale review, effective settings and capabilities. Add separate packaged visual,
  keyboard/focus/native-dialog checks. Native Windows locking/rename/power-loss,
  Wine helper containment and real gameplay remain platform validation gates.

Tests are a design requirement, not executed evidence in this audit. Unit and
filesystem/negative-path persistence tests fit this bug shape; no live DB needed.
All future compiling Cargo runs use the build lane on the allowed native host.
