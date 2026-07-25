# `launcher` test inventory

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-07-25  
> **Total tests**: 176 across 22 files  
> **CI-gated**: **no** — see [CI exclusion](#ci-exclusion)  
> **Index**: [README](README.md) · **Playbook**: [TESTING.md](../../../TESTING.md) · **Design**: [sgw-launcher.md](../../client/sgw-launcher.md), [dev-session-telemetry.md](../../architecture/dev-session-telemetry.md)

`crates/launcher` is the **egui native launcher**, package name `sgw-launcher`.
It installs and patches the game client, and it hosts the entire dev-session
telemetry pipeline (log tailing, event queueing, chunk upload, bundle upload).

> **Not to be confused with [tools-sgwlauncher.md](tools-sgwlauncher.md)**, which
> catalogues the separate **Tauri** launcher at `tools/SGWLauncher/src-tauri/`.
> Until 2026-07-25 this file was a verbatim duplicate of that one — all 22 rows
> described Tauri-launcher tests filed under `crates/launcher` paths that never
> contained them. The two crates are independent; this file now catalogues the
> real thing.

All tests live in `crates/launcher/src/**`. There is no `tests/` integration
directory and no benches. 157 are synchronous `#[test]`; 19 are
`#[tokio::test]`, and every one of those drives a local
[`wiremock`](https://docs.rs/wiremock) HTTP server — none needs network access
or a database.

Counted with `grep -rnE "^[[:space:]]*#\[(tokio::)?test(\(.*\))?\]" crates/launcher --include=*.rs`.

## CI exclusion

**None of these 176 tests run in CI.** `.github/workflows/test.yml` passes
`--exclude sgw-launcher`, and clippy, build, and nextest all inherit that same
exclude list (alongside `cimmeria-app`, `cimmeria-content-editor`,
`cimmeria-scene-editor`, `cimmeria-client-telemetry`).

This crate is **176 of the 245-test gap** between the workspace's 3,012 tests
and the 2,767 gated on every PR — by far the largest single contributor.

That matters more here than the raw number suggests, because several of these
are explicit regression guards whose entire value is failing when someone
reverts a fix — `download_to_file_returns_unexpected_status_on_non_2xx`,
`patch_hostname_any_repatches_when_host_changes` — and several are security
boundaries: ed25519 manifest-signature verification, the path-traversal guard
in `safe_sha_prefix`, and hostname-injection validation in `patch_rdata.rs`.
Run them locally before touching this crate:

```bash
cargo nextest run -p sgw-launcher
```

Two hazards if the exclusion is ever lifted. Tests in `client_paths.rs` and
`worker/mod.rs` mutate process environment and serialise through
`crate::client_paths::env_test_lock()`; the env-mutating tests in `config.rs`
and `identity.rs` do **not** take that lock and would be race-prone under a
parallel harness. And `config.rs::default_uses_localappdata_when_set` is
`#[cfg(target_os = "windows")]`, so it is invisible on a Linux runner.

---

## Install and patch

### `src/config.rs` — launcher config load/save, schema versioning (7)

| Test | Kind | What it tests |
|---|---|---|
| [`save_and_load_roundtrip`](../../../crates/launcher/src/config.rs#L165) | unit | install path, server host, manifest URL, and a `telemetry.enabled = false` opt-out all survive a round trip. |
| [`load_legacy_config_without_telemetry_field_defaults_to_enabled`](../../../crates/launcher/src/config.rs#L191) | unit | Config JSON predating the `telemetry` field loads with telemetry enabled. |
| [`telemetry_settings_default_is_enabled`](../../../crates/launcher/src/config.rs#L208) | unit | Pins the opt-out (not opt-in) default. |
| [`load_missing_file_errors`](../../../crates/launcher/src/config.rs#L214) | unit | Absent config file is an error, not a silent default. |
| [`load_legacy_without_schema_version`](../../../crates/launcher/src/config.rs#L221) | unit | Missing `schema_version` defaults to current; other fields survive. |
| [`load_unsupported_schema_version_errors`](../../../crates/launcher/src/config.rs#L238) | unit | `schema_version: 99` → `UnsupportedSchema { got: 99, expected: 1 }`. |
| [`default_uses_localappdata_when_set`](../../../crates/launcher/src/config.rs#L268) | unit (Windows-only) | `LOCALAPPDATA` drives the default install path. |

### `src/manifest.rs` — manifest schema, patch chain, blob URLs, ed25519 signatures (22)

The largest single file in the suite, and the one carrying the crate's trust
boundary: a forged or tampered manifest is a remote-code-execution path into a
player's install.

| Test | Kind | What it tests |
|---|---|---|
| [`validates_simple_chain`](../../../crates/launcher/src/manifest.rs#L329) | unit | `a` then `b after a` is a valid patch chain. |
| [`rejects_forward_after_ref`](../../../crates/launcher/src/manifest.rs#L339) | unit | A patch referencing one declared later is `BrokenChain`. |
| [`rejects_unknown_after_ref`](../../../crates/launcher/src/manifest.rs#L349) | unit | `after` naming a nonexistent patch is `BrokenChain`. |
| [`rejects_duplicate_ids`](../../../crates/launcher/src/manifest.rs#L359) | unit | Two patches sharing an id is `BrokenChain`. |
| [`rejects_unsupported_schema`](../../../crates/launcher/src/manifest.rs#L369) | unit | `schema: 99` → `UnsupportedSchema(99)`. |
| [`blob_url_strips_manifest_filename`](../../../crates/launcher/src/manifest.rs#L382) | unit | Relative blob paths resolve against the manifest's directory. |
| [`blob_url_strips_sas_query_before_resolving`](../../../crates/launcher/src/manifest.rs#L391) | unit | An Azure SAS query is dropped before joining, not carried into the path. |
| [`blob_url_strips_fragment_before_resolving`](../../../crates/launcher/src/manifest.rs#L400) | unit | `#fragment` dropped before joining. |
| [`blob_url_strips_both_query_and_fragment`](../../../crates/launcher/src/manifest.rs#L409) | unit | Query and fragment stripped together. |
| [`blob_url_passes_absolute_https_through`](../../../crates/launcher/src/manifest.rs#L418) | unit | Absolute https blob (GitHub Releases mode) is returned verbatim. |
| [`blob_url_passes_absolute_http_through`](../../../crates/launcher/src/manifest.rs#L432) | unit | Absolute http passes through — scheme enforcement is a fetch-time concern, not a URL-building one. |
| [`fetch_manifest_rejects_non_https`](../../../crates/launcher/src/manifest.rs#L444) | unit (async) | An `http://` manifest URL is `InsecureUrl` before any request goes out. |
| [`verify_manifest_signature_accepts_signed_body`](../../../crates/launcher/src/manifest.rs#L467) | unit | A correctly signed body verifies. |
| [`verify_manifest_signature_rejects_wrong_body`](../../../crates/launcher/src/manifest.rs#L474) | unit | Tampered body with an otherwise valid signature is `BadSignature`. |
| [`verify_manifest_signature_rejects_wrong_key`](../../../crates/launcher/src/manifest.rs#L484) | unit | A signature from an unrelated key is `BadSignature`. |
| [`verify_manifest_signature_rejects_malformed_hex`](../../../crates/launcher/src/manifest.rs#L498) | unit | Five malformed hex forms (empty, short, non-hex, 127 and 129 chars) are `BadSignatureFormat`. |
| [`sig_url_for_appends_when_no_query`](../../../crates/launcher/src/manifest.rs#L510) | unit | `m.json` → `m.json.sig`. |
| [`sig_url_for_inserts_before_query`](../../../crates/launcher/src/manifest.rs#L518) | unit | `.sig` goes before the query string, query preserved verbatim. |
| [`sig_url_for_strips_fragment_before_appending`](../../../crates/launcher/src/manifest.rs#L529) | unit | Fragment dropped. |
| [`sig_url_for_handles_fragment_and_query`](../../../crates/launcher/src/manifest.rs#L537) | unit | Query kept, fragment dropped. |
| [`hex_decode_32_round_trip`](../../../crates/launcher/src/manifest.rs#L545) | unit | 32-byte key → hex → decoded identically. |
| [`parses_minimal_manifest_json`](../../../crates/launcher/src/manifest.rs#L553) | unit | A minimal manifest deserializes, validates, and exposes the right seed blob. |

### `src/install.rs` — download, sha256 verify, zip extraction, adopt-existing-install (12)

| Test | Kind | What it tests |
|---|---|---|
| [`hashes_a_known_file`](../../../crates/launcher/src/install.rs#L449) | unit | `hash_file` of `"hello world"` matches the known sha256. |
| [`verify_sha256_succeeds_on_match`](../../../crates/launcher/src/install.rs#L460) | unit | Matching digest passes. |
| [`verify_sha256_fails_on_mismatch`](../../../crates/launcher/src/install.rs#L473) | unit | Wrong digest is `HashMismatch`. |
| [`extract_zip_writes_expected_files`](../../../crates/launcher/src/install.rs#L482) | unit | Nested zip entries extract to the right relative paths with the right content. |
| [`download_to_file_returns_unexpected_status_on_non_2xx`](../../../crates/launcher/src/install.rs#L517) | unit (async) | **Regression guard** — a 418 surfaces as `UnexpectedStatus { status, url }` instead of panicking through `error_for_status_ref().unwrap_err()`. |
| [`safe_sha_prefix_accepts_lower_and_upper_hex`](../../../crates/launcher/src/install.rs#L549) | unit | Both hex cases yield the first 12 chars. |
| [`safe_sha_prefix_rejects_path_traversal`](../../../crates/launcher/src/install.rs#L555) | unit | **Security guard** — seven traversal payloads (`../foo`, `/etc/passwd`, `..\evil`, …) are all `InvalidSha256`. The prefix becomes a path component, so this is the gate. |
| [`safe_sha_prefix_short_input_passes`](../../../crates/launcher/src/install.rs#L574) | unit | `"abc"` passes — the gate is all-hex, not length. |
| [`adopt_existing_install_writes_marker_when_sgw_exe_present`](../../../crates/launcher/src/install.rs#L599) | unit | Adopting writes a marker with `seed_adopted = true` and the manifest seed hash. |
| [`adopt_existing_install_rejects_empty_directory`](../../../crates/launcher/src/install.rs#L624) | unit | Empty dir is `NoGameExe`. |
| [`adopt_existing_install_rejects_when_sgw_exe_is_a_directory`](../../../crates/launcher/src/install.rs#L637) | unit | A *directory* named `SGW.exe` is still `NoGameExe` — the check is `is_file`. |
| [`adopt_existing_install_rejects_already_managed_install`](../../../crates/launcher/src/install.rs#L650) | unit | Adopting twice is `AlreadyManaged`. |

### `src/patch_rdata.rs` — hostname patching in `SGW.exe`'s `.rdata` (15)

Writes a player-supplied string into the game binary, so the validation tests
here are the injection boundary.

| Test | Kind | What it tests |
|---|---|---|
| [`patch_hostname_success`](../../../crates/launcher/src/patch_rdata.rs#L187) | unit | Returns offset 40, writes the host, zeroes the rest of the slot. |
| [`patch_hostname_at_max_length`](../../../crates/launcher/src/patch_rdata.rs#L196) | unit | A 22-char host exactly fills the original literal's slot. |
| [`patch_hostname_rejects_too_long`](../../../crates/launcher/src/patch_rdata.rs#L206) | unit | Over-length host is `AddressTooLong`, never a buffer overrun. |
| [`patch_hostname_pattern_not_found`](../../../crates/launcher/src/patch_rdata.rs#L214) | unit | A zeroed buffer is `PatternNotFound`. |
| [`patch_hostname_rejects_empty`](../../../crates/launcher/src/patch_rdata.rs#L221) | unit | Empty host is `InvalidHostname`. |
| [`patch_hostname_rejects_disallowed_chars`](../../../crates/launcher/src/patch_rdata.rs#L230) | unit | Five payloads with spaces, `@`, or `/` are `InvalidHostname`. |
| [`patch_hostname_rejects_leading_or_trailing_dash_or_dot`](../../../crates/launcher/src/patch_rdata.rs#L245) | unit | Four boundary-character payloads are `InvalidHostname`. |
| [`patch_hostname_accepts_typical_dns_names`](../../../crates/launcher/src/patch_rdata.rs#L260) | unit | Four ordinary DNS names all succeed — the validator is not over-tight. |
| [`needs_patching_flips_after_patch`](../../../crates/launcher/src/patch_rdata.rs#L276) | unit | The original CME literal is detectable before and gone after. |
| [`patch_hostname_any_repatches_when_host_changes`](../../../crates/launcher/src/patch_rdata.rs#L289) | unit | **Regression guard** — with a `previous_host`, a second patch finds the same offset, writes the new host, and leaves no trace of the old one. Guards against a silent no-op re-patch. |
| [`patch_hostname_any_falls_back_to_original_when_previous_none`](../../../crates/launcher/src/patch_rdata.rs#L327) | unit | With no `previous_host`, falls back to matching the original CME literal. |
| [`host_differs_false_when_slot_matches_expected`](../../../crates/launcher/src/patch_rdata.rs#L337) | unit | An already-correct binary reports in sync. |
| [`host_differs_true_when_binary_still_has_original`](../../../crates/launcher/src/patch_rdata.rs#L348) | unit | An unpatched binary reports out of sync. |
| [`host_differs_true_when_expected_changed`](../../../crates/launcher/src/patch_rdata.rs#L354) | unit | Editing the configured host reports out of sync. |
| [`patch_exe_on_disk_round_trip`](../../../crates/launcher/src/patch_rdata.rs#L367) | unit | Patching a real file on disk returns offset 40 and the re-read bytes carry the new host. |

### `src/launch.rs` — client detection and launch, install-dir writability (6)

| Test | Kind | What it tests |
|---|---|---|
| [`detect_finds_only_present_files`](../../../crates/launcher/src/launch.rs#L201) | unit | Each presence flag matches on-disk reality; Atera unavailable without its bat. |
| [`detect_marks_atera_available_when_pair_present`](../../../crates/launcher/src/launch.rs#L214) | unit | Loader plus `AtreaGameDebug.bat` marks Atera available. |
| [`launch_errors_when_missing`](../../../crates/launcher/src/launch.rs#L223) | unit | Missing `SGW.exe` is `NotFound`. |
| [`detect_empty_dir`](../../../crates/launcher/src/launch.rs#L230) | unit | Empty dir detects nothing. |
| [`install_dir_writable_succeeds_on_temp`](../../../crates/launcher/src/launch.rs#L238) | unit | Writability probe succeeds and cleans up its `.launcher-write-probe`. |
| [`install_dir_writable_creates_missing_parents`](../../../crates/launcher/src/launch.rs#L246) | unit | Probe creates missing parent directories. |

### `src/state.rs` — on-disk install / telemetry state and the upload ledger (12)

| Test | Kind | What it tests |
|---|---|---|
| [`installed_state_roundtrip`](../../../crates/launcher/src/state.rs#L171) | unit | All four fields round-trip; `has_applied` is correct both ways. |
| [`installed_state_roundtrip_adopted`](../../../crates/launcher/src/state.rs#L193) | unit | `seed_adopted` and the seed hash survive save/load. |
| [`installed_state_legacy_without_seed_adopted_defaults_false`](../../../crates/launcher/src/state.rs#L211) | unit | Legacy state without the field defaults to not-adopted. |
| [`installed_state_loads_legacy_without_patched_host`](../../../crates/launcher/src/state.rs#L227) | unit | Legacy state without `patched_host` loads as `None`, rest intact. |
| [`installed_state_missing_file_is_default`](../../../crates/launcher/src/state.rs#L241) | unit | No state file is a default, not an error. |
| [`telemetry_state_default_is_zero_value`](../../../crates/launcher/src/state.rs#L252) | unit | All four fields at zero, including `kill_switch_active = false`. |
| [`telemetry_state_roundtrip_preserves_all_fields`](../../../crates/launcher/src/state.rs#L263) | unit | Full struct equality after write and load. |
| [`telemetry_state_load_missing_file_returns_default`](../../../crates/launcher/src/state.rs#L283) | unit | Missing file is a default, not an error. |
| [`telemetry_state_load_corrupt_file_returns_default`](../../../crates/launcher/src/state.rs#L294) | unit | Invalid JSON degrades to default — tolerates a crash mid-save. |
| [`telemetry_state_load_ignores_unknown_future_fields`](../../../crates/launcher/src/state.rs#L307) | unit | Pins the *absence* of `deny_unknown_fields`, so a newer launcher's state file does not brick an older one. |
| [`telemetry_state_load_partial_json_defaults_missing_fields`](../../../crates/launcher/src/state.rs#L325) | unit | Partial JSON honours what is set, defaults what is not. |
| [`uploaded_ledger_dedupe`](../../../crates/launcher/src/state.rs#L335) | unit | A recorded blob is absent before and present after; the ledger holds one entry with the right name. |

### `src/client_paths.rs` — client user-data paths and wipe helpers (6)

The wipe helpers delete user directories, so the env-resolution tests here are
a safety boundary, not a formality.

| Test | Kind | What it tests |
|---|---|---|
| [`wipe_dir_contents_removes_files_and_subdirs`](../../../crates/launcher/src/client_paths.rs#L166) | unit | A populated tree is emptied; the report counts entries and bytes; the directory itself survives. |
| [`wipe_dir_contents_handles_missing_path_as_success`](../../../crates/launcher/src/client_paths.rs#L183) | unit | A missing path is a zero report, not an error. |
| [`wipe_dir_contents_on_empty_dir_is_zero_report`](../../../crates/launcher/src/client_paths.rs#L191) | unit | Empty dir yields a zero report. |
| [`firesky_root_uses_userprofile_when_set`](../../../crates/launcher/src/client_paths.rs#L201) | unit | Root resolves under `Documents/My Games/Firesky`, cache under `Cache.en-US`. |
| [`firesky_root_returns_none_when_both_env_vars_empty`](../../../crates/launcher/src/client_paths.rs#L232) | unit | **Safety guard** — empty-string `USERPROFILE` and `HOME` are treated as unset, so a wipe can never resolve to a cwd-relative path. |
| [`firesky_root_falls_back_to_home_when_userprofile_unset`](../../../crates/launcher/src/client_paths.rs#L250) | unit | Falls back to `HOME` when `USERPROFILE` is absent. |

### `src/identity.rs` — per-install identity for dev-session telemetry (9)

| Test | Kind | What it tests |
|---|---|---|
| [`hash_to_16_hex_is_deterministic_and_16_lowercase_hex`](../../../crates/launcher/src/identity.rs#L206) | unit | Same input hashes identically; different input differs; always 16 lowercase hex chars. |
| [`hash_to_16_hex_empty_input`](../../../crates/launcher/src/identity.rs#L224) | unit | Pins `hash_to_16_hex(b"") == "e3b0c44298fc1c14"`. |
| [`load_or_mint_mints_once_and_reuses`](../../../crates/launcher/src/identity.rs#L232) | unit | First call persists `install.json`; the second returns the same identity. |
| [`save_load_roundtrip_preserves_all_fields`](../../../crates/launcher/src/identity.rs#L244) | unit | Full struct equality after a round trip. |
| [`load_rejects_unsupported_schema_version`](../../../crates/launcher/src/identity.rs#L259) | unit | `schema_version: 99` → `UnsupportedSchema { got: 99, expected: 1 }`. |
| [`load_or_mint_surfaces_corrupt_file_instead_of_silently_reminting`](../../../crates/launcher/src/identity.rs#L282) | unit | **Mint-once invariant** — corrupt JSON is an error, never a silent re-mint that would fork the install's identity. |
| [`distinct_mints_produce_distinct_install_ids`](../../../crates/launcher/src/identity.rs#L294) | unit | Two mints differ in `install_id` but share `machine_id`. |
| [`mint_records_current_launcher_version`](../../../crates/launcher/src/identity.rs#L305) | unit | Records `CARGO_PKG_VERSION` at mint time. |
| [`derive_machine_id_returns_16_hex_chars`](../../../crates/launcher/src/identity.rs#L316) | unit | 16 hex chars whether sourced from the registry or the hostname fallback. |

### `src/logs.rs` — log collection, zip bundling, content digest, blob SAS URLs (13)

| Test | Kind | What it tests |
|---|---|---|
| [`collect_picks_up_logs_and_sessions`](../../../crates/launcher/src/logs.rs#L209) | unit | Finds both debug logs and the session log. |
| [`collect_is_sorted_for_stable_digest`](../../../crates/launcher/src/logs.rs#L217) | unit | Ordering is stable across calls — the digest depends on it. |
| [`collect_empty_when_dirs_missing`](../../../crates/launcher/src/logs.rs#L226) | unit | Missing `Binaries/` yields an empty set. |
| [`build_zip_returns_none_when_no_logs`](../../../crates/launcher/src/logs.rs#L232) | unit | No logs means no zip, not an empty one. |
| [`build_zip_round_trips_through_zip_reader`](../../../crates/launcher/src/logs.rs#L238) | unit | The zip contains the exact expected relative entry names, nested session path included. |
| [`content_digest_is_stable`](../../../crates/launcher/src/logs.rs#L254) | unit | Unchanged content digests identically. |
| [`content_digest_changes_when_logs_change`](../../../crates/launcher/src/logs.rs#L263) | unit | Editing a log changes the digest — the dedupe key actually tracks content. |
| [`content_digest_none_when_no_logs`](../../../crates/launcher/src/logs.rs#L273) | unit | No logs means no digest. |
| [`insert_blob_path_with_sas_query`](../../../crates/launcher/src/logs.rs#L279) | unit | The path is inserted before the SAS query, not after it. |
| [`insert_blob_path_handles_trailing_slash`](../../../crates/launcher/src/logs.rs#L291) | unit | A trailing slash on the container URL does not double up. |
| [`insert_blob_path_without_query`](../../../crates/launcher/src/logs.rs#L297) | unit | Plain append when there is no query string. |
| [`upload_blob_rejects_non_https`](../../../crates/launcher/src/logs.rs#L303) | unit (async) | An `http://` target is `InsecureUrl` before any request — logs never leave over cleartext. |
| [`blob_name_includes_digest_prefix`](../../../crates/launcher/src/logs.rs#L312) | unit | Blob name starts `logs/` and ends with the 12-char digest prefix plus `.zip`. |

### `src/inject.rs` — DLL injection for `cimmeria-client-telemetry` (5)

| Test | Kind | What it tests |
|---|---|---|
| [`encode_dll_path_w_rejects_missing_file`](../../../crates/launcher/src/inject.rs#L498) | unit | A nonexistent DLL is `DllMissing(path)` before anything is written to the target process. |
| [`encode_dll_path_w_appends_nul_terminator`](../../../crates/launcher/src/inject.rs#L507) | unit | The UTF-16 buffer is NUL-terminated. |
| [`check_wide_len_accepts_at_cap`](../../../crates/launcher/src/inject.rs#L523) | unit | Exactly `MAX_DLL_PATH_W` is accepted. |
| [`check_wide_len_rejects_above_cap`](../../../crates/launcher/src/inject.rs#L529) | unit | One over the cap is `DllPathTooLong { got, max }` with exact values. |
| [`encode_dll_path_w_succeeds_on_short_existing_path`](../../../crates/launcher/src/inject.rs#L540) | unit | A real temp file encodes within the cap. |

---

## Telemetry pipeline

Design context: [dev-session-telemetry.md](../../architecture/dev-session-telemetry.md)
and the operator runbook at [telemetry.md](../../operations/telemetry.md).

Two failure modes recur across this half of the suite and are worth reading as a
pair: **401 must surface as `TokenRejected`** (the token expired — refresh and
retry) and **503 with a `retry-after` header must surface as `KillSwitch`** (the
server has paused ingest deliberately — back off, do not retry). Conflating
either with a generic 5xx would make the kill switch unenforceable. Every
network-facing module tests both.

### `src/telemetry/auth.rs` — `/auth/dev-session` HMAC token fetch and refresh (6)

| Test | Kind | What it tests |
|---|---|---|
| [`should_refresh_below_threshold_returns_true`](../../../crates/launcher/src/telemetry/auth.rs#L142) | unit | 80% elapsed and fully expired both trigger a refresh. |
| [`should_refresh_above_threshold_returns_false`](../../../crates/launcher/src/telemetry/auth.rs#L152) | unit | 50% elapsed and just-issued do not. |
| [`fetch_dev_session_happy_path`](../../../crates/launcher/src/telemetry/auth.rs#L162) | unit (async) | A 200 parses token, session id, and upload endpoint. |
| [`refresh_dev_session_posts_existing_token`](../../../crates/launcher/src/telemetry/auth.rs#L179) | unit (async) | Refresh POSTs the old token to `/refresh` and returns the new one. |
| [`fetch_dev_session_503_surfaces_kill_switch`](../../../crates/launcher/src/telemetry/auth.rs#L199) | unit (async) | 503 plus `retry-after: 180` is `KillSwitch { retry_after_secs: 180 }`. |
| [`fetch_dev_session_5xx_other_surfaces_status`](../../../crates/launcher/src/telemetry/auth.rs#L217) | unit (async) | 500 is `Status { status: 500 }`, distinct from the kill switch. |

### `src/telemetry/queue.rs` — crash-safe JSONL disk queue with size-cap compaction (6)

| Test | Kind | What it tests |
|---|---|---|
| [`enqueue_then_drain_returns_events_in_order`](../../../crates/launcher/src/telemetry/queue.rs#L191) | unit | Five events drain FIFO; the queue file is removed by the drain. |
| [`drain_on_missing_file_returns_empty`](../../../crates/launcher/src/telemetry/queue.rs#L207) | unit | No file drains empty rather than erroring. |
| [`drain_skips_unparseable_lines`](../../../crates/launcher/src/telemetry/queue.rs#L218) | unit | A garbage line is skipped and the surrounding valid events still come through in order. |
| [`enqueue_compacts_on_overflow_dropping_oldest`](../../../crates/launcher/src/telemetry/queue.rs#L239) | unit | At the cap, oldest events are dropped, the counter moves, and the freshest event is retained. |
| [`compaction_starts_at_line_boundary`](../../../crates/launcher/src/telemetry/queue.rs#L262) | unit | Compaction never leaves a partial leading line — the first line after it parses as JSON. |
| [`dropped_counter_persists_across_compactions`](../../../crates/launcher/src/telemetry/queue.rs#L283) | unit | The dropped count is cumulative and never resets. |

### `src/telemetry/chunk.rs` — NDJSON + gzip chunk encoding and upload (8)

| Test | Kind | What it tests |
|---|---|---|
| [`encode_ndjson_gzip_roundtrips_to_n_newline_delimited_lines`](../../../crates/launcher/src/telemetry/chunk.rs#L126) | unit | Three events encode to three newline-delimited lines, each deserializing back. |
| [`encode_ndjson_gzip_empty_input_produces_valid_gzip`](../../../crates/launcher/src/telemetry/chunk.rs#L141) | unit | An empty slice still produces valid gzip. |
| [`join_endpoint_collapses_trailing_slash`](../../../crates/launcher/src/telemetry/chunk.rs#L150) | unit | Endpoint join is trailing-slash agnostic. |
| [`post_chunk_empty_events_skips_network`](../../../crates/launcher/src/telemetry/chunk.rs#L166) | unit (async) | An empty slice short-circuits before any request (proven with a bogus host). |
| [`post_chunk_sends_gzip_with_bearer_and_content_type`](../../../crates/launcher/src/telemetry/chunk.rs#L174) | unit (async) | Exactly one request carrying `Bearer`, `application/x-ndjson`, and `content-encoding: gzip`. |
| [`post_chunk_401_surfaces_token_rejected`](../../../crates/launcher/src/telemetry/chunk.rs#L197) | unit (async) | 401 is `TokenRejected`. |
| [`post_chunk_503_surfaces_kill_switch_with_retry_after`](../../../crates/launcher/src/telemetry/chunk.rs#L213) | unit (async) | 503 plus `retry-after: 120` is `KillSwitch`. |
| [`post_chunk_5xx_other_than_503_surfaces_as_status`](../../../crates/launcher/src/telemetry/chunk.rs#L234) | unit (async) | 500 is `Status { status, body }` — a transient hiccup, not paused ingest. |

### `src/telemetry/events.rs` — event wire shapes and Atera log-line parsing (10)

| Test | Kind | What it tests |
|---|---|---|
| [`client_log_serializes_with_type_tag`](../../../crates/launcher/src/telemetry/events.rs#L194) | unit | `type == "client_log"` with `seq` and `packet_no` at expected values. |
| [`client_log_omits_absent_packet_no`](../../../crates/launcher/src/telemetry/events.rs#L213) | unit | An absent `packet_no` is omitted entirely rather than serialized as `null` — Cosmos indexing depends on it. |
| [`session_meta_kind_serializes_snake_case`](../../../crates/launcher/src/telemetry/events.rs#L231) | unit | `FileRotated` serializes as `"file_rotated"`. |
| [`event_roundtrip_preserves_variant`](../../../crates/launcher/src/telemetry/events.rs#L243) | unit | All five variants survive serialize then deserialize. |
| [`client_native_serializes_with_expected_shape`](../../../crates/launcher/src/telemetry/events.rs#L297) | unit | Pins the `client_native` byte shape including nested `fields.*` — admin-api declares this shape independently, so the two must not drift. |
| [`parse_client_log_line_standard_shape`](../../../crates/launcher/src/telemetry/events.rs#L319) | unit | A standard Atera line yields level, category, packet number, and message. |
| [`parse_client_log_line_packet_alt_spellings`](../../../crates/launcher/src/telemetry/events.rs#L328) | unit | Both `packet=12` and `#7` yield the packet number. |
| [`parse_client_log_line_ignores_non_packet_hash`](../../../crates/launcher/src/telemetry/events.rs#L338) | unit | `#define` is not mistaken for a packet number. |
| [`parse_client_log_line_falls_back_to_raw_on_unknown_shape`](../../../crates/launcher/src/telemetry/events.rs#L345) | unit | An unrecognised line degrades to level `info`, category `raw`, message verbatim — never dropped. |
| [`parse_client_log_line_first_packet_match_wins`](../../../crates/launcher/src/telemetry/events.rs#L354) | unit | With two candidates, the first wins. |

### `src/telemetry/tail.rs` — multi-file log tailer (7)

| Test | Kind | What it tests |
|---|---|---|
| [`fresh_tail_skips_existing_content_on_first_tick`](../../../crates/launcher/src/telemetry/tail.rs#L196) | unit | Pre-existing content is not re-shipped on attach. |
| [`tick_emits_lines_appended_since_last_tick`](../../../crates/launcher/src/telemetry/tail.rs#L210) | unit | Only newly appended lines are returned, tagged with the source file. |
| [`tick_strips_crlf`](../../../crates/launcher/src/telemetry/tail.rs#L227) | unit | Windows line endings are stripped. |
| [`tick_holds_back_partial_final_line`](../../../crates/launcher/src/telemetry/tail.rs#L242) | unit | A newline-less tail is withheld, then emitted whole once complete — no torn lines. |
| [`tick_recovers_after_truncation`](../../../crates/launcher/src/telemetry/tail.rs#L261) | unit | A shrinking file resets to offset 0 and reads from the top. |
| [`refresh_picks_up_new_files_without_disturbing_existing_state`](../../../crates/launcher/src/telemetry/tail.rs#L282) | unit | A per-minute log roll adds the new file while preserving the existing file's offset. |
| [`refresh_drops_files_that_no_longer_match`](../../../crates/launcher/src/telemetry/tail.rs#L305) | unit | A deleted file stops being watched. |

### `src/telemetry/bundle.rs` — end-of-session multipart log-bundle upload (4)

| Test | Kind | What it tests |
|---|---|---|
| [`upload_bundle_returns_empty_when_no_logs`](../../../crates/launcher/src/telemetry/bundle.rs#L156) | unit (async) | An empty install dir is `Empty` with no network call. |
| [`upload_bundle_posts_multipart_and_returns_outcome`](../../../crates/launcher/src/telemetry/bundle.rs#L172) | unit (async) | Exactly one POST; the outcome carries a 64-char sha256 and a non-zero byte count. |
| [`upload_bundle_401_surfaces_token_rejected`](../../../crates/launcher/src/telemetry/bundle.rs#L197) | unit (async) | 401 is `TokenRejected`. |
| [`upload_bundle_503_surfaces_kill_switch_with_retry_after`](../../../crates/launcher/src/telemetry/bundle.rs#L220) | unit (async) | 503 plus `retry-after: 90` is `KillSwitch`. |

### `src/telemetry/mod.rs` — orchestrator: session start, seq stamping, recovery (4)

| Test | Kind | What it tests |
|---|---|---|
| [`recover_pending_returns_zero_on_empty_queue`](../../../crates/launcher/src/telemetry/mod.rs#L284) | unit | An empty disk queue recovers nothing. |
| [`recover_pending_drains_and_reenqueues`](../../../crates/launcher/src/telemetry/mod.rs#L291) | unit | Three queued events are recovered and remain drainable — recovery does not consume them. |
| [`stamp_seq_writes_seq_on_every_variant`](../../../crates/launcher/src/telemetry/mod.rs#L316) | unit | Every event variant gets the sequence number; none is missed. |
| [`start_session_writes_marker_then_enqueue_flush_round_trips`](../../../crates/launcher/src/telemetry/mod.rs#L371) | unit (async) | Writes `current-session.json`, then enqueue plus flush POSTs exactly one chunk. |

### `src/telemetry/session.rs` — current-session marker file (3)

| Test | Kind | What it tests |
|---|---|---|
| [`write_creates_nested_dirs_and_roundtrips`](../../../crates/launcher/src/telemetry/session.rs#L92) | unit | Creates `Binaries/sessions/current-session.json` and round-trips its content. |
| [`write_overwrites_previous_file`](../../../crates/launcher/src/telemetry/session.rs#L103) | unit | A second write replaces rather than appends or merges. |
| [`load_ignores_unknown_future_fields`](../../../crates/launcher/src/telemetry/session.rs#L116) | unit | A marker from a newer launcher still loads. |

### `src/telemetry/runner.rs` — tailed line to event routing (2)

| Test | Kind | What it tests |
|---|---|---|
| [`line_to_event_routes_sgwdebuglog_to_debug_log`](../../../crates/launcher/src/telemetry/runner.rs#L220) | unit | A `sgwdebuglog` source routes to the `DebugLog` variant. |
| [`line_to_event_parses_atera_format_into_client_log`](../../../crates/launcher/src/telemetry/runner.rs#L232) | unit | An Atera-format line from a `.log` becomes a `ClientLog` with level, category, and packet number. |

### `src/telemetry/process_watch.rs` — async child-process exit watcher (2)

| Test | Kind | What it tests |
|---|---|---|
| [`wait_for_exit_resolves_with_pid_and_exit_code`](../../../crates/launcher/src/telemetry/process_watch.rs#L72) | unit (async) | Reports a real pid and exit code 0. |
| [`wait_for_exit_surfaces_non_zero_exit_code`](../../../crates/launcher/src/telemetry/process_watch.rs#L80) | unit (async) | A child exiting 7 reports 7 — a client crash is distinguishable from a clean quit. |

---

## UI shell and worker

### `src/app/mod.rs` — egui app shell: formatting, status log, event mapping (11)

| Test | Kind | What it tests |
|---|---|---|
| [`human_bytes_formats_units`](../../../crates/launcher/src/app/mod.rs#L284) | unit | `0 B`, `512 B`, `2.00 KB`, `5.00 MB`. |
| [`push_status_caps_at_max_lines`](../../../crates/launcher/src/app/mod.rs#L303) | unit | The status buffer caps, dropping oldest and retaining newest — unbounded growth is the bug guarded. |
| [`push_status_under_cap_does_not_drain`](../../../crates/launcher/src/app/mod.rs#L318) | unit | Below the cap, nothing is dropped. |
| [`should_show_adopt_button_false_on_empty_dir`](../../../crates/launcher/src/app/mod.rs#L330) | unit | Nothing to adopt, button hidden. |
| [`should_show_adopt_button_true_when_unmanaged_install_present`](../../../crates/launcher/src/app/mod.rs#L338) | unit | `SGW.exe` with no marker shows the button. |
| [`should_show_adopt_button_false_when_already_managed`](../../../crates/launcher/src/app/mod.rs#L348) | unit | A marker hides it. |
| [`status_line_for_formats_adopt_complete`](../../../crates/launcher/src/app/mod.rs#L364) | unit | The adopt-complete line says both "Adopted" and "not verified" — surfaces the trust trade-off to the player. |
| [`status_line_for_formats_adopt_error`](../../../crates/launcher/src/app/mod.rs#L374) | unit | Exact error string. |
| [`status_line_for_formats_wiped_with_human_bytes`](../../../crates/launcher/src/app/mod.rs#L380) | unit | Exact wipe-summary string with humanised bytes. |
| [`status_line_for_formats_wipe_error`](../../../crates/launcher/src/app/mod.rs#L395) | unit | Exact wipe-error string. |
| [`status_line_for_returns_none_for_progress_and_manifest_events`](../../../crates/launcher/src/app/mod.rs#L401) | unit | Progress and manifest-error events drive widgets, not the log, so they map to `None`. |

### `src/worker/mod.rs` — background command dispatch (6)

| Test | Kind | What it tests |
|---|---|---|
| [`launch_sgw_with_client_telemetry_routes_through_dispatch`](../../../crates/launcher/src/worker/mod.rs#L434) | unit | The telemetry-launch command reaches the launch path — pins the wiring, not the kernel call, by asserting a missing `SGW.exe` surfaces as `LaunchError`. |
| [`spawn_adopt_emits_complete_and_writes_marker`](../../../crates/launcher/src/worker/mod.rs#L462) | unit | Adopt emits `AdoptComplete` and the on-disk state records the seed hash. |
| [`spawn_adopt_emits_error_when_install_dir_empty`](../../../crates/launcher/src/worker/mod.rs#L489) | unit | Failure emits `AdoptError` naming `SGW.exe` rather than being swallowed. |
| [`spawn_wipe_cache_clears_cache_subdir`](../../../crates/launcher/src/worker/mod.rs#L521) | unit | Cache wipe reports entries and bytes; the directory survives, contents do not. |
| [`spawn_wipe_all_clears_firesky_tree`](../../../crates/launcher/src/worker/mod.rs#L574) | unit | Full wipe empties the Firesky tree while keeping the directory. |
| [`spawn_wipe_emits_error_when_no_profile_env`](../../../crates/launcher/src/worker/mod.rs#L617) | unit | **Safety guard** — with neither `USERPROFILE` nor `HOME` set, wipe errors naming the missing variable instead of deleting relative to the current directory. |
