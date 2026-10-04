---
name: Desktop updater parity research
description: Legacy checksum trust differs from Tauri signed package verification and platform handoff.
type: reference
---

2026-10-04 source audit at `1193bc46d`: the legacy self-updater checks size and
GitHub-hosted SHA-256, not detached release signatures. Tauri updater 2.13.1
requires artifact signatures; its feed version needs signed-version binding to
prevent relabeling an old valid artifact. Windows installer launch and Mac bundle
replacement do not establish legacy rollback parity. See
`docs/analysis/playtests/2026-10-03-macos-wine/worknotes/updater-parity-research.md`
for pinned first-party sources, native command proposal and minimum-gate packet.
No runtime validation or publication was performed. Shared index registration is
left to the integrating coordinator to avoid parallel edits.
