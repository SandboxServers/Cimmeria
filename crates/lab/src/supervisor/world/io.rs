//! The seam between the world tools and the client.
//!
//! [`WorldIo`] is everything the world tools read from or do to the game.
//! [`LiveWorld`] implements it over the supervisor (bridge memory reads,
//! Lua reads, the lab's real-input path); the tests implement it with a
//! simulated client (`sim.rs`), so the planners and the tool orchestration
//! are exercised end to end without a game.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::geometry::{Pose, Screen, Vec3};
use super::lua::{self, UnitInfo, UnitSlots};
use super::memory::{self, WorldSnapshot};
use crate::supervisor::entity_table::{decode_hex, Memory, DEFAULT_MAX_NODES};
use crate::supervisor::flows::ui_state::lua_results_of;
use crate::supervisor::{process, Supervisor};

/// How long a projection may take to come back (a few frames).
const PROJECTION_TIMEOUT: Duration = Duration::from_millis(1500);
/// If the frame counter has not moved by then, re-install the chain.
const PROJECTION_STALL: Duration = Duration::from_millis(500);
const PROJECTION_POLL: Duration = Duration::from_millis(30);

/// A projection: the screen and one pixel (or `None`) per point.
#[derive(Debug, Clone, PartialEq)]
pub struct Projected {
    pub screen: Screen,
    pub points: Vec<Option<(f64, f64)>>,
    pub host: String,
}

/// Everything the world tools need from the client.
#[allow(async_fn_in_trait)]
pub trait WorldIo {
    /// Milliseconds since this tool call started.
    fn now_ms(&self) -> u64;
    async fn sleep(&mut self, ms: u64);
    /// All entities with poses, plus the unit-slot map.
    async fn snapshot(&mut self) -> Result<WorldSnapshot, String>;
    /// One actor's pose (one read; used per tick).
    async fn actor_pose(&mut self, actor: u32) -> Result<Pose, String>;
    /// The slot map only (a few reads).
    async fn slots(&mut self) -> Result<Vec<(i32, u32)>, String>;
    /// `Unit.Player`, `Unit.Target`, `Unit.MouseOver`.
    async fn unit_slots(&mut self) -> Result<UnitSlots, String>;
    /// Point a private unit slot at an entity (a journaled native call).
    async fn pin(&mut self, slot: i32, entity: u32) -> Result<(), String>;
    /// The stock unit API's answers for the given slots.
    async fn unit_info(&mut self, slots: &[i32]) -> Result<Vec<UnitInfo>, String>;
    /// Project client-space points with the game's own view.
    async fn project(&mut self, points: &[Vec3]) -> Result<Projected, String>;
    /// The local camera actor's pose, when the chain resolves.
    async fn camera_pose(&mut self) -> Result<Pose, String>;
    /// Mouse-look motion / wheel (DirectInput counts).
    async fn look(&mut self, dx: i32, dy: i32, wheel: i32) -> Result<(), String>;
    /// Press (`down`) or release a key.
    async fn key(&mut self, key: &str, down: bool) -> Result<(), String>;
    /// Place the UI cursor (UI pixels); also posts a `WM_MOUSEMOVE` there
    /// when `mouse_move` is set.
    async fn place_cursor(&mut self, x: i32, y: i32, mouse_move: bool) -> Result<(), String>;
    /// Press or release a mouse button at the cursor (0 left, 1 right).
    async fn button(&mut self, button: usize, down: bool) -> Result<(), String>;
    /// Visible top-level UI windows.
    async fn visible_windows(&mut self) -> Result<Vec<String>, String>;
    /// The stock `targetUnit(slot)` (N3).
    async fn target_unit(&mut self, slot: i32) -> Result<(), String>;
    /// Make sure the game reads the lab's input (virtual focus).
    async fn ensure_focus(&mut self) -> Result<(), String>;
}

/// [`WorldIo`] over the running client.
pub struct LiveWorld<'a> {
    sup: &'a Supervisor,
    t0: Instant,
    slide: Option<i64>,
    manager: Option<u32>,
    units: Option<UnitSlots>,
}

