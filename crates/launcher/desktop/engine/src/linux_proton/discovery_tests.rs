//! Discovery tests on a fake Steam tree in a `TempDir`, shaped after the
//! Deck inventory (F-04, F-05): a client, a second library on another mount,
//! a compatibilitytools.d and a system tools folder.

use super::{choose, discover, steam_roots, ProtonInstall, ProtonKind, SteamRoots};
use crate::linux_proton::vdf;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const RUNTIME_4: &str = "4183110";

/// A Proton folder with the three files discovery requires.
fn proton(dir: &Path, version: &str, appid: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("proton"), "#!/bin/sh\n").unwrap();
    let manifest = format!("\"manifest\"\n{{\n\t\"require_tool_appid\"\t\t\"{appid}\"\n}}\n");
    fs::write(dir.join("toolmanifest.vdf"), manifest).unwrap();
    fs::write(dir.join("version"), format!("{version}\n")).unwrap();
}

/// The client, a second library and the system tools folder under one TempDir.
/// The GE-Proton11 runtime is installed; no other runtime is.
fn fixture() -> (TempDir, SteamRoots, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let client = tmp.path().join("home/.local/share/Steam");
    let sd = tmp.path().join("sdcard/SteamLibrary");
    let system = tmp.path().join("system/compatibilitytools.d");
    fs::create_dir_all(client.join("steamapps")).unwrap();
    fs::create_dir_all(sd.join("steamapps/common")).unwrap();
    fs::create_dir_all(&system).unwrap();

    let folders = format!(
        "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
        client.display(),
        sd.display()
    );
    fs::write(client.join("steamapps/libraryfolders.vdf"), folders).unwrap();

    let common = client.join("steamapps/common");
    proton(
        &common.join("Proton - Experimental"),
        "1792000001 Proton - Experimental",
        "1493710",
    );
    proton(
        &common.join("Proton 8.0"),
        "1700000000 Proton 8.0",
        "1628350",
    );
    proton(
        &common.join("Proton 9.0 (Beta)"),
        "1750000000 Proton 9.0 (Beta)",
        "2805730",
    );
    proton(
        &sd.join("steamapps/common/Proton 10.0"),
        "1790000000 Proton 10.0",
        RUNTIME_4,
    );

    let tools = client.join("compatibilitytools.d");
    proton(
        &tools.join("GE-Proton11-11-x86_64"),
        "1792432585 GE-Proton11-11",
        RUNTIME_4,
    );
    proton(
        &tools.join("GE-Proton7-38"),
        "1680000000 GE-Proton7-38",
        "1628350",
    );
    // No `proton` script: skipped.
    fs::create_dir_all(tools.join("GE-Proton-broken")).unwrap();
    fs::write(
        tools.join("GE-Proton-broken/toolmanifest.vdf"),
        "\"manifest\"\n{\n}\n",
    )
    .unwrap();
    fs::write(
        tools.join("GE-Proton-broken/version"),
        "1 GE-Proton-broken\n",
    )
    .unwrap();

    proton(
        &system.join("Proton 9.0 (Beta)"),
        "1751000000 Proton 9.0 (Beta)",
        "2805730",
    );

    let runtime_dir = client.join("steamapps/common/SteamLinuxRuntime_4");
    fs::create_dir_all(&runtime_dir).unwrap();
    fs::write(runtime_dir.join("_v2-entry-point"), "#!/bin/sh\n").unwrap();
    let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"4183110\"\n\t\"installdir\"\t\t\"SteamLinuxRuntime_4\"\n}\n";
    fs::write(client.join("steamapps/appmanifest_4183110.acf"), acf).unwrap();

    let roots = steam_roots(&tmp.path().join("home")).unwrap();
    (tmp, roots, system)
}

fn ids(found: &[ProtonInstall]) -> Vec<&str> {
    found.iter().map(|install| install.id.as_str()).collect()
}

