# Legacy launcher import

> **Type:** Reference
> **Audience:** Native launcher and shell contributors
> **Last updated:** 2026-10-12

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
nothing about Play. A verified adopted copy later launches with the imported
patch setting and login servers, as described under
[Adopting an existing folder](#adopting-an-existing-folder).

After import the UI says historical content is unverified and grants no Play,
Repair or Uninstall capability. To play this installation with this launcher,
adopt the existing folder from Settings → **Use an existing game folder…** (see
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
in reconciliation and is never replayed automatically.

Signed RAR and RAR/CAB fixtures passed through the real Windows helper under Wine,
source comparison and copy publication in isolated prefixes. That helper predates
the new shared archive preflight. Full published multi-cabinet validation with a
rebuilt helper and game Update of an adopted copy remain required. Windows
adopts in place instead of copying (see below).
See the [preparation handoff](../../../../docs/analysis/playtests/2026-10-03-macos-wine/worknotes/published-adoption-preparation.md).

## Adopting an existing folder

Settings → **Use an existing game folder…** opens one native folder picker. Pick
the game root, its `Working` folder or its `Working\Binaries` folder; the launcher
resolves all three to the same root. No old-launcher import is needed. The import
is optional ("Optional: bring settings from the old launcher"); if it names the
same folder, its settings are used. Decision D-LX12 makes in-place adoption the
default on every host other than macOS, so a multi-gigabyte install is neither
copied nor downloaded a second time.

**Modes.**

- **Windows and Linux** offer two choices. **Adopt it in place** moves the release
  files into `game\` inside your folder. **Make a verified copy** asks for a second
  folder and writes the copy to `<chosen>\Stargate Worlds`; your folder is untouched. The two modes are rows with a selected state, and the choose
  button names the selected mode; the native picker that follows is the confirm step
  (D-AU8).
- **macOS** offers the copy only. The copy is extracted by the build-pinned Windows
  helper under Wine, so the adopted installation records the Wine backend that
  prerequisite setup and Play require.
- A folder the launcher cannot write to (for example one under Program Files) is
  refused for in-place adoption before any download, and the copy is offered instead.

**Preflight (before any download).**

- The folder must hold `SGW.exe` in `Working\Binaries` (`not_a_game_folder`). In place,
  the folder must be writable (`folder_not_writable`): the check creates and deletes a
  probe file in the root and renames one inside `Working\Binaries`.
- A picked `Working` or `Binaries` folder is resolved to its root first. An old
  launcher import whose recorded game folder points straight at `Working` or
  `Binaries` cannot be adopted in place and is refused once (`unsupported_layout`).
- A `game` folder, `.cimmeria-install.json` or `content-ready.json` already in the
  folder is refused (`leftover_launcher_files`). The exception is what an admission
  leaves when it dies before its operation begins, or what Abandon leaves when it
  cannot remove `game`: an owner marker that no journal operation names, and an
  empty `game`. The launcher removes those, logs a warning and continues.
- The folder must not contain the old launcher's own folder.
- An out-of-date folder is not refused. Files that only lack a patch are listed as
  needing patching.
- A folder that holds a link, a junction or another reparse point (including OneDrive
  and other cloud-files placeholders), a hard-linked file, a special file, an index name
  that is not ASCII, over 4096 bytes or holding `:` or `\`, or two names equal apart from
  case, is refused (`unsafe_file`), anywhere in the folder tree. On a refusal, a
  read-only survey (`engine/src/storage/adoption/survey.rs`) names up to five offenders
  in `AdoptionStatus.unsafe_entries` (`reason`, a `/`-separated path relative to the
  folder, `"."` for the folder itself, `null` when the folder's own location is not
  canonical) and a `total`. It reads Windows reparse tags without recalling cloud files,
  never descends into an offender, stops at the scan's 200 000-entry and 64-level
  bounds, and warn-logs at most 100 offenders under target `adoption` with relative
  paths only. A survey that finds nothing leaves the generic `unsafe_file` text (a
  race). Owner questions OQ-1..OQ-3 of the
  [launcher-adopt-ux README](../../../../docs/analysis/launcher-adopt-ux/README.md)
  keep these refusals hard for now.

Three fields explain a review and its refusals. `AdoptionStatus.last_rejection` names the check that stopped a worker (the preview or the confirmation): `check`, the file (`entry`), `expected`, `found` and `http_status` for a failed download; it is `null` when no worker refused. `Review.index_fallback` is present only when the review fell back from the published file index, and gives `reason`, `check`, `http_status` and `seed_bytes`: why the signed seed archive is downloaded instead. `Progress.item` names the step in progress (`seed`, `patch`, `file_index`, `index_build`, `reference`) and is absent for the copy phase and for an unknown label. A refusal that a command returns directly is logged as `adoption_rejected` with its check, and is not stored in `last_rejection`; the command's error code is its response.

**Verification.** With a published release index, the launcher fetches
`file-index.json` first (paths, sizes, SHA-256) and checks it against the manifest's
hash. It keeps the index in the state root as `file-index-<sha256>.json`, so a second
review does not download it again. If the manifest publishes no index, or the index
does not match its hash, the launcher logs the reason and builds the same index from
the seed. Every release file in the folder is compared by size and SHA-256 and
classified:

- **matched**: no action.
- **known launcher change**: the ASLR flag, a stock file's case, or `LoginInternal.lua`.
  The launcher re-applies its own setup change; this is the only accepted difference.
- **needs patching**: a stock file that lacks the current patches. It is updated
  without extra consent.
- **modified or missing**: replaced from the release only after the player ticks the
  unchecked box in the review. Without it, confirmation is refused
  (`consent_required`) and nothing is admitted. A release file that cannot be
  rebuilt still blocks the review (`verification_failed`).
- **extra**: a file the release does not list. It is left alone.

**Downloads.** With a published index, only the patches the changed files need are
downloaded, each from its signed blob URL. The seed is fetched only when a needed
pre-patch file is itself wrong. The review shows the download, or "Uses your game
archive" when the player's own archive covers the seed. The seed comes only from
archive.org (through the signed blob URL) or the player's own archive. Nothing is
uploaded or mirrored (D-AI5). The needed files are rebuilt into a private reference,
the same preparation a verified copy uses.

**Copy consent.** In copy mode the review shows the replacement box, but the engine
does not require it, because the source folder is never changed. The copy gets the
signed versions; your folder is not changed.

**Before you confirm in place.** The review warns that `SGW.exe` and every release
file move into `game`, so desktop shortcuts and the old launcher stop working; start
the game from this launcher afterwards. Do not run the old launcher on this folder
again, because it may download the game a second time. Files you added inside the
game folders, such as PAKs placed by hand in `SourceCache.en-us`, stay where they are
and the game no longer uses them. Settings and the client cache in Documents are
not affected.

**Light mode (in place).** Light adoption reads no seed. It scans your folder, plans which patches the folder does not yet carry, and replays them over the folder's own files in a scratch tree. The replay is tolerant per operation: an operation whose source file differs from the signed one (a modded file, a different build) is skipped and your file stays as it is, and the patch is still recorded as applied. Each such patch is also listed in `incomplete_patches` in `launcher-installed.json`, with the skipped files and a closed reason code. Nothing in your folder blocks a light adoption; only an empty folder is refused (`no_reusable_files`). The published file index is compared and shown but never decides the outcome. The adoption's provenance records `check = light`; its digest covers the folder's files with the replacements, moves and setup writes applied, so a recovered adoption verifies the same way. Every other adoption keeps the full check and writes no `check` key.

**Admission and moves (in place).**

1. **Admission.** Confirmation first opens `Working\Binaries\SGW.exe` exclusively. A
   sharing violation means the game is running, and confirmation is refused with
   nothing admitted (`game_running`). Admission then writes the owner marker
   `.cimmeria-install.json`, creates an empty `game` folder, and saves the plan and a
   checkpoint before the Adopt operation begins. If admission fails before the
   operation exists, the marker and the empty folder are taken back.
2. **Moves.** Admission first copies every rebuilt replacement into
   `.cimmeria-adopt-<work>/replacements/` inside the folder. A modified original
   moves into `cimmeria-adoption-backup-<id>` and a stock original that needed
   patching moves into `.cimmeria-adopt-<work>/originals/`, so no player file is
   overwritten; the replacement is then copied into `game`. A missing file is copied
   in. Each verified release file is renamed into `game` at the same relative path,
   the launcher's setup and a synthesized ledger are written, and the tree is checked
   against the signed reference digest. Every file outside the release stays where it
   was, including the old launcher's `launcher-installed.json`. Folders the moves
   emptied are removed. Admitted moves are not cancelled part-way.
3. **Publication.** The receipt, the installed-content index (with adoption
   provenance) and the preferences are written as for a verified copy. From then on
   Play, Repair and Update treat the folder as owned: its `game` tree is managed
   exactly like a fresh install's.

**Backups.** After publication `cimmeria-adoption-backup-<id>` is never removed by
the launcher, not even by Uninstall; `.cimmeria-adopt-<work>` is removed once the
adoption is published. Abandon empties the backup by moving the originals back, then
removes the empty folder.

**Recover.** An interruption after admission leaves the operation in reconciliation.
Recover finishes the remaining moves and publication. A file in neither place is
refused. The old launcher or a running game can recreate a file at its old path after
the review, so a file in both places is settled by content. Admission saves each
release file's reviewed hash (`adoption-reviewed-<work>.json`). If the copy in `game`
matches, the other copy stays where it is, as a file the release does not list. If
only the old-path copy matches, it replaces the copy in `game`, and the `game` copy is
kept beside the old path as `<name>.adopt-conflict-<owner>`. If neither matches,
recovery stops and names the file in the log (`duplicate_release_file`). Nothing is
deleted in any case. A crash during verification leaves only the private reference,
which the existing preparation removal clears.

**Abandon.** Offered only while the operation is still staged, meaning nothing but
renames has happened. Abandon moves every release file back to where the review found
it, moves each replaced original back out of `cimmeria-adoption-backup-<id>`, removes
the replacements and the marker. A file in both places keeps the folder's copy, and
`game` keeps the other. Once every file is in `game`, the checkpoint advances before
the launcher's setup rewrites `LoginInternal.lua` and the ledger. Putting the files
back after that would leave the setup in them, so only Recover is offered then.

**Uninstall.** Uninstall of an adopted folder never detaches or deletes the folder. It
removes only the entries the launcher manages there: `game`, its Repair and Update
work, `content-ready.json`, and the owner marker last. The backup folder stays. Files
the release does not list stay too, so the folder itself is removed only when nothing
else is left in it. Files the client writes inside `game` after adoption, such as
`SGWDebugLog.log`, go with it, as on a fresh install.

**Verified copy: what happens to the source.** The copy runs on a retained worker
and never changes the original game folder, the old launcher folder or
`legacy-import.json`; all three stay byte-identical. The launcher creates a new
`Stargate Worlds` folder inside the chosen folder and refuses an existing folder of
that name. The signed release is downloaded with a size bound and extracted into a
private reference, with Progress and Cancel shown. Changing the diagnostics preference
during a copy makes publication refuse the stale preferences; the copy then needs
Abandon and a new location. An interrupted copy offers Recover when its verified files
are staged, otherwise Abandon. Nothing is replayed automatically, and a destination is
never deleted.

After publication a Wine-backed copy is offered prerequisite setup like a fresh
install, and Play once prerequisites succeed. Its imported patch setting and login
server order are bound as the [launch contract](launch.md#patch-policy-per-installation)
describes. Imported identity, the game telemetry answer and its auth URL are preserved
unchanged; this build still sends no game telemetry.

**Tests and UAT.** Engine tests in `engine/src/storage/adoption/in_place_tests.rs` and
`in_place_recovery_tests.rs` cover rename-not-copy adoption, uninstall that keeps
unlisted files, recovery and abandonment after
a crash mid-move, a release file recreated at its old path, a running game, and a crash
between verification and the marker. `in_place_replace_tests.rs` covers consent,
the backup, a missing file, and recover or abandon after a crash between the backup
move and the replacement copy. `folder_tests.rs` adopts a folder with no import, and
`verification_tests.rs` and `replay_tests.rs` cover classification and the minimal
download. `shell/src/host/adoption/in_place_tests.rs` drives
the same flow through the production host. From `crates/launcher/desktop/frontend`,
`npm run uat:adoption` (with the shell test binary in `ADOPTION_UAT_BINARY`) drives the
production Effect workflow; on Windows it runs `adoption-in-place-uat.mjs` against the
`host::adoption::uat_bridge_in_place` bridge. With `CIMMERIA_WINE_HELPER`,
`CIMMERIA_WINE_HELPER_SHA256` and `CIMMERIA_WINE_RUNTIME_TREE` set, the copy path
extracts through the pinned helper under Wine. Every archive in these checks is a small
inert fixture; no game is started. Native Windows power-loss behavior and the
adopted-folder Play, Repair, Update and Uninstall views are checked by the LX-26
Windows UAT (G-06, G-10). `survey_tests.rs` and `unsafe_source_tests.rs` cover the
reasons, the caps and the junction and hard-link refusals; the shell's `in_place_tests.rs`
checks `unsafe_entries` end to end; scenario 0 of `adoption-in-place-uat.mjs` plants a
hard link and checks the on-screen list.

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
