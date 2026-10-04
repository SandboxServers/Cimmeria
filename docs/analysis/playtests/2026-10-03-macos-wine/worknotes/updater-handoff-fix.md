# Updater handoff persistence regression

2026-10-04 · bounded follow-up to `updater-apply.md` · base `2827c464a`

The retained native Apply worker now receives an explicit successful-spawn
notification. It shuts down once even when the subsequent restart-state save
fails. Failed spawn never sends the notification; durable pre-spawn intent is
not treated as success. The engine still returns persistence errors, retains
reconciliation ownership, and rejects duplicate Apply. No renderer contract or
installation acknowledgment changed.

The regression crosses the real signed-artifact engine Apply and retained host
worker, using an inert Windows installer fixture with simulated spawn. It injects
failure into the next real atomic save both before and after file replacement,
checks shutdown exactly once, rejects duplicate replay, and verifies reopened
reconciliation ownership. The negative case verifies that failed spawn does not
shut down despite durable handoff intent. The existing lost-reply guard remains.

Actual packaged Mac
IPC/exit/restart and Windows installer/UAC/cancellation remain release gates; no
real application bundle or game was launched. This native-only fix changes no
frontend logic, so a new JS REPL pass is not applicable.

## Validation

- Engine/shell library and binary tests through the build lane: 412 passed,
  23 ignored (`20261004-125714-41044`). Ignored UAT bridges were not run here.
- Final focused updater host suite: 5 passed (`20261004-125847-43998`), including
  both fault positions, identical owner on reopen, failed spawn and lost reply.
- Temporarily restoring the pre-fix `apply(...)?` early return made the regression
  fail with shutdown count 0 instead of 1 (`20261004-125659-40879`). The fix was
  restored before the passing runs.
- Strict all-target engine/shell Clippy passed (`20261004-125759-41727`).
  Standalone workspace formatting and `git diff --check` passed.
- Frontend `npm ci` and `npm run build` passed to supply the embedded Tauri assets.
  No frontend code changed; no new JS REPL or visual UAT was run.

The test-only Minisign/base64 dependency declarations and lockfile are isolated
in integration commit `47bf3268`. No production dependencies were added.
