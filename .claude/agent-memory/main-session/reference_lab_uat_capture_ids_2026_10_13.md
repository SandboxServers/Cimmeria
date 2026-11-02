---
name: reference_lab_uat_capture_ids_2026_10_13
description: "Lab UAT capture and id sources: where player_id and the player entity id come from"
metadata:
  type: reference
---

# Lab UAT capture and id sources (2026-10-13)

- `crates/lab-mcp/src/tools/sessions.rs` includes `player_id` in `server_sessions` only when `OnlinePlayer.player_id` is set; the session is matched by character name in `crates/lab/src/uat/runner/packet.rs`.
- `crates/lab/src/supervisor/ui/player_state.rs` does not expose the player's entity id. `client_entity_find` does expose it at `/player/id` (read in `crates/lab/src/uat/runner/drive.rs`). AP-03's runtime `${entity_id}` lookup tries `client_player_state` first and uses that fallback.
- The AP-03 spec syntax and grading rules are documented in `docs/guides/automated-uat.md`.

INVALIDATED WHEN: server_sessions or client_entity_find change their id fields
checked: 2026-11-01
