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
