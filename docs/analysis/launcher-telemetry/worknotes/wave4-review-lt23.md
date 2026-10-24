# Wave 4 review: LT-23a (999a1f4eb) and LT-23b (01b5b2431)

Worktree `.claude/worktrees/lt-22`, range `bea3060ee..01b5b2431`, pass 1. N7 (LT-22; the unstaged `endpoint.rs` edit) ignored as briefed.

## Verdict

decision

## Fix notes

N1 [LT-23b]. docs/analysis/launcher-telemetry/worknotes/LT-23b.md:1 — the coder's worknote is untracked (`git status` shows `?? docs/analysis/launcher-telemetry/worknotes/LT-23b.md`) and was not part of `01b5b2431`, while LT-22.md and LT-23a.md were committed with their packets; the note itself says "Done and committed" — `git add` the file and commit it on the lt-22 branch with the message `LT-23b: worknote`, keeping its CRLF endings — verify: `git -C <lt-22> status --short` prints nothing for the worknotes folder and `git -C <lt-22> show --stat HEAD` lists `LT-23b.md`.

N2 [LT-23a]. crates/launcher/desktop/shell/src/host/summary.rs:410 and :425-426 — two test comments still say "This build configures no endpoint" / "This build has no endpoint". After LT-23a that is only true because the default login server (`http://play.cimmeria.app:8081` in `default_servers()`) is plain http; an https first login server now gives an endpoint — replace the :410 text `This build configures no endpoint, and that alone` with `The default login server is plain http, so no endpoint is configured, and that alone`, and replace the :425 text `This build has no endpoint; give the open state one that is never` with `The default login server is plain http, so there is no endpoint; give the open state one that is never` (keep the rest of each sentence; re-wrap if a line passes 100 columns) — verify: Grep `This build` in `shell/src/host/summary.rs` returns no match; `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop summary` still passes.

## Not fixed

- **D1. The debug line is never written (needs a spec call).** `shell/src/host/summary.rs:25-30` logs `summary_endpoint_resolved` at `debug!` on `launcher.pipeline`, as spec step 3 and amendment decision 5 say. But the local file filter (`shell/src/telemetry.rs:86-94`, `file_filter`) caps the `launcher` prefix at INFO, and the test at `telemetry.rs:281-285` asserts `launcher.pipeline` DEBUG is not enabled. So the outcome (`login_server`, `default` or `no_https_endpoint`) reaches no log, and this line is the only place the source is recorded. `launcher.pipeline` INFO is local-file-only (`telemetry.rs:271-278`: not forwarded), so `info!` would be safe. Recommended change if the owner agrees: `tracing::debug!` to `tracing::info!` at `summary.rs:25`, and change the spec text "debug line" to "info line on launcher.pipeline". The outcome values match `SummaryEndpoint::resolve` (`login_server`, `default`, `no_https_endpoint`) and carry no URL. No test pins the event (the spec asks for none).
- **D2. Player-facing docs are now false; route to LT-26.** `docs/client/launcher-guide.md:338` ("This build sends nothing either way."), `crates/launcher/desktop/README.md:21` ("this build configures no endpoint and sends nothing"), `crates/launcher/desktop/docs/launcher-summaries.md:10` and :448-460 (the "What a rollout packet must change" items 1 and 2), and `docs/analysis/launcher-adopt-ux/README.md:46` (F19). None are in the LT-23a or LT-23b file lists. `launcher-guide.md` is not in LT-26's file list (`work-packets-coverage.md:190-194`): add it there.
- **D3. Optional extra test, not required by the spec.** The `!consent` branch of the done text (`view.ts:131`, "Nothing is sent.") has no test, so a regression there would pass.

## Checked and fine

- Anchors: `first_login_server` mirrors `launch_login_servers` (`preparation.rs:17-23`) for the default fallback. `summary::start` is called with a fresh `Arc<Mutex>` that is not yet locked (`host.rs:229-230`), so the new `state.lock()` cannot deadlock, and the guard is dropped before `launcher_summary::start`. `Gate::NoEndpoint` is exactly `active.is_none()` (`tracker.rs:147-149`).
- Boolean-only frontend field: Rust `setup_diagnostics_endpoint: bool` and TS `Schema.optionalKey(Schema.Boolean)` land in one commit (R8); `optionalKey` is the style the other frontend schemas use. No `launcher_summary_consent` text added anywhere.
- First-press text: "Saving your diagnostics choice…" shows on the first click (`view.ts:93`), and the three done texts match the spec word for word. Reverting `view.ts` fails the `saved but not sent yet` case; always-sent or always-not-sent also fails one case.
- Tests: `the_snapshot_says_whether_summaries_have_an_endpoint` fails if `summary_endpoint_configured` is made constant. `the_first_login_server_is_the_default_list_when_none_is_saved` fails without the default fallback (the default list is non-empty). No capture test is added, so the SERIAL / `_second` / `rebuild_interest_cache` standard does not apply.
- File caps: summary.rs 479, view.test.ts 210, view.ts 190, mod.rs 250, commands.rs 235, batch_seam.rs 233 (all under 500). Nothing staged; the only unstaged change is the N7 edit to `endpoint.rs`.