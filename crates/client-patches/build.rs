//! Embeds an `asInvoker` UAC manifest in this crate's 32-bit Windows
//! binaries.
//!
//! Windows' installer detection treats a 32-bit executable with no manifest
//! and "patch" in its file name as an installer, and refuses to start it
//! without elevation (os error 740). The unit-test harness is
//! `cimmeria_client_patches-<hash>.exe`, so `cargo test --target
//! i686-pc-windows-msvc` fails on any machine with UAC on. 64-bit
//! executables are exempt, and nothing here touches other targets.
//!
//! `rustc-link-arg` rather than `rustc-link-arg-tests`, because the latter
//! only reaches `tests/` integration targets, not the library's own test
//! harness. So the DLL gets the manifest too, as resource 2. A DLL's
//! manifest only steers how the loader binds the DLL's own imports, and
//! this one declares no dependent assemblies, so it changes nothing when
//! the DLL is injected.

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if arch == "x86" && env == "msvc" {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTUAC:level='asInvoker'");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
