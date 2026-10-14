# Lab UAT capture and id sources (2026-10-13)

- `crates/lab-mcp/src/tools/sessions.rs` includes `player_id` in `server_sessions` only when `OnlinePlayer.player_id` is set; the session is matched by character name in `crates/lab/src/uat/runner/packet.rs`.
- `crates/lab/src/supervisor/ui/player_state.rs` does not expose the player's entity id. `client_entity_find` does expose it at `/player/id` (`crates/lab/src/supervisor/world/find.rs`). AP-03's runtime `${entity_id}` lookup tries `client_player_state` first and uses that fallback.
- The AP-03 spec syntax and grading rules are documented in `docs/guides/automated-uat.md`.
