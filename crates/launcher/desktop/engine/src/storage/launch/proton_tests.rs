//! Proton preparation on a fake Steam tree and a fake `proton` script (PP-07).
//! Nothing here reads the real `~/.steam`: `Search` points at the tempdir.
use super::*;
use crate::storage::install_intent::ExtractionBackend;
use crate::storage::launch::tests::artifact;
use crate::storage::runtime_setup::tests::fixture_with_extraction;
use std::ffi::{OsStr, OsString};
use std::os::unix::fs::PermissionsExt;

const VERSION: &str = "1792432585 GE-Proton11-11";
const SETUP_OK: &str = "#!/bin/sh\nmkdir -p \"$STEAM_COMPAT_DATA_PATH/pfx\"\n: > \"$STEAM_COMPAT_DATA_PATH/pfx/system.reg\"\n";

/// A tempdir with a Steam client, one Proton build with a `proton` script, a
/// game folder whose binaries folder is lower case, and a helper.
struct Fixture {
    _root: tempfile::TempDir,
    base: PathBuf,
    search: Search,
    state: PathBuf,
    proton_root: PathBuf,
    game: PathBuf,
    plan: Plan,
}

fn fixture(script: &str) -> Fixture {
    let (root, _state, intent) =
        fixture_with_extraction(ExtractionBackend::Native, &"a".repeat(64), 1);
    let base = root.path().canonicalize().unwrap();
    let home = base.join("home");
    let proton_root = home.join(".local/share/Steam/compatibilitytools.d/GE-Proton11-11");
    std::fs::create_dir_all(&proton_root).unwrap();
    let proton = proton_root.join("proton");
    std::fs::write(&proton, script).unwrap();
    std::fs::set_permissions(&proton, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(proton_root.join("version"), format!("{VERSION}\n")).unwrap();
    std::fs::write(proton_root.join("toolmanifest.vdf"), "\"manifest\" { }\n").unwrap();
    let search = Search {
        home,
        system_tools: base.join("no-system-tools"),
    };
    let id = discovery::discover(
        &discovery::steam_roots(&search.home).unwrap(),
        &search.system_tools,
    )
    .remove(0)
    .id;
    let game = intent.content_root();
    std::fs::remove_dir_all(game.join("Working/Binaries")).unwrap();
    std::fs::create_dir_all(game.join("Working/binaries")).unwrap();
    let game = game.canonicalize().unwrap();
    let plan = Plan {
        id: Uuid::new_v4(),
        installation: intent,
        runtime: None,
        resources: Resources {
            proton: Some(ProtonSelection {
                id,
                root: proton_root.clone(),
                version: VERSION.into(),
            }),
            helper: artifact(&base, "helper.exe"),
            client_patches: None,
            client_telemetry: None,
            graphics: None,
        },
    };
    Fixture {
        _root: root,
        state: base.join("state"),
        base,
        search,
        proton_root,
        game,
        plan,
    }
}

/// Runs `prepare_in` inside a runtime, as the worker's `spawn_blocking` does.
fn run(f: &Fixture, plan: &Plan) -> Result<(HelperCommand, Request, Prefix), PrepareError> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    prepare_in(
        plan,
        &f.state,
        None,
        &crate::client_setup::login_servers::default_servers(),
        DisplayOptions::default(),
        &[],
        &f.search,
    )
}

fn run_ok(f: &Fixture, plan: &Plan) -> (HelperCommand, Request, Prefix) {
    run(f, plan).unwrap_or_else(|err| panic!("preparation refused: {:?}", err.0))
}

#[test]
fn argv_is_bare_runinprefix_with_the_worker_guest_path() {
    let f = fixture(SETUP_OK);
    let (spec, _, _) = run_ok(&f, &f.plan);
    assert_eq!(spec.executable, f.proton_root.join("proton"));
    let helper = paths::guest(&f.base.join("helper.exe")).unwrap();
    assert_eq!(
        spec.arguments,
        vec![OsString::from("runinprefix"), OsString::from(helper)]
    );
}

#[test]
fn guest_paths_follow_the_lower_case_binaries_folder() {
    let f = fixture(SETUP_OK);
    let (_, request, _) = run_ok(&f, &f.plan);
    let binaries = f.game.join("Working/binaries");
    assert_eq!(
        request.directory,
        PathBuf::from(paths::guest(&binaries).unwrap())
    );
    assert_eq!(
        request.exe,
        PathBuf::from(paths::guest(&binaries.join("SGW.exe")).unwrap())
    );
    assert!(request
        .directory
        .to_string_lossy()
        .ends_with("Working\\binaries"));
}

#[test]
fn prepare_writes_the_login_servers_into_the_lower_case_install() {
    let f = fixture(SETUP_OK);
    run_ok(&f, &f.plan);
    let lua = std::fs::read_to_string(crate::client_setup::login_servers::path(&f.game)).unwrap();
    let server = &crate::client_setup::login_servers::default_servers()[0];
    assert!(lua.contains(&server.url));
    assert!(!f.game.join("Working/Binaries").exists());
}

#[test]
fn client_patches_are_the_dlls_in_order() {
    let f = fixture(SETUP_OK);
    let mut plan = f.plan.clone();
    plan.resources.client_patches = Some(artifact(&f.base, "patch.dll"));
    let (_, request, _) = run_ok(&f, &plan);
    assert_eq!(
        request.dlls,
        vec![PathBuf::from(
            paths::guest(&f.base.join("patch.dll")).unwrap()
        )]
    );
}

#[test]
fn no_proton_log_is_passed_to_the_helper() {
    let f = fixture(SETUP_OK);
    let (spec, _, _) = run_ok(&f, &f.plan);
    assert!(!spec.environment.contains_key(OsStr::new("PROTON_LOG")));
    assert!(spec
        .stderr_log
        .is_some_and(|log| log.ends_with("proton.log")));
}

#[test]
fn a_fresh_setup_writes_the_stamp() {
    let f = fixture(SETUP_OK);
    let (_, _, prefix) = run_ok(&f, &f.plan);
    let stamp = prefix.stamp().expect("the setup wrote a stamp");
    assert_eq!(stamp.proton_version, VERSION);
}

#[test]
fn a_changed_version_file_is_proton_missing() {
    let f = fixture(SETUP_OK);
    std::fs::write(f.proton_root.join("version"), "1792432999 GE-Proton11-12\n").unwrap();
    assert_eq!(
        run(&f, &f.plan).map(|_| ()),
        Err(PrepareError(FailureCode::ProtonMissing))
    );
}

#[test]
fn a_failed_setup_is_proton_setup_failed() {
    let f = fixture("#!/bin/sh\nexit 1\n");
    assert_eq!(
        run(&f, &f.plan).map(|_| ()),
        Err(PrepareError(FailureCode::ProtonSetupFailed))
    );
}
