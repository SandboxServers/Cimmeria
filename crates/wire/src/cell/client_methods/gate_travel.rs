//! GateTravel interface ClientMethods (indices 65–68).

/// Initialize stargate address book (world/known/hidden lists).
pub const SETUP_STARGATE_INFO: u16 = 65;
/// Add or remove a known stargate address.
pub const UPDATE_STARGATE_ADDRESS: u16 = 66;
/// Override stargate ring rotation angle.
pub const STARGATE_ROTATION_OVERRIDE: u16 = 67;
/// Stargate passage event (gate travel completed).
pub const ON_STARGATE_PASSAGE: u16 = 68;
