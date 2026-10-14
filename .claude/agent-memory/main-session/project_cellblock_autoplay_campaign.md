---
name: project_cellblock_autoplay_campaign
description: Cellblock autoplay campaign — plan and packets in docs/analysis/cellblock-autoplay/; AP-02 runner campaign lifecycle
metadata:
  type: project
---

Campaign to drive the whole Castle Cellblock tutorial from the live research lab as one repeatable `lab_uat_run` section. Planned 2026-09-29 at `d8bff1baa`: [README](../../../docs/analysis/cellblock-autoplay/README.md) (goal, decisions D-AP1..11, ledger), [work-packets.md](../../../docs/analysis/cellblock-autoplay/work-packets.md), [scenario-map.md](../../../docs/analysis/cellblock-autoplay/scenario-map.md), [livewire-autosolve.md](../../../docs/analysis/cellblock-autoplay/livewire-autosolve.md).

Facts found in the research that matter before any live work:

- The labd under `%LOCALAPPDATA%\cimmeria-lab\bin` was built at or before #1094: no world, combat, UI or UAT tools. AP-00 adds an install script.
- `client_move_to` is straight-line steering, not pathing; the Cellblock needs recorded routes (AP-11).
- Livewire is won by clicking goal wires; the lab needs the server's board plus a hit map from the client's `Livewire.upk` (MG-L0..L5).
- Cover in the Cellblock is server-only proximity (5 m of a cover node, 1 Hz); there is no client key.

**How to apply:** resume from the README ledger; all D-AP rows were PROPOSED, awaiting the owner, when planned.

AP-02 runner facts (2026-10-13): `RunManifest.fresh_characters` persists each fresh section's chosen name and archetype before character creation, so resuming after a successful create adopts the listed character. It has a serde default for older bundles. The section id is mixed into the generated last name, with collision resolution at manifest creation; a saved name wins on resume. `sequential = true` blocks rows after an earlier FAIL/BLOCKED, while row `depends_on` can name earlier rows explicitly. NEEDS_HUMAN, UNVERIFIED and NATIVE_SHORTFALL keep the chain moving. The runner asks `lab_characters` at character select before deciding whether to create; `finish_intro = false` keeps the intro dialog open after creation. Source: `crates/lab/src/uat/runner/{mod,session}.rs` and `campaign_tests.rs`.

Concurrent runner IDs use time plus six random letters; `RunDir::create_exclusive` atomically reserves each bundle path, and the full ID remains in fresh section names. This fixes decisecond collisions between parallel lab lanes. A resumed old bundle without `fresh_characters` adopts the prior ten-letter run-id name. Source: `crates/lab/src/uat/runner/{mod,campaign,session}.rs`, `crates/lab/src/uat/evidence.rs`, `crates/lab/src/uat/runner/campaign_tests.rs`.

AP-02 live check (2026-10-14, local server): `lab_play_character` reported `dialog_open: false` when the world HUD first appeared, but the fresh character's intro dialog appeared before the next row. A resumed row then timed out trying to log out under that dialog. The runner now waits briefly for the dialog when `finish_intro = true` and finishes it before starting the row; `finish_intro = false` still leaves it open. Source: `crates/lab/src/uat/runner/session.rs` and `campaign_tests.rs`.