impl<'a> LiveWorld<'a> {
    pub fn new(sup: &'a Supervisor) -> Self {
        Self {
            sup,
            t0: Instant::now(),
            slide: None,
            manager: None,
            units: None,
        }
    }

    async fn slide(&mut self) -> Result<i64, String> {
        if let Some(s) = self.slide {
            return Ok(s);
        }
        let info = self
            .sup
            .bridge
            .call("module_info", json!({}))
            .await
            .map_err(|e| format!("module_info: {e}"))?;
        let s = info.get("slide").and_then(Value::as_i64).unwrap_or(0);
        self.slide = Some(s);
        Ok(s)
    }

    async fn manager(&mut self) -> Result<u32, String> {
        if let Some(m) = self.manager {
            return Ok(m);
        }
        let slide = self.slide().await?;
        let m = memory::manager(&mut Reader(self.sup), slide).await?;
        self.manager = Some(m);
        Ok(m)
    }

    async fn lua(&self, chunk: &str) -> Result<Vec<String>, String> {
        // Reads go straight to the bridge: a movement loop polls every
        // 100 ms and would flush the crash journal. Native calls and input
        // go through `bridge_call` (journaled).
        let v = self
            .sup
            .bridge
            .call("lua_eval", json!({ "chunk": chunk }))
            .await
            .map_err(|e| format!("lua_eval: {e}"))?;
        lua_results_of(&v)
    }

    async fn hwnd(&self) -> Result<isize, String> {
        let pid = self.sup.state.lock().await.pid.ok_or("no client running")?;
        tokio::task::spawn_blocking(move || process::find_main_window(pid))
            .await
            .map_err(|e| format!("window lookup: {e}"))?
            .ok_or_else(|| "the client has no main window yet".to_string())
    }
}

/// Unjournaled bridge memory reads (fault-guarded, never quarantined).
struct Reader<'a>(&'a Supervisor);

impl Memory for Reader<'_> {
    async fn read(&mut self, addr: u32, len: u32) -> Result<Vec<u8>, String> {
        let v = self
            .0
            .bridge
            .call(
                "mem_read",
                json!({ "addr": format!("{addr:#x}"), "len": len }),
            )
            .await
            .map_err(|e| format!("mem_read {addr:#x}+{len}: {e}"))?;
        let hex = v
            .get("hex")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("mem_read {addr:#x}: no hex in {v}"))?;
        decode_hex(hex)
    }
}

const WM_MOUSEMOVE: u32 = 0x0200;

