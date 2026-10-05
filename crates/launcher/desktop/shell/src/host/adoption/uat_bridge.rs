//! JSON-lines bridge from adoption-native-uat.mjs to the production host. All
//! state lives under `ADOPTION_UAT_ROOT`, so the script can kill this process
//! and reopen the same store.
use super::fixture::*;
use super::*;
use adoption::test_support::CopyFault;
use cimmeria_launcher_engine::{
    install_worker::fixtures, migration::LegacySource, ExtractionBackend, NativeCommand,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, Write},
    path::Path,
};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

enum Origin {
    Mock(MockServer),
    Stalled(StalledOrigin),
}
impl Origin {
    fn new(runtime: &tokio::runtime::Handle, mode: &str, seed: &[u8]) -> Self {
        let response = match mode {
            "signed" => ResponseTemplate::new(200).set_body_bytes(seed.to_vec()),
            "missing" => ResponseTemplate::new(404),
            // One byte more than the signed size.
            "oversized" => ResponseTemplate::new(200).set_body_bytes([seed, &[0]].concat()),
            "stalled" => return Self::Stalled(StalledOrigin::new(seed.to_vec())),
            other => panic!("unknown origin {other}"),
        };
        Self::Mock(runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(path("/seed.zip"))
                .respond_with(response)
                .mount(&server)
                .await;
            server
        }))
    }
    fn url(&self) -> String {
        match self {
            Self::Mock(server) => format!("{}/manifest.json", server.uri()),
            Self::Stalled(held) => held.url.clone(),
        }
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn names(directory: &Path) -> Option<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .ok()?
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Some(names)
}
/// What is actually on disk and in the store, independent of the status reply.
fn evidence(host: &NativeHost, source: &LegacySource, root: &Path) -> serde_json::Value {
    let mut digest = Sha256::new();
    for tree in source_bytes(source) {
        for (path, bytes) in tree {
            digest.update(path.to_string_lossy().as_bytes());
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
    }
    let destination = root
        .join("library")
        .canonicalize()
        .unwrap()
        .join(COPY_FOLDER);
    let store = host.store().unwrap();
    let mut state = store.lock().unwrap();
    let installed = state.installed_content().ok().flatten();
    let preferences = state.preferences().clone();
    // The reviewed patch setting, as Play admission derives it from the records.
    let binding = match state.effective_launch_binding() {
        Ok(None) => serde_json::Value::Null,
        Ok(Some(binding)) if binding.client_patches_enabled => "patches_on".into(),
        Ok(Some(_)) => "patches_off".into(),
        Err(_) => "refused".into(),
    };
    drop(state);
    let state_names = names(&host.root).unwrap();
    serde_json::json!({
        "source_sha256": hex(&digest.finalize()),
        "import_sha256": hex(&Sha256::digest(fs::read(host.root.join("legacy-import.json")).unwrap())),
        "destination": destination,
        "destination_entries": names(&destination),
        "game_executable": destination.join("game/Working/Binaries/SGW.exe").is_file(),
        "owned_directory": installed.as_ref().map(|installed| installed.intent.destination.clone()),
        "backend": installed.as_ref().map(|installed| match installed.intent.backend {
            ExtractionBackend::Native => "native",
            ExtractionBackend::Wine { .. } => "wine",
        }),
        "uninstall_directory": host.install_status().ok().and_then(|status| status.uninstall).map(|target| target.directory),
        "installation_id": installed.as_ref().map(|installed| installed.intent.operation_id),
        "settings_binding": binding,
        "prerequisite_target": host.install_status().ok().and_then(|status| status.runtime_setup),
        "artifacts": names(&host.root.join("adoption-artifacts")),
        "references": state_names.iter().filter(|name| name.starts_with(".adoption-reference-")).count(),
        "plans": state_names.iter().filter(|name| name.starts_with("adoption-plan-")).count(),
        "launcher_summary_consent": preferences.launcher_summary_consent,
        "install_directory": preferences.install_directory,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "JSON-lines production-host bridge for adoption-native-uat.mjs"]
async fn adoption_native_uat_bridge() {
    // Host commands run on a blocking worker, as the Tauri adapter runs them.
    tokio::task::spawn_blocking(bridge).await.unwrap();
}
fn bridge() {
    let runtime = tokio::runtime::Handle::current();
    let root = PathBuf::from(std::env::var_os("ADOPTION_UAT_ROOT").expect("isolated UAT root"));
    let seed_path = root.join("signed-seed.zip");
    let (source, seed) = if root.join("state").exists() {
        (
            LegacySource {
                launcher_directory: root.join("legacy").canonicalize().unwrap(),
                game_directory: root.join("old-game").canonicalize().unwrap(),
            },
            fs::read(&seed_path).unwrap(),
        )
    } else {
        let seed = fixtures::archive(true);
        fs::write(&seed_path, &seed).unwrap();
        fs::create_dir(root.join("library")).unwrap();
        let source = legacy_source(&root, &seed, serde_json::json!({}));
        drop(imported_host(&root, &source));
        (source, seed)
    };
    let helper = super::wine_tests::helper();
    if helper.is_some() {
        super::wine_tests::seed_runtime(&root.join("state"));
    }
    let mut origin = Origin::new(&runtime, "signed", &seed);
    let open = |manifest_url: String| {
        let mut host = NativeHost::new(root.join("state"));
        bundle_prerequisite_helper(&mut host, &root);
        host.adoption_fixture = Some(TestDispatch {
            manifest_url,
            helper: helper.clone(),
            copy_fault: Mutex::new(None),
        });
        host
    };
    let mut host = open(origin.url());
    let mut resume = None;
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let status = |result: Result<AdoptionStatus, AdoptionError>| {
            result.map(|status| serde_json::to_value(status).unwrap())
        };
        let result = match value["command"].as_str().unwrap() {
            "hello" => Ok(serde_json::json!({
                "backend": if helper.is_some() { "wine" } else { "portable" }
            })),
            "origin" => {
                origin = Origin::new(&runtime, value["mode"].as_str().unwrap(), &seed);
                host.adoption_fixture.as_mut().unwrap().manifest_url = origin.url();
                Ok(serde_json::json!({}))
            }
            "fault" => {
                let fault = match value["mode"].as_str().unwrap() {
                    "hold" => {
                        let (send, held) = std::sync::mpsc::channel();
                        resume = Some(send);
                        Some(CopyFault::Hold(held))
                    }
                    "release" => {
                        resume.take().unwrap().send(()).unwrap();
                        None
                    }
                    "after_promotion" => Some(CopyFault::AfterPromotion),
                    other => panic!("unknown fault {other}"),
                };
                if fault.is_some() {
                    *host
                        .adoption_fixture
                        .as_ref()
                        .unwrap()
                        .copy_fault
                        .lock()
                        .unwrap() = fault;
                }
                Ok(serde_json::json!({}))
            }
            // The script stands in for the native folder dialog only.
            "choose" => {
                status(host.begin_adoption_preview(root.join("library"), fixtures::verified(&seed)))
            }
            "toggle_diagnostics" => {
                let preferences = host.adoption_status().unwrap().native.preferences;
                host.dispatch(NativeCommand::SavePreferences {
                    schema_version: 1,
                    expected_revision: preferences.revision,
                    install_directory: preferences.install_directory,
                    launcher_summary_consent: !preferences.launcher_summary_consent,
                })
                .unwrap();
                Ok(serde_json::json!({}))
            }
            "reopen" => {
                host = open(origin.url());
                status(host.adoption_status())
            }
            "evidence" => Ok(evidence(&host, &source, &root)),
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
