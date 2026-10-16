---
name: workbench slash transport
description: Stock chat candidate and evidence limits for DW-24B
type: reference
---

# Workbench slash transport (2026-10-16)

- `docs/analysis/hub-pass-uat/README.md` Chat 10 records `/gmsendgmshout 1 HPUshout1` submitted through stock `ChatMod.onTextAccepted` with an independently observed native server receipt. `docs/reverse-engineering/findings/chat-speaker-echo.md:103-108` says that handler passes raw input to `processTextCommand`. This is a precedent for one plain-token shape, not a parser contract for patch 012's 162 words.
- `docs/reverse-engineering/findings/chat-wire-formats.md:199-211` traces the GM shout event and outgoing method but leaves tokenization unresolved. `crates/client-telemetry/src/bridge/console.rs` keeps `CONSOLE_EXEC_ADDR=0` because the known `0x00539850` entry takes `FFrame`, not raw string; DW-24B closes the caller-address escape.
- `crates/lab/src/workbench/slash_transport.rs` enforces the plain-token candidate and separates submission, acknowledgement, outgoing and server receipt. The stock adapter has an exact input readback before `ChatMod.onTextAccepted` but its verified fingerprint is absent, so all live slash forms remain disabled pending binary evidence and an approved lab run.
