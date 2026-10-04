# Legacy launcher import

> **Type:** Reference
> **Audience:** Native launcher and shell contributors
> **Last updated:** 2026-10-04

The native `DesktopState` import API preserves a legacy launcher's identity,
configuration and historical patch ledger in app-data. It does not establish
content ownership or install/launch readiness. Updater migration and installed
update/launch parity are separate work; this does not complete packet 8.

## Native integration

Use `cimmeria_launcher_engine::migration::{LegacySource, LegacyImport, MigrationError}`.
`LegacySource` contains native-selected `launcher_directory` (the folder beside
the old executable) and `game_directory` (the game root). Both must exist and be
absolute. This explicit mapping accommodates a Windows config path on Mac;
the original `config.install_path` is retained separately and displayed during
confirmation. Do not silently infer a Windows-to-Mac path conversion.

1. Call `DesktopState::preview_legacy_import(&source)` under the command mutex.
   Show source folders, original install path, identity, configuration and the
   ordered ledger. Explain that historical/adopted claims are unverified.
2. Require a visible user confirmation. Pass the returned `confirmation` digest
   and current preferences revision to `import_legacy(&source, &digest, revision)`.
   The engine rereads all three sources under the legacy process lock and rejects
   changed bytes. A changed source requires a fresh preview and confirmation.
3. Refresh preferences and `legacy_import()` after success. Exact repeated imports
   are idempotent, including a response lost after commit. Different imports
   conflict; the engine never replaces the saved identity.

The engine registers migration under storage, re-exports the public migration
API, and calls `state.recover_legacy_import()?` in `DesktopState::open` before
returning it. No new dependency is required. Shell IPC and confirmation
presentation are described under Settings preview and confirmation below.

## Sources and consent

All three files are required: executable-adjacent `launcher-config.json` and
`install.json`, plus game-root `launcher-installed.json`. Config versions 1, 2
and absent-version schema 1 are accepted; identity schema 1 is required.
Missing, malformed, newer-schema, nonregular and oversized sources fail closed.
Each source is limited to 12 KiB and the final record to the existing 64 KiB
storage limit. Large valid imports report `too_large` without changing sources.

The record retains exact UTF-8 source bytes, including whitespace and unknown
fields. Typed views preserve identity metadata, explicit config values, login
servers, DLL override/patch settings, ordered duplicate patch IDs, seed claim and
`seed_adopted`. Missing legacy fields get the old loader's defaults. Unlike the
old schema-1 loader, import does not rewrite the user's telemetry auth URL.

Only `telemetry.opted_in` preserves game telemetry consent. Historical `enabled`
is ignored as consent. Launcher-summary consent is independent: the current
value remains unchanged, including the default false. Import sends nothing and
starts no telemetry service.

## Persistence and coexistence

Preview and import acquire an exclusive OS lock on the actual executable-adjacent
`launcher.lock`. Import holds it through durable completion. An active legacy
launcher returns `busy`. The lock file is retained; it is never unlinked or
truncated. Preview may create it, but never rewrites source JSON.

`legacy-import.json` is an atomic transaction record containing exact source
bytes, typed values, source digest, previous and intended preferences, and a
completion flag. The engine first persists this record, then changes the selected
game folder with a preference revision increment, then marks completion. It never
creates `installed-content.json`, an install intent, or a signed content receipt.
An existing desktop receipt or operation history blocks first-time import.

If replacement is uncertain, or a later transaction step fails, mutations require
reopening. Startup validates the saved record and completes only the exact saved
preference transition while holding the legacy lock. Recovery uses archived
bytes; subsequent changes to source JSON cannot change an admitted transaction.
A divergent preferences record fails closed. A completed import does not overwrite
later preference changes. Windows power-loss durability still needs native proof.

The legacy ledger alone exposes no uninstall target and cannot authorize Repair,
destructive replacement or launch. The import lock guards this transaction only;
any future workflow operating legacy game files must establish its own ownership
and coexistence contract.

## Verification boundaries

The engine fixtures cover schema variants, defaults, exact byte and metadata
preservation, consent separation, ledger ordering, stale revisions, source edits,
conflicting imports, corruption/missing/oversized files, symlinks, idempotence,
reopen, failures before/after atomic replacement, interrupted preference and
completion writes, and absent destructive ownership. A Unix subprocess holding
`flock` verifies real cross-process contention against the legacy lock primitive.
Windows-native lock interoperability, hardware power-loss durability, visual UAT
and actual legacy-user installs remain unverified. Shell/UI confirmation has
additional native fixture evidence described below.

