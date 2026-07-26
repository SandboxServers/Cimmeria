---
name: clippy-items-after-test-module
description: Workspace clippy -D warnings rejects any item declared after a #[cfg(test)] mod — the test module must be last in the file
metadata:
  type: project
---

`clippy::items_after_test_module` is enabled (via workspace `-D warnings`), so
a `#[cfg(test)] mod tests { .. }` block **must be the last item in the file**.

**Why:** Many repos tolerate a test module in the middle of a file; this one
fails the blocking clippy job for it. It compiles and tests pass — only clippy
catches it, so it's easy to miss until CI.

**How to apply:** When adding tests to an existing module, append the block at
EOF even if the code under test lives at the top. Free functions that trail the
`impl` block (helpers like `collect_package_files`) must stay above it.
