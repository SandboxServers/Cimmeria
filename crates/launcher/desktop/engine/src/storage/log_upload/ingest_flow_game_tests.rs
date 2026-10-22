//! Game-log rows through the upload flow (LF-19).
use super::*;

/// Every NDJSON line the double received, in order.
async fn posted_rows(server: &MockServer) -> Vec<String> {
    use std::io::Read;
    let mut rows = Vec::new();
    for request in server.received_requests().await.unwrap() {
        if request.url.path() != CHUNK {
            continue;
        }
        let mut text = String::new();
        flate2::read::GzDecoder::new(&request.body[..])
            .read_to_string(&mut text)
            .unwrap();
        rows.extend(text.lines().map(str::to_string));
    }
    rows
}

#[tokio::test]
async fn one_upload_carries_both_targets_under_one_upload_id() {
    let root = tempfile::tempdir().unwrap();
    let server = accepting_server().await;
    let state = state_at(root.path(), &server.uri());
    write_log(root.path(), 3);
    let game = root.path().join("game");
    let binaries = game.join("Working").join("Binaries");
    std::fs::create_dir_all(&binaries).unwrap();
    std::fs::write(
        binaries.join("SGWDebugLog.log"),
        "[Say] Bob: hello\nsecond\n",
    )
    .unwrap();
    let uploader = LogUploader::new();
    let mut ingest = ingest_of(state.begin_log_upload(upload(), &uploader).unwrap());
    ingest.game = Some(game.clone());
    let mut pacer = FakePacer::default();
    let status = ingest.run_with(&mut pacer).await.unwrap().state;
    let LogUploadState::Sent {
        lines_sent,
        lines_total,
        upload_id,
    } = status
    else {
        panic!("expected sent, got {status:?}");
    };
    assert_eq!((lines_sent, lines_total), (5, 5));
    let rows = posted_rows(&server).await;
    let launcher = rows
        .iter()
        .filter(|r| r.contains(LAUNCHER_LOG_TARGET))
        .count();
    let game_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.contains(GAME_LOG_TARGET))
        .collect();
    assert_eq!((launcher, game_rows.len()), (3, 2));
    assert!(rows.iter().all(|r| r.contains(&upload_id)), "one upload_id");
    assert!(game_rows[0].contains("[Say] Bob: hello"), "unfiltered");
    assert!(game_rows[0].contains("SGWDebugLog.log"));
    assert!(!game_rows[0].contains("Binaries"), "a name, never a path");
    let saved = Offsets::load(&state.directory.root);
    assert_eq!(
        saved.files.get("game/SGWDebugLog.log").copied(),
        Some(24),
        "the game offset moved with the accepted chunk"
    );
    assert!(saved.heads.contains_key("game/SGWDebugLog.log"));
    let seqs: Vec<u64> = rows
        .iter()
        .map(|r| {
            let v: serde_json::Value = serde_json::from_str(r).unwrap();
            v["seq"].as_u64().unwrap()
        })
        .collect();
    assert_eq!(seqs, vec![0, 1, 2, 3, 4], "game rows continue the seq");
    let again = ingest_of(state.begin_log_upload(upload(), &uploader).unwrap());
    let mut again = again;
    again.game = Some(game);
    assert_eq!(
        again.run_with(&mut pacer).await.unwrap().state,
        LogUploadState::NothingNew
    );
}

#[tokio::test]
async fn a_throttled_launcher_chunk_sends_no_game_rows() {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "600"))
        .mount(&server)
        .await;
    write_log(root.path(), 1_001);
    let game = root.path().join("game");
    let binaries = game.join("Working").join("Binaries");
    std::fs::create_dir_all(&binaries).unwrap();
    std::fs::write(binaries.join("SGWDebugLog.log"), "[Say] Bob: hello\n").unwrap();
    let state = state_at(root.path(), &server.uri());
    let uploader = LogUploader::new();
    let mut ingest = ingest_of(state.begin_log_upload(upload(), &uploader).unwrap());
    ingest.game = Some(game);
    let mut pacer = FakePacer::default();
    let status = ingest.run_with(&mut pacer).await.unwrap().state;
    assert!(
        matches!(status, LogUploadState::Partial { .. }),
        "expected partial, got {status:?}"
    );
    let sent = posted_rows(&server).await;
    assert!(!sent.is_empty(), "the first launcher chunk was posted");
    assert!(
        sent.iter().all(|r| !r.contains(GAME_LOG_TARGET)),
        "no game row after a 429 on the launcher rows"
    );
    let saved = Offsets::load(&state.directory.root);
    assert!(!saved.files.contains_key("game/SGWDebugLog.log"));
}

#[tokio::test]
async fn game_logs_alone_are_enough_to_send() {
    let root = tempfile::tempdir().unwrap();
    let server = accepting_server().await;
    let state = state_at(root.path(), &server.uri());
    let game = root.path().join("game");
    let binaries = game.join("Working").join("Binaries");
    std::fs::create_dir_all(&binaries).unwrap();
    std::fs::write(binaries.join("SGWDebugLog.log"), "only game\n").unwrap();
    let uploader = LogUploader::new();
    let mut ingest = ingest_of(state.begin_log_upload(upload(), &uploader).unwrap());
    ingest.game = Some(game);
    assert!(matches!(
        ingest
            .run_with(&mut FakePacer::default())
            .await
            .unwrap()
            .state,
        LogUploadState::Sent { lines_sent: 1, .. }
    ));
}
