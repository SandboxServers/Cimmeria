//! Duel tests that need nothing but this crate: the registry's state machine
//! on an injected clock. The handler and end-path tests drive the duel end to
//! end and live with the duel plugin (`cimmeria-cell-duel`, #962).

mod registry;
