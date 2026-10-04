# Main-Session Project Memory — Index

Dated, sourced facts from top-level sessions, committed with the change that produced them; verified facts graduate to `docs/`. Rules: [development-workflow.md § Project memory](../../../docs/agents/development-workflow.md#project-memory). One line per memory, newest first within a section.

## Open investigations

- [project_mercury_tx_hole_size.md](project_mercury_tx_hole_size.md) — 2026-09-29 stall on 1488-byte datagrams (client reads 1472); cause was uncapped piggybacked ACKs, capped 2026-10-03; send/recv fingerprint telemetry
- [project_cellblock_autoplay_campaign.md](project_cellblock_autoplay_campaign.md) — Cellblock autoplay planned 2026-09-29 (docs/analysis/cellblock-autoplay/); nothing built; installed labd predates #1099-#1102
- [project_invisible_cellblock_guard.md](project_invisible_cellblock_guard.md) — Cellblock NID guard often unrendered on a fresh character though the client creates it; evidence, ruled-out causes (#838)
- [project_enemy_combat_runtime_blockers.md](project_enemy_combat_runtime_blockers.md) — enemy-combat v3 handoff not imported; blockers: MITIGATION 0/0, forced DT_PHYSICAL, EF_DONT_USE_QR, DoT death

## External handoffs reviewed, not imported

- [project_final_re_bundles.md](project_final_re_bundles.md) — 2026-10-02 SGW final RE ZIPs: useful build/world indexes, absent databases and raw sources, evidence boundaries
- [project_character_editor_handoff.md](project_character_editor_handoff.md) — block CharDefs 9/10/19; no server allowlist exists; live-DB fixture uses CharDef 9; WQHD layout not available
- [project_shadow_lighting_handoff.md](project_shadow_lighting_handoff.md) — client-only render freeze; shadow resolution (capped 1014, #836) is not the lever; next: ApplyToPawn light environment
- [project_texture_upscale_forensics.md](project_texture_upscale_forensics.md) — 512x256 10-mip DXT1 shape is stock-valid; silent failures mean bad data below mip 0; pixel-format enum fixed in #839

## Reference

- [Native updater handoff](../updater-handoff-fix/reference_native_handoff.md) — shutdown follows successful spawn even if later persistence fails; fixture regression and platform limits.

- [reference_current_release_identity_2026_10_04.md](reference_current_release_identity_2026_10_04.md) — immutable owner with separate signed current-release reference; Update publication remains required.

- [reference_owner_lock_release_2026_10_04.md](reference_owner_lock_release_2026_10_04.md) — explicit unlock at Repair/prefix logical-owner drop; duplicate-handle regression and native CI boundary.

- [reference_archive_preflight_2026_10_04.md](reference_archive_preflight_2026_10_04.md) — RAR/FDI name inventory before output; rebuilt Windows helper and spanning-cabinet validation required.

- [reference_launcher_repair_ui.md](reference_launcher_repair_ui.md) — Settings Repair retains preparation-to-commit ownership; native-persistence Effect UAT covers cancellation/reopen/abandonment, with real-client, Windows and visual gates open.

- [reference_launcher_implementation_plan.md](reference_launcher_implementation_plan.md) — 2026-10-04 Tauri/Effect settings shell and persistent native state; game workers and summary export pending; self-contained startup gate last

- [reference_launcher_platform_options.md](reference_launcher_platform_options.md) — 2026-10-03 proposed egui/wgpu versus Tauri versus SwiftUI evaluation; no accepted architecture or performance benchmark; scoped Mac packaging proof later authorized

- [reference_macos_wgl_forward_compat.md](reference_macos_wgl_forward_compat.md) — 2026-10-03: WoWSilicon launcher WGL rejection is missing forward-compatible flag; CX_FWD_COMPAT_GL_CTX=1 opens launcher and manifest; game UAT pending

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

- [reference_launcher_minimum_poll_2026_10_04.md](reference_launcher_minimum_poll_2026_10_04.md) — signed minimum state must survive automatic Play inspection.
- [reference_launcher_updater_integration_2026_10_04.md](reference_launcher_updater_integration_2026_10_04.md) — early mutation gates and updater revision refresh across other operations.
- [reference_adoption_contract_audit_2026_10_04.md](reference_adoption_contract_audit_2026_10_04.md) — verified separate-copy adoption and effective settings remain distinct from settings import.
- [reference_launcher_game_update_review.md](reference_launcher_game_update_review.md) — native signed game offers, stale-review invalidation and actual store-reopen Effect UAT; Apply UI integration remains pending.
