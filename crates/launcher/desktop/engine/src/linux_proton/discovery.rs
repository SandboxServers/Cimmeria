//! Finds the Proton builds a player already has and ranks them (C-1, D-PP4).
//!
//! Nothing is downloaded. The search covers every Steam library's
//! `steamapps/common`, the client's `compatibilitytools.d` and the system
//! tools folder. `runtime` is logged only (D-PP3): the launch never wraps
//! Proton in its Steam Linux Runtime container.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::vdf;

/// Where a Proton build came from, in default rank order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtonKind {
    Ge,
    Experimental,
    Valve,
    Other,
}

impl ProtonKind {
    /// The snake_case name used in ids.
    fn as_str(self) -> &'static str {
        match self {
            ProtonKind::Ge => "ge",
            ProtonKind::Experimental => "experimental",
            ProtonKind::Valve => "valve",
            ProtonKind::Other => "other",
        }
    }
}

/// One usable Proton installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtonInstall {
    /// `"<kind>:<directory name>"`, e.g. `"ge:GE-Proton11-11-x86_64"`.
    pub id: String,
    /// The second token of `version`, else the directory name.
    pub name: String,
    pub kind: ProtonKind,
    /// Canonical directory holding the `proton` script.
    pub root: PathBuf,
    /// The `version` file, trimmed, e.g. `"1792432585 GE-Proton11-11"`.
    pub version: String,
    /// `<runtime dir>/_v2-entry-point` when the toolmanifest's runtime is
    /// installed. Logged only (D-PP3); the invocation never uses it.
    pub runtime: Option<PathBuf>,
}

/// The Steam client and the library folders it knows about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamRoots {
    pub client: PathBuf,
    pub libraries: Vec<PathBuf>,
}

/// Steam's client folder under `home` and the libraries it lists.
/// None when no client folder exists.
pub fn steam_roots(home: &Path) -> Option<SteamRoots> {
    let client = [".steam/root", ".local/share/Steam", ".steam/steam"]
        .iter()
        .find_map(|rel| existing_dir(&home.join(rel)))?;

    let mut libraries = vec![client.clone()];
    let folders = client.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = std::fs::read_to_string(folders) {
        for path in vdf::values(&text, "path") {
            if let Some(library) = existing_dir(Path::new(&*path)) {
                if !libraries.contains(&library) {
                    libraries.push(library);
                }
            }
        }
    }
    Some(SteamRoots { client, libraries })
}

/// Every Proton install under the roots and `system_tools`, best first.
pub fn discover(roots: &SteamRoots, system_tools: &Path) -> Vec<ProtonInstall> {
    let mut found = Vec::new();
    for library in &roots.libraries {
        let common = library.join("steamapps").join("common");
        for dir in subdirectories(&common) {
            if name_of(&dir).is_some_and(|name| name.starts_with("Proton")) {
                found.extend(install_at(&dir, true, &roots.libraries));
            }
        }
    }
    let client_tools = roots.client.join("compatibilitytools.d");
    for dir in subdirectories(&client_tools)
        .into_iter()
        .chain(subdirectories(system_tools))
    {
        found.extend(install_at(&dir, false, &roots.libraries));
    }

    // The first copy wins: libraries are searched in order.
    let mut seen_ids = BTreeSet::new();
    let mut seen_roots = BTreeSet::new();
    found.retain(|install| {
        seen_ids.insert(install.id.clone()) && seen_roots.insert(install.root.clone())
    });
    found.sort_by(rank);
    found
}

/// The install `preferred` names when it is still present, else the best one.
pub fn choose<'a>(
    found: &'a [ProtonInstall],
    preferred: Option<&str>,
) -> Option<&'a ProtonInstall> {
    preferred
        .and_then(|id| found.iter().find(|install| install.id == id))
        .or_else(|| found.first())
}

/// A directory that exists, canonicalized.
fn existing_dir(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok().filter(|p| p.is_dir())
}

