# Adopt an install without relocating it

> **Type:** Explanation / approved design
> **Audience:** Launcher engineers, reviewers, and the project owner
> **Last updated:** 2026-10-23
> **Status:** Approved for implementation (owner, 2026-10-23): macOS in-place adoption included; explicit content changes at the selected root permitted. No implementation tests executed by this design pass.
> **Companions:** [Resume](RESUME.md), [ledger](README.md), [desktop launcher](../../../crates/launcher/desktop/README.md), [doc-update map](../../agents/doc-update-map.md)

## Requirement and baseline

Adopting means using the selected installation at its existing location. The launcher must never move, copy, or relocate that installation. Add **Adopt an install** beside the other buttons at the bottom of the main page. Show it only when no valid managed or adopted installation exists. Reuse the existing adoption workflow and acknowledge the first press visibly.

The implementation baseline is origin/main `4ed823288`, which already includes U4 (#1987), U5 (#1988), and close-out (#1989). The older resume card's pending-PR rows are historical; do not build on those worktrees. This task has its own worktree from current origin/main.

## Evidence and defects

The shipped engine violates the location requirement. In `crates/launcher/desktop/engine/src/storage/adoption/in_place.rs`, preparation creates `root/game` (line 95), and `move_files` renames each adopted release file into that directory. Its recovery path resumes those moves. Calling this operation in-place does not make it compliant.

`light.rs` and `in_place_replace.rs` also rename `Cache.en-US` to `SourceCache.en-us`; replacement adoption backs up original files. The owner approved those explicit per-file content changes and backups at the selected root. The previous macOS flow copied the installation under D-LX12; the owner now requires direct-root adoption there too.

Audit the inventory, session, installed-content, launch, update, repair, uninstall, and log-upload consumers together. Removing `move_files` alone leaves readers pointing at `root/game`. Review the F1/F3-F6 copy and native prompts so every displayed statement matches the new behavior.

## What counts as a valid installation

Derive the predicate from `storage/installed_content/mod.rs` and `install_worker::content_valid`, rather than from whether Play is currently enabled:

- The installed owner and current release receipt/evidence authenticate successfully, including adoption provenance where present.
- The installed ledger matches the signed seed hash, is not `seed_adopted`, and has no missing required patches.
- The executable is a contained, nonempty regular file, and the expected `SGWGame` directory exists in the resolved content root.

This is content validity. Missing runtime or PhysX prerequisites, a launcher minimum-version gate, an active game, or an unrelated operation does not make an otherwise valid installation disappear. Preserve existing admission checks for Play independently.

The adoption `owned` flag conservatively treats errors as owned, so it cannot supply this predicate. `LaunchStatus.installation_id` is suppressed by runtime, busy, and updater-minimum conditions, so it cannot supply it either. Expose a dedicated native content-validity view to the frontend. On unreadable or uncertain state, show an explanatory status and disable admission until refreshed; do not infer permission to adopt over an existing owner from a validation error.

## Proposed layout and persistence

Add a small layout-resolution module that authenticates the owner, provenance, and retained pinned adoption plan. Use the existing digested `Plan.schema_version` together with `InstallIntent.schema_version`: schema 1 keeps the legacy/managed layout; schema 2 selects direct-root adoption deterministically. The retained authenticated plan must enforce matching schema-2 intent. After owner/provenance/journal validation, `InstallIntent::content_root()` resolves the content directory. Add no fields to `Plan`. A sidecar is unnecessary: binding an independently editable layout value to a plan digest would not authenticate that value.

The resolver distinguishes managed/legacy content in `root/game` from new adoption whose content root is the selected directory itself. Validate the plan schema and provenance before resolving the root; reject missing, mismatched, unsupported, or escaping records. The authenticated plan schema selects one immutable layout, rather than relying on directory-existence guesses or a self-authenticated sidecar.

Retain the schema-2 plan durably before publishing content-ready state. Readers must reject partially persisted state and recover through the established persistence-uncertain/reopen contract. Route launch, update, repair, uninstall, executable checks, game-log discovery, and receipt/ledger consumers through this resolver. Preserve selective uninstall of authenticated launcher-managed files; never recursively delete the adopted root or its unrelated extras. Repair and update must use per-file operations for direct-root adoption, never swap the selected directory.

Existing successful legacy adoptions keep their current layout. Moving their files back would itself violate the requirement and could overwrite newer files at the old location. New adoption preserves the selected installation root and unrelated files. The owner permits explicit per-file content changes, cooked-data normalization and replacement backups within that root; it must never bulk re-home the install into a game subfolder. Managed installation behavior remains unchanged.

Persisted interrupted legacy adoption is a migration hazard: it must never silently resume a rename or copy after upgrading. Detect it before admitting resume. Recovery escape: leave all files untouched, mark the operation as needing explicit recovery, and let the player abandon launcher ownership metadata without deleting content. Explain that partially moved files need manual inspection before a fresh adoption. Do not offer an automatic rollback that moves them back. Implement and test that recovery escape as part of the no-relocation requirement; do not add an approval gate to leaving the files untouched.

Keep additions to `in_place.rs`, `inventory.rs`, `session.rs`, `log_upload/tests.rs`, and `upload_gate.rs` to call lines where possible. Put layout resolution, compatibility admission, and regression fixtures in cohesive sibling modules; respect the 500/700-line limits.

## Main-page interaction

Use the existing `mountAdoption` choose handler as the single entry point. The footer button first opens the visible Settings/adoption panel and sets synchronous live feedback, then invokes the folder picker through that handler. Do not mount a second workflow or duplicate state.

On the first press, show an immediate choosing/busy message before awaiting the native bridge. Disable repeated admission while busy. Cancellation returns to an actionable state with visible feedback; picker and validation errors appear in the same panel. Successful publication refreshes durable installation state and hides the footer button. Reopening the application derives visibility from persisted authenticated content, not a remembered UI boolean.

## Regression and UAT plan

Use storage lifecycle tests for the location invariant and frontend composition tests for interaction. The decisive adoption guard snapshots relative paths, bytes, and stable filesystem file IDs before adoption, then verifies unchanged matched files remain at those exact paths afterward. Assert approved replacements, setup writes, and cooked normalization separately. File IDs catch copy-and-delete implementations that a hash-only test misses. Include unrelated extras and non-ASCII paths. Exercise the production host adoption entry point, rather than a test-only no-op mover.

Cover direct-root adoption, reopen/launch resolution, repair/update resolution, selective uninstall, managed installation regression, crash and resume boundaries, cancellation/abandon, malformed or unsupported retained plans, and interrupted old-operation refusal. Include the approved stock/replacement/macOS direct-root contract. Schema-2 abandon undoes approved per-file changes; schema-1 interrupted abandon retains existing bytes, staged originals, and backups for manual inspection.

Frontend tests cover valid managed and adopted installs hiding the button; absent/invalid installs showing it; runtime/PhysX/update-minimum/busy states retaining the validity distinction; and cancel/error/success feedback. Verify only one adoption workflow exists.

Run the required JS REPL logic UAT with linkedom, the real frontend composition, and a deferred native bridge. Press once and inspect visible acknowledgement before resolving the picker promise. Exercise picker cancel, error, busy rejection, success, and refresh from durable native state. Record assertions and results. This REPL does not cover actual native folder-picker windows, physical filesystem effects, packaged visual layout, or a real game launch; list those separately for owner UAT and obtain permission before laptop/lab use.

Run frontend tests/builds and appropriate engine tests through `pwsh tools/build-lane/lane.ps1 cargo ...`. Before claiming regression proof, commit a checkpoint, temporarily restore the old move behavior in only the relevant source, and show that the location test fails. Restore the fix and show the targeted checks pass. Repeat for the visibility/first-feedback guard without discarding other work.

## Documentation walk

The desktop-launcher row of the doc-update map applies: update desktop README and adoption contracts, `docs/client/launcher-guide.md`, `docs/guides/macos.md`, and the launcher implementation ledger. Replace inaccurate F1/F3-F6 promises and add matching steps to this campaign's UAT checklist and `docs/guides/unified-uat.md`. Record layout/recovery facts in shared project memory. The campaign README indexes this design; no global status or generated block needs editing. No new general review rule is proposed, so the Copilot checklist does not need a policy change.

## Owner decisions (2026-10-23)

1. **macOS:** implement in-place adoption now; remove verified-copy adoption as a way to adopt an installation.
2. **Content changes:** explicit individual setup/patch changes, cooked-data normalization and original-file backups at the selected root are permitted. Bulk relocation or copying of the installation is prohibited.

No implementation, native picker UAT, migration execution, or lab use is claimed here.
