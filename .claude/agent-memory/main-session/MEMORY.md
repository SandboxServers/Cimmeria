# Main-Session Project Memory — Index

Dated, sourced facts from top-level sessions, committed with the change that produced them; verified facts graduate to `docs/`. Rules: [development-workflow.md § Project memory](../../../docs/agents/development-workflow.md#project-memory). One line per memory, newest first within a section.

## Open investigations

- [project_dakara_dk05_population.md](project_dakara_dk05_population.md) — 2026-10-07 DK-05: four static cast spawns; owner chose a speculative military-outpost Repository/Loth'ta point at (290, -17, 95); mission dialog restore binds still await DK-10 onward; M2 client UAT pending

- [project_dakara_dk04_travel.md](project_dakara_dk04_travel.md) — 2026-10-07 DK-04: two exterior tent flaps share world 62 and one return point; world-gated interact tags, placement/test anchors, client UAT pending
- [project_dialog_quarantine_2026_10_07.md](project_dialog_quarantine_2026_10_07.md) — stock PAK census weakens the speaker, button-type and screen-count theories; dialog crash cause still unproven

- [project_bank_vault_closeout_2026_10_07.md](project_bank_vault_closeout_2026_10_07.md) — BV-10 release status verified and Banker entity-id reuse guard added; owner UAT remains pending

- [project_launcher_summary_observability_2026_10_04.md](project_launcher_summary_observability_2026_10_04.md) — 2026-10-04 consented launcher summaries: anonymous strict ingest (12/min per address), public login-port mount approved, inert (no endpoint); production endpoint is a later rollout
- [project_mac_client_cooked_version_zero.md](project_mac_client_cooked_version_zero.md) — under Wine the client sent cooked version 0 at every login: Wine's msvcp80 `strstreambuf::underflow` returns EOF after a write; repaired in-process by the patches DLL (verified live 2026-10-05), which also made character creation work
- [project_mercury_tx_hole_size.md](project_mercury_tx_hole_size.md) — >1472 B reliable datagrams wedge the client: uncapped ACKs (fixed 2026-10-03), data-sized single sends e.g. NPC cascade (fragmented 2026-10-05)
- [project_cellblock_autoplay_campaign.md](project_cellblock_autoplay_campaign.md) — Cellblock autoplay planned 2026-09-29 (docs/analysis/cellblock-autoplay/); nothing built; installed labd predates #1099-#1102
- [project_invisible_cellblock_guard.md](project_invisible_cellblock_guard.md) — Cellblock NID guard often unrendered on a fresh character though the client creates it; evidence, ruled-out causes (#838)
- [project_enemy_combat_runtime_blockers.md](project_enemy_combat_runtime_blockers.md) — enemy-combat v3 handoff not imported; blockers: MITIGATION 0/0, forced DT_PHYSICAL, EF_DONT_USE_QR, DoT death

## Closed campaigns

- [project_named_telemetry_campaign.md](project_named_telemetry_campaign.md) — Named telemetry closed 2026-10-05: IDs paired with names, unpaired 7,039 to 0, 795 nt:id-only marks; not yet read in a live SigNoz session

## External handoffs reviewed, not imported
- [project_class_start_v5_handoff.md](project_class_start_v5_handoff.md) — 2026-10-05 class-start/gear/abilities v5 handoff: reverses D-SA1 + M687 two-way split, wrong mission names, needs GrantAbility action/start-level/Asgard ship world; not imported
- [project_pass21_implementation_package.md](project_pass21_implementation_package.md) — 2026-10-03 Pass 21 package (86 Beta Site E2/Dakara/SGC W2 missions): duplicates the seed exactly apart from whitespace/mojibake; mission→dialog pairing is moniker-derived, not proven
- [project_lore_vo_handoff.md](project_lore_vo_handoff.md) — 2026-10-03 Castle lore VO handoff: no audio included; seed claims verified (EventSet 282 → dialog_castle/lore/*); playback needs dialog_castle.fev + client patch

- [project_final_re_bundles.md](project_final_re_bundles.md) — 2026-10-02 SGW final RE ZIPs: useful build/world indexes, absent databases and raw sources, evidence boundaries
- [project_character_editor_handoff.md](project_character_editor_handoff.md) — block CharDefs 9/10/19; no server allowlist exists; live-DB fixture uses CharDef 9; WQHD layout not available
- [project_shadow_lighting_handoff.md](project_shadow_lighting_handoff.md) — client-only render freeze; shadow resolution (capped 1014, #836) is not the lever; next: ApplyToPawn light environment
- [project_texture_upscale_forensics.md](project_texture_upscale_forensics.md) — 512x256 10-mip DXT1 shape is stock-valid; silent failures mean bad data below mip 0; pixel-format enum fixed in #839

## Reference

- [reference_lab_cli_powershell_traps_2026_10_10.md](reference_lab_cli_powershell_traps_2026_10_10.md) — lab CLI traps: one-element unroll before splat, StrictMode $LASTEXITCODE, REG_SZ PATH, squash vs ancestor, -Value:-x, --retire cwd

- [reference_first_session_lab_calibration_2026_10_10.md](reference_first_session_lab_calibration_2026_10_10.md) — first-session Praxis rows calibrated live (16 s movie hold, pitch +200/yaw +230 floor clicks, tutorial X before dialog); Haiku runs them in one `lab_uat_run` call; never lend Haiku a lease
- [reference_lab_parallel_clients_2026_10_10.md](reference_lab_parallel_clients_2026_10_10.md) — up to five lab clients in one daemon, a lease per instance; per-instance USERPROFILE fixes the cache-lock freeze; unrouted calls hit the first instance; LP-07 live check pending
- [reference_cellblock_asset_audit.md](reference_cellblock_asset_audit.md) — 2026-10-06 static audit: map count correction, existing exit/cinematic wiring, parsed asset candidates, extraction coverage gaps; full restoration/UAT campaign indexed under docs/analysis/cellblock-asset-audit/
- [reference_lab_mcp_token_cost_2026_10_10.md](reference_lab_mcp_token_cost_2026_10_10.md) — lab-driver tokens are mostly fixed context (33.9k of 37.3k before any result); images as paths; labd restart needs /mcp; windowed 1280x720 launch; native camera pitch clamp
- [reference_quick_xml_042_migration.md](reference_quick_xml_042_migration.md) — 2026-10-07 quick-xml 0.42 string API, escaped dialog attributes, and GUI check boundary
- [reference_client_action_bar_events.md](reference_client_action_bar_events.md) — stock action bar Lua (2026-10-05): hidden windows are deaf without `DeafWhenHidden=False`, subscribe takes several handlers, bandolier-bound buttons, `InventoryUpdateContainerActiveSlot`; from patch 015
- [reference_custom_debug_map_editor_2026_10_06.md](reference_custom_debug_map_editor_2026_10_06.md) — Ghidra confirms New Level, BigWorld chunk save, map-thumbnail and cover build paths; editor-to-game package load remains untested; prior wizard label corrected
- [reference_debug_area_map_selection_2026_10_06.md](reference_debug_area_map_selection_2026_10_06.md) — current QA/nav extraction and station layout for Castle, Tollana, Dakara_E1 and Agnos; Castle remains first pending client-memory and ring/clearance UAT
- [reference_physxloader_local_core.md](reference_physxloader_local_core.md) — bundled PhysXLoader uses bundled PhysXCore iff HKLM `enableLocalPhysXCore` == last adapter MAC (or `"AGEIA\0"` if GetAdaptersInfo fails); registry-only PhysX fix for #1121/#1150

- [reference_desktop_launcher_windows_native.md](reference_desktop_launcher_windows_native.md) — first native Windows run (2026-10-04): release key for dev builds, staged resources, verbatim-path Play failure, #1194 dialog, lock and socket test traps
- [reference_desktop_game_telemetry_2026_10_04.md](reference_desktop_game_telemetry_2026_10_04.md) — desktop launcher game telemetry (PR #1241) proven under Wine; plan-digest trap; rosettax87 breaks launch supervision; engine tests run on Linux
- [Native updater handoff](../updater-handoff-fix/reference_native_handoff.md) — shutdown follows successful spawn even if later persistence fails; fixture regression and platform limits.

- [reference_current_release_identity_2026_10_04.md](reference_current_release_identity_2026_10_04.md) — immutable owner with separate signed current-release reference; Update publication remains required.
- [reference_loopback_exporter_test_seams_2026_10_04.md](reference_loopback_exporter_test_seams_2026_10_04.md) — WSL2 mirrored networking hangs on a closed 127.0.0.1 port (use `[::1]:9`); wiremock responder and pooled-server traps; engine reqwest has no `json`

- [reference_desktop_engine_test_seams_2026_10_04.md](reference_desktop_engine_test_seams_2026_10_04.md) — desktop engine suite runs on Linux/WSL through the lane; launch, install-worker and uninstall test seams; `FileJournal::commit` sees every journal write

- [reference_owner_lock_release_2026_10_04.md](reference_owner_lock_release_2026_10_04.md) — explicit unlock at Repair/prefix logical-owner drop; duplicate-handle regression and native CI boundary.

- [reference_archive_preflight_2026_10_04.md](reference_archive_preflight_2026_10_04.md) — RAR/FDI name inventory before output; rebuilt Windows helper and spanning-cabinet validation required.

- [reference_launcher_repair_ui.md](reference_launcher_repair_ui.md) — Settings Repair retains preparation-to-commit ownership; native-persistence Effect UAT covers cancellation/reopen/abandonment, with real-client, Windows and visual gates open.

- [reference_launcher_implementation_plan.md](reference_launcher_implementation_plan.md) — 2026-10-04 Tauri/Effect settings shell and persistent native state; game workers and summary export pending; self-contained startup gate last

- [reference_launcher_platform_options.md](reference_launcher_platform_options.md) — 2026-10-03 proposed egui/wgpu versus Tauri versus SwiftUI evaluation; no accepted architecture or performance benchmark; scoped Mac packaging proof later authorized

- [reference_macos_wgl_forward_compat.md](reference_macos_wgl_forward_compat.md) — 2026-10-03: WoWSilicon launcher WGL rejection is missing forward-compatible flag; CX_FWD_COMPAT_GL_CTX=1 opens launcher and manifest; game UAT pending

- [reference_npc_costume_components.md](reference_npc_costume_components.md) — NPC components must be BodyComponent exports (query-index scan); Ra kit + NPC_Ra_Head_00/RaG fingernail = placeholder cube; female Goa'uld armour is Anat's kit only
- [reference_ihpet_crater_light_render_gaps.md](reference_ihpet_crater_light_render_gaps.md) — world 1300/73 map: terrain draws white, north palace terrace white/magenta with grey void east of it; put showcase NPCs on paving
- [reference_lab_local_server.md](reference_lab_local_server.md) — lab client on a local branch server: temp Local row in LoginInternal.lua, shard 'Test', seeded lab account, chat-focus and teleport gotchas
- [reference_colo_docker_layout.md](reference_colo_docker_layout.md) — colo = two compose projects on signoz-net, mirrored in docker/ (2026-10-04); watchtower ignores compose edits; s6 cont-init can't gate the server; uid 1001; player telemetry via :8081
- [reference_agent_board.md](reference_agent_board.md) — board.cimmeria.app (2026-10-04): per-agent accounts and keys, broker for campaigns; colo /24 is Spamhaus-SBL-listed so mail goes via Graph; some Azure storage clusters unreachable from the colo
- [reference_lab_client_controls.md](reference_lab_client_controls.md) — lab driving: Q/E rotate, B bag, Tab target; mouse-look broken (no DI button); .gotoxyz moves the selected target; inventory reader fallback

- [reference_auto_cycle_client_telemetry_2026_10_03.md](reference_auto_cycle_client_telemetry_2026_10_03.md) — AutoAttack.lua icon vs T binding, Sep 29 target-0 clears; #1144 colo UAT passed 2026-10-03; auto-cycle no longer persisted (owner: always off on login, #1148)
- [reference_castle_signoz_dossier_2026_10_02.md](reference_castle_signoz_dossier_2026_10_02.md) — full CellBlock→Castle→Harset dev run, partial colo Castle run, three gate crossings, dossier coverage limits; detailed report in docs/analysis/playtests/
- [reference_mercury_selective_acks.md](reference_mercury_selective_acks.md) — client ACKs are per-packet incl. buffered-behind-gap; old cumulative drain wedged reliable streams (fixed 2026-09-27); `mercury.tx_hole` diagnoses a stuck client
- [reference_mercury_selective_acks.md](reference_mercury_selective_acks.md) — client ACKs are per-packet, incl. buffered-behind-gap; cumulative drain wedged streams (fixed 2026-09-27); `mercury.tx_hole` finds a stuck client
- [reference_map_arrival_points.md](reference_map_arrival_points.md) — cooked maps: PlayerStart only in Agnos/Beta Site/Tollana, no stargate/teleporter actors; bad gate rows (Agnos fixed, Menfa_Light 192 m under mesh)
- [reference_client_ui_lua_overlay_testing.md](reference_client_ui_lua_overlay_testing.md) — client UI Lua is ASCII+CRLF, Debug:log logger, window-global naming, stock BlackMarket_ErrorText missing, Lua 5.1 UAT via lupa
- [reference_client_patch_delivery.md](reference_client_patch_delivery.md) — server picked by LoginInternal.lua (not .rdata), ASLR = PE byte 0x186, patch sets are deltas vs stock; what ships
- [reference_client_rar_and_cache_tiers.md](reference_client_rar_and_cache_tiers.md) — archive.org client RAR is the 2009 installer's MakeCAB set (hashes, layout); Cache.en-US (writable, pushed) vs SourceCache.en-us (bundled) tiers
- [reference_signoz_log_mining.md](reference_signoz_log_mining.md) — SigNoz MCP: condense oversized results, filters and keys that work, session anchors, what is normal
- [reference_client_idle_send_cadence.md](reference_client_idle_send_cadence.md) — idle client sends ~6 pkt/s, perfStats every 15 s; NetInactivityTimeout=15 is client-side; read before changing timeouts

- [reference_desktop_updater_parity_research.md](reference_desktop_updater_parity_research.md) — legacy checksum updater and Tauri signed-package ownership/recovery differences.

- [reference_launcher_uat_integration_2026_10_04.md](reference_launcher_uat_integration_2026_10_04.md) — adoption UI + effective settings + Wine identity combined: checkpoint lifetime, helper eligibility, UAT bridge selection, hidden-ancestor regression and partial native evidence.
- [reference_launcher_minimum_poll_2026_10_04.md](reference_launcher_minimum_poll_2026_10_04.md) — signed minimum state must survive automatic Play inspection.
- [reference_launcher_updater_integration_2026_10_04.md](reference_launcher_updater_integration_2026_10_04.md) — early mutation gates and updater revision refresh across other operations.
- [reference_adoption_contract_audit_2026_10_04.md](reference_adoption_contract_audit_2026_10_04.md) — verified separate-copy adoption and effective settings remain distinct from settings import.
- [reference_launcher_game_update_review.md](reference_launcher_game_update_review.md) — native signed game offers, stale-review invalidation and actual store-reopen Effect UAT; Apply UI integration remains pending.

- [reference_custom_debug_map.md](reference_custom_debug_map.md) — purpose-built Debug Map means newly authored geometry, not a stock-map alias; native editor commands are statically confirmed, but editor-save to clean-game load is the CM-00 gate