/// The directories directly inside `parent`, sorted by path. Empty when unreadable.
fn subdirectories(parent: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn name_of(dir: &Path) -> Option<&str> {
    dir.file_name().and_then(|name| name.to_str())
}

/// The install in `dir`, when it holds the three files a Proton needs.
/// `in_common` marks a folder under `steamapps/common`, where `Proton <digit>`
/// is a Valve build.
fn install_at(dir: &Path, in_common: bool, libraries: &[PathBuf]) -> Option<ProtonInstall> {
    let complete = dir.join("proton").is_file()
        && dir.join("toolmanifest.vdf").is_file()
        && dir.join("version").is_file();
    if !complete {
        return None;
    }
    let dir_name = name_of(dir)?.to_owned();
    let root = std::fs::canonicalize(dir).ok()?;
    let version = std::fs::read_to_string(dir.join("version"))
        .ok()?
        .trim()
        .to_owned();
    let kind = kind_of(&dir_name, in_common);
    let name = version
        .split_whitespace()
        .nth(1)
        .map_or_else(|| dir_name.clone(), str::to_owned);
    let manifest = std::fs::read_to_string(dir.join("toolmanifest.vdf")).unwrap_or_default();
    Some(ProtonInstall {
        id: format!("{}:{dir_name}", kind.as_str()),
        name,
        kind,
        root,
        version,
        runtime: runtime_of(&manifest, libraries),
    })
}

fn kind_of(dir_name: &str, in_common: bool) -> ProtonKind {
    let numbered = dir_name
        .strip_prefix("Proton ")
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()));
    if dir_name.starts_with("GE-Proton") {
        ProtonKind::Ge
    } else if dir_name.starts_with("Proton - Experimental") {
        ProtonKind::Experimental
    } else if in_common && numbered {
        ProtonKind::Valve
    } else {
        ProtonKind::Other
    }
}

/// The `_v2-entry-point` of the runtime the toolmanifest requires, when a
/// library holds it.
fn runtime_of(manifest: &str, libraries: &[PathBuf]) -> Option<PathBuf> {
    let appid = vdf::values(manifest, "require_tool_appid")
        .first()?
        .trim()
        .to_owned();
    for library in libraries {
        let steamapps = library.join("steamapps");
        let Ok(text) = std::fs::read_to_string(steamapps.join(format!("appmanifest_{appid}.acf")))
        else {
            continue;
        };
        let Some(installdir) = vdf::values(&text, "installdir")
            .first()
            .map(|d| d.to_string())
        else {
            continue;
        };
        let entry = steamapps
            .join("common")
            .join(installdir)
            .join("_v2-entry-point");
        if entry.is_file() {
            return Some(entry);
        }
    }
    None
}

/// Kind, then newer first, then (Experimental and Other) the build stamp,
/// then the id.
fn rank(a: &ProtonInstall, b: &ProtonInstall) -> Ordering {
    a.kind
        .cmp(&b.kind)
        .then_with(|| version_numbers(dir_name_of(b)).cmp(&version_numbers(dir_name_of(a))))
        .then_with(|| match a.kind {
            ProtonKind::Experimental | ProtonKind::Other => build_stamp(b).cmp(&build_stamp(a)),
            ProtonKind::Ge | ProtonKind::Valve => Ordering::Equal,
        })
        .then_with(|| a.id.cmp(&b.id))
}

/// The directory name, which is the part of the id after the kind.
fn dir_name_of(install: &ProtonInstall) -> &str {
    install
        .id
        .split_once(':')
        .map_or(install.id.as_str(), |(_, dir)| dir)
}

/// The first two numbers in a directory name: `GE-Proton11-11` gives (11, 11),
/// `Proton 9.0 (Beta)` gives (9, 0). Missing numbers count as 0.
fn version_numbers(name: &str) -> (u64, u64) {
    let mut numbers = name
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|digits| digits.parse::<u64>().ok());
    (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0))
}

/// The build timestamp, the first token of `version`.
fn build_stamp(install: &ProtonInstall) -> u64 {
    install
        .version
        .split_whitespace()
        .next()
        .and_then(|token| token.parse::<u64>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
