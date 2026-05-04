# Tests — `tauri-app`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 6  
> **CI-gated**: no  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Cimmeria desktop app shell (Tauri, non-CI).

## All tests (6)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [normalize_mission_id_defaults_to_empty_scope](../../../src-tauri/src/content.rs#L1085) | unit | Content | 2026-03-07 | Asserts equality on `normalize_mission_id(None)` |  |
| [normalize_mission_id_preserves_specific_scope](../../../src-tauri/src/content.rs#L1090) | unit | Content | 2026-03-07 | Asserts equality on `normalize_mission_id(Some("638".to_string()))` |  |
| [parse_scope_payload_extracts_runtime_rows](../../../src-tauri/src/content.rs#L1095) | unit | Content | 2026-03-07 | Parse scope payload extracts runtime rows |  |
| [parse_scope_payload_rejects_scope_mismatch](../../../src-tauri/src/content.rs#L1202) | unit | Content | 2026-03-07 | Asserts on `error.contains("payload spaceId")` |  |
| [normalize_mission_id_defaults_to_empty_scope](../../../src-tauri/src/drafts.rs#L88) | unit | Drafts | 2026-03-07 | Asserts equality on `normalize_mission_id(None)` |  |
| [normalize_mission_id_preserves_specific_scope](../../../src-tauri/src/drafts.rs#L93) | unit | Drafts | 2026-03-07 | Asserts equality on `normalize_mission_id(Some("638".to_string()))` |  |
