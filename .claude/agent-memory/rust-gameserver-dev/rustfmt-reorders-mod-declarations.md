---
name: rustfmt-reorders-mod-declarations
description: rustfmt's reorder_modules is on by default, so "append your `mod` line at the END of the shared mod.rs" coordination instructions cannot survive `cargo fmt` — plan for an alphabetical three-way merge instead.
metadata:
  type: project
---

# `cargo fmt` sorts `mod` declarations — end-append instructions don't survive

Multi-packet campaigns (Harset, Castle, Cellblock) hand several parallel workers
the same shared `mod.rs` — `crates/services/src/cell/content/chain_replay_tests/mod.rs`
is the usual one — with the instruction "append your `mod` line at the **end**,
the coordinator resolves the merge". That instruction cannot be honoured:
**rustfmt's `reorder_modules` defaults to `true`**, so `cargo fmt --all` sorts
the whole `mod` block alphabetically, and `fmt` is a gating CI job.

**Why:** the coordinator's intent is to turn N concurrent edits into N clean
appends rather than N conflicting insertions at the same line. rustfmt defeats
that silently — you append correctly, run the mandatory fmt, and the line moves.

**How to apply:** add your `mod` line wherever, run `cargo fmt`, and tell the
coordinator in the handoff that the merge will be an alphabetical three-way,
not three appends. Don't "fix" it by skipping fmt or by hand-reordering after
fmt — the next worker's fmt undoes it. If a campaign genuinely needs
append-only semantics on a shared file, the file needs `#[rustfmt::skip]` on
the module block, which is a change to make deliberately and once.

Related: [[revert-verification-loses-uncommitted-fmt]] — fmt before you
checkpoint, for the same "fmt touches more than you expect" reason.
