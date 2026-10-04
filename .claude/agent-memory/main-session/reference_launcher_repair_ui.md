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
