//! JSON-lines bridge from adoption-in-place-uat.mjs to the production host,
//! where adoption is in place (D-LX12). All state lives under
//! `ADOPTION_UAT_ROOT`; the signed origin is a loopback mock.
use super::fixture::*;
use super::*;
use cimmeria_launcher_engine::install_worker::fixtures;
use std::{
    fs,
    io::{BufRead, Write},
    path::Path,
};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

/// Index names use `/`; a verbatim folder takes separators literally.
fn under(root: &Path, name: &str) -> PathBuf {
    name.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}
/// What is actually on disk and in the store, independent of the status reply.
fn evidence(host: &NativeHost, folder: &Path) -> serde_json::Value {
    let store = host.store().unwrap();
    let installed = store.lock().unwrap().installed_content().ok().flatten();
    serde_json::json!({
        "folder": folder,
        "owner_marker": folder.join(".cimmeria-install.json").is_file(),
        "owned_directory": installed.map(|installed| installed.intent.destination),
        "executable_in_game": under(folder, "game/Working/Binaries/SGW.exe").is_file(),
        "executable_in_old_place": under(folder, "Working/Binaries/SGW.exe").is_file(),
        "player_file": fs::read(folder.join("unknown.dll")).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        "uninstall_directory": host.install_status().ok().and_then(|status| status.uninstall).map(|target| target.directory),
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "JSON-lines production-host bridge for adoption-in-place-uat.mjs"]
async fn adoption_in_place_uat_bridge() {
    // Host commands run on a blocking worker, as the Tauri adapter runs them.
    tokio::task::spawn_blocking(bridge).await.unwrap();
}
fn bridge() {
    let runtime = tokio::runtime::Handle::current();
    let root = PathBuf::from(std::env::var_os("ADOPTION_UAT_ROOT").expect("isolated UAT root"));
    let seed = fixtures::archive(true);
    let source = legacy_source(&root, &seed, serde_json::json!({}));
    let folder = source.game_directory.clone();
    let mut host = imported_host(&root, &source);
    let server = runtime.block_on(async {
        let server = MockServer::start().await;
        Mock::given(path("/seed.zip"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(seed.clone()))
            .mount(&server)
            .await;
        server
    });
    host.adoption_fixture = Some(TestDispatch {
        manifest_url: format!("{}/manifest.json", server.uri()),
        #[cfg(target_os = "macos")]
        helper: None,
        copy_fault: Mutex::new(None),
    });
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let status = |result: Result<AdoptionStatus, AdoptionError>| {
            result.map(|status| serde_json::to_value(status).unwrap())
        };
        let result = match value["command"].as_str().unwrap() {
            "hello" => Ok(serde_json::json!({"backend": "in_place"})),
            "choose" => status(host.begin_adoption_in_place(fixtures::verified(&seed))),
            "evidence" => Ok(evidence(&host, &folder)),
            // The engine's confirmed uninstall of the adopted folder; its own
            // view is not part of this UAT.
            "uninstall" => {
                let store = host.store().unwrap();
                let mut state = store.lock().unwrap();
                let installed = state.installed_content().unwrap().unwrap();
                let revision = state.operations().snapshot().revision;
                state
                    .uninstall(
                        Uuid::new_v4(),
                        revision,
                        installed.intent.operation_id,
                        true,
                    )
                    .unwrap();
                Ok(serde_json::json!({}))
            }
            "plant_hard_link" => {
                fs::hard_link(folder.join("later.txt"), folder.join("later-link.txt")).unwrap();
                Ok(serde_json::json!({}))
            }
            "remove_hard_link" => {
                fs::remove_file(folder.join("later-link.txt")).unwrap();
                Ok(serde_json::json!({}))
            }
            _ => status(host.adoption_command(serde_json::from_value(value).unwrap())),
        };
        let reply = match result {
            Ok(value) => serde_json::json!({"ok": value}),
            Err(error) => serde_json::json!({"error": error}),
        };
        println!("ADOPTION_NATIVE_UAT {reply}");
        std::io::stdout().flush().unwrap();
    }
}
