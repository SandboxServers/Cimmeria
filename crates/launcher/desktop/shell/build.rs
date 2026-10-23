mod build_payload;
#[path = "src/release_version.rs"]
mod release_version;

fn main() {
    build_payload::generate().expect("embedded launcher payload could not be generated");
    println!("cargo:rerun-if-env-changed=CIMMERIA_LAUNCHER_VERSION");
    // A release build is a plain `cargo build` (tools/launcher-release), not
    // `tauri build --config`, so the version goes in here, on every build path.
    let version = std::env::var("CIMMERIA_LAUNCHER_VERSION")
        .ok()
        .filter(|version| !version.trim().is_empty());
    if let Some(version) = version {
        let patch = release_version::validate(&version).and_then(|()| {
            release_version::config_patch(std::env::var("TAURI_CONFIG").ok().as_deref(), &version)
        });
        match patch {
            Ok(patch) => {
                // tauri-build reads it from this process, `generate_context!`
                // from rustc's environment. Both merge it over tauri.conf.json.
                std::env::set_var("TAURI_CONFIG", &patch);
                println!("cargo:rustc-env=TAURI_CONFIG={patch}");
            }
            Err(message) => {
                eprintln!("error: {message}");
                std::process::exit(1);
            }
        }
    }
    tauri_build::build()
}
