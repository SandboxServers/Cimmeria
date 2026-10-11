//! The camera readout without the camera tool (lab-record LR-06).

// Dead until LR-07 wires the record tools in (cimmeria-lab is a binary).
#![cfg_attr(not(test), allow(dead_code))]

use serde_json::Value;

use super::io::{LiveWorld, WorldIo};
use crate::supervisor::Supervisor;

impl Supervisor {
    /// The same readout `client_camera` reports (zoom, offsets, gains and
    /// the camera actor's pose), read from memory and nothing else: no
    /// `ensure_focus`, no `look`. A recording samples this while a person
    /// plays, and the person owns the mouse (D-LR2): turning virtual focus
    /// on would fight their input.
    pub(crate) async fn camera_read(&self) -> Result<Value, String> {
        LiveWorld::new(self).camera_readout().await
    }
}
