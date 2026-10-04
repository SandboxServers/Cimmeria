//! Self-spawning headless process harness: no Wine, GUI or game files.
use cimmeria_launcher_engine::{archive_worker::*, helper_supervisor::*};
use std::{
    collections::BTreeMap,
    io::{BufReader, Write},
    time::Duration,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn fixture(mode: &str) -> ! {
    let mut input = BufReader::new(std::io::stdin());
    let Some(frame) = read_frame(&mut input).unwrap() else {
        std::process::exit(2);
    };
    let request: ExtractRequest = serde_json::from_slice(&frame).unwrap();
    let event = |kind| WorkerEvent {
        schema_version: 1,
        operation_id: Some(request.operation_id),
        event: kind,
    };
    let emit = |event: &WorkerEvent| {
        println!("{}", serde_json::to_string(event).unwrap());
        std::io::stdout().flush().unwrap();
    };
    let mut code = 0;
    match mode {
        "success" => emit(&event(EventKind::Finished { error: None })),
        "flood" => {
            for n in 0..1000 {
                emit(&event(EventKind::Progress {
                    current: n,
                    total: 1000,
                }));
            }
            emit(&event(EventKind::Finished { error: None }));
        }
        "failure" => {
            emit(&event(EventKind::Finished {
                error: Some(ExtractError::Archive),
            }));
            code = 1;
        }
        "marker" => {
            std::fs::write(&request.destination, b"dispatched").unwrap();
            emit(&event(EventKind::Finished { error: None }));
        }
        "silent" => {}
        "wrong-id" => emit(&WorkerEvent {
            operation_id: Some(Uuid::new_v4()),
            ..event(EventKind::Finished { error: None })
        }),
        "duplicate" => {
            emit(&event(EventKind::Finished { error: None }));
            emit(&event(EventKind::Finished { error: None }));
        }
        "bad-exit" => {
            emit(&event(EventKind::Finished { error: None }));
            code = 1;
        }
        "oversized" => println!("{}", "x".repeat(MAX_FRAME + 1)),
        "cancel" => {
            emit(&event(EventKind::Progress {
                current: 0,
                total: 1,
            }));
            let control = read_frame(&mut input).unwrap().unwrap();
            assert!(cancellation_matches(&control, request.operation_id));
            emit(&event(EventKind::Finished {
                error: Some(ExtractError::Cancelled),
            }));
            code = 1;
        }
        "lock" => {
            let guard = std::fs::File::create(&request.destination).unwrap();
            guard.lock().unwrap();
            emit(&event(EventKind::Progress {
                current: 0,
                total: 1,
            }));
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        "stubborn" => {
            emit(&event(EventKind::Progress {
                current: 0,
                total: 1,
            }));
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        other => panic!("unknown fixture {other}"),
    }
    std::process::exit(code);
}
fn limits() -> Deadlines {
    Deadlines {
        request: Duration::from_secs(2),
        operation: Duration::from_secs(5),
        cancel_grace: Duration::from_millis(500),
        exit: Duration::from_secs(2),
    }
}
fn spec(mode: &str, dir: &std::path::Path) -> HelperCommand {
    HelperCommand {
        executable: std::env::current_exe().unwrap(),
        arguments: vec!["--fixture".into(), mode.into()],
        directory: dir.into(),
        environment: BTreeMap::new(),
    }
}
fn request(dir: &std::path::Path) -> ExtractRequest {
    ExtractRequest {
        schema_version: 1,
        operation_id: Uuid::new_v4(),
        archive: dir.join("fixture.archive"),
        destination: dir.join("output"),
        sha256: "00".repeat(32),
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--fixture") {
        fixture(&args[2]);
    }
    for (mode, expected) in [
        ("success", Outcome::Completed),
        ("failure", Outcome::Failed(ExtractError::Archive)),
        ("silent", Outcome::ReconciliationRequired(Fault::Exit)),
        ("wrong-id", Outcome::ReconciliationRequired(Fault::Protocol)),
        (
            "duplicate",
            Outcome::ReconciliationRequired(Fault::Protocol),
        ),
        ("bad-exit", Outcome::ReconciliationRequired(Fault::Exit)),
        (
            "oversized",
            Outcome::ReconciliationRequired(Fault::Protocol),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (progress, _) = watch::channel(None);
        let mut recorded = false;
        let result = run(
            spec(mode, root.path()),
            request(root.path()),
            CancellationToken::new(),
            progress,
            limits(),
            |pid| {
                assert!(pid > 0);
                recorded = true;
                Ok(())
            },
        )
        .await;
        assert_eq!(result, expected, "{mode}");
        assert!(recorded);
        println!("PASS supervisor {mode}");
    }
    let root = tempfile::tempdir().unwrap();
    let (progress, _) = watch::channel(None);
    let result = run(
        spec("marker", root.path()),
        request(root.path()),
        CancellationToken::new(),
        progress,
        limits(),
        |_| Err(()),
    )
    .await;
    assert_eq!(result, Outcome::NotStarted(Fault::Ownership));
    assert!(!root.path().join("output").exists());
    println!("PASS refused ownership prevents request dispatch");

    for mode in ["cancel", "stubborn"] {
        let root = tempfile::tempdir().unwrap();
        let (progress, mut observations) = watch::channel(None);
        let cancel = CancellationToken::new();
        let signal = cancel.clone();
        let cancellation = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(3), observations.changed())
                .await
                .unwrap()
                .unwrap();
            signal.cancel();
        });
        let result = run(
            spec(mode, root.path()),
            request(root.path()),
            cancel,
            progress,
            limits(),
            |_| Ok(()),
        )
        .await;
        cancellation.await.unwrap();
        assert_eq!(
            result,
            if mode == "cancel" {
                Outcome::Cancelled
            } else {
                Outcome::ReconciliationRequired(Fault::Deadline)
            }
        );
        println!("PASS supervisor {mode}: cooperative result or uncertain forced stop");
    }
    let root = tempfile::tempdir().unwrap();
    let (progress, latest) = watch::channel(None);
    assert_eq!(
        run(
            spec("flood", root.path()),
            request(root.path()),
            CancellationToken::new(),
            progress,
            limits(),
            |_| Ok(())
        )
        .await,
        Outcome::Completed
    );
    assert_eq!(latest.borrow().unwrap().current, 999);
    println!("PASS stalled progress observer retains latest value across 1000 process frames");
    let root = tempfile::tempdir().unwrap();
    let command = spec("lock", root.path());
    let extraction = request(root.path());
    let (progress, mut observation) = watch::channel(None);
    let running = tokio::spawn(run(
        command,
        extraction,
        CancellationToken::new(),
        progress,
        limits(),
        |_| Ok(()),
    ));
    tokio::time::timeout(Duration::from_secs(3), observation.changed())
        .await
        .unwrap()
        .unwrap();
    let guard = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join("output"))
        .unwrap();
    assert!(
        guard.try_lock().is_err(),
        "fixture must hold the lock before abort"
    );
    running.abort();
    assert!(running.await.unwrap_err().is_cancelled());
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if guard.try_lock().is_ok() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "aborted supervisor leaked its direct child"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    println!(
        "PASS supervisor abort stops direct child and releases its OS lock; no Wine guest claim"
    );
}
