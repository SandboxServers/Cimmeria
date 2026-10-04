//! The executable's real Windows stdio path, not a mock or a Wine compatibility test.
#![cfg(windows)]
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[test]
fn helper_reports_one_terminal_result_with_operation_identity() {
    for valid_hash in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("input with spaces.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        zip.start_file("payload.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"fixture").unwrap();
        zip.finish().unwrap();
        let hash = Sha256::digest(std::fs::read(&archive).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let id = Uuid::new_v4();
        let destination = root.path().join("output ü with spaces");
        let request = serde_json::json!({"schema_version":1,"operation_id":id,"archive":archive,"destination":destination,"sha256":if valid_hash {hash}else{"00".repeat(32)}});
        let mut child = Command::new(env!("CARGO_BIN_EXE_cimmeria-archive-worker"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
        // Keep the ownership pipe open until completion; EOF means cancel.
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("helper timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        drop(input);
        assert_eq!(output.status.success(), valid_hash);
        let events = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        let terminal = events
            .iter()
            .filter(|event| event["event"] == "finished")
            .collect::<Vec<_>>();
        assert_eq!(terminal.len(), 1);
        assert_eq!(terminal[0]["operation_id"], id.to_string());
        if valid_hash {
            assert!(terminal[0]["error"].is_null());
            assert_eq!(
                std::fs::read(destination.join("payload.txt")).unwrap(),
                b"fixture"
            );
        } else {
            assert_eq!(terminal[0]["error"], "hash_mismatch");
            assert!(!destination.exists());
        }
    }
}
