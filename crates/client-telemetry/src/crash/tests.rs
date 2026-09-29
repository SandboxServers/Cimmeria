use super::*;
use crate::queue::{channel, Consumer};

fn state_in(dir: &std::path::Path) -> (CrashState, Consumer, &'static FlushSignal) {
    let (producer, consumer) = channel();
    let flush: &'static FlushSignal = Box::leak(Box::new(FlushSignal::new()));
    let state = CrashState::new();
    state.configure(CrashContext {
        sessions_dir: dir.to_path_buf(),
        session_id: "sess-1".into(),
        started: Instant::now(),
        producer,
        flush,
        // No uploader runs in these tests; keep the waits short.
        crash_flush_timeout: Duration::from_millis(50),
        exit_flush_timeout: Duration::from_millis(50),
    });
    (state, consumer, flush)
}

fn fault() -> Fault {
    Fault {
        code: report::EXCEPTION_ACCESS_VIOLATION,
        flags: 0,
        address: 0x0041_6ec5,
        params: [Some(0), Some(0)],
        module: Some(FaultModule {
            name: "SGW.exe".into(),
            base: 0x0040_0000,
        }),
        thread_id: 7,
    }
}

fn drain(c: &Consumer) -> Vec<crate::events::ClientNativeEvent> {
    std::iter::from_fn(|| c.try_recv()).collect()
}

/// The crash path emits `client.crash` before touching the dump (so a
/// hung dump write cannot cost the event), writes the sidecar, hands the
/// dump writer the path named in the event, then reports the dump.
#[test]
fn crash_emits_event_then_dump_outcome_and_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let (state, consumer, flush) = state_in(dir.path());
    let mut dump_path = None;
    let recorded = state.on_crash(
        CrashSource::GameMinidump,
        fault(),
        Some(2),
        1_700_000_000_000,
        |p| {
            // The crash event is already queued when the dump is written.
            assert_eq!(consumer.try_recv().unwrap().target, "client.crash");
            dump_path = Some(p.to_path_buf());
            std::fs::write(p, b"MDMP").unwrap();
            DumpOutcome::Written {
                bytes: 4,
                dump_type: 0x1021,
            }
        },
    );
    assert!(recorded);
    let name = "crash-sess-1-1700000000000.dmp";
    assert_eq!(dump_path.unwrap(), dir.path().join(name));

    let rest = drain(&consumer);
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].target, "client.crash.dump");
    assert_eq!(rest[0].fields["dump_file"], name);
    assert_eq!(rest[0].fields["dump_bytes"], 4);
    // The flush was requested (nothing delivers it in this test).
    assert!(flush.pending());

    let sidecar = std::fs::read_to_string(dir.path().join("crash-sess-1-1700000000000.jsonl"))
        .expect("sidecar written");
    let v: serde_json::Value = serde_json::from_str(sidecar.trim_end()).unwrap();
    assert_eq!(v["fault"], "SGW.exe+0x00016ec5");
    assert_eq!(v["game_dump_type"], 2);
    assert!(state.has_crashed());
}

/// A second fault (including one inside the capture code) is passed
/// through untouched: no second event, no second dump.
#[test]
fn only_the_first_crash_is_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let (state, consumer, _) = state_in(dir.path());
    assert!(
        state.on_crash(CrashSource::UnhandledFilter, fault(), None, 1, |_| {
            DumpOutcome::Failed {
                stage: "create_file",
                os_error: 5,
            }
        })
    );
    let first = drain(&consumer);
    assert_eq!(first.len(), 2);
    assert_eq!(first[1].fields["written"], false);

    let again = state.on_crash(CrashSource::GameMinidump, fault(), None, 2, |_| {
        panic!("a second crash must not write a dump")
    });
    assert!(!again);
    assert!(drain(&consumer).is_empty());
}

/// Before `configure` (the session never loaded) a crash is not
/// recorded, and nothing is written.
#[test]
fn unconfigured_state_records_nothing() {
    let state = CrashState::new();
    assert!(
        state.on_crash(CrashSource::UnhandledFilter, fault(), None, 1, |_| {
            panic!("no dump without a sessions dir")
        })
    );
    assert!(state.on_exit(ExitPath::CrtExit, 0, 1));
}

#[test]
fn exit_is_reported_once_and_flags_a_prior_crash() {
    let dir = tempfile::tempdir().unwrap();
    let (state, consumer, _) = state_in(dir.path());
    state.on_crash(CrashSource::GameMinidump, fault(), Some(0), 1, |_| {
        DumpOutcome::Written {
            bytes: 1,
            dump_type: 0,
        }
    });
    drain(&consumer);

    assert!(state.on_exit(ExitPath::ExitProcess, 1, 9));
    let ev = drain(&consumer);
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].target, "client.exit");
    assert_eq!(ev[0].fields["after_crash"], true);
    assert_eq!(ev[0].fields["path"], "exit_process");

    // The CRT's exit reaching a hooked ExitProcess is one exit.
    assert!(!state.on_exit(ExitPath::CrtExit, 0, 9));
    assert!(drain(&consumer).is_empty());
}

#[test]
fn clean_quit_is_not_after_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    let (state, consumer, _) = state_in(dir.path());
    assert!(state.on_exit(ExitPath::CrtExit, 0, 9));
    let ev = drain(&consumer);
    assert_eq!(ev[0].fields["after_crash"], false);
    assert_eq!(ev[0].level, "info");
}
