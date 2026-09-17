---
name: stale-branch-clippy-toolchain-drift
description: CI clippy floats to current stable with no toolchain pin, so any idle branch (esp. dependabot) can fail clippy on a brand-new lint unrelated to its own diff — rebase/update-branch before investigating.
type: project
---

Cimmeria CI uses `dtolnay/rust-toolchain@stable` at every job in `.github/workflows/test.yml`, and
the repo has **no `rust-toolchain.toml`**. Clippy therefore runs against whatever stable is current
*at run time*, not a pinned version.

**Consequence:** a branch whose last CI run is more than a few weeks old can show
`cargo clippy -D warnings` FAILING for a lint that did not exist when the branch was cut, in files
the branch never touched. The failure is real but belongs to `main`, not the branch.

**Why it matters:** this reads exactly like "the dependency bump broke clippy" on a dependabot PR,
and it is easy to burn a lot of time bisecting crates that are innocent.

**How to apply — triage order for any stale red CI:**

1. Read the failing log and note **which files** the lint fires in. If they are outside the PR's
   diff (`gh pr diff <N> --name-only`), it is almost certainly toolchain drift, not the PR.
2. `git log --oneline origin/main -- <those files>` — look for an already-landed fix.
3. Trigger the "update branch" API (merges `main` in) and re-run CI before doing any analysis.

Worked example: PR #634 (cargo-patch-and-minor group, 14 crates) showed clippy failing on
`clippy::slice_as_chunks` ("using `chunks_exact` with a constant chunk size") in
`crates/upk/src/{properties,reader}.rs`. The PR's diff was **`Cargo.lock` only** — it could not have
caused it. `main` had already fixed it in `cc84ca9e`; merging main in turned CI green with zero
code changes.

**Corollary for dependabot PRs generally:** if `gh pr diff <N> --name-only` prints only
`Cargo.lock`, no lint or compile error can originate from the PR itself unless a bumped crate
changed its *public API or generated code*. Check the bumped crates' changelogs for behavioral
notes (e.g. tower-http 0.7.1 changed `ServeDir::try_call` error propagation) and grep whether the
workspace even uses the affected surface before assuming breakage.
