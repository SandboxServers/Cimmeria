//! Boot tests for the injected client DLLs, without the game.
//!
//! Each test copies `sgw-testhost.exe` (a 32-bit stand-in for `SGW.exe`)
//! into a scratch directory, starts it through `sgw-start32` with one or
//! both DLLs injected, the way the launcher does, and waits for it to exit.
//! The host has none of the game's code at the hook addresses, so every
//! DLL must:
//!
//! - run `DllMain` and its bootstrap thread (the log's `attached` line);
//! - fail its fingerprint gate closed: log every site, install nothing,
//!   and say so;
//! - leave the process alive and let it exit normally (exit code 0, and
//!   a main loop that kept running).
//!
//! The telemetry DLL's upload reaches a mock endpoint, and the lab bridge
//! answers its handshake on loopback.
//!
//! Needs the staged i686 build: `tools/testhost/stage.sh`. Without it the
//! tests skip, unless `CIMMERIA_TESTHOST_REQUIRE` is set (CI sets it).

#![cfg(windows)]

mod support;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use support::{free_port, messages, session, Install, MockUpload, Stage};

const PATCHES_LOG: &str = "cimmeria-client-patches.log";
const TELEMETRY_LOG: &str = "cimmeria-client-telemetry.log";
/// The DLLs share the workspace version with this crate.
const VERSION: &str = env!("CARGO_PKG_VERSION");
const RUN_MS: u64 = 3_000;
const TOKEN: &str = "test-token";

/// The host ran its whole main loop and exited 0: no DLL froze or killed it.
fn assert_clean_exit(install: &Install, code: i32, run_ms: u64) {
    assert_eq!(code, 0, "host exit code");
    let (frames, reported) = install
        .host_report()
        .expect("the host wrote sgw-testhost.out");
    assert_eq!(reported, 0);
    // 16 ms frames; allow for a slow runner, but a frozen loop gives ~0.
    let expected = run_ms / 16;
    assert!(
        frames * 3 >= expected,
        "only {frames} frames in {run_ms} ms (expected about {expected})"
    );
}

/// The patches DLL booted, failed its gate on every site, and installed
/// nothing, without waiting for a lua51.dll that never comes.
fn assert_patches_failed_closed(log: &str) {
    let m = messages(log);
    assert!(
        m.iter()
            .any(|l| l.starts_with(&format!("attached, version {VERSION}"))),
        "no attach line:\n{log}"
    );
    let sites: Vec<_> = m
        .iter()
        .filter(|l| l.contains(" at 0x") && !l.contains(" loaded at 0x"))
        .collect();
    assert_eq!(
        sites.len(),
        cimmeria_client_patches::fingerprint::SITES.len(),
        "one line per site:\n{log}"
    );
    assert!(
        sites.iter().all(|l| !l.ends_with(": stock")),
        "a site matched in a process that is not SGW.exe:\n{log}"
    );
    assert!(
        m.last().is_some_and(|l| l.ends_with("nothing installed")),
        "verdict:\n{log}"
    );
    assert!(!m.iter().any(|l| l.starts_with("hooked ")), "{log}");
    assert!(
        !m.iter().any(|l| l.contains("lua51.dll")),
        "waited for Lua:\n{log}"
    );
}

/// The telemetry DLL booted with its session, failed its gate on every
/// site, and installed no hooks.
fn assert_telemetry_failed_closed(log: &str) {
    let m = messages(log);
    assert!(
        m.iter()
            .any(|l| l.starts_with(&format!("attached, version {VERSION}"))),
        "no attach line:\n{log}"
    );
    assert!(
        m.iter().any(|l| l.starts_with("session s-testhost loaded")),
        "session:\n{log}"
    );
    let sites: Vec<_> = m
        .iter()
        .filter(|l| l.contains(" at 0x") && !l.contains(" loaded at 0x"))
        .collect();
    let expected = cimmeria_client_telemetry::fingerprint::CODE_SITES.len()
        + cimmeria_client_telemetry::fingerprint::SLOT_SITES.len();
    assert_eq!(sites.len(), expected, "one line per site:\n{log}");
    assert!(sites.iter().all(|l| !l.ends_with(": stock")), "{log}");
    assert!(
        m.iter()
            .any(|l| l.starts_with("fingerprint mismatch") && l.ends_with("no hooks installed")),
        "verdict:\n{log}"
    );
    assert!(!m.iter().any(|l| l == "hooks installed"), "{log}");
    assert!(
        !m.iter()
            .any(|l| l.contains("client.hooks.inline.installed")),
        "{log}"
    );
}

