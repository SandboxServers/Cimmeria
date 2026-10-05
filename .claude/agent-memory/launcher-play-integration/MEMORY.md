# Play integration reference

- 2026-10-04: Desktop `launch_command` accepts identity/revision only; shell bundle resources require build-pinned helper and client patches, plus Mac D3D9. Optional x87 uses adjacent `rosettax87` and `libRuntimeRosettax87`. See `crates/launcher/desktop/docs/launch.md`.
- Native launch worker outlives its UI and records durable process observations. Reopened unfinished Launch remains unknown/reconciliation-required; process start is not login proof. The headless launch UAT exercises actual Effect and native persistence with inert lifecycle inputs, not Windows/Wine execution.
