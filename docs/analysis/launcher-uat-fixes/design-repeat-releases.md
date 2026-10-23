# Offer every signed launcher release

> **Type:** Explanation / proposed decision
> **Audience:** Launcher release engineers, reviewers, and the project owner
> **Last updated:** 2026-10-23
> **Status:** D-LX15 APPROVED by owner, 2026-10-23; implementation authorized. No implementation or regression tests executed by this design pass.
> **Companions:** [Resume](RESUME.md), [ledger](README.md), [release contract](../../../crates/launcher/desktop/docs/release.md), [self-update](../../client/launcher-self-update.md), [consolidation decisions](../launcher-consolidation/README.md)

## Problem and evidence

The second release on a UTC day gets a `YYYY.M.D-N` prerelease version from `tools/launcher-release/stamp.sh`. The workflow's `same_day == 'false'` condition skips signing, and publishing treats skipped signing as unsigned and leaves `launcher-current` unchanged. The dated zip and `.sha256` are already attached by `publish.sh`, but users are never offered the build.

Signing alone cannot fix this: `storage/updater/policy.rs::Config::newer` rejects offered prereleases, and `release_feed.rs` refuses to write their feed. D-LX8 explicitly chose that behavior. Replace D-LX8 with a new owner-approved decision, rather than treating the current policy as an incidental bug. D-LX9 is already key provisioning; use D-LX15 for this proposal, subject to a final collision check at implementation.

The implementation baseline is origin/main `4ed823288`, including U4/U5 and close-out. Keep this task in its separate worktree. No real release is authorized by this design.

## Proposed D-LX15: stable day slots

Compile each new release as `YYYY.M.(D*100+N)`, where the date is UTC and `N` is its place among that day's releases, starting at 1. Permit slots 1 through 99 and fail closed before building slot 100. Keep dated tags as `launcher-YYYYMMDD-<sha7>`.

Examples: the first three October 22 releases become `2026.10.2201`, `2026.10.2202`, and `2026.10.2203`. Existing releases on that day count toward the ordinal: migrating after two old-scheme releases uses slot 3. Do not reuse a lower slot because an older release used another version format.

| Boundary | SemVer precedence |
|---|---|
| Old stable install | `2026.10.22 < 2026.10.2201` |
| Old prerelease install | `2026.10.22-2 < 2026.10.22 < 2026.10.2201` |
| Repeated day | `2026.10.2201 < 2026.10.2202 < 2026.10.2203` |
| Last supported slot to next day | `2026.10.2299 < 2026.10.2301` |
| Month boundary | `2026.10.3199 < 2026.11.101` |
| Year boundary | `2026.12.3199 < 2027.1.101` |

SemVer compares major, then minor, then patch numerically, so the smaller patch at month/year rollover is safe. The updater normalizes an installed old prerelease to its stable base; the new stable version remains greater than both. Keep the existing stable-channel rejection of unknown prereleases and its downgrade protection. Update its old-scheme explanation and tests; do not broaden acceptance unnecessarily.

The maximum patch is 3199. The Tauri build path (`build.rs` through `release_version.rs` and `TAURI_CONFIG`) embeds this numeric version. The inspected `tauri-build` 2.7.1 (the desktop lockfile version) Windows resource conversion shifts major/minor/patch/build by 48/32/16/0 bits. Year 2026, month 12, and patch 3199 each fit the Windows resource's 16-bit components. Test explicit bounds and fail on unsupported components rather than relying on truncation. [Microsoft VERSIONINFO reference](https://learn.microsoft.com/windows/win32/menurc/versioninfo-resource) documents that resource format.

