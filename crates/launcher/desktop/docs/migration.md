# Legacy launcher import

> **Type:** Reference
> **Audience:** Native launcher and shell contributors
> **Companions:** [Launch](launch.md), [desktop README](../README.md), [adoption design](../../../../docs/analysis/launcher-uat-fixes/design-adopt-in-place.md)
> **Last updated:** 2026-10-23

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

Adopting an existing folder fetches only what the review says it needs. The
replay downloads only the patches that the folder's changed files need, each
from its signed blob URL in the release manifest. The seed archive is fetched
only when a needed pre-patch file is wrong, from archive.org through the signed
blob URL or from the player's own archive. Nothing is ever uploaded or
mirrored (D-AI5). The review's download estimate names the patches and the seed
before confirmation.

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
legacy record retains its separate game telemetry choice. Import alone changes
nothing about Play. An adopted installation later launches with the imported
patch setting and login servers, as described under
[Adopting an existing folder](#adopting-an-existing-folder).

After import the UI says historical content is unverified and grants no Play,
Repair or Uninstall capability. To play this installation with this launcher,
adopt the existing folder from Settings → **Use an existing game folder → Choose SGW.exe…** (see
[Adopting an existing folder](#adopting-an-existing-folder)), or select a separate
empty folder for a new desktop installation. Import does not modify legacy game
files, replace the updater, or complete installed/update/launch parity.

Shell tests exercise native-held confirmation, hostile extra fields, changed
sources/preferences, repeat/reopen semantics and absent launch/removal ownership.
`npm run uat:migration` in the frontend consumes the real native host test bridge
(selected by `MIGRATION_UAT_BINARY`) and checks disk-backed identity, config,
ledger and consent after reopening. This supplements frontend tests; native
dialog interaction, packaged visual verification and Windows lock behavior
remain separate gates.

## Adopting an existing folder

**Adopt an install** at the bottom of the main page opens the same workflow as
Settings → **Use an existing game folder → Choose SGW.exe…**. The main-page button appears only
when there is no valid managed or adopted installation. Choosing acknowledges the
first press before opening the native picker; cancellation and errors remain
visible in that workflow. Importing legacy settings is optional and does not
establish installation ownership.

The native file picker is titled **Choose SGW.exe in your existing install**
and filters for `.exe` files. Only `SGW.exe` is accepted; native resolution maps
its `Working/Binaries` parent to the installation root. Cancellation changes
nothing. Selection opens the existing in-place confirmation review before any
installation changes. The IPC remains `choose_existing_game_folder` with
`mode: light`; no new IPC or Plan field is added.
The selected installation stays at that root on Windows and macOS. The launcher
does not bulk copy it, move it into `game`, or ask for a copy destination.

### Modes and content changes

The shell offers only **Light**, and production copy admission returns
`UnsupportedMode`. The copy engine primitive remains test-only for legacy-journal
fixtures. Hidden verified in-place engine tests are separate from the shell's
Light workflow. macOS retains its Wine backend for runtime preparation and Play;
that does not require copying the installation.

The owner approved explicit per-file setup and patch changes, cooked-data
normalization, and replacement backups at the selected root (2026-10-23).
Matched files that need no such change keep their paths and filesystem identities.
Unrelated files stay untouched. Normalizing stock `Cache.en-US` to
`SourceCache.en-us` and backing up a replaced original are content changes, not
permission to relocate the installation.

Light scans the source and plans missing patches by replaying them in scratch
storage. A patch operation whose input differs can be skipped; its closed reason
and files are retained in `incomplete_patches` in `launcher-installed.json`.
The file-index comparison informs the review; it does not turn Light into a full
verification. A successful adoption is not proof of runtime or PhysX readiness.

Preflight still requires a writable game root with `Working/Binaries/SGW.exe`,
no conflicting launcher ownership, and safe source entries. Reparse points,
links, hard-linked files, special files, unsafe paths, and case collisions are
refused. Stock non-ASCII names are allowed. Refusals show bounded
`unsafe_entries` and a reason; they never authorize deleting the source.

### Layout identity and validity

New adoption persists both `Plan.schema_version = 2` and
`InstallIntent.schema_version = 2`. No field is added to the digested `Plan`.
The pinned authenticated plan must match the intent's schema. After existing
owner, provenance, and journal validation, `InstallIntent::content_root()`
resolves schema 2 to the selected root. Schema 1 managed installations and
successful legacy adoptions retain `root/game`; the launcher never moves an
already successful legacy adoption back.

Content validity combines authenticated installed owner/current release evidence
with `install_worker::content_valid`: matching ledger seed hash, no `seed_adopted`
claim, no missing required patches, a contained nonempty regular executable, and
the expected `SGWGame` directory. Runtime/PhysX readiness, launcher minimums,
active Play, and operation admission are separate checks. Neither the conservative
adoption `owned` flag nor `LaunchStatus.installation_id` supplies this predicate.
Unreadable ownership must not authorize adoption over uncertain state.

Authenticated schema-1 installed-content/Uninstall display and read-only
owner/current-release inspection use the historical published-record check,
preserving identity when imported settings are refused or the historical adoption
checkpoint is missing. Owner, release, receipt and record validation still apply;
this display compatibility does not authorize unvalidated mutations. It preserves the historical
launcher-minimum view even when imported settings are refused. It does not admit
those settings or grant Play: `effective_settings::launch_binding` validates them
strictly before launch. Schema-2 inspection additionally requires the authenticated
retained adoption plan; legacy compatibility never bypasses direct-root provenance.

### Recovery, maintenance, and removal

Schema-2 recovery resumes validated per-file work at the selected root. Abandon
undoes the approved per-file changes using retained originals and backups; it
does not relocate the installation. Direct-root Repair and Update also commit
per file, preserving unrelated extras rather than swapping the whole root.
Launch, maintenance, receipt checks, and game-log discovery use the validated
content-root resolver.

All interrupted schema-1 adoption, including legacy copy journals, is refused
recovery/publication: upgrading must never resume relocation or finish a copy. Their abandon escape removes ownership metadata
only, retaining existing game bytes, staged originals, and backups for manual
inspection. It does not automatically move files back. Do not mistake this
legacy escape for schema-2 rollback. The legacy-copy fault fixture covers six
checkpoints, preserving paths, bytes, and file identities through refusal and
metadata-only abandon; it does not validate a real old installation.

Uninstall of a new adopted installation removes authenticated release-managed
files and launcher metadata selectively. It retains the selected root, unrelated
extras, and retained original-file backups. A managed install keeps its existing
managed-layout removal behavior.

### Verification boundaries

The regression plan covers stable file IDs as well as bytes and names, reopen
and launch resolution, direct-root Repair/Update, selective Uninstall, legacy
interruption refusal/abandon, and managed-layout regression. File IDs distinguish
keeping a matched file from copying it back to the same path. See the
[approved design](../../../../docs/analysis/launcher-uat-fixes/design-adopt-in-place.md)
and [follow-up UAT rows](../../../../docs/guides/unified-uat.md#launcher-uat-fixes).

Frontend composition tests and the JS REPL pass exercise button visibility,
first-press feedback, deferred picker cancel/error/busy/success, and durable-state
refresh. They do not verify the physical native picker, packaged visual layout,
macOS filesystem behavior, or a real game launch. No native macOS adoption UAT or
launcher release is claimed by this change. Obtain owner permission before
laptop or lab use.

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
releasing ownership. For schema-1 managed/legacy layout, replacement records Planned → OriginalMoved → Promoted →
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
changed files for review. For schema-1 managed/legacy full reconstruction, the UI
must disclose that existing modifications and game-local user files remain only
in the retained backup; they are not merged into the newly active game.
Schema-2 direct-root adoption preserves unrelated live extras and backs up
replaced outputs during per-file commit. The seedless Update below also preserves
files outside its write set.

### Seedless (incremental) Update: light-adopted installs only

The staging description below retains the managed/legacy `game` layout. Schema-2
adoption resolves paths at the selected root and publishes per file; commit never
swaps that root or removes its unrelated extras.

A light-adopted install already holds most of the target release, so its Update skips
the 4.1 GB seed download (D-LA14).

- **Who takes it.** `incremental::seedless_allowed` is true only when the installed
  record's adoption provenance says `check = light`. A copy adoption (`full`), a legacy
  import and a launcher-downloaded install update exactly as before: same requests
  (the seed included), same `seed_source_for`, strict `install_all` into an empty stage,
  same ledger bytes. Extending the seedless path to every install is a later option
  (D-LA14b) and is not done. The provenance survives the Update, so the next Update of
  a light-adopted install is seedless again.
- **When it declines.** `incremental::plan` returns a plan only if
  The resolved content root's `launcher-installed.json` loads, its `seed_sha256` equals the target's, and its
  `applied_patches` are, in order, a prefix of the target's patch state keys, and the
  previous release verifies. Otherwise the Update runs the full rebuild and logs
  `update_seedless_declined` with `reason = previous_release_unverified` or
  `ledger_mismatch`.
- **Write set and hard links.** The write set is the outputs of every patch not yet
  applied, every path in `incomplete_patches[].skipped[]`, `launcher-installed.json`
  and the setup-rule paths. `stage_seed::seed_stage` builds the stage from the live
  `game\`: files outside the write set are hard-linked (Windows; copied when the link
  fails, and always copied elsewhere, because the stage digest refuses `nlink != 1`
  on unix), and write-set files are copied. A symbolic link or reparse point in the content root
  refuses the stage. The tolerant install then runs over the stage and skips the seed
  because the ledger matches. The live `game\` is not touched until commit; swap,
  backup and recovery are unchanged.
- **Failure.** `update_incremental_failed` (WARN) names a closed `reason`:
  `fetch_failed`, `hash_mismatch`, `write_set_unreadable`, `seed_stage_failed` or
  `rename_failed`. Cancellation is not logged as a failure.
- **Repair is the full rebuild.** Repair always rebuilds from the seed, strictly
  (`tolerate_ops: false`), and never reads `incomplete_patches`.
- **Telemetry.** `update_staged` gains `mode`, `ops_skipped`, `stage_linked` and
  `stage_copied`; the event rows are in
  [telemetry.md](../../../../docs/operations/telemetry.md#launcher-light-adoption-and-update-events).

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
