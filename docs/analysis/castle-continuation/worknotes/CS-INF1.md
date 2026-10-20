# CS-INF1 worknote: `MINIGAME_PORT` env override

> **Type:** Worknote. **Packet:** CS-INF1. **Branch:** `castle/cs-inf1-minigame-port`. **Base:** `c481008904ba39c80168424f67d96b9b271b9ac9` (feature tip).

## Change

- `crates/server/src/main.rs`: `config_from_env` reads `MINIGAME_PORT` after `MINIGAME_HOST`. A value that parses as a port sets `cfg.minigame_port`; any other value logs a WARN and keeps the default `30000`. The header table gains a row for it, and the unit test `minigame_port_env_overrides_default` covers both cases.
- `docs/operations/container.md`: a row for `MINIGAME_PORT` in the "Not baked into the image" table, saying to publish the same port.

No client patch. `config.minigame_port` already reaches the listener and the client advertisement.

## Lane checks

- `cargo fmt -p cimmeria-server -- --check`: `status=ok exit=0 ran=0.583s`
- `cargo clippy -p cimmeria-server --all-targets -- -D warnings`: `status=ok exit=0 ran=56.625s`
- `cargo nextest run -p cimmeria-server -E 'test(minigame_port)'`: `status=ok exit=0 ran=70.272s`, `tests (nextest): 1 passed, 128 skipped`
- `cargo fmt -p cimmeria-server` (final): `status=ok exit=0 ran=0.479s`, no changes

`crates/server/src/main.rs` is 533 lines, under the 560 limit set for this packet.

## Whitespace

Lane fmt, whole workspace: `cargo fmt --all -- --check` (through the lane): `status=ok exit=0 ran=10.076s`, no diff.

`git diff --check c481008904ba39c80168424f67d96b9b271b9ac9 HEAD`: exit 0, no output. Clean.