## Settings preview and confirmation

Settings → **Select legacy folders…** opens two native folder dialogs: first the
folder beside the old launcher, then the corresponding game root. Cancelling
either leaves no confirmable preview. The review shows both selected folders,
the original configured path, identity, configuration, separate consent choices,
and the ordered historical ledger. **Confirm settings import** admits exactly
that native-held preview and preferences revision; **Cancel import** discards it.
The webview passes no source path in import IPC. Source edits or settings changes
require a new preview. Unsupported records and an active old launcher show an
actionable error rather than synthesizing identity or ownership.

The Effect workflow suppresses duplicate clicks, exposes immediate pending
feedback, and never retries a confirmation after a lost response. **Recheck
import status** reads the saved native record. A reopening requirement is shown
explicitly. Restart restores the import and preferences from app-data; a preview
itself is session-only. Launcher-summary consent remains unchanged, while the
legacy record retains its separate game telemetry choice. Imported configuration
is archived for future parity work; it is not currently consumed by desktop Play.

After import the UI says historical content is unverified and grants no Play,
Repair or Uninstall capability. Continue using the old launcher for that game
installation until verified adoption is available, or select a separate empty
folder for a new desktop installation. Import does not modify legacy game files,
replace the updater, or complete installed/update/launch parity.

Shell tests exercise native-held confirmation, hostile extra fields, changed
sources/preferences, repeat/reopen semantics and absent launch/removal ownership.
`npm run uat:migration` in the frontend consumes the real native host test bridge
(selected by `MIGRATION_UAT_BINARY`) and checks disk-backed identity, config,
ledger and consent after reopening. This supplements frontend tests; native
dialog interaction, packaged visual verification and Windows lock behavior
remain separate gates.

## Retained verified-copy preparation

The native adoption API prepares a separately owned reference before presenting
file comparisons. `adoption::start_preview_wine` uses the existing authenticated
Windows helper and private Wine prefix; `start_preview` retains ZIP fixture/native
composition support. A nonterminal Adopt operation owns reference preparation,
so another Install, preview or launcher update cannot run beside it. Confirmation
hands off to distinct copy work under the same native mutex. Original files and
the imported JSON remain unchanged.

`list_preparations` and `inspect_preparation` expose retained native records.
Explicit `abandon_preparation` checks the recorded directory identity and helper
quiescence before deleting that private reference. Interrupted helper work stays
in reconciliation and is never replayed automatically. This API is not exposed
in Settings yet and does not enable Play for adopted content.

Signed RAR and RAR/CAB fixtures passed through the real Windows helper under Wine,
source comparison and copy publication in isolated prefixes. That helper predates
the new shared archive preflight. Full published multi-cabinet validation with a
rebuilt helper, effective imported settings, permanent owner/current-release
separation, game Update, native Windows adoption and the UI remain required.
See the [preparation handoff](../../../../docs/analysis/playtests/2026-10-03-macos-wine/worknotes/published-adoption-preparation.md).

## Permanent owner and current release

The original `InstallIntent` and `.cimmeria-install.json` remain immutable owner
identity. `ReleaseIdentity` independently binds retained signed bytes by evidence
UUID and manifest digest. Installed-content schema 3 carries this current release
beside the original owner and optional adoption provenance; schema 1/2 records
continue deriving the release from the original intent. Content-ready schema 2
binds the permanent installation UUID to the current release. A mismatched owner,
index or receipt is refused.

Repair binds its plan and extraction to the current signed release, and republishes
that receipt. Uninstall retains the permanent owner UUID, checks the current
receipt before detaching, and preserves both releases' signed evidence. Existing
plans omit the optional current-release field, preserving their serialized hashes.
Play/minimum and prerequisite inspection obtain the current authenticated release
through the installed-content reader. The original backend and setup inputs remain
unchanged by this storage contract.

Two-release filesystem fixtures exercise reopen, Repair admission/extraction
identity, receipt mismatch refusal and uninstall. They directly construct the
post-publication state. A signed local-seed test also reconstructs and commits
Repair against a distinct current manifest, preserving the original owner and old
backup. These tests do **not** prove an Update transition, Update replacement or
rollback, effective imported configuration or UI. Those earlier fixtures did not
exercise the Update journey described below.

