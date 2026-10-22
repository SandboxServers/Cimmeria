# LF-16 ruling (review pass 1, item D1)

Source: `.claude/reports/launcher-uat-fixes/LF-16-review-1.md` in the `lf-16` worktree. Ruling by the architect on 2026-10-22. This is not an owner call. It is a bug against the LF-15 spec, so no `BlockedDecision`.

## Decision

Take option (a). A failed mint during `Upload` is a network error. It is not `Unreachable`.

- LF-15 item 2 limits `Unreachable` to one case: "no launcher telemetry target can be built for this install". LF-15 item 3 maps send results only to Sent, Partial or Failed. `IngestOutcome::Unreachable` (`ingest.rs:344-346`, a first-chunk `MintFailed`) is a transient failure. Under the spec it must come out as Failed, which is `Err(LogUploadError::Network)`, the same as `IngestOutcome::Failed`.
- `LogUploadState::Unreachable` and the LF-16 text `Debug logs can't be sent: this install has no telemetry server.` stay, but only for the no-target case. That is `begin_log_upload` with Inspect (`ingest_flow.rs:120-124`) or Upload (`:129-131`). The LF-16 view does not change. After this fix its copy and disabled button are correct.
- `IngestOutcome::Unreachable` stays in `ingest.rs`. Its telemetry label `unreachable` (`ingest.rs:396`) still tells a mint failure apart from a rejected chunk in SigNoz. Only the mapping to the player's state changes.
- The player then sees `logUploadErrorText.network` ("The upload did not reach the log store. Check your connection and try again."). The button comes back enabled (`log-upload-view.ts:77`), so they can retry at once.

Correction to the review: `ingest_flow_tests.rs:216-223` is the no-target test (`no_telemetry_target_is_unreachable_never_disabled`). It is correct and **must not change**. No existing flow test covers the mint-failure mapping, so the fix adds one.

## Where it lands

Put the fix on the **LF-16 branch** (`launcher-uat-fixes/lf-16-log-view`, worktree `lf-16`) as a second, separate commit. Do not put it on `lf-w4` directly.

- The LF-16 branch is cut from the `lf-w4` tip (fe09b1dd4), which already has LF-15. The view's copy depends on this mapping, so both should merge into `lf-w4` together.
- LF-16 review pass 2 then covers both commits, so `lf-w4` needs no extra review loop.
- Merge order in wave 4 does not change: LF-16 → `lf-w4`, then LF-19, then LF-17. If LF-19 also edits the `Upload` arm of `ingest_flow.rs`, whichever merges second rebases over a one-line change.

Fix packet: **LF-16-F1**. Host: windows. Agent: packet-coder. Worktree: `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lf-16`.

## Exact change

Edit only these two files:

1. `crates/launcher/desktop/engine/src/storage/log_upload/ingest_flow.rs`, line 251. Replace
   `IngestOutcome::Unreachable => Ok(status(LogUploadState::Unreachable)),`
   with
   ```rust
   // A failed mint is a transient outage (server down, DNS, offline), not a
   // missing target: the player can press again (LF-15 item 3).
   IngestOutcome::Unreachable => Err(LogUploadError::Network),
   ```
   If the compiler then says `IngestOutcome::Unreachable | IngestOutcome::Failed` can be merged, keep them as two arms anyway. The comments explain different causes.
2. `crates/launcher/desktop/engine/src/storage/log_upload/ingest_flow_tests.rs`. Add one `#[tokio::test]` after `a_rejected_upload_is_a_network_error_and_moves_no_offset` (around line 388). Use the existing helpers (`MockServer`, `write_log`, `state_at`, `run_upload`, `Offsets`, `upload`, `ingest_of`, `chunk_posts`):
   ```rust
   #[tokio::test]
   async fn a_failed_mint_is_a_network_error_the_player_can_retry() {
       let root = tempfile::tempdir().unwrap();
       let server = MockServer::start().await;
       Mock::given(method("POST"))
           .and(path("/api/auth/dev-session"))
           .respond_with(ResponseTemplate::new(503))
           .mount(&server)
           .await;
       write_log(root.path(), 4);
       let state = state_at(root.path(), &server.uri());
       let uploader = LogUploader::new();
       assert_eq!(
           run_upload(&state, &uploader).await,
           Err(LogUploadError::Network),
           "a transient mint failure must not read as `no telemetry server`"
       );
       assert_eq!(chunk_posts(&server).await, 0);
       assert!(Offsets::load(&state.directory.root).files.is_empty());
       // The busy flag is released: a second press is admitted.
       assert!(state.begin_log_upload(upload(), &uploader).is_ok());
   }
   ```
   Do not edit `no_telemetry_target_is_unreachable_never_disabled`. The file goes from 517 to about 540 lines, which is under the 700 hard cap.

Do not touch the frontend, the TS contract, `ingest.rs`, or any docs.

## Test and revert proof

- Lane, foreground, run from the worktree:
  `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lf-16; pwsh tools/build-lane/lane.ps1 cargo test -p cimmeria-launcher-engine log_upload`
- Revert proof:
  1. Put line 251 back to `Ok(status(LogUploadState::Unreachable))`.
  2. Run the same lane command. The new test must fail with `Ok(Unreachable)` against `Err(Network)`.
  3. Restore the fix and run again. It must be green.
  4. Paste both lane results (status line and test counts) into the report. Check that the log is under `logs/` for the `lf-16` worktree and not for main.
- Also run `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-launcher-engine --all-targets` and check that it is clean.
- Commit only the two files, by explicit path: `fix(launcher): a failed log-upload mint is a retryable network error (LF-16-F1)`.

## Rules the fix packet carries

- PowerShell only. No Bash, and no cargo outside `tools/build-lane/lane.ps1`.
- Run lane calls in the foreground and commit as soon as they are green.
- Edit only the two named files. No docs, so there is nothing to convert to CRLF.
- Files: soft cap 500 lines, hard cap 700.
- The test must fail when the fix is reverted (the proof above).
- The new failure path already emits telemetry through `ingest.rs` (`unreachable` outcome), so no new event is owed.

## Not in this ruling (no packet)

The view re-inspects only when the journal operation changes. So after the player fixes their login servers, a real no-target `Unreachable` stays disabled until the next operation or `refresh`. This was a gap before LF-16 too, and it does not block LF-16. The wave-4 reviewer may raise it as a follow-up if the U4 UAT hits it.
