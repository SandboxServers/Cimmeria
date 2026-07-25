# Tests — `launcher`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-07-25  
> **Total tests**: 22 *(but see the mis-filing notice below — these 22 are not `crates/launcher`'s tests)*  
> **CI-gated**: no  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

> **⚠ This file is mis-filed and duplicates [tools-sgwlauncher.md](tools-sgwlauncher.md).**
>
> Every one of the 22 rows below describes a test that lives in
> **`tools/SGWLauncher/src-tauri/`** (the Tauri launcher), not in
> `crates/launcher` (the egui launcher, package `sgw-launcher`). All 22 were
> catalogued with `crates/launcher/src/...` paths that have never contained
> these tests — `crates/launcher/src/` has no `extract.rs`, `patch.rs`, or
> `updater.rs` at all. On 2026-07-25 the links were repointed to the real
> `tools/SGWLauncher` locations, which makes them resolve correctly but also
> makes this file an exact duplicate of
> [tools-sgwlauncher.md](tools-sgwlauncher.md) (same 22 tests, same paths).
>
> **`crates/launcher`'s real test suite — 176 tests across 22 files — is not
> catalogued anywhere.** Resolving this means deleting this file in favour of
> `tools-sgwlauncher.md` and generating a genuine `crates/launcher` catalogue
> in its place. Left for the owning agent to decide rather than actioned here.

SGW game launcher. The rows below cover the **Tauri** launcher
(`tools/SGWLauncher`, workspace-excluded, non-CI): patch client, login
redirect, install pipeline.

## All tests (22)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [test_default_config](../../../tools/SGWLauncher/src-tauri/src/config.rs#L53) | unit | Config | 2026-03-07 | Asserts equality on `config.server_address` |  |
| [test_save_and_load](../../../tools/SGWLauncher/src-tauri/src/config.rs#L62) | unit | Config | 2026-03-07 | Asserts equality on `loaded.install_path` |  |
| [test_load_missing_file](../../../tools/SGWLauncher/src-tauri/src/config.rs#L80) | unit | Config | 2026-03-07 | Asserts on `LauncherConfig::load(&path).is_err()` |  |
| [test_build_extract_args](../../../tools/SGWLauncher/src-tauri/src/extract.rs#L53) | unit | Extract | 2026-03-07 | Asserts equality on `args` |  |
| [test_count_cab_files](../../../tools/SGWLauncher/src-tauri/src/extract.rs#L59) | unit | Extract | 2026-03-07 | Asserts equality on `cabs.len()` |  |
| [test_count_cab_files_empty](../../../tools/SGWLauncher/src-tauri/src/extract.rs#L71) | unit | Extract | 2026-03-07 | Asserts equality on `cabs.len()` |  |
| [test_verify_installation_missing](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L50) | unit | Launch | 2026-03-07 | Asserts on `matches!(result, Err(LaunchError::NotFound(_)))` |  |
| [test_verify_installation_exists](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L56) | unit | Launch | 2026-03-07 | Asserts on `result.is_ok()` |  |
| [test_check_installation_empty_path](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L64) | unit | Launch | 2026-03-07 | Asserts on `matches!(state, InstallState::NotInstalled)` |  |
| [test_check_installation_with_exe](../../../tools/SGWLauncher/src-tauri/src/launch.rs#L70) | unit | Launch | 2026-03-07 | Asserts on `matches!(state, InstallState::Installed)` |  |
| [test_patch_hostname_success](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L60) | unit | Patch | 2026-03-07 | Asserts equality on `offset` |  |
| [test_patch_hostname_max_length](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L69) | unit | Patch | 2026-03-07 | Asserts equality on `host.len()` |  |
| [test_patch_hostname_too_long](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L79) | unit | Patch | 2026-03-07 | Asserts on `matches!(result, Err(PatchError::AddressTooLong { .. }))` |  |
| [test_patch_hostname_not_found](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L87) | unit | Patch | 2026-03-07 | Asserts on `matches!(result, Err(PatchError::PatternNotFound))` |  |
| [test_needs_patching](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L94) | unit | Patch | 2026-03-07 | Asserts on `needs_patching(&data)` |  |
| [test_patch_idempotent_check](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L103) | unit | Patch | 2026-03-07 | Asserts on `!needs_patching(&data)` |  |
| [test_patch_exe_on_disk](../../../tools/SGWLauncher/src-tauri/src/patch.rs#L114) | unit | Patch | 2026-03-07 | Asserts equality on `offset` |  |
| [test_hash_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L95) | unit | Updater | 2026-03-07 | Asserts equality on `hash` |  |
| [test_check_manifest_missing_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L107) | unit | Updater | 2026-03-07 | Asserts equality on `updates.len()` |  |
| [test_check_manifest_matching_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L124) | unit | Updater | 2026-03-07 | Test check manifest matching file |  |
| [test_check_manifest_changed_file](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L143) | unit | Updater | 2026-03-07 | Test check manifest changed file |  |
| [test_verify_hash_success](../../../tools/SGWLauncher/src-tauri/src/updater.rs#L161) | unit | Updater | 2026-03-07 | Asserts on `result.is_ok()` |  |