#[test]
fn client_patches_fails_closed_and_the_host_exits_cleanly() {
    let Some(stage) = Stage::find() else { return };
    let install = Install::new(&stage);
    let dll = install.add_dll(&stage.patches());

    let code = install.launch(&[dll], RUN_MS).wait();

    assert_clean_exit(&install, code, RUN_MS);
    assert_patches_failed_closed(&install.log(PATCHES_LOG));
}

#[test]
fn telemetry_fails_closed_and_uploads_why() {
    let Some(stage) = Stage::find() else { return };
    let upload = MockUpload::start();
    let install = Install::new(&stage);
    install.write_session(&session(&upload.url, TOKEN, None));
    let dll = install.add_dll(&stage.telemetry());

    let code = install.launch(&[dll], RUN_MS).wait();

    assert_clean_exit(&install, code, RUN_MS);
    let log = install.log(TELEMETRY_LOG);
    assert_telemetry_failed_closed(&log);
    assert!(
        !messages(&log).iter().any(|l| l.contains("lab bridge")),
        "a default build has no bridge:\n{log}"
    );

    // The "out" path: the uploader reached the endpoint with the session's
    // token, and SigNoz would see both the attach and the gate's verdict.
    let uploads = upload.uploads();
    assert!(!uploads.is_empty(), "nothing uploaded; log:\n{log}");
    assert!(uploads
        .iter()
        .all(|u| u.path == "/api/telemetry/upload-chunk"
            && u.authorization.as_deref() == Some(&*format!("Bearer {TOKEN}"))));
    let events = upload.events();
    let attached = events
        .iter()
        .find(|e| e["target"] == "client.dll.attached")
        .expect("client.dll.attached uploaded");
    assert_eq!(attached["fields"]["session_id"], "s-testhost");
    assert_eq!(attached["fields"]["dll_flavor"], "player");
    let gate = events
        .iter()
        .find(|e| e["target"] == "client.hooks.fingerprint")
        .expect("client.hooks.fingerprint uploaded");
    assert_eq!(gate["level"], "warn");
    assert_eq!(gate["fields"]["usable"], false);
    let tick = &gate["fields"]["site.FEngineLoop::Tick"];
    assert!(tick == "unreadable" || tick == "mismatch", "{gate}");
}

fn image_has_lab_marker(dll: &std::path::Path) -> bool {
    let image = std::fs::read(dll).expect("read the staged DLL");
    let marker = cimmeria_client_telemetry::LAB_BRIDGE_MARKER.as_bytes();
    image.windows(marker.len()).any(|w| w == marker)
}

/// The image check the launcher and the release `verify` stage run on the
/// telemetry DLL, against the real builds: the default (player) build is
/// clean, and the lab-bridge build is caught, so the check can fail.
#[test]
fn only_the_lab_bridge_build_carries_the_lab_marker() {
    let Some(stage) = Stage::find() else { return };
    assert!(
        !image_has_lab_marker(&stage.telemetry()),
        "the default build carries the lab-bridge marker"
    );
    assert!(
        image_has_lab_marker(&stage.telemetry_lab()),
        "the lab-bridge build lacks the marker; the packaging check would pass it"
    );
}

