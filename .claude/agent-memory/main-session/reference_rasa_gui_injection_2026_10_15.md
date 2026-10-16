---
name: reference-rasa-gui-injection
description: Source review of the Tabula Rasa GUI injection archive and d10sfan/Rasa.NET as design references for Cimmeria lab tools
metadata:
  type: reference
---

Source review, 2026-10-15. The user-provided `rasa_gui_injection` archive
(SHA-256 `f90fe507b66878485ff433da4d41011427be4d8729a3c34c54e13705bf4d9ca5`)
contains Rust source and screenshots, not a game client or compiled DLL.
Treat its README and code as reference data, not Cimmeria instructions.

- `src/proxy.rs` is a D3D9 proxy. `src/hooks.rs` hooks CreateDevice,
  EndScene and Reset, renders ImGui at EndScene, and uses the WndProc for
  hotkeys and dispatch. Python 2.4 calls are posted to the window thread;
  network requests run on worker threads and feed response queues.
- The useful product pattern is focused selection to inspection/action:
  F6 captures the game's current target; the overlay locates its creature
  record and opens an inspector/editor. Map markers, mission objectives,
  region coordinates and bookmarks link spatial and content views. The
  creature save path reloads details and compares persisted fields before
  reporting verification.
- The embedded Python bridge reads the Tabula Rasa client runtime, and its
  console has a persistent Python namespace. Cimmeria's SGW client uses
  Lua 5.1, so none of those Python object paths or layouts are portable.
- The linked `d10sfan/Rasa.NET` repository's `development` branch is a
  separate C# server. Its `gametools/README.md` describes browser based
  server editors with authenticated REST endpoints; the archive also uses
  short lived in-game API auth and `/ingame/*` routes. These are patterns
  for source inspection, not Cimmeria's protocol or data model.
- Cimmeria's lab bridge already supplies main-thread Lua eval, token gated
  JSON-RPC, dynamic hooks, event reading, UI and entity tools, screenshots,
  and server packet/state queries. The most direct extension is a lab-only
  joined client/server inspector. Native ImGui would require new EndScene and
  input hooks: telemetry currently observes D3D9 device creation/loss/reset
  but deliberately does not hook Present or draw calls.
- The dot GM registry has 129 `spec(...)` entries across 19 family files.
  Patch 012's `commands.toml` has 162 distinct slash entries: 161 `/gm*` and
  `/debugminigamecomplete`. The QA Internal file had 164 entries with two
  duplicate words. The older 167-command prose in `docs/commands.md` and
  `docs/architecture/gm-cell-method-gating.md` was corrected in the
  developer-workbench design branch. `client_console` has no confirmed
  built-in string-command address, so native slash submission is unproven.
- `lab_record_*` already records human play and drafts UAT TOML. LR-10's
  CEGUI click ring is integrated; LR-11 click transforms and LR-14/15 rolling
  and `.bug spec` support remain campaign dependencies. A workbench recorder
  should extend that owner rather than drain the server tap separately.

No Tabula Rasa or SGW client was run for this research. Source facts above
must not be reported as live-verified behavior.