#[cfg(unix)]
#[test]
fn roots_follow_the_steam_root_symlink_and_the_second_library() {
    let tmp = TempDir::new().unwrap();
    let real = tmp.path().join("real/Steam");
    let sd = tmp.path().join("sd/SteamLibrary");
    fs::create_dir_all(real.join("steamapps")).unwrap();
    fs::create_dir_all(&sd).unwrap();
    let folders = format!(
        "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
        real.display(),
        sd.display()
    );
    fs::write(real.join("steamapps/libraryfolders.vdf"), folders).unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(home.join(".steam")).unwrap();
    std::os::unix::fs::symlink(&real, home.join(".steam/root")).unwrap();

    let roots = steam_roots(&home).unwrap();
    assert_eq!(roots.client, fs::canonicalize(&real).unwrap());
    assert_eq!(
        roots.libraries,
        vec![
            fs::canonicalize(&real).unwrap(),
            fs::canonicalize(&sd).unwrap()
        ]
    );
}

#[test]
fn no_steam_client_means_no_roots() {
    let tmp = TempDir::new().unwrap();
    assert!(steam_roots(tmp.path()).is_none());
}

#[test]
fn a_directory_without_a_proton_script_is_skipped() {
    let (_tmp, roots, system) = fixture();
    let found = discover(&roots, &system);
    assert!(!found.iter().any(|install| install.id.contains("broken")));
    assert_eq!(found.len(), 7);
}

#[test]
fn kinds_names_and_roots_follow_the_folder_layout() {
    let (_tmp, roots, system) = fixture();
    let found = discover(&roots, &system);
    let kind_of = |id: &str| found.iter().find(|i| i.id == id).unwrap().kind;
    assert_eq!(kind_of("ge:GE-Proton11-11-x86_64"), ProtonKind::Ge);
    assert_eq!(kind_of("ge:GE-Proton7-38"), ProtonKind::Ge);
    assert_eq!(
        kind_of("experimental:Proton - Experimental"),
        ProtonKind::Experimental
    );
    assert_eq!(kind_of("valve:Proton 9.0 (Beta)"), ProtonKind::Valve);
    assert_eq!(kind_of("valve:Proton 10.0"), ProtonKind::Valve);
    assert_eq!(kind_of("other:Proton 9.0 (Beta)"), ProtonKind::Other);

    let ge = found
        .iter()
        .find(|i| i.id == "ge:GE-Proton11-11-x86_64")
        .unwrap();
    assert_eq!(ge.name, "GE-Proton11-11");
    assert_eq!(ge.version, "1792432585 GE-Proton11-11");
    assert_eq!(
        ge.root,
        fs::canonicalize(
            roots
                .client
                .join("compatibilitytools.d/GE-Proton11-11-x86_64")
        )
        .unwrap()
    );
}

#[test]
fn ranking_puts_ge_then_experimental_then_valve_then_other() {
    let (_tmp, roots, system) = fixture();
    let found = discover(&roots, &system);
    assert_eq!(
        ids(&found),
        vec![
            "ge:GE-Proton11-11-x86_64",
            "ge:GE-Proton7-38",
            "experimental:Proton - Experimental",
            "valve:Proton 10.0",
            "valve:Proton 9.0 (Beta)",
            "valve:Proton 8.0",
            "other:Proton 9.0 (Beta)",
        ]
    );
}

#[test]
fn runtime_resolves_when_installed_and_is_none_otherwise() {
    let (_tmp, roots, system) = fixture();
    let found = discover(&roots, &system);
    let runtime = |id: &str| found.iter().find(|i| i.id == id).unwrap().runtime.clone();
    let entry = roots
        .client
        .join("steamapps/common/SteamLinuxRuntime_4/_v2-entry-point");
    assert_eq!(runtime("ge:GE-Proton11-11-x86_64"), Some(entry));
    assert_eq!(runtime("valve:Proton 9.0 (Beta)"), None);
    assert_eq!(runtime("ge:GE-Proton7-38"), None);
    fs::remove_file(
        roots
            .client
            .join("steamapps/common/SteamLinuxRuntime_4/_v2-entry-point"),
    )
    .unwrap();
    let found = discover(&roots, &system);
    assert_eq!(
        found
            .iter()
            .find(|i| i.id == "ge:GE-Proton11-11-x86_64")
            .unwrap()
            .runtime,
        None
    );
}

