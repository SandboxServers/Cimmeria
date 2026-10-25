use super::super::discovery::{ProtonInstall, ProtonKind};
use super::*;

fn install(runtime: Option<PathBuf>) -> ProtonInstall {
    ProtonInstall {
        id: "ge:GE-Proton11-11-x86_64".into(),
        name: "GE-Proton11-11".into(),
        kind: ProtonKind::Ge,
        root: PathBuf::from("/opt/proton/GE-Proton11-11-x86_64"),
        version: "1792432585 GE-Proton11-11".into(),
        runtime,
    }
}

#[test]
fn runinprefix_is_the_bare_proton_script_with_target_and_extra_args() {
    let inv = runinprefix(
        &install(None),
        r"Z:\launch\cimmeria-launch-worker.exe",
        &[OsString::from("--flag")],
    );
    assert_eq!(
        inv,
        Invocation {
            program: PathBuf::from("/opt/proton/GE-Proton11-11-x86_64/proton"),
            args: vec![
                OsString::from("runinprefix"),
                OsString::from(r"Z:\launch\cimmeria-launch-worker.exe"),
                OsString::from("--flag"),
            ],
        }
    );
}

#[test]
fn runinprefix_without_extra_args_has_only_the_target() {
    let inv = runinprefix(&install(None), "game.exe", &[]);
    assert_eq!(
        inv.args,
        vec![OsString::from("runinprefix"), OsString::from("game.exe")]
    );
}

#[test]
fn setup_runs_wineboot_through_proton_run() {
    let inv = setup(&install(None));
    assert_eq!(
        inv,
        Invocation {
            program: PathBuf::from("/opt/proton/GE-Proton11-11-x86_64/proton"),
            args: vec![OsString::from("run"), OsString::from("wineboot.exe")],
        }
    );
}

#[test]
fn a_runtime_container_is_never_used_for_runinprefix() {
    let runtime = PathBuf::from("/opt/steam/steamapps/common/SteamLinuxRuntime_4/_v2-entry-point");
    let inv = runinprefix(&install(Some(runtime.clone())), "game.exe", &[]);
    assert_eq!(
        inv.program,
        PathBuf::from("/opt/proton/GE-Proton11-11-x86_64/proton")
    );
    assert!(inv.args.iter().all(|arg| arg != "_v2-entry-point"));
    assert!(!inv.args.contains(&runtime.into_os_string()));
}

#[test]
fn a_runtime_container_is_never_used_for_setup() {
    let runtime = PathBuf::from("/opt/steam/steamapps/common/SteamLinuxRuntime_4/_v2-entry-point");
    let inv = setup(&install(Some(runtime)));
    assert_eq!(
        inv.program,
        PathBuf::from("/opt/proton/GE-Proton11-11-x86_64/proton")
    );
    assert_eq!(
        inv.args,
        vec![OsString::from("run"), OsString::from("wineboot.exe")]
    );
}