impl WorldIo for LiveWorld<'_> {
    fn now_ms(&self) -> u64 {
        self.t0.elapsed().as_millis() as u64
    }

    async fn sleep(&mut self, ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    async fn snapshot(&mut self) -> Result<WorldSnapshot, String> {
        let slide = self.slide().await?;
        let snap = memory::snapshot(&mut Reader(self.sup), slide, DEFAULT_MAX_NODES).await?;
        self.manager = Some(snap.manager);
        Ok(snap)
    }

    async fn actor_pose(&mut self, actor: u32) -> Result<Pose, String> {
        memory::actor_pose(&mut Reader(self.sup), actor).await
    }

    async fn slots(&mut self) -> Result<Vec<(i32, u32)>, String> {
        let m = self.manager().await?;
        memory::read_slots(&mut Reader(self.sup), m).await
    }

    async fn unit_slots(&mut self) -> Result<UnitSlots, String> {
        if let Some(u) = self.units {
            return Ok(u);
        }
        let u = lua::parse_unit_constants(&self.lua(lua::UNIT_CONSTANTS_CHUNK).await?)?;
        self.units = Some(u);
        Ok(u)
    }

    async fn pin(&mut self, slot: i32, entity: u32) -> Result<(), String> {
        let m = self.manager().await?;
        let slide = self.slide().await?;
        let func = (memory::SET_UNIT_SLOT_VA as i64 + slide) as u32;
        let v = self
            .sup
            .bridge_call(
                "call_native",
                json!({
                    "addr": format!("{func:#x}"),
                    "conv": "thiscall",
                    "args": [format!("{m:#x}"), slot, entity],
                    "ret": "void",
                }),
            )
            .await?;
        if v.get("ok").and_then(Value::as_bool) == Some(false) {
            return Err(format!("pin slot {slot} -> {entity}: {v}"));
        }
        Ok(())
    }

    async fn unit_info(&mut self, slots: &[i32]) -> Result<Vec<UnitInfo>, String> {
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        Ok(lua::parse_unit_info(
            &self.lua(&lua::unit_info_chunk(slots)).await?,
        ))
    }

    async fn project(&mut self, points: &[Vec3]) -> Result<Projected, String> {
        let mut force = false;
        loop {
            let ticket = lua::parse_projection_ticket(
                &self
                    .lua(&lua::projection_request_chunk(points, force))
                    .await?,
            )?;
            let t0 = Instant::now();
            let mut last_frame = ticket.frame;
            let mut frame_moved = false;
            while t0.elapsed() < PROJECTION_TIMEOUT {
                tokio::time::sleep(PROJECTION_POLL).await;
                let r = lua::parse_projection_read(
                    &self.lua(&lua::projection_read_chunk(ticket.reqid)).await?,
                );
                if r.done {
                    if !r.errors.is_empty() {
                        return Err(format!("worldToPixel: {}", r.errors.join("; ")));
                    }
                    return Ok(Projected {
                        screen: ticket.screen,
                        points: r.points,
                        host: ticket.host,
                    });
                }
                frame_moved |= r.frame != last_frame;
                last_frame = r.frame;
                if !frame_moved && !force && t0.elapsed() >= PROJECTION_STALL {
                    break;
                }
            }
            if force || frame_moved {
                return Err(format!(
                    "projection did not complete within {} ms (host {}, PreRender {})",
                    PROJECTION_TIMEOUT.as_millis(),
                    ticket.host,
                    if frame_moved {
                        "running"
                    } else {
                        "not running"
                    }
                ));
            }
            // The chained PreRender never ran: the UI was reloaded or the
            // host lost its subscription. Re-install once.
            force = true;
        }
    }

    async fn camera_pose(&mut self) -> Result<Pose, String> {
        let slide = self.slide().await?;
        let mut r = Reader(self.sup);
        let actor = memory::camera_actor(&mut r, slide).await?;
        memory::actor_pose(&mut r, actor).await
    }

    async fn look(&mut self, dx: i32, dy: i32, wheel: i32) -> Result<(), String> {
        if dx == 0 && dy == 0 && wheel == 0 {
            return Ok(());
        }
        self.sup
            .bridge_call("input_mouse", json!({ "dx": dx, "dy": dy, "wheel": wheel }))
            .await
            .map(|_| ())
    }

    async fn key(&mut self, key: &str, down: bool) -> Result<(), String> {
        self.sup
            .input_key(key, if down { "down" } else { "up" }, None)
            .await
            .map(|_| ())
    }

    async fn place_cursor(&mut self, x: i32, y: i32, mouse_move: bool) -> Result<(), String> {
        self.sup.move_cursor(x, y).await?;
        if mouse_move {
            let hwnd = self.hwnd().await?;
            let lp = (((y as u32 & 0xFFFF) << 16) | (x as u32 & 0xFFFF)) as isize;
            process::post_message(hwnd, WM_MOUSEMOVE, 0, lp)?;
        }
        Ok(())
    }

    async fn button(&mut self, button: usize, down: bool) -> Result<(), String> {
        self.sup
            .input_mouse(
                0,
                0,
                0,
                Some(button),
                Some(if down { "down" } else { "up" }),
                None,
            )
            .await
            .map(|_| ())
    }

    async fn visible_windows(&mut self) -> Result<Vec<String>, String> {
        Ok(lua::parse_window_list(
            &self.lua(lua::VISIBLE_WINDOWS_CHUNK).await?,
        ))
    }

    async fn target_unit(&mut self, slot: i32) -> Result<(), String> {
        let v = self
            .sup
            .bridge_call("lua_eval", json!({ "chunk": lua::target_unit_chunk(slot) }))
            .await?;
        lua_results_of(&v).map(|_| ())
    }

    async fn ensure_focus(&mut self) -> Result<(), String> {
        self.sup.input_focus(true).await.map(|_| ())
    }
}
