# DW-25A worknote — 2026-10-16

Added 27 typed, evidence-backed disabled descriptors for the first patch-012 slash slice. Each carries source TOML arguments, per-field help, server handler status, target scope, effect/impact, readback or diagnostic limitation, compensation and a specific blocked reason. The live GM gate remains server-sourced. The command path still lacks proven native submission, client event-to-method binding and cross-source receipts, so no form is runnable and no submission is treated as acceptance.

`slash_descriptors_1a` is a standalone slice; DW-27B owns the six-slice catalog merge. This keeps the existing 162-row conservative catalog unchanged until all slices exist. The optional `/gmshowflag` omission, target-argument mismatches and absent server handlers stay disabled. Boolean/optional display formatting is only a UI preview, never a native line serializer.

Checks: `cargo fmt -p cimmeria-lab` status=ok; `cargo clippy -p cimmeria-lab --all-targets -- -D warnings` status=ok; `cargo test -p cimmeria-lab` status=ok (1554 passed). The exact set, TOML type/order and mutation guard, Boolean/optional preview and submitted-versus-accepted receipt states passed.

Live-client visual UAT: not run; this packet adds data contracts only, with no rendered frontend. Real client submission remains under DW-24B and DW-35 approval.