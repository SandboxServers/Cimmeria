---
name: lab-flow-first-live-run-findings
description: First live lab smoke run (colo, 2026-10-04) - bridge dispatch timeouts after Create, SelfStatusWin is the only HUD test, use_ability outcome must read ability.* rows, place needs include_empty
metadata:
  type: project
---

Facts from the first live smoke run of the `cimmeria-lab` tools (2026-10-04), fixed in the lab-fixes PR:

- Right after character Create the client main thread is busy; a bridge `lua_eval` can return `bridge error -32603: dispatch timeout` (the DLL's 5 s `RESPONSE_TIMEOUT`) although the create succeeded. Flow waits must treat `dispatch timeout` / `dispatch queue full` as transient (`flows/create_confirm.rs::transient_bridge_error`), never as a failure. `poll_until` still ends on any bridge error - other flows may need the same tolerance.
- A new character's Bink arrival cutscene is NOT `MoviePlayerWin`. `DialogWin` and `MinimapWin` read visible under it; only `SelfStatusWin` proves the HUD. Escape until it shows.
- `client_use_ability` outcome: the DLL mirrors `client.ability.*` to the lab ring as `ability.*` (kind = target minus `client.`; the sub-kind is the `kind` FIELD, e.g. `ability.applied` + `kind: cooldown|stat`). A self-heal shows only timers + a stat apply, no `cme.event` effect names.
- A fresh character's hotbar is 100 registered EMPTY buttons; the bound-only hotbar read (`include_empty=false`) returns zero buttons.

**Why:** these were invisible to fake-bridge tests; each looked fine until a real client ran.
**How to apply:** when touching lab flows, assume the client can be busy for >5 s after any screen transition, and pin new fake-bridge fixtures to these live shapes. See [[lab-uat-runner-in-process-tool-calls]].
