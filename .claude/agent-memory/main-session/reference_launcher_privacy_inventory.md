---
name: Launcher privacy inventory and raw-stream boundaries
description: Collector coverage, separate consent, running DLL opt-out and raw log limits.
type: reference
---

2026-10-23: [Privacy inventory](../../../crates/launcher/desktop/docs/privacy-inventory.md)
owns the source-backed disclosure. Fixed launcher summaries lack persistent
sender IDs/arbitrary text; rich game diagnostics do not. Game identity retains
random install UUID after opt-out, machine_id is truncated SHA256(hostname).
Manual uploads are explicit consent even with toggles off and preserve ordinary
chat/character text after pattern scrubbing. Automatic exit ZIP is raw selected
log bytes, not manual-scrubbed. Live tail accepts session .log AND .txt; it has
no special *-keys.txt exclusion, although manual/ZIP .log discovery excludes it.
Never claim global no-key-file/no-chat/no-path guarantees.

Launcher queue consent generation stops future launcher writes/posts and purges
unsent data on change; already injected DLL keeps separate uploader/token and
only next-Play injection follows the checkbox. Exit game to stop it. HTTPS-only
is true of summaries, not every rich-event configuration (HTTP login-origin
exception). Retention is operator-configured, not a promise of the client.
Player releases reject lab-bridge builds; virtual camera/input observations
are distinct from webcam/global keyboard recording. No actual game or live
retention/telemetry was inspected for this audit.

Disclosure implementation: eight offline nested UI sections and eleven wire
field inventories. After this extension, 294 frontend tests, TypeScript/build,
actual-DOM persistent JS REPL and 1280x720 fixture visual review passed. Removing
event_id failed the DTO coverage assertion; restored thirteen privacy checks
passed. Prior native total remains 1,186 passed/13 ignored with no native
collector change. Corrected-head tests/build/REPL/native asset rebuild and fresh final review ship passed; final-head CI pending
at this checkpoint. Privacy UI verification is not collector/upload/native
persistence/retention or packaged-game acceptance.

Source-review corrections: protocol observations can include remote IPv4 game
peer/port; no-receiving-peer logging is not a no-address-in-payload guarantee.
Summary destination is only the first effective login-server entry if HTTPS,
not the first HTTPS entry found by searching the list. No fallback configured.
Native asset build passed before these wording corrections; final corrected-head
checks/rebuild/review/CI remain pending at this note.
