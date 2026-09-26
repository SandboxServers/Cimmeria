//! # cimmeria-wire-log
//!
//! The decoded wire-message stream: every inbound bundle message and every
//! outbound entity-method call, logged once per message on the `wire.in` /
//! `wire.out` targets with its method name and, when a schema decoder is
//! registered, a structured `decoded` field; plus the per-session packet tap
//! (`wire_log::tap`) the lab MCP endpoint drains.
//!
//! Split out of `cimmeria-services` (wave W3b of
//! `docs/architecture/services-crate-split.md`). The module keeps its old
//! path, so `crate::wire_log::…` and `super::…` paths inside it are
//! unchanged, and `cimmeria-services` re-exports it as
//! `cimmeria_services::wire_log`. Its only split-crate dependency is
//! `cimmeria-wire`, for the cell-method names.

#![warn(unreachable_pub)]

pub mod wire_log;
