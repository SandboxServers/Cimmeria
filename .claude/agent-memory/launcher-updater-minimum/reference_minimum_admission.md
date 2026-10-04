# Native minimum-launcher admission reference

2026-10-04: `desktop/engine/src/launcher_compatibility/` separates package identity
from legacy minimum ordering. `DesktopState::open` uses compiled stamps; native
`open_with_compatibility` injects trusted known ordering. No release discovery or
ordering persistence is implemented. Launch must use the read-only installed
identity view: `installed_content()` can migrate a legacy index and therefore
write before a gate rejects. Original signed cache bytes remain bound to the
installation digest and are reverified offline.

Verified contract and parity exemptions:
`docs/analysis/playtests/2026-10-03-macos-wine/worknotes/updater-minimum-admission.md`.
