//! The NPC AI's behaviour, under the `cell::service::npc_ai` path it had in
//! `cimmeria-services`. The rest of `cell::service` (the message loop, the
//! ticks, the Base message handlers) sits above this crate; the AI's state
//! primitives are in `cimmeria-cell-world`.

pub mod npc_ai;