#[test]
fn choose_honours_a_present_preference_and_falls_back() {
    let (_tmp, roots, system) = fixture();
    let found = discover(&roots, &system);
    let picked = choose(&found, Some("valve:Proton 8.0")).unwrap();
    assert_eq!(picked.id, "valve:Proton 8.0");
    let gone = choose(&found, Some("valve:Proton 3.7")).unwrap();
    assert_eq!(gone.id, "ge:GE-Proton11-11-x86_64");
    assert_eq!(choose(&found, None).unwrap().id, "ge:GE-Proton11-11-x86_64");
    assert!(choose(&[], None).is_none());
}

#[test]
fn vdf_values_read_every_depth_and_skip_comments() {
    let text = r#""libraryfolders"
{
	// "path"		"/commented/out"
	"0"
	{
		"path"		"/home/deck/.local/share/Steam"
		"apps"
		{
			"4183110"		"123"
		}
	}
	"1"
	{
		"path"		"C:\\games"
	}
}
"#;
    assert_eq!(
        vdf::values(text, "path"),
        vec!["/home/deck/.local/share/Steam", "C:\\games"]
    );
    assert_eq!(vdf::values(text, "4183110"), vec!["123"]);
    assert!(vdf::values(text, "missing").is_empty());
}

#[test]
fn the_first_folder_wins_when_two_share_a_directory_name() {
    let (_tmp, roots, system) = fixture();
    proton(&system.join("GE-Proton7-38"), "1 GE-Proton7-38", "1628350");
    let found = discover(&roots, &system);
    assert_eq!(found.len(), 7);
    let kept = found.iter().find(|i| i.id == "ge:GE-Proton7-38").unwrap();
    assert_eq!(kept.version, "1680000000 GE-Proton7-38");
}

#[cfg(unix)]
#[test]
fn a_folder_reached_through_a_symlink_is_listed_once() {
    let (_tmp, roots, system) = fixture();
    let real = roots.client.join("compatibilitytools.d/GE-Proton7-38");
    std::os::unix::fs::symlink(&real, system.join("GE-Proton7-38-link")).unwrap();
    assert_eq!(discover(&roots, &system).len(), 7);
}

#[test]
fn other_builds_with_equal_numbers_rank_by_build_stamp_then_id() {
    let (_tmp, roots, system) = fixture();
    // No digits in either name, so only the stamp separates them; the older
    // stamp has the smaller id, so an id-only order would put it first.
    proton(&system.join("Alpha"), "10 Alpha", "1628350");
    proton(&system.join("Zeta"), "20 Zeta", "1628350");
    let found = discover(&roots, &system);
    let others: Vec<&str> = ids(&found)
        .into_iter()
        .filter(|id| id.starts_with("other:"))
        .collect();
    assert_eq!(
        others,
        vec!["other:Proton 9.0 (Beta)", "other:Zeta", "other:Alpha"]
    );
}

#[test]
fn a_relative_library_path_in_the_vdf_is_ignored() {
    let tmp = TempDir::new().unwrap();
    let client = tmp.path().join("home/.local/share/Steam");
    fs::create_dir_all(client.join("steamapps")).unwrap();
    // "." canonicalizes to the test's working directory, which exists.
    let folders = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\".\"\n\t}\n}\n";
    fs::write(client.join("steamapps/libraryfolders.vdf"), folders).unwrap();
    let roots = steam_roots(&tmp.path().join("home")).unwrap();
    assert_eq!(roots.libraries, vec![roots.client.clone()]);
}
