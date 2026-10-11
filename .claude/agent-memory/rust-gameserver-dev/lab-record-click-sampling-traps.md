---
name: lab-record-click-sampling-traps
description: lab record draft tests and tick ordering - parsed specs resolve @aliases, toml writes newline strings as """, a tick pumps clicks before tap and UI reads
metadata:
  type: project
---

Facts found in LR-11 (2026-10-11), `crates/lab/src/uat/record/`:

- `spec::parse` resolves a step's `@alias` to its tool (`@window_click` parses
  as `client_window_click`), so a test over parsed `ActionSpec`s must compare
  the resolved name; text checks on the draft see the alias.
- `Inline::Str(..).render()` goes through `toml::Value::to_string`, which writes
  a string containing a newline as a `"""` multi-line string. It still parses,
  but the draft stops being one step per line; LR-11 makes a click with a
  control character in its name a TODO instead.
- A sampler tick runs the pump (which reads `ui.click` into the event store)
  before the tap drain and the UI read, so a click and its call or window
  change can land one tick apart in either direction. Pair them over the
  click's tick and its neighbours, not the same tick.

**Why:** each of these made a first-draft test or rule wrong.
**How to apply:** recorder transform work (LR-13 onwards, `.bug spec`); see
[[lab-record-emit-rules]] if one is written.
