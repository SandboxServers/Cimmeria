//! Opt-in macOS application identity for the Wine processes of an interactive Play.
//!
//! A Wine guest process owns its own macOS windows, and it always re-executes the
//! loader that sits beside the `ntdll.so` it loaded. So the only process that can
//! carry a bundle identifier for the game window is that loader, and it carries one
//! only when its real path is `<bundle>.app/Contents/MacOS/`. This stages such a
//! bundle from the verified runtime: real copies of the loader directory, links back
//! to the runtime for everything else. A separate wrapper process never owns a window.
use super::*;
use std::{
    ffi::OsStr,
    fs, io,
    os::unix::fs::{symlink, OpenOptionsExt, PermissionsExt},
};

pub(crate) const BUNDLE_IDENTIFIER: &str = "app.cimmeria.stargate-worlds";
pub(crate) const DISPLAY_NAME: &str = "Stargate Worlds";
/// Set to `1` in the launcher's own environment. Never read from the webview.
const OPT_IN: &str = "CIMMERIA_WINE_APP_IDENTITY";
const DIRECTORY: &str = "wine-app-identity";
const BUNDLE: &str = "Stargate Worlds.app";
const UNIX_LIBRARIES: &str = "x86_64-unix";
const LOADER: &str = "wine";
const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";

/// The loader Play executes. Called only after the runtime is verified and locked.
/// Anything short of a fully staged bundle keeps the stock loader, so Play never
/// depends on this.
pub(crate) fn loader(runtime: &Path, state_root: &Path) -> PathBuf {
    let requested = requested(std::env::var_os(OPT_IN).as_deref());
    select(requested, runtime, state_root, register)
}

fn requested(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

fn select(
    requested: bool,
    runtime: &Path,
    state_root: &Path,
    register: impl FnOnce(&Path) -> io::Result<()>,
) -> PathBuf {
    let stock = runtime.join("bin/wine");
    if !requested {
        return stock;
    }
    match stage(state_root, runtime) {
        Ok(bundle) => {
            // Launch Services lookups by name or identifier need the registration; the
            // running process has its identity without it.
            if let Err(error) = register(&bundle) {
                eprintln!("{OPT_IN}: bundle staged but not registered: {error}");
            }
            bundle.join("Contents/MacOS").join(LOADER)
        }
        Err(error) => {
            eprintln!("{OPT_IN}: using the stock Wine loader: {error}");
            stock
        }
    }
}

/// Rebuilds the bundle from the runtime and returns its path.
fn stage(state_root: &Path, runtime: &Path) -> io::Result<PathBuf> {
    let libraries = runtime.join("lib/wine");
    let unix = libraries.join(UNIX_LIBRARIES);
    plain_directory(&unix)?;
    let data = runtime.join("share");
    plain_directory(&data)?;
    let root = state_root.join(DIRECTORY);
    match fs::create_dir(&root) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
        _ => (),
    }
    plain_directory(&root)?;
    let staging = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(&root)?;
    let bundle = staging.path().join(BUNDLE);
    let contents = bundle.join("Contents");
    let macos = contents.join("MacOS");
    fs::create_dir_all(&macos)?;
    plist::Value::Dictionary(info())
        .to_file_xml(contents.join("Info.plist"))
        .map_err(io::Error::other)?;
    for entry in fs::read_dir(&unix)? {
        let entry = entry?;
        let staged = macos.join(entry.file_name());
        if entry.file_type()?.is_symlink() {
            // The runtime's own relative links would dangle from here; keep their target.
            let target = entry.path().canonicalize()?;
            if !target.starts_with(runtime) || !target.is_file() {
                return Err(refused("runtime link leaves the runtime"));
            }
            symlink(target, staged)?;
        } else {
            copy_verified(&entry.path(), &staged)?;
        }
    }
    // Wine resolves the loader and ntdll.so through realpath: a link loses the bundle.
    for required in [LOADER, "ntdll.so"] {
        if !fs::symlink_metadata(macos.join(required)).is_ok_and(|metadata| metadata.is_file()) {
            return Err(refused("runtime loader directory is incomplete"));
        }
    }
    // Wine finds a builtin's unix library at `<loader dir>/x86_64-unix/`, its PE
    // directories beside that, and its data at `<loader dir>/../../share/wine`.
    symlink(".", macos.join(UNIX_LIBRARIES))?;
    for entry in fs::read_dir(&libraries)? {
        let entry = entry?;
        if entry.file_name() != UNIX_LIBRARIES {
            plain_directory(&entry.path())?;
            symlink(entry.path(), macos.join(entry.file_name()))?;
        }
    }
    symlink(&data, bundle.join("share"))?;
    let destination = root.join(BUNDLE);
    match fs::symlink_metadata(&destination) {
        // Removes links inside the old bundle without following them.
        Ok(existing) if existing.is_dir() => fs::remove_dir_all(&destination)?,
        Ok(_) => return Err(refused("existing bundle path is not a directory")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    fs::rename(&bundle, &destination)?;
    Ok(destination)
}

fn info() -> plist::Dictionary {
    let text = |value: &str| plist::Value::String(value.into());
    plist::Dictionary::from_iter([
        ("CFBundleIdentifier", text(BUNDLE_IDENTIFIER)),
        ("CFBundleName", text(DISPLAY_NAME)),
        ("CFBundleDisplayName", text(DISPLAY_NAME)),
        ("CFBundleExecutable", text(LOADER)),
        ("CFBundlePackageType", text("APPL")),
        ("CFBundleInfoDictionaryVersion", text("6.0")),
        // Every Wine process shares this bundle. Only one that shows a window is
        // promoted by Wine's Mac driver; the rest stay out of the Dock as before.
        ("LSUIElement", plist::Value::Boolean(true)),
    ])
}

/// The runtime tree digest pins the source; this pins the copy that executes.
fn copy_verified(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file() || metadata.len() > 128 * 1024 * 1024 {
        return Err(refused("runtime loader directory has an unexpected entry"));
    }
    let bytes = fs::read(source)?;
    io::Write::write_all(
        &mut OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(metadata.permissions().mode() & 0o755)
            .open(destination)?,
        &bytes,
    )?;
    verify_file(destination, &Sha256::digest(&bytes).into())
        .map_err(|_| refused("staged copy differs from the verified runtime"))
}

fn plain_directory(path: &Path) -> io::Result<()> {
    if !fs::symlink_metadata(path)?.is_dir() || path.canonicalize()? != path {
        return Err(refused("expected an ordinary directory"));
    }
    Ok(())
}

fn refused(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

fn register(bundle: &Path) -> io::Result<()> {
    let status = std::process::Command::new(LSREGISTER)
        .arg("-f")
        .arg(bundle)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        return Err(io::Error::other("lsregister failed"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "app_identity_tests.rs"]
mod tests;
