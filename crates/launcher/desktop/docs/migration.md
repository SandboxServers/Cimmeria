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
[Verified-copy adoption in Settings](#verified-copy-adoption-in-settings).

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
in reconciliation and is never replayed automatically.

Signed RAR and RAR/CAB fixtures passed through the real Windows helper under Wine,
source comparison and copy publication in isolated prefixes. That helper predates
the new shared archive preflight. Full published multi-cabinet validation with a
rebuilt helper and game Update of an adopted copy remain required. Windows
adopts in place instead of copying (see below).
See the [preparation handoff](../../../../docs/analysis/playtests/2026-10-03-macos-wine/worknotes/published-adoption-preparation.md).

## Verified-copy adoption in Settings

On macOS, Settings offers adoption after a legacy import. The copy is extracted
by the build-pinned Windows helper under Wine, so the adopted installation
records the Wine backend that prerequisite setup and Play require. Windows
adopts the imported folder in place instead; see
[In-place adoption on Windows](#in-place-adoption-on-windows).

1. **Choose a location.** The launcher creates a new `Stargate Worlds` folder
   inside the chosen folder. An existing folder of that name is refused.
2. **Preparation.** The signed release is downloaded with a size bound and
   extracted into a private reference. Progress and Cancel are shown. A second
   choice during preparation is refused, and Dismiss stops the download.
3. **Review.** The view lists source, destination, signed release, file
   classification counts with the first 60 differing paths, ordered login servers,
   the patch setting and the telemetry statement. Required confirmations start
   unchecked.
4. **Confirm.** The copy runs on a retained worker without holding the state
   lock, so status, Play and installation polling keep answering. Publication
   takes the lock again briefly.
5. **Recovery.** An interrupted preparation is offered for explicit removal. An
   interrupted copy offers Recover when its verified files are staged, otherwise
   Abandon. Nothing is replayed automatically, and a destination is never deleted.

The original game folder, the old launcher folder and `legacy-import.json` stay
byte-identical. Changing the diagnostics preference during a copy makes
publication refuse the stale preferences; the copy then needs Abandon and a new
location.

After publication a Wine-backed adopted copy is offered prerequisite setup like
a fresh install, and Play once prerequisites succeed. Its imported patch setting
and login server order are bound as the
[launch contract](launch.md#patch-policy-per-installation) describes. Imported
identity, the game telemetry answer and its auth URL are preserved unchanged;
this build still sends no game telemetry.

Checks, from `crates/launcher/desktop/frontend` with the shell test binary in
`ADOPTION_UAT_BINARY`:

```bash
npm run uat:adoption
```

It drives the production Effect workflow and view against the production host
with a real isolated store, legacy tree and signed loopback origin. With
`CIMMERIA_WINE_HELPER`, `CIMMERIA_WINE_HELPER_SHA256` and
`CIMMERIA_WINE_RUNTIME_TREE` set it extracts through the pinned helper under
Wine and asserts that the reopened copy is offered prerequisites. The same three
variables enable the ignored Wine host tests, including the adoption-to-Play
journeys in `shell/src/host/adoption/play_tests.rs`. Every archive in these
checks is a small inert fixture; no game is started.

## In-place adoption on Windows

Decision D-LX12: on every host other than macOS the launcher adopts the old
launcher's game folder itself, so an existing multi-gigabyte install is neither
copied nor downloaded again as a second client. Settings offers **Verify and
adopt this folder** after a legacy import; there is no location to choose.

1. **Preflight.** No saved import is needed. Pick the game folder itself, its `Working` folder or its `Working\Binaries` folder: the launcher resolves all three to the same root, and refuses a folder that does not hold `SGW.exe` in `Working\Binaries` (`not_a_game_folder`) before anything is downloaded. It also refuses a root it cannot write to (`folder_not_writable`). That check creates and deletes a probe file in the root and renames one inside `Working\Binaries`, and runs before any download; a verified copy into a new folder is offered later. A saved import adds the checks below: the old launcher's `launcher-installed.json` need not record the
   signed release. A folder it left out of date is classified, not refused: its
   files that only lack a patch are listed as needing patching. The folder must
   not hold the old launcher's own
   folder. It must be in the full layout the installer and a fresh install
   write, with `SGW.exe` in `Working\Binaries`. The old launcher also manages
   folders that point straight at `Working` or `Binaries`. Their release files
   all sit at another path than the release keeps them, so such a folder is
   refused once, before anything is downloaded (`unsupported_layout`), rather
   than listing every file as blocked. A `game` folder,
   `.cimmeria-install.json` or `content-ready.json` already in the folder is
   refused (`leftover_launcher_files`). The exception is what an admission
   leaves when it dies before its operation begins, or when Abandon could not
   remove `game`. Those leftovers are an owner marker that no journal
   operation or begun plan names, and an empty `game`. The launcher removes
   them, logs a warning and continues.
2. **Verification.** When the release publishes a file index, it is fetched first, and it alone names every release file: nothing else is downloaded for the comparison. The signed seed (a zip or a RAR, as the archive.org client is) and the patches are fetched only as needed. Patches are fetched for the files that lack the current patches, and the seed only when a needed pre-patch file is itself wrong. The seed comes from archive.org or from the player's own archive. Nothing is ever uploaded or mirrored. The needed files are reconstructed into a private reference, the same preparation the verified copy uses. Every release file in the folder is compared by size and SHA-256. A stock file that only lacks the current patches is listed as needing patching and is updated without extra consent. A modified or missing release file is listed as blocked (`blocked_files`), and confirmation is refused (`verification_failed`) unless the review's explicit consent to replace it is given in step 3. Nothing in the player's folder is ever rewritten from the reference. The only accepted difference is the old launcher's own setup change, which the launcher re-applies.
3. **Admission.** Confirmation first opens `Working\Binaries\SGW.exe` exclusively. A sharing violation means the game is running, and confirmation is refused with nothing admitted (`game_running`). Replacing a modified or missing release file needs that explicit consent; without it nothing is admitted. Admission then writes the owner marker `.cimmeria-install.json` into the folder, creates an empty `game` folder, and saves the plan and a checkpoint before the Adopt operation begins. If admission fails before the operation exists, the marker and the empty folder are taken back.
4. **Moves.** Each original that the consent replaces moves first into `cimmeria-adoption-backup-<id>` in the folder, so no player file is overwritten. That backup folder is never removed by the launcher, not even by Uninstall. Then each verified release file is renamed into `game` at the same relative path, and the launcher's setup and a synthesized ledger are written and the tree is checked against the signed reference digest. Every file that is not part of the release stays exactly where it was, outside `game`. That includes the old launcher's `launcher-installed.json`, which is left in place. Folders the moves emptied are removed. Admitted moves are not cancelled part-way.
5. **Publication.** The receipt, installed-content index (with adoption provenance) and preferences are written as for a verified copy. From then on Play, Repair and Update treat the folder as owned: its `game` tree is managed exactly like a fresh install's.

The review warns about the consequences before Confirm. `SGW.exe` and every
release file move into the `game` folder, so desktop shortcuts and the old
launcher stop working; start the game from this launcher afterwards. Don't run
the old launcher on this folder again, because it may download the game a
second time. Files you added inside the game folders, such as PAKs placed by
hand in `SourceCache.en-us`, stay where they are and the game no longer uses
them. Your settings and the client cache live in Documents and are not
affected.

An interruption after admission leaves the operation in reconciliation. Recover finishes the remaining moves and publication. A file in neither place is refused. The old launcher or a running game can recreate a file at its old path after the review, so a file in both places is settled by content. Admission saves each release file's reviewed hash (`adoption-reviewed-<work>.json`). If the copy in `game` matches, the other copy stays where it is as a file the release does not list. If only the old-path copy matches, it replaces the copy in `game`, which is kept beside the old path as `<name>.adopt-conflict-<owner>`. If neither matches, recovery stops and names the file in the log (`duplicate_release_file`). Nothing is deleted in any case.
Abandon, offered only while the operation is still staged, moves every release file back to where the review found it, moves each replaced original back out of `cimmeria-adoption-backup-<id>`, removes the replacements, and removes the marker. A file in both places keeps the folder's copy, and `game` keeps the other. The backup folder itself is never removed, not even by Abandon. Abandon is offered only while nothing but renames has happened: once every file is in `game` the checkpoint advances before the launcher's setup rewrites `LoginInternal.lua` and the ledger. Putting the files back after that would leave the launcher's setup in them, so only Recover is offered. A crash during verification leaves only the private reference. The folder has no marker yet, and the reference is removed with the existing preparation removal.

**Uninstall** of an adopted folder never detaches or deletes the folder. It
removes only the entries the launcher manages there: `game`, its Repair and
Update work and backups, `content-ready.json`, and the owner marker last. Files
the release does not list stay, so the folder itself is removed only when
nothing else is left in it. Files the client writes inside `game` after
adoption, such as `SGWDebugLog.log`, go with it, as on a fresh install.

Engine tests in `engine/src/storage/adoption/in_place_tests.rs` cover the
rename-not-copy adoption, a modified release file named and refused, an
out-of-date ledger refused before download, uninstall that keeps unlisted files,
recovery and abandonment after a crash mid-move, and a crash between
verification and the marker. `in_place_recovery_tests.rs` covers a release file
recreated at its old path after a crash (recovered both ways, refused when
neither copy verifies, and abandoned), recovery-only after setup, a running game
held open, a folder outside the full layout, and admission leftovers taken back
or refused. `shell/src/host/adoption/in_place_tests.rs` drives
the same flow through the production host. On Windows, `npm run uat:adoption`
runs `adoption-in-place-uat.mjs` against the
`host::adoption::uat_bridge_in_place` bridge, again with the shell test binary
in `ADOPTION_UAT_BINARY`.

Verification uses the signed file index when the manifest publishes one. The
launcher fetches it, checks it against the manifest's hash, keeps it in the
state root as `file-index-<sha256>.json`, and a second review does not download
the seed again. When the manifest has no index, or the index does not match its
hash, the launcher logs the reason, then fetches the seed and patches and builds
the same index locally; either way the folder is classified against that index.
Native Windows power-loss behavior and the adopted-folder Play, Repair, Update
and Uninstall views are checked by the LX-26 Windows UAT (G-06, G-10).

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
