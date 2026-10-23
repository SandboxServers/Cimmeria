# Repeat-release implementation and validation

D-LX15 was owner approved before implementation. Baseline: origin/main
`4ed823288`; task branch `fix/launcher-repeat-releases`. No real release,
production signing key access, production feed writes or lab actions occurred.

## Change

- `stamp.sh` delegates stable UTC day-slot policy to `release_identity.py`.
  Versions are YYYY.M.(D*100+N), N=1..99. It uses every release-list page,
  legacy daily count, highest surviving encoded slot and rolling-feed slot.
  Slots cannot regress if a dated release was deleted. Slot 100 refuses.
- The workflow reserves tag/version/epoch as an artifact before builds.
  Reruns recover it across midnight; a missing rerun reservation refuses.
  Another run for an already published dated commit tag refuses.
- Every Windows release requires signing in the existing `launcher-release`
  environment. Missing key/public key, skipped or failed signing prevents
  publication. The signed, verified feed refreshes launcher-current last.
  A rerun cannot lower its version. No Authenticode or macOS self-update work.
- Old stable/prerelease installs upgrade through the unchanged stable-only
  updater predicate. Build components above Windows' 16-bit limit refuse.
- `applied_content_identity` on existing `launcher.install` reads retained
  signed installed state and the live ledger after success or Play admission.
  Fully applied SgwGame overlay IDs use exact @sgw_game keys; partial IDs are
  separate. The server keeps this event's exact public SHA-256 field while
  retaining the generic identifier scrub on other fields/events/targets.

## Passed

All compiling Rust commands used the PowerShell build lane.

- `python tools/launcher-release/test_release_identity.py`: 11 tests.
  Daily order, legacy migration, rollovers, pagination/deleted slots,
  slot 99/100, malformed identities, Windows limits, reservation retries,
  current-feed order and required-signing workflow policy.
- Engine `cargo test --lib --examples`: 927 passed, 3 ignored.
- Shell binary tests filtered to `release_version`: 4 passed.
- Admin API `applied_content_public_manifest_digest` unit test: passed.
- Engine and admin API `cargo clippy --all-targets -- -D warnings`: passed.
- Desktop and admin API formatting checks, workflow YAML parsing and
  `git diff --check`: passed. Frontend npm ci/build passed to supply Tauri's
  required frontendDist; no frontend feature changed in this task.

## Regression proof

After checkpoint `3a369647a`, temporary source mutations were restored:

1. Restoring the legacy version formula made the identity suite fail:
   daily-order/migration failed; boundary-related cases errored. Restored suite
   passed (10 tests then; 11 with the later rolling-slot guard).
2. Restoring the same_day signing gate failed
   `test_workflow_requires_signing_on_every_release`; restored suite passed.
3. Logging the original owner's manifest after Update, instead of its
   committed current release, failed
   `applied_identity_uses_committed_update_hash_and_failed_apply_logs_no_success`.
   Mutation lane log: `20261022-203727-338720.log`; restored targeted run:
   `20261022-203739-351884.log`, 2 passed.

## Remaining validation and limits

- Actual stamp.sh dry runs (frozen mocked run clock, stub gh, first and second
  releases, API failure), publish script stubs and shellcheck are wired to
  existing Linux PR CI; they were not run locally because this host is
  PowerShell-only. PR CI must pass before shipping.
- Native macOS bundle/signing checks, real same-day publication, physical
  updater replacement and SigNoz end-to-end ingestion were not exercised.
- Historical releases whose notes lack compiled-version evidence produce an
  explicit warning. This is not an exhaustive binary audit of old artifacts.
- Identity reservation retention is 90 days; an older rerun with an expired
  reservation refuses and needs a new dispatch. No new owner decision needed.

## Documentation map

Walked desktop workflow/release/IPC and telemetry rows. Updated desktop README,
release and launch contracts, self-update guide, implementation ledger,
consolidation decision ledger, telemetry architecture/operator runbook, tools
README, campaign design index and committed shared memory. No route/status,
secret, wire protocol, crate API or generated inventory changed; those rows
needed no extra change. Documentation-writer dispatch was attempted, but the
team's concurrency limit prevented starting it; these edits need fresh review.

## Review fixes (N1/N2)

- Delayed macOS publication compares the reserved VERSION against the current
  feed with the existing stable parser before any rolling upload or note edit.
  An older retry repairs only its dated assets, returning `skipped_older`;
  an equal retry refreshes normally. The workflow summary reports the outcome.
  Missing or invalid rolling evidence refuses before mutation. The SC2015
  signature/feed check uses an explicit conditional and still fails closed.
- Install, Update and Repair recovery log the applied content identity exactly
  once after successful terminal reconciliation. Install releases its marker
  handle first; Update/Repair finish the locked recovery body before logging,
  because identity observation rereads the owner marker on Windows. No normal
  commit hooks changed; partial/refused recovery does not log success.
- Captured production recovery tests assert one exact committed manifest hash
  and phase. Install exercises full/partial `@sgw_game` overlay IDs; Update
  and Repair fixtures have no overlays and assert both ID lists are empty.
  Update covers all twelve rename/publication boundaries and distinguishes
  target from original owner hash; Repair covers six boundaries with and
  without an old tree. Partial Install, refused Repair and stale/replayed
  Update recovery assert zero identity events.
- Stub publishing regression now keeps every asset's bytes. It publishes A,
  then B, retries equal-version B, then repairs A's dated macOS assets and
  verifies every rolling asset and its notes still match B, with skip output.
  Signature-missing and feed-missing paths remain refusal tests.

Regression proofs: temporarily making the Python macOS comparison always
return `refreshed` failed the new retry test (`refreshed != skipped_older`);
restored suite passes 12 tests. Removing all three recovery identity calls
failed each captured production recovery assertion (`0 != 1`): lane logs
`20261022-205524-265672.log` (Install), `20261022-205541-189020.log` (Repair),
`20261022-205544-159284.log` (Update). All mutations were restored. The actual
shell publication regression and its mutation proof still require Linux PR
CI; no local Bash ran. No actual publishing, signing secret access, physical
self-update, native macOS runtime or SigNoz ingestion was tested.

Documentation map: updated desktop release contract, applied identity launch
contract and player self-update ordering section. No route, secret, public
API, frontend, wire layout, build policy or inventory threshold changed.

Final review-fix checks (PowerShell lane): fmt `status=ok exit=0 ran=1.169s`
(`20261022-205600-440560.log`); engine clippy all targets
`status=ok exit=0 ran=6.075s` (`20261022-205607-451196.log`); full engine
lib/examples `status=ok exit=0 ran=74.056s`, 927 passed, 3 ignored
(`20261022-205615-400784.log`). Python identity suite: 12 passed. Workflow
YAML parsing and `git diff --check` passed. Linux CI remains pending the
parent's push; the earlier SC2015 failure has been corrected in source.
