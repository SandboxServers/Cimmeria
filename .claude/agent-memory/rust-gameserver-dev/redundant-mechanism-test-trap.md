---
name: redundant-mechanism-test-trap
description: When a feature has two redundant mechanisms, revert-verification must disable them one at a time or the guard passes for the wrong reason.
metadata:
  type: feedback
---

**Always revert-verify a regression guard by disabling ONE mechanism at a
time. If a feature is protected by two redundant mechanisms, a guard that
exercises both passes even when one is completely deleted.**

**Why:** hit concretely while building `ChildGuard` in
`crates/server-harness/`. It reaps a child process two independent ways —
`Drop` (kill + wait) and a Windows Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. The first reap tests looked correct and
passed. Deleting the **entire `Drop` impl** did not fail a single one: closing
the job handle killed the child anyway. The tests were measuring the property
("child dies") without isolating the mechanism they named in their assertion
messages ("Drop did not reap it").

The fix was to make the redundancy selectable — `OrphanProtection::{Kernel,
DropOnly}` — so the `Drop` layer can be tested alone, plus one test covering
the real both-layers configuration. `OrphanProtection` is a genuine runtime
option (job assignment can be refused by a restrictive container), not a
test-only affordance, which is what keeps it honest.

**How to apply:**

1. Never mark a guard "done" on a green run. Comment out the fix and confirm
   it goes red. TESTING.md requires this; it is easy to skip because the test
   already passes.
2. If it stays green, do **not** assume the test is fine — find the second
   mechanism covering for the first. Disable both to confirm the test can fail
   at all (proves non-vacuity), then add a seam that isolates each layer.
3. Watch for this shape wherever defense-in-depth exists: retry + timeout,
   cache invalidation + TTL, client-side guard + server-side validation, an
   `if` gate plus a downstream `WHERE` clause.
4. Assertion messages that name a specific mechanism ("Drop did not reap it")
   are a promise the test should actually be keeping — treat a mechanism-named
   message as a prompt to check that the mechanism is isolated.
