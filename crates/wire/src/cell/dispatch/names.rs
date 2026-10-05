//! Human-readable names for cell method indices (used in logging).

use crate::names::{cell_method, SGWPLAYER_CLASS_ID};

/// The name of SGWPlayer exposed cell method `index`, or `"unknown"` past
/// the last one (108). Reads the table `crate::names` generates from
/// `entities/defs/`; the GM tail (109+) is `crate::names::player_cell_method`.
pub fn cell_method_name(index: u16) -> &'static str {
    cell_method(SGWPLAYER_CLASS_ID, index).unwrap_or("unknown")
}
