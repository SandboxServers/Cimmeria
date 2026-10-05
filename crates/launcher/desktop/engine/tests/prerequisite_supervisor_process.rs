//! Actual subprocess/pipe tests; fixtures perform no MSI or game execution.
use cimmeria_launcher_engine::{
    helper_supervisor::{Deadlines, Fault, HelperCommand},
    prerequisites::supervisor::{self, Outcome},
};
use cimmeria_runtime_probe::{collect, physx::SdkResult, prerequisite::*, LoadResult};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
fn fixture(mode: &str) {
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes).unwrap();
    let Ok(request) = decode_request(&bytes) else {
        return;
    };
    assert_eq!(
        std::fs::read_to_string("host-record").unwrap(),
        std::process::id().to_string(),
        "identity persisted before request"
    );
    if mode == "hang" {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    if mode == "oversized" {
        println!("{}", "x".repeat(MAX_RESULT + 1));
        return;
    }
    if mode == "empty" {
        return;
    }
    let mut report = collect(LoadResult::Loaded {}, |_| LoadResult::Loaded {});
    report.physx_sdk = SdkResult::CreateFailed { sdk_error: Some(1) };
    let result = PrepareResult {
        schema_version: 1,
        operation_id: if mode == "wrong-id" {
            Uuid::nil()
        } else {
            request.operation_id
        },
        prefix_generation: request.prefix_generation,
        result: ResultKind::Probed { report },
    };
    println!("{}", serde_json::to_string(&result).unwrap());
    std::io::stdout().flush().unwrap();
    if mode == "duplicate" {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
    if mode == "bad-exit" {
        std::process::exit(1);
    }
    if mode == "no-exit" {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
fn spec(root: &Path, mode: &str) -> HelperCommand {
    HelperCommand {
        executable: std::env::current_exe().unwrap(),
        arguments: vec!["--fixture".into(), mode.into()],
        directory: root.into(),
        environment: BTreeMap::new(),
    }
}
fn request(_root: &Path) -> PrepareRequest {
    PrepareRequest {
        schema_version: 1,
        operation_id: Uuid::new_v4(),
        prefix_generation: Uuid::new_v4(),
        game_binaries: r"C:\fixture\game".into(),
        package: r"C:\fixture\package.exe".into(),
        scratch: r"C:\fixture\scratch".into(),
    }
}
fn limits() -> Deadlines {
    Deadlines {
        request: Duration::from_secs(2),
        operation: Duration::from_secs(3),
        exit: Duration::from_secs(1),
        cancel_grace: Duration::ZERO,
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--fixture") {
        fixture(&args[2]);
        return;
    }
    let root = tempfile::tempdir().unwrap();
    for (mode, expected) in [
        ("observed", None),
        ("wrong-id", Some(Fault::Protocol)),
        ("oversized", Some(Fault::Protocol)),
        ("empty", Some(Fault::Protocol)),
        ("duplicate", Some(Fault::Protocol)),
        ("bad-exit", Some(Fault::Exit)),
        ("no-exit", Some(Fault::Deadline)),
        ("hang", Some(Fault::Deadline)),
    ] {
        let outcome = supervisor::run(
            spec(root.path(), mode),
            request(root.path()),
            CancellationToken::new(),
            limits(),
            |pid| std::fs::write(root.path().join("host-record"), pid.to_string()).map_err(|_| ()),
        )
        .await;
        match (outcome, expected) {
            (Outcome::Observed(result), None) => assert!(
                matches!(result.result, ResultKind::Probed { report } if report.physx_sdk == (SdkResult::CreateFailed { sdk_error: Some(1) }))
            ),
            (Outcome::ReconciliationRequired(actual), Some(expected)) => {
                assert_eq!(actual, expected, "{mode}")
            }
            (actual, expected) => panic!("{mode}: {actual:?} vs {expected:?}"),
        }
        println!("PASS prerequisite process {mode}");
    }
    assert!(matches!(
        supervisor::run(
            spec(root.path(), "observed"),
            request(root.path()),
            CancellationToken::new(),
            limits(),
            |_| Err(())
        )
        .await,
        Outcome::NotStarted(Fault::Ownership)
    ));
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        supervisor::run(
            spec(root.path(), "observed"),
            request(root.path()),
            token,
            limits(),
            |_| panic!("pre-cancel cannot spawn")
        )
        .await,
        Outcome::NotStarted(Fault::Cancelled)
    ));
    let token = CancellationToken::new();
    let canceller = token.clone();
    let outcome = supervisor::run(
        spec(root.path(), "hang"),
        request(root.path()),
        token,
        limits(),
        |pid| {
            std::fs::write(root.path().join("host-record"), pid.to_string()).unwrap();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                canceller.cancel();
            });
            Ok(())
        },
    )
    .await;
    assert!(matches!(
        outcome,
        Outcome::ReconciliationRequired(Fault::Cancelled)
    ));
    println!("PASS ownership refusal, pre-dispatch cancellation, in-flight cancellation");
}