/// The build players get opens no command port, even when the session
/// file carries a lab block: the bridge is not compiled in.
#[test]
fn the_player_build_ignores_a_lab_block_and_opens_no_port() {
    let Some(stage) = Stage::find() else { return };
    let upload = MockUpload::start();
    let install = Install::new(&stage);
    let port = free_port();
    install.write_session(&session(
        &upload.url,
        TOKEN,
        Some(serde_json::json!({ "bind": "127.0.0.1", "port": port, "token": "ab".repeat(32) })),
    ));
    let dll = install.add_dll(&stage.telemetry());
    let run_ms = 6_000;
    let host = install.launch(&[dll], run_ms);

    // Once the session is loaded, the lab build would be listening.
    let deadline = Instant::now() + Duration::from_secs(4);
    while !messages(&install.log(TELEMETRY_LOG))
        .iter()
        .any(|l| l.starts_with("session ") && l.contains(" loaded;"))
    {
        assert!(
            Instant::now() < deadline,
            "the DLL never loaded its session"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(500));
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    assert!(
        std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_err(),
        "the player build accepted a connection on the lab port"
    );

    let code = host.wait();
    assert_clean_exit(&install, code, run_ms);
    let log = install.log(TELEMETRY_LOG);
    let m = messages(&log);
    assert!(
        m.iter()
            .any(|l| l == cimmeria_client_telemetry::BUILD_MARKER),
        "{log}"
    );
    assert!(!m.iter().any(|l| l.contains("lab bridge")), "{log}");
}

/// Without a session file the DLL logs why and stays inert.
#[test]
fn telemetry_without_a_session_stays_inert() {
    let Some(stage) = Stage::find() else { return };
    let install = Install::new(&stage);
    let dll = install.add_dll(&stage.telemetry());

    let code = install.launch(&[dll], RUN_MS).wait();

    assert_clean_exit(&install, code, RUN_MS);
    let log = install.log(TELEMETRY_LOG);
    let m = messages(&log);
    assert!(
        m.iter()
            .any(|l| l.starts_with(&format!("attached, version {VERSION}"))),
        "{log}"
    );
    assert!(
        m.iter()
            .any(|l| l.starts_with("no telemetry session") && l.ends_with("nothing installed")),
        "{log}"
    );
    assert!(!m.iter().any(|l| l.contains(" at 0x")), "{log}");
}

/// Both DLLs in one process, in each injection order: both boot, both fail
/// closed, and the host exits cleanly.
#[test]
fn both_dlls_in_either_order() {
    let Some(stage) = Stage::find() else { return };
    for patches_first in [true, false] {
        let upload = MockUpload::start();
        let install = Install::new(&stage);
        install.write_session(&session(&upload.url, TOKEN, None));
        let patches = install.add_dll(&stage.patches());
        let telemetry = install.add_dll(&stage.telemetry());
        let order = if patches_first {
            vec![patches, telemetry]
        } else {
            vec![telemetry, patches]
        };

        let code = install.launch(&order, RUN_MS).wait();

        assert_clean_exit(&install, code, RUN_MS);
        assert_patches_failed_closed(&install.log(PATCHES_LOG));
        assert_telemetry_failed_closed(&install.log(TELEMETRY_LOG));
    }
}

/// The lab build: the bridge comes up on the session's loopback port,
/// accepts the token handshake, refuses a wrong token, and answers a
/// request. With no `FEngineLoop::Tick` hook here nothing drains the
/// queue, so the answer is the documented dispatch timeout.
#[test]
fn lab_bridge_answers_its_handshake_on_loopback() {
    let Some(stage) = Stage::find() else { return };
    let upload = MockUpload::start();
    let install = Install::new(&stage);
    let port = free_port();
    let token = "ab".repeat(32);
    install.write_session(&session(
        &upload.url,
        TOKEN,
        Some(serde_json::json!({ "bind": "127.0.0.1", "port": port, "token": token })),
    ));
    let dll = install.add_dll(&stage.telemetry_lab());
    let run_ms = 12_000;
    let host = install.launch(&[dll], run_ms);
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    // A wrong token is refused: the bridge closes the connection.
    let mut intruder = support::connect(addr, Duration::from_secs(8));
    support::write_frame(
        &mut intruder,
        serde_json::json!({ "token": "cd".repeat(32) })
            .to_string()
            .as_bytes(),
    );
    assert!(
        support::read_frame(&mut intruder).is_err(),
        "wrong token accepted"
    );

    let mut client = support::connect(addr, Duration::from_secs(2));
    support::write_frame(
        &mut client,
        serde_json::json!({ "token": token }).to_string().as_bytes(),
    );
    let ack: serde_json::Value =
        serde_json::from_slice(&support::read_frame(&mut client).expect("handshake ack")).unwrap();
    assert_eq!(ack, serde_json::json!({ "ok": true }));

    let started = Instant::now();
    support::write_frame(
        &mut client,
        br#"{"jsonrpc":"2.0","id":7,"method":"heartbeat"}"#,
    );
    let reply: serde_json::Value =
        serde_json::from_slice(&support::read_frame(&mut client).expect("a reply")).unwrap();
    assert_eq!(reply["id"], 7);
    assert!(
        reply["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("dispatch timeout")),
        "{reply}"
    );
    assert!(
        started.elapsed() >= Duration::from_secs(4),
        "answered before the timeout"
    );
    drop(client);

    let code = host.wait();
    assert_clean_exit(&install, code, run_ms);
    let log = install.log(TELEMETRY_LOG);
    assert_telemetry_failed_closed(&log);
    assert!(
        messages(&log).iter().any(|l| l
            .starts_with(&format!("lab bridge listening on 127.0.0.1:{port}"))
            && l.contains("not hooked")),
        "{log}"
    );
}
