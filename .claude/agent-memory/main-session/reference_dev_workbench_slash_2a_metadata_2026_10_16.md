---
name: workbench slash 2a metadata
description: Disabled typed forms and audit preview for patch-012 slash entries 55–81
type: reference
---

# Slash descriptor slice 55–81 (2026-10-16)

- `crates/lab/src/workbench/slash_descriptors_2a.rs` rejects drift in all 27 canonical patch words, events, access masks and typed argument declarations before producing a disabled form. It marks every patch String argument sensitive and redacts it in `audit_argument`; numeric/Boolean output is only a typed audit preview, never a native slash line. Source: `data/client-patches/012-gm-slash-commands/commands.toml`, `docs/analysis/dev-workbench/evidence/DW-26A-E.md`.
- `/debugminigamecomplete` has one patch `Integer ResultCode`, no GM handler, and no link to the ticketed SmartFox result path. `/gmrechargeitem` likewise has no GM handler and must not be substituted with paid vendor recharge. The evidence source and form readback notes preserve these boundaries. Source: `docs/analysis/dev-workbench/evidence/DW-26A-E.md`, `crates/lab/src/workbench/slash_descriptors_2a.rs`.
- Selected-mob words and explicit target-ID words carry source-backed fresh target/kind policies. The unproved `-1` kill sentinel remains a typed-validation refusal. Combat-debug readbacks use `client_chat_log`, and world-action recipes require multiple independent read sources before any future receipt can claim verification. Source: `crates/lab/src/workbench/slash_descriptors_2a.rs`, `crates/lab/src/workbench/slash_descriptors_2a_tests.rs`.
