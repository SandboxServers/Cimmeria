# DW-27B — final slash metadata and six-slice reconciliation

`crates/lab/src/workbench/slash_descriptors_3b.rs` pins patch-012 words 136–162, their exact typed arguments and events, and the evidence-backed handler, scope, impact, blocker and readback limits. `slash_catalog.rs` combines all six 27-word descriptor slices and rejects a changed, duplicated, missing or reordered patch entry. All 162 forms remain unsupported; no native slash submission or server effect is claimed.

Remaining gates: DW-24B must prove native prefixed-word registration, serialization and receipt with the actual client; DW-27C must reconcile any stock-only GM words against the client XML. The ordinary method 91 ring source-interaction guard is separately missing, and queue/effect response methods do not establish accepted state. No live lab action was performed in this packet.

Recheck from this packet worktree with `python tools/client-patches/gm_slash_commands.py --check`, then the `cimmeria-lab` fmt, Clippy and tests through `tools/build-lane/lane.ps1`.

Review correction: legacy space-queue trace stubs are classified as absent handlers, while `/gmconfirmeffect` remains receipt-only. The combined catalog now carries each slice's separately sourced handler, scope, precondition, readback and evidence fields; missing fields stay unknown.
