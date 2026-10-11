//! The camera readout without the camera tool (lab-record LR-06).

use serde_json::Value;

use super::io::{LiveWorld, WorldIo};
use crate::supervisor::Supervisor;

impl Supervisor {
    /// The same readout `client_camera` reports (zoom, offsets, gains and
    /// the camera actor's pose), read from memory and nothing else: no
    /// `ensure_focus`, no `look`. A recording samples this while a person
    /// plays, and the person owns the mouse (D-LR2): turning virtual focus
    /// on would fight their input.
    ///
    /// The controller and camera found by one read are kept for the next
    /// (re-checked by vtable before use), so a tick every 500 ms does not
    /// rescan the actor list; a failed read drops them.
    pub(crate) async fn camera_read(&self) -> Result<Value, String> {
        let mut world = LiveWorld::new(self).with_camera(self.record.cached_camera());
        let out = world.camera_readout().await;
        let keep = out.as_ref().ok().and_then(|_| world.camera());
        self.record.cache_camera(keep);
        out
    }
}