MSI ProductVersion has different bounds: 255.255.65535. A year-based major already exceeds its major limit. D-LX3 ships a zip, not an MSI; this design does not promise MSI compatibility. An MSI packaging change would need its own version mapping. See [Microsoft ProductVersion reference](https://learn.microsoft.com/windows/win32/msi/productversion).

## Alternatives

| Scheme | Reason to reject |
|---|---|
| `YYYY.M.D+N` | Build metadata does not affect SemVer precedence; an existing same-day stable version would refuse it. |
| `YYYY.M.D.N` | Four numeric components are not SemVer. |
| `YYYY.M.D-N` | Prereleases sort below the day's stable version and old updaters reject them. |
| `YYYY.M.YYYYMMDD` | The patch exceeds the Windows resource's 16-bit limit. |
| Epoch or greatly increased major | Can be made monotonic, but sacrifices the existing readable date structure and makes migration harder to reverse. |

The factor 100 is a deliberate bounded allocation. If 99 releases a day ever becomes insufficient, choose and test a new scheme before publishing that day's hundredth release.

## Stamping, reruns, and publication

Extend the existing stamping script and release-script test harness. Enumerate the complete release list with pagination; the current `--limit 200` must not silently omit relevant history. Validate API output and fail on listing errors or malformed identities. Derive a deterministic daily ordinal from dated releases, with a defined tie breaker for equal creation times.

A successful same-run rerun retains its recorded version, release date, tag, and identity. It must not recompute the ordinal from today's clock or release count. Refuse a different workflow run for the same already-published commit/tag as today. Persist or recover the immutable release identity before publication so retry behavior is reviewable. If a partial attempt has reserved an identity, reuse that identity rather than assigning an opportunistic new one.

Compare the proposed version against all relevant published compiled identities and the current rolling feed before accepting a new release. Tag dates alone do not establish compiled-version precedence. Paginated release-note inspection confirms the latest October 23 version is `2026.10.23`, October 22 includes `2026.10.22` and `2026.10.22-2`, and the inspected October 21/18/12/11 releases use the old scheme. Pre-October-11 notes do not identify their compiled version. Inspect their tag's release-version/build metadata or released feed before claiming an exhaustive historical comparison. Missing evidence must be explicit, never interpreted as version zero.

Remove `same_day` outputs and their consumers, the signing gate, and unsigned same-day release-note/summary text. Sign every main release in the existing GitHub `launcher-release` environment. Keep key provisioning and secret storage unchanged. Do not print or commit signing material.

Publish must fail if required signing is failed, skipped, or missing; eliminate the unsigned fallback. Verify signatures and the generated feed before updating `launcher-current`. Upload the feed last, and have it reference immutable dated artifact URLs. Verify the candidate remains newer than the rolling version immediately before refreshing it. A rerun of an older release may repair its dated assets, but must not regress the rolling feed. Workflow serialization plus this final guard protects concurrent/rerun publication.

Distinguish macOS ad-hoc application signing from Minisign updater artifact signing in the documentation and test assertions. Every release requires the Windows Minisign updater signature. The existing macOS ad-hoc signing remains; macOS self-update and Apple notarization are outside scope.

## Regression and dry-run plan

Use unit tests for updater precedence and signed feed behavior, and the existing mock-`gh` shell harness for stamping/publication. Cover:

- Old stable and old prerelease installations accepting the new stable version; three same-day releases accepted successively; equal/older offers and unknown prereleases refused.
- Day, month, and year transitions; numeric component limits; slot 99 succeeds and slot 100 refuses.
- First/second/third release stamping, migration with old same-day releases, paginated history, equal-time ordering, listing failure, malformed data, and existing version evidence higher than the candidate.
- Same-run reruns preserving recorded identity across a date boundary; another run for the same commit refusing; partial-publication retry; older rerun leaving the rolling feed unchanged.
- Every release requiring signatures; feed/signature version agreement; missing/skipped signing aborting publication; feed referencing immutable artifacts and refreshing only after verification.

Run a dry run of the actual `stamp.sh` using mocked `gh`, clock, output file, and GitHub variables, with no production signing key or release writes. The local host is PowerShell-only: do not invoke Bash locally. Exercise the existing `test-release-scripts.sh` harness in PR CI on its supported runner and retain the mock-output evidence. A PowerShell arithmetic sanity check is useful design evidence, but does not replace the actual script dry run or regression tests.

Run compiling Rust checks only through `pwsh tools/build-lane/lane.ps1 cargo ...`. Commit a checkpoint before reversible mutation checks. Restore only the old version formula/signing condition in targeted sources and show the relevant regression checks fail; restore the fix and rerun the targeted checks. Report individual regression proof rather than claiming every added assertion was mutation tested.

No real same-day release, live key access, physical updater replacement, or production feed mutation is part of this task.

## Follow-up: log the content actually applied

After successful installation, adoption, and update, emit a structured event naming the verified applied content manifest SHA-256 and applied overlay IDs. Also emit that identity at actual Play admission/load from authenticated installed state. A fetched offer or staged download is not evidence of applied content.

Read the installed ledger and authenticated manifest mapping. Use `InstalledState::has_applied_patch` and its correct keys, including the `@sgw_game` suffix where the signed manifest root is `SgwGame`; a string prefix check can misclassify overlays. Report incomplete patch IDs separately so a partially applied light overlay is never represented as fully applied. Define stable field names and event timing in the desktop logging contract. Emit no paths, tokens, or secrets.

Capture-log tests verify the applied hash/IDs, old-versus-new release selection, suffix handling, partial application, and absence of success events after failed apply or staging only. Revert the event source/placement to prove the guard catches logging a fetched offer rather than installed state. Do not infer that a game process loaded an overlay merely because the launcher published it; distinguish the applied event from the Play event.

## Documentation walk and owner confirmation

Update `tools/launcher-release/stamp.sh`, `publish.sh`, the workflow, `policy.rs`/`release_feed.rs` tests and their version explanations. The doc-update map's desktop release row requires `crates/launcher/desktop/docs/release.md`, desktop README where its contract changes, `docs/client/launcher-self-update.md`, and the launcher implementation ledger. Mark D-LX8 superseded by D-LX15 only after owner approval, preserving the historical rationale. Keep D-LX9 and the signing environment unchanged. Update tools documentation and shared project memory; update telemetry contracts/runbooks if the structured event schema is introduced. Index this design from the campaign README in the same PR.

The existing map already covers release workflows and launcher telemetry; no new general policy row or Copilot checklist rule is proposed. Campaign UAT changes belong in its checklist and `docs/guides/unified-uat.md`; generated totals and global status docs remain outside this change.

**Owner approved (2026-10-23):** D-LX15 with `YYYY.M.(D*100+N)`, UTC slots 1..99, stable-only offers, signing required for every release, and the rerun/no-regression rules above. This replaces D-LX8. Cutting a release still requires separate owner authorization; no release is performed here.
