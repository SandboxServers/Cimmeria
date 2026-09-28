# Main-Session Project Memory — Index

Project and reference facts that top-level sessions (not subagents) learned while researching or writing code. They're committed with the change that produced them. The bar is "dated and sourced", lower than `docs/`; verified facts graduate to `docs/`. Rules for what goes here and what stays in personal memory: [development-workflow.md § Project memory](../../../docs/agents/development-workflow.md#project-memory). One line per memory, newest context first within a section.

## Open investigations

- [project_invisible_cellblock_guard.md](project_invisible_cellblock_guard.md) — Cellblock NID guard often never rendered on a fresh character though the client creates it; SigNoz evidence, what is ruled out, dropped requestEntityUpdate (#838)
- [project_enemy_combat_runtime_blockers.md](project_enemy_combat_runtime_blockers.md) — enemy-combat v3 handoff reviewed not imported; #819/#822 landed; MITIGATION 0/0, forced DT_PHYSICAL, EF_DONT_USE_QR, DoT death still block it

## External handoffs reviewed, not imported

- [project_character_editor_handoff.md](project_character_editor_handoff.md) — block CharDefs 9/10/19; no server allowlist exists; live-DB fixture uses CharDef 9; WQHD layout not available
- [project_shadow_lighting_handoff.md](project_shadow_lighting_handoff.md) — client-only render freeze; shadow resolution capped at 1014 (PR #836) and not the lever; next target ApplyToPawn light environment
- [project_texture_upscale_forensics.md](project_texture_upscale_forensics.md) — 512x256 10-mip DXT1 shape is stock-valid; silent failures mean bad data below mip 0; pixel-format enum off by two (#839)

## Reference

- [reference_client_ui_lua_overlay_testing.md](reference_client_ui_lua_overlay_testing.md) — client UI Lua is ASCII+CRLF, Debug:log logger, window-global naming, stock BlackMarket_ErrorText missing, Lua 5.1 UAT via lupa
- [reference_signoz_log_mining.md](reference_signoz_log_mining.md) — SigNoz MCP: condense oversized results, filters and keys that work, session anchors, what is normal
- [reference_client_idle_send_cadence.md](reference_client_idle_send_cadence.md) — idle client sends ~6 pkt/s and perfStats every 15 s; NetInactivityTimeout=15 is client-side; read before touching inactivity timeouts
