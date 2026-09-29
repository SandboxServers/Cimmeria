---
name: lab-event-store-and-ui-lua-hooks
description: cimmeria-lab event ring is drain-and-clear (supervisor store is the only drainer); how to hook stock UI Lua handlers safely; /gmsethealth 0 does not kill
metadata:
  type: project
---

Facts from building the lab combat tools (2026-09-29, branch feat/lab-combat-tools):

- The bridge's `events_read` ring is **drain-and-clear**. Any new reader must go through
  `supervisor::events` (`pump_events` + a named cursor on the store), never call
  `events_read` directly, or it steals events from `client_wait_event` and `client_events_read`.
- Stock UI handlers are subscribed **by name** (`SCTWin:subscribe(Events.UnitCombat,
  'SCTMod.onUnitCombat')`), and the client allows **one subscription per window per event**.
  Wrap the global (chain to the original), then re-subscribe the *same* event with the same
  name; never touch another event on that window (the world tools own `SCTWin`'s PreRender).
- Lua `unit*` functions take unit *slots* (`Unit.Target`, ...), not entity ids.
- `/gmsethealth 0 0` only writes the stat (cell-console `gm/stats.rs`); it does not run the
  death sequence, so the defeat window (`onBeginAidWait`) needs a real lethal hit.
- Flows that post input need the game HWND; call `input_focus` lazily right before the first
  key/click so the input-free paths are testable against `events::fake_bridge`.

**Why:** these cost a design round each; the drain race would silently lose events.
**How to apply:** any new lab tool that reads client events or hooks UI Lua.
