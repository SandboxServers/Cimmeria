fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let icon = "icons/icon.ico";
        if std::path::Path::new(icon).exists() {
            let mut res = winres::WindowsResource::new();
            let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
            // A version resource gives the launcher an identity antivirus
            // software and the Windows file properties can show; it starts
            // SGW.exe through sgw-start32, which injects DLLs.
            res.set_icon(icon)
                .set("ProductName", "Cimmeria SGW launcher")
                .set("CompanyName", "SandboxServers Cimmeria")
                .set("FileDescription", "Stargate Worlds launcher (Cimmeria)")
                .set("InternalName", "sgw-launcher")
                .set("OriginalFilename", "sgw-launcher.exe")
                .set("ProductVersion", &version)
                .set("FileVersion", &version)
                .set("LegalCopyright", "SandboxServers Cimmeria contributors");
            if let Err(e) = res.compile() {
                println!("cargo:warning=winres compile failed (skipping icon embed): {e}");
            }
        }
    }
    // The launcher stays 64-bit, but all of these are 32-bit: the DLLs are
    // injected into the 32-bit SGW.exe, and the helper does the injection
    // at that bitness. See src/bundled.rs for how they reach the disk.
    embed(
        "CIMMERIA_CLIENT_PATCHES_DLL",
        "cimmeria-client-patches.dll",
        None,
    );
    embed("CIMMERIA_START32_EXE", "sgw-start32.exe", None);
    // Injected only when the player opted in to telemetry. It must be the
    // player build: a lab-bridge build opens an inbound TCP listener.
    embed(
        "CIMMERIA_CLIENT_TELEMETRY_DLL",
        "cimmeria-client-telemetry.dll",
        Some(LAB_BRIDGE_MARKER),
    );
}

/// Text only a `lab-bridge` build of the telemetry DLL carries
/// (`cimmeria_client_telemetry::LAB_BRIDGE_MARKER`; the launcher's
/// `client_telemetry_dll` tests pin that the copies agree).
const LAB_BRIDGE_MARKER: &str = "cimmeria-client-telemetry build flavour: lab-bridge";

/// Bundle an i686 artifact into the launcher.
///
/// The release workflow builds it first and points `var` at it; the
/// launcher `include_bytes!`s the copy written here and writes it to disk
/// at launch. Without the variable (every dev and CI build) the copy is
/// empty, and the launcher looks for the file beside itself instead.
/// An image containing `forbidden` fails the build.
fn embed(var: &str, file_name: &str, forbidden: Option<&str>) {
    println!("cargo:rerun-if-env-changed={var}");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join(file_name);
    let bytes = match std::env::var(var) {
        Ok(path) if !path.trim().is_empty() => {
            println!("cargo:rerun-if-changed={path}");
            // A release build that asked for the file must not ship without
            // it, so a bad path fails the build rather than warning.
            let bytes = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("{var}={path}: cannot read {file_name}: {e}"));
            assert!(
                bytes.starts_with(b"MZ"),
                "{var}={path}: not a PE image (no MZ header)"
            );
            if let Some(text) = forbidden {
                assert!(
                    !bytes.windows(text.len()).any(|w| w == text.as_bytes()),
                    "{var}={path}: {file_name} is a lab-bridge build ({text:?}); \
                     players get the build without the feature"
                );
            }
            bytes
        }
        _ => Vec::new(),
    };
    std::fs::write(&out, bytes).unwrap_or_else(|e| panic!("write the embedded {file_name}: {e}"));
}
