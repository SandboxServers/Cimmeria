fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    println!("cargo:warning=Building launcher from: {}", manifest_dir);

    let config_path = std::path::Path::new(&manifest_dir).join("tauri.conf.json");
    let ui_path = std::path::Path::new(&manifest_dir).join("ui");
    let index_path = ui_path.join("index.html");

    println!("cargo:warning=Config exists: {}", config_path.exists());
    println!("cargo:warning=UI dir exists: {}", ui_path.exists());
    println!("cargo:warning=Index.html exists: {}", index_path.exists());

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tauri_build::build()
    })) {
        Ok(_) => println!("cargo:warning=tauri_build succeeded"),
        Err(_) => println!("cargo:warning=tauri_build PANICKED"),
    }
}
