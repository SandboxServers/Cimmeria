---
name: Desktop repair ownership
description: Retained repair handoff and native-persistence UAT boundaries
type: project
---

# Desktop repair UI ownership and evidence

2026-10-04. The Settings Repair journey uses the existing install IPC and Effect
workflow. `shell/src/host/repair/` retains preparation's cancellation/progress
handle and transfers its result receiver into a task that passes `Prepared`
directly into commit. Never release the prepared root/work/prefix ownership
between these phases. Lost webview replies do not cancel or replay native work.

Repair identity comes from saved installed content, independent of the selected
preferences directory and missing game files. Avoid opening the installed owner
marker while preparation owns its lock: active status returns progress without
reverifying that marker. Reconciliation actions still validate native evidence.

`frontend/repair-uat.mjs` drives actual Effect/view code against native durable
admission, cancellation, reopen and abandonment. Its test-only worker seam holds
preparation before reconstruction; it does not prove real Wine or replacement.
The shell tests and engine repair fault matrix are separate evidence. Windows
native and packaged visual/focus validation remain required. Source: the
[repair contract](../../../crates/launcher/desktop/docs/repair.md).

## 2026-10-04 review-fix evidence

- Successful Repair is historical outcome, not current backup evidence. Engine
  `repair::cleanup::status` validates the current plan/checkpoint/role/cleanup
  record. Settings must hide deletion after cleanup and for missing-content
  repairs, while retaining resumable partial cleanup.
- Shell `host/repair/integration_tests.rs` exercises real production dispatch and
  the retained preparation-to-commit coordinator with a shared signed inert ZIP.
  A dev-only engine test-support seam changes transport/platform adapters, not
  durable admission or coordinator logic. Native Windows/Wine and packaged UI
  remain separate evidence gates. Details: `desktop/docs/repair.md` under
  `crates/launcher/`; packet evidence is in `repair-review-fixes.md`.
