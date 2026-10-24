# Wave 4 review, lanes 1-3 (LT-18, LT-19, LT-20)

Reviewed heads: lt-18 `d851d1884`, lt-19 `fbe34e080`, lt-20 `c894f8757`. Read-only: I ran no build or test; the check results below are the coders' own reports.

## Verdict

fix (2 fix notes: one for LT-19, one for LT-20; LT-18 ships as is)

- LT-18: ship
- LT-19: fix (N1)
- LT-20: fix (N2)

Pairwise `git merge-tree --write-tree` on the three branches (18/19, 19/20, 18/20): all clean, no conflicts. No stray files in any commit (no `launcher_targets.txt`, `dist`, `node_modules`, worknotes). `git diff --check` is clean on all three. `package.json` already lists all six LT-20 test files, so it correctly stays unchanged.

## Fix notes

### LT-19

N1 [LT-19]. crates/launcher/desktop/engine/src/commands.rs:93-101 (the `SavePreferences` arm) — the before/after compare that decides whether `setup_diagnostics_changed` fires has no test. Reverting it (always firing, never firing, or comparing the wrong pair) leaves every test green. The same goes for "a stale save logs nothing", which the module doc of `settings_events.rs` promises. The packet only named the `SaveClientOptions` test, but the compare is the one piece of logic in the packet — add one test to the `tests` module of `commands.rs`, next to `saving_client_options_reports_a_count_not_the_servers` (it reuses that module's `SERIAL`/`serial()`, `_second` and `capture`):

    #[test]
    fn saving_preferences_reports_the_consent_change_only_when_it_changes() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let save = |expected_revision, consent| NativeCommand::SavePreferences {
            schema_version: 1, expected_revision, install_directory: None, launcher_summary_consent: consent,
        };
        let named = |events: &[crate::test_capture::Captured], name: &str| {
            events.iter().filter(|e| e.field("event") == Some(name)).count()
        };
        let (stale, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.dispatch(save(7, true))
        });
        assert!(matches!(stale, Err(StorageError::StaleRevision)));
        assert!(events.iter().all(|e| e.target != "launcher.settings"));
        let (first, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.dispatch(save(0, true))
        });
        assert!(first.is_ok());
        assert_eq!(named(&events, "preferences_saved"), 1);
        assert_eq!(named(&events, "setup_diagnostics_changed"), 1);
        let (second, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.dispatch(save(1, true))
        });
        assert!(second.is_ok());
        assert_eq!(named(&events, "preferences_saved"), 1);
        assert_eq!(named(&events, "setup_diagnostics_changed"), 0);
    }

Adjust only what the compiler forces (for example the `Captured` import path). The fresh state starts at revision 0 (the existing `stale_client_options_are_refused_and_write_nothing` test saves with `expected_revision` 0 first). If `save(0, true)` is refused by the summary-consent gate on a state with no configured summaries, set the state up the way `storage/launcher_summary/tests/consent.rs` does for a successful consent save, and do not weaken the assertions. Do not write the text `launcher_summary_consent` in `settings_events.rs`. — verify: `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine commands` passes; then temporarily change `!=` to `==` on commands.rs:99 and the new test must fail (restore afterwards); also run `... -p cimmeria-launcher-engine only_the_allowed_files_mention_the_summary_consent_flag`.

### LT-20

