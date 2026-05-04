# Tests — `launcher`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 22  
> **CI-gated**: no  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

SGW game launcher (Tauri app, non-CI). Patch client, login redirect, install pipeline.

## All tests (22)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [test_default_config](../../../crates/launcher/src/config.rs#L53) | unit | Config | 2026-03-07 | Asserts equality on `config.server_address` |  |
| [test_save_and_load](../../../crates/launcher/src/config.rs#L62) | unit | Config | 2026-03-07 | Asserts equality on `loaded.install_path` |  |
| [test_load_missing_file](../../../crates/launcher/src/config.rs#L80) | unit | Config | 2026-03-07 | Asserts on `LauncherConfig::load(&path).is_err()` |  |
| [test_build_extract_args](../../../crates/launcher/src/extract.rs#L53) | unit | Extract | 2026-03-07 | Asserts equality on `args` |  |
| [test_count_cab_files](../../../crates/launcher/src/extract.rs#L62) | unit | Extract | 2026-03-07 | Asserts equality on `cabs.len()` |  |
| [test_count_cab_files_empty](../../../crates/launcher/src/extract.rs#L79) | unit | Extract | 2026-03-07 | Asserts equality on `cabs.len()` |  |
| [test_verify_installation_missing](../../../crates/launcher/src/launch.rs#L48) | unit | Launch | 2026-03-07 | Asserts on `matches!(result, Err(LaunchError::NotFound(_)))` |  |
| [test_verify_installation_exists](../../../crates/launcher/src/launch.rs#L54) | unit | Launch | 2026-03-07 | Asserts on `result.is_ok()` |  |
| [test_check_installation_empty_path](../../../crates/launcher/src/launch.rs#L62) | unit | Launch | 2026-03-07 | Asserts on `matches!(state, InstallState::NotInstalled)` |  |
| [test_check_installation_with_exe](../../../crates/launcher/src/launch.rs#L68) | unit | Launch | 2026-03-07 | Asserts on `matches!(state, InstallState::Installed)` |  |
| [test_patch_hostname_success](../../../crates/launcher/src/patch.rs#L61) | unit | Patch | 2026-03-07 | Asserts equality on `offset` |  |
| [test_patch_hostname_max_length](../../../crates/launcher/src/patch.rs#L70) | unit | Patch | 2026-03-07 | Asserts equality on `host.len()` |  |
| [test_patch_hostname_too_long](../../../crates/launcher/src/patch.rs#L80) | unit | Patch | 2026-03-07 | Asserts on `matches!(result, Err(PatchError::AddressTooLong { .. }))` |  |
| [test_patch_hostname_not_found](../../../crates/launcher/src/patch.rs#L88) | unit | Patch | 2026-03-07 | Asserts on `matches!(result, Err(PatchError::PatternNotFound))` |  |
| [test_needs_patching](../../../crates/launcher/src/patch.rs#L95) | unit | Patch | 2026-03-07 | Asserts on `needs_patching(&data)` |  |
| [test_patch_idempotent_check](../../../crates/launcher/src/patch.rs#L104) | unit | Patch | 2026-03-07 | Asserts on `!needs_patching(&data)` |  |
| [test_patch_exe_on_disk](../../../crates/launcher/src/patch.rs#L115) | unit | Patch | 2026-03-07 | Asserts equality on `offset` |  |
| [test_hash_file](../../../crates/launcher/src/updater.rs#L92) | unit | Updater | 2026-03-07 | Asserts equality on `hash` |  |
| [test_check_manifest_missing_file](../../../crates/launcher/src/updater.rs#L104) | unit | Updater | 2026-03-07 | Asserts equality on `updates.len()` |  |
| [test_check_manifest_matching_file](../../../crates/launcher/src/updater.rs#L121) | unit | Updater | 2026-03-07 | Test check manifest matching file |  |
| [test_check_manifest_changed_file](../../../crates/launcher/src/updater.rs#L140) | unit | Updater | 2026-03-07 | Test check manifest changed file |  |
| [test_verify_hash_success](../../../crates/launcher/src/updater.rs#L158) | unit | Updater | 2026-03-07 | Asserts on `result.is_ok()` |  |
