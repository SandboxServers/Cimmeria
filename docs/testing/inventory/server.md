# Tests — `server`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 2  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Binary entry point for the Cimmeria server. Wires the services together.

## All tests (2)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [rfc1123_format_known_date](../../../crates/server/src/cosmos_log.rs#L357) | unit | Cosmos Log | 2026-03-15 | Asserts equality on `(y, m, d)` |  |
| [percent_encode_auth](../../../crates/server/src/cosmos_log.rs#L369) | unit | Cosmos Log | 2026-03-15 | Asserts equality on `encoded` |  |