N2 [LT-20]. crates/launcher/desktop/frontend/src/launch-view.test.ts:58-60 — the `launcher_too_old` half of `a refused Play is reported to telemetry, a minimum-version refusal is not` is vacuous. After the first click rejects `'busy'`, `launch-workflow.ts:36` (`failure`) sets `uncertain:true`; `launch-view.ts:26` then sets `button.disabled=true`, and `launch-view.ts:36` (`click`) ignores clicks on a disabled button. So the second click never calls `invoke`, and the final `assert.deepEqual(uiCalls,[])` passes whether or not `launch-workflow.ts:18` has its `error!=='launcher_too_old'` guard. Deleting the guard would not fail the test. — In the existing test, delete lines 58-60 (`uiCalls.length=0;refusal='launcher_too_old';` through the second `assert.deepEqual(uiCalls,[]);`), delete `let refusal='busy';` (line 54) and write `throw 'busy'` at line 55 in place of `throw refusal`, and rename the test to `a refused Play is reported to telemetry`. Then add a second test directly after it, with its own mount, that counts the Play calls the way the neighbouring test at :41-47 does:

    test('a minimum-version Play refusal is not reported to telemetry',async()=>{
     const uiCalls:unknown[]=[];
     Object.assign(globalThis,{window:{__TAURI__:{core:{invoke:async(command:string,args:unknown)=>{uiCalls.push([command,args]);}}}}});
     try{
      const status=initial();let plays=0;const {document,window}=parseHTML(html);
      const app=mountLaunch(document,async(_,{request}:any)=>{if(request.command==='play'){plays++;throw 'launcher_too_old';}return status;});
      try{await app.ready;await new Promise(resolve=>setImmediate(resolve));document.getElementById('launch')!.dispatchEvent(new window.Event('click'));await app.settled();await new Promise(resolve=>setImmediate(resolve));
       assert.equal(plays,1);
       assert.deepEqual(uiCalls,[]);
      }finally{await app.dispose();}
     }finally{Reflect.deleteProperty(globalThis,'window');}
    });

— verify: `npm test --prefix crates/launcher/desktop/frontend` passes; then temporarily make `launch-workflow.ts:18` unconditional (`reportUiEvent('launch','invoke_failed',uiCode(error));` without the `if`) and the new test must fail (restore afterwards); `npm run check --prefix crates/launcher/desktop/frontend` stays clean.

## Not fixed

- **LT-18, no fix notes.** Checked against the spec: `Stage` and the two struct variants match; the five `windows.rs` sites match the packet table (`InjectError::Win32 { code: u32 }` and `windows.rs` are both `cfg(windows)`); the `events.rs` helpers keep the event names `launch_not_started` and `launch_failed`; the three supervisor anchors are applied (:98, :110, :131); no other constructor of `Event::NotStarted` or `Event::Unknown` exists under `crates/launcher`; no path, URL or message goes into an event; `Event` stays `Copy`; the touched files are under 500 lines (events.rs 387). The coder reports D1, D2p, D3p, D2e and D3e all `status=ok`, and the old-event decode test shows `deny_unknown_fields` accepts `{"event":"not_started"}`.
- **LT-18, supervisor wiring is unpinned on this host.** The three `supervisor.rs` anchors have no test that runs on Windows (`supervisor_tests.rs` is `cfg(unix)`; the packet forbids adding one). Reverting them would still pass here. Accepted by the packet; a Linux CI run is the only guard. The coder did not run a revert check; the two `mod.rs` tests and the `events.rs` test fail to compile or fail on assertion if their source change is reverted.
- **Branch names differ from the specs.** The worktrees sit on `launcher-telemetry/lt-18-launch-stage` (spec: `lt-18-worker-codes`) and `launcher-telemetry/lt-20-frontend-ui` (spec and brief: `lt-20-ui-errors`; the LT-20 coder report also claims `lt-20-ui-errors`). `lt-19-settings` matches. Merging by the spec name will fail. `ops` must merge the real names or rename with `git branch -m`. A coder cannot fix this.
- **LT-19 commit trailer.** The commit says `Co-Authored-By: Claude Haiku 5.5`; LT-18 and LT-20 say `Claude Sonnet 5.5`. Fixing it means amending a commit, which the packet rules forbid. Cosmetic; leave it or let `ops` decide.
- **LT-19 file sizes.** `storage/mod.rs` is 542 lines (over the soft cap of 500, but it was 541 before and the hard cap is 700); the change is the one `mod` line the spec allows. `settings_events.rs` is 159 lines (cap 160). No action.
- **LT-20 observation, no action.** `launch-workflow.ts` reports `invoke_failed` for every failed bridge call, including the 1 s inspect poll. The view stops polling once `current.error` or `uncertain` is set (`launch-view.ts:39`), so a failure reports at most once. The packet specified this behaviour.
- **LT-20, deviations accepted.** The log-upload test answers `{state: undefined}` instead of `{}` because the mount helper wraps the answer; the view still takes the schema path and the test asserts both the report and the error text. The coder did not run revert checks; the other new tests each assert a `ui_event` call that only the source change produces, so they fail on revert (reasoned, not run).
