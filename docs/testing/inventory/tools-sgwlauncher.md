# Tests — `tools/SGWLauncher`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 22  
> **CI-gated**: no  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

The **Tauri** launcher at `tools/SGWLauncher/src-tauri/` — workspace-excluded,
non-CI. Patch client, login redirect, install pipeline.

> **Not the same crate as [launcher.md](launcher.md)**, which catalogues
> `crates/launcher` — the **egui** launcher, package `sgw-launcher`, 176 tests.
> The two are independent. Until 2026-07-25 `launcher.md` was a verbatim
> duplicate of this file; that is fixed.

## All tests (22)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [test_default_config](../../../tools/SGWLauncher/src-tauri/src/config.rs#L53) | unit | Src-Tauri / Config | 2026-03-06 | Asserts equality on `config.server_address` |  |
| [test_save_and_load](../../../tools/SGWLauncher/src-tauri/src/config.rs#L62) | unit | Src-Tauri / Config | 2026-03-06 | Asserts equality on `loaded.install_path` |  |
| [test_load_missing_file](../../../tools/SGWLauncher/src-tauri/src/config.rs#L80) | unit | Src-Tauri / Config | 2026-03-06 | Asserts on `LauncherConfig::load(&path).is_err()` |  |
| [test_build_extract_args](../../../tools/SGWLauncher/src-tauri/src/extract.rs#L53) | unit | Src-Tauri / Extract | 2026-03-06 | Asserts equality on `args` |  |
| [test_count_cab_files](../../../tools/SGWLauncher/src-tauri/src/extract.rs#L59) | unit | Src-Tauri / Extract | 2026-03-06 | Asserts equality on `cabs.len()` |  |
| [test_count_cab_files_empty](../../../tools/SGWLauncher/src-tauri/src/extract.rs#L71) | unit | Src-Tauri / Extract | 2026-03-06 | Asserts equality on `cabs.len()` |  |
| [test_verify_installation_missing](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L50) | unit | Src-Tauri / Launch | 2026-03-06 | Asserts on `matches!(result, Err(LaunchError::NotFound(_)))` |  |
| [test_verify_installation_exists](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L56) | unit | Src-Tauri / Launch | 2026-03-06 | Asserts on `result.is_ok()` |  |
| [test_check_installation_empty_path](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L64) | unit | Src-Tauri / Launch | 2026-03-06 | Asserts on `matches!(state, InstallState::NotInstalled)` |  |
| [test_check_installation_with_exe](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L70) | unit | Src-Tauri / Launch | 2026-03-06 | Asserts on `matches!(state, InstallState::Installed)` |  |
| [test_patch_hostname_success](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L60) | unit | Src-Tauri / Patch | 2026-03-06 | Asserts equality on `offset` |  |
| [test_patch_hostname_max_length](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L69) | unit | Src-Tauri / Patch | 2026-03-06 | Asserts equality on `host.len()` |  |
| [test_patch_hostname_too_long](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L79) | unit | Src-Tauri / Patch | 2026-03-06 | Asserts on `matches!(result, Err(PatchError::AddressTooLong { .. }))` |  |
| [test_patch_hostname_not_found](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L87) | unit | Src-Tauri / Patch | 2026-03-06 | Asserts on `matches!(result, Err(PatchError::PatternNotFound))` |  |
| [test_needs_patching](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L94) | unit | Src-Tauri / Patch | 2026-03-06 | Asserts on `needs_patching(&data)` |  |
| [test_patch_idempotent_check](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L103) | unit | Src-Tauri / Patch | 2026-03-06 | Asserts on `!needs_patching(&data)` |  |
| [test_patch_exe_on_disk](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L114) | unit | Src-Tauri / Patch | 2026-03-06 | Asserts equality on `offset` |  |
| [test_hash_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L95) | unit | Src-Tauri / Updater | 2026-03-06 | Asserts equality on `hash` |  |
| [test_check_manifest_missing_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L107) | unit | Src-Tauri / Updater | 2026-03-06 | Asserts equality on `updates.len()` |  |
| [test_check_manifest_matching_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L124) | unit | Src-Tauri / Updater | 2026-03-06 | Test check manifest matching file |  |
| [test_check_manifest_changed_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L143) | unit | Src-Tauri / Updater | 2026-03-06 | Test check manifest changed file |  |
| [test_verify_hash_success](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L161) | unit | Src-Tauri / Updater | 2026-03-06 | Asserts on `result.is_ok()` |  |
