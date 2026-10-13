# Cellblock route recording, 2026-10-13

Source: AP-11 in `docs/analysis/cellblock-autoplay/work-packets.md`; implementation in `crates/lab/src/supervisor/world/routes.rs`.

The route recorder samples client player positions every 500 ms and emits a candidate TOML leg. It keeps start/end, 20-degree bends and 8 m spans. The committed catalog has no measured legs yet: a usable game display and two native replays per leg are still required. Preparation and topside are separate navmesh components joined by rings, so they must be separate legs.
