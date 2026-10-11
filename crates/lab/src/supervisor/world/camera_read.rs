//! The camera readout without the camera tool (lab-record LR-06), and the
//! cached controller/camera pair it reuses.

use serde_json::Value;

use super::io::{LiveWorld, WorldIo};
use super::memory::{self, PlayerCamera};
use crate::supervisor::entity_table::Memory;
use crate::supervisor::Supervisor;

impl Supervisor {
    /// The same readout `client_camera` reports (zoom, offsets, gains and
    /// the camera actor's pose), read from memory and nothing else: no
    /// `ensure_focus`, no `look`. A recording samples this while a person
    /// plays, and the person owns the mouse (D-LR2): turning virtual focus
    /// on would fight their input.
    ///
    /// Its memory reads go straight to the bridge, not through
    /// `bridge_call`, so the lease permit is checked here: a recorder whose
    /// lease was taken over reads nothing more.
    ///
    /// The controller and camera found by one read are kept for the next
    /// ([`resolve_player_camera`] re-checks them), so a tick every 500 ms
    /// does not rescan the actor list; a failed read drops them.
    pub(crate) async fn camera_read(&self) -> Result<Value, String> {
        crate::lease::permit::ensure("bridge camera_read")?;
        let mut world = LiveWorld::new(self).with_camera(self.record.cached_camera());
        let out = world.camera_readout().await;
        let keep = out.as_ref().ok().and_then(|_| world.camera());
        self.record.cache_camera(keep);
        out
    }
}

/// The player's controller and camera: `cached` while both vtables still
/// match and the controller still points at that camera (`+0x2b0`), else a
/// fresh actor scan. A camera freed and replaced at the same address keeps
/// its vtable, so the controller's pointer is the check that catches it.
pub(super) async fn resolve_player_camera<M: Memory>(
    mem: &mut M,
    cached: Option<PlayerCamera>,
    slide: i64,
) -> Result<PlayerCamera, String> {
    if let Some(c) = cached {
        if still_current(mem, c, slide).await {
            return Ok(c);
        }
    }
    memory::find_player_camera(mem, slide).await
}

async fn still_current<M: Memory>(mem: &mut M, c: PlayerCamera, slide: i64) -> bool {
    if !memory::has_vtable(
        mem,
        c.controller,
        memory::PLAYER_CONTROLLER_VTABLE_VA,
        slide,
    )
    .await
        || !memory::has_vtable(mem, c.camera, memory::PLAYER_CAMERA_VTABLE_VA, slide).await
    {
        return false;
    }
    mem.read(c.controller + memory::CONTROLLER_CAMERA_OFF, 4)
        .await
        .is_ok_and(|b| b.len() == 4 && u32::from_le_bytes([b[0], b[1], b[2], b[3]]) == c.camera)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Word-addressed fake memory that counts reads of the level pointer
    /// (one per actor scan).
    struct Fake {
        words: HashMap<u32, u32>,
        scans: u32,
        glevel: u32,
    }

    impl Memory for Fake {
        async fn read(&mut self, addr: u32, len: u32) -> Result<Vec<u8>, String> {
            if addr == self.glevel {
                self.scans += 1;
            }
            let mut out = Vec::new();
            for i in 0..len.div_ceil(4) {
                let w = self
                    .words
                    .get(&(addr + i * 4))
                    .ok_or_else(|| format!("unmapped {:#x}", addr + i * 4))?;
                out.extend_from_slice(&w.to_le_bytes());
            }
            out.truncate(len as usize);
            Ok(out)
        }
    }

    const SLIDE: i64 = 0x10000;

    fn rebased(va: u32) -> u32 {
        (va as i64 + SLIDE) as u32
    }

    /// A level whose one actor is the controller `pc`, pointing at `cam`.
    fn level(pc: u32, cam: u32) -> Fake {
        let glevel = rebased(memory::GLEVEL_PTR_VA);
        let (world, level, data) = (0x5000, 0x6000, 0x7000);
        let words = HashMap::from([
            (glevel, world),
            (world + 0x50, level),
            (level + 0x3C, data),
            (level + 0x40, 1),
            (data, pc),
            (pc, rebased(memory::PLAYER_CONTROLLER_VTABLE_VA)),
            (pc + memory::CONTROLLER_CAMERA_OFF, cam),
            (cam, rebased(memory::PLAYER_CAMERA_VTABLE_VA)),
        ]);
        Fake {
            words,
            scans: 0,
            glevel,
        }
    }

    /// LR-07 review: the cached pair is reused without a scan while the
    /// controller still points at it; once the controller points at a new
    /// camera (the old one still carrying its vtable), the next read
    /// rescans and finds the new one.
    #[tokio::test]
    async fn a_cached_camera_is_reused_until_the_controller_points_elsewhere() {
        let (pc, cam, cam2) = (0x9000, 0xA000, 0xB000);
        let mut m = level(pc, cam);
        let first = resolve_player_camera(&mut m, None, SLIDE).await.unwrap();
        assert_eq!(
            first,
            PlayerCamera {
                controller: pc,
                camera: cam
            }
        );
        assert_eq!(m.scans, 1);

        let again = resolve_player_camera(&mut m, Some(first), SLIDE)
            .await
            .unwrap();
        assert_eq!(again, first);
        assert_eq!(m.scans, 1, "a current cache is not rescanned");

        m.words.insert(pc + memory::CONTROLLER_CAMERA_OFF, cam2);
        m.words
            .insert(cam2, rebased(memory::PLAYER_CAMERA_VTABLE_VA));
        let moved = resolve_player_camera(&mut m, Some(first), SLIDE)
            .await
            .unwrap();
        assert_eq!(
            moved,
            PlayerCamera {
                controller: pc,
                camera: cam2
            }
        );
        assert_eq!(m.scans, 2, "a changed camera pointer rescans");
    }
}
