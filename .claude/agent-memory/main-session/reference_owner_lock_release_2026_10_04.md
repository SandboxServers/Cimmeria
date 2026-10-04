# Explicit owner-lock release

Mac desktop CI run `37219800831` failed immediate re-lock in prerequisite prefix
ownership and Repair cleanup after raw File guard drop. The precise originating
extra handle was not captured. A deterministic duplicate-handle regression covers
the same lifetime hazard: closing one File does not release a shared open-file
description's lock while its duplicate survives.

`engine/src/owner_lock.rs` now explicitly unlocks at final logical-owner drop,
following the existing DesktopState Directory destructor. Prerequisite prefix
owner handles and Repair root/work handles use it. Held-owner exclusion remains
tested; the handles are not dropped or unlocked before native work completes.
The integrated local engine suite passes 363 tests with 18 ignored. Native Windows
and Mac CI remain the cross-platform verification gate.

Stress validation with 32 test threads exceeded the local soft descriptor limit
of 256. Raising the limit to 2048 for that command alone let the same suite pass;
no process-global or machine configuration was changed.