### Update admission

`admit_update` now creates a distinct `OperationKind::Update` plan from native
confirmed inputs: permanent installation UUID, expected current release and an
already authenticated target. It checks launcher minimum, operation revision,
owner identity and unused work/evidence paths before saving the target's original
signed bytes and the plan. Admission changes no game files or content receipts.
An identical retry returns the plan without dispatch; reopening leaves the
operation in reconciliation and reverifies both signed releases. A different
current release, conflicting UUID or occupied artifact path is refused.

### Retained Update execution and recovery

`update::preparation::prepare_native` (Windows) and `prepare_wine` reconstruct the
authenticated target release in an operation-owned stage. They retain root/work
ownership, publish progress and accept durable precommit cancellation; the Wine
handoff also retains its owned adapter. Preparation preserves the existing game.
A derived fingerprint binds staged paths and file contents to the plan and is
rechecked before promotion/publication. It is not a publisher-signed file
inventory or a review of modifications in the old game.

`update::commit::commit_native` / `commit_wine` consume the prepared handoff without
releasing ownership. Replacement records Planned → OriginalMoved → Promoted →
Published checkpoints, moves the old game to its owned backup, promotes the stage,
then publishes the content-ready receipt and installed-content index. These are
separate renames and writes. Cancellation cannot interrupt commit after entry;
losing the observer does not abort the retained worker. Failure preserves evidence
and requires reconciliation. Success retains the backup and does not prove runtime
readiness.

Explicit `update::recovery::recover_*` finishes only a validated checkpointed
replacement. `update::abandon::abandon_*` cancels eligible pre-checkpoint work while
preserving game and stage content. Confirmed `update::discard::discard_*` removes
the owned preparation directory after terminal cancellation/failure; interrupted
nonterminal preparation must first be abandoned. `update::cleanup::cleanup_*`
separately removes the current successful Update's backup through resumable
deletion checkpoints; `cleanup::status` inspects that backup. Native evidence,
operation identity/revision and Wine process-ownership checks gate these actions.
Missing or conflicting evidence does not authorize success or deletion. These are
internal APIs: a UI must separately confirm destructive consequences before
calling them. `update_plan()` and the operation snapshot provide durable status.

`admit_update_rollback(completed_update, id, revision, expected_current, confirmed)`
creates another confirmed Update to reconstruct the prior authenticated release
from retained signed evidence, checking that release's launcher minimum again.
It never promotes the old backup or trusts modified backup files as release
content. The permanent installation owner, original setup inputs and adoption
provenance remain unchanged; current-release receipts change through publication.
Uninstall recognizes completed Update/Repair auxiliary directories only against
their durable plans, owner and checkpoint/role evidence.

Settings now mounts the IPC/Effect Update journey. Confirmation binds the
reviewed old/new signed identities, permanent directory and operation revision;
a fresh native inspection invalidates stale reviews before admission. The shell
retains preparation through commit independently of the IPC reply. It exposes
progress/cancellation, explicit recovery, abandonment, partial-file cleanup,
backup cleanup and separately confirmed signed rollback. These actions remain
subject to native checkpoint and ownership validation. It does not enumerate
changed files for review. The UI must disclose full reconstruction:
existing modifications and game-local user files remain only in the retained
backup; they are not merged into the newly active game.

Native Mac fixtures exercise two distinct signed HTTP seeds through Update,
reopen, Repair of the new release and Uninstall; both sides of both renames and
both receipt/index publications; cancelled and lost observers; foreign ownership
and modified-stage refusal; explicit abandonment/discard/backup cleanup; and
separately confirmed signed rollback. Adoption provenance and exact imported JSON
are covered by a separate native fixture. These tests do not establish native
Windows locking, rename or power-loss behavior, original-client Wine reconstruction
or packaged visual/focus behavior. The host integration tests exercise confirmed Apply, duplicate dispatch,
cancellation, interrupted publication recovery and backup cleanup.
`npm run uat:game-update-apply` drives the mounted production Effect view against
a real host with a signed inert archive, loopback download and real replacement:
confirmation, lost-reply inspection without replay, cleanup and store reopen pass.
This JS pass does not execute rollback, platform-specific helpers or the actual
game, and does not replace packaged visual UAT. See the
[native Update worknote](../../../../docs/analysis/playtests/2026-10-03-macos-wine/worknotes/game-update-native.md).
