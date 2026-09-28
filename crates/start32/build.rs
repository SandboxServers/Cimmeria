//! Gives the 32-bit `sgw-start32` helper an identity Windows and antivirus
//! software can see:
//!
//! - an `asInvoker` UAC manifest. Windows' installer detection treats an
//!   unmanifested 32-bit executable whose name suggests an installer
//!   (setup, install, update, patch) as one and refuses to start it without
//!   elevation (os error 740). The helper's name avoids those words, and a
//!   manifested executable is never subject to the heuristic;
//! - a version resource (product, company, description, version).
//!
//! Only the i686 MSVC build is touched: that is the only helper that ships,
//! and the library the launcher and the lab link is unaffected.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if arch != "x86" || env != "msvc" {
        return;
    }
    println!("cargo:rustc-link-arg-bin=sgw-start32=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bin=sgw-start32=/MANIFESTUAC:level='asInvoker'");

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let mut res = winres::WindowsResource::new();
    res.set("ProductName", "Cimmeria SGW launcher")
        .set("CompanyName", "SandboxServers Cimmeria")
        .set(
            "FileDescription",
            "Cimmeria sgw-start32: starts SGW.exe and loads the Cimmeria client DLLs",
        )
        .set("InternalName", "sgw-start32")
        .set("OriginalFilename", "sgw-start32.exe")
        .set("ProductVersion", &version)
        .set("FileVersion", &version)
        .set("LegalCopyright", "SandboxServers Cimmeria contributors");
    if let Err(e) = res.compile() {
        // A missing resource compiler must not break a dev build; the
        // release build checks the version resource is there.
        println!("cargo:warning=winres compile failed (no version resource): {e}");
    }
}
