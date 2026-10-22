//! The local player controller's input words, decoded from a [`Mem`] read.
//!
//! Offsets come from the RE finding `controller-mouse-input-model.md`, section
//! 2. Every address is computed with wrapping arithmetic, so a corrupt pointer
//! reads as `None` instead of panicking inside the game's process.

use crate::hooks::entity_trace::map::Mem;

/// `PlayerInput*` inside the controller.
pub const PC_INPUT_PTR_OFF: u32 = 0x3f4;
/// Mode bits.
pub const PC_MODE_OFF: u32 = 0x498;
/// Button bits.
pub const PC_BUTTONS_OFF: u32 = 0x49c;
/// Drag x, `i32` mouse counts, reset on press.
pub const PC_DRAG_X_OFF: u32 = 0x4a0;
/// Drag y, `i32` mouse counts, reset on press.
pub const PC_DRAG_Y_OFF: u32 = 0x4a4;
/// Hover entity id.
pub const PC_HOVER_OFF: u32 = 0x4c0;
/// Turn axis inside `PlayerInput`, `f32`.
pub const INPUT_TURN_OFF: u32 = 0xa4;
/// Forward axis inside `PlayerInput`, `f32`.
pub const INPUT_FWD_OFF: u32 = 0xa8;
/// Strafe axis inside `PlayerInput`, `f32`.
pub const INPUT_STRAFE_OFF: u32 = 0xc0;
/// Mouselook mode bit.
pub const MODE_MOUSELOOK: u32 = 0x1;
/// Jump mode bit.
pub const MODE_JUMP: u32 = 0x2;
/// Autorun mode bit.
pub const MODE_AUTORUN: u32 = 0x4;
/// Mousewalk mode bit.
pub const MODE_MOUSEWALK: u32 = 0x8;
/// Every mode bit this client reads.
pub const MODE_MASK: u32 = 0xf;
/// Left button bit.
pub const BUTTON_LMB: u32 = 0x1;
/// Right button bit.
pub const BUTTON_RMB: u32 = 0x2;
/// Every button bit this client reads.
pub const BUTTON_MASK: u32 = 0x3;

/// The `PlayerInput` axes, as the engine last wrote them.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct InputAxes {
    /// Turn axis.
    pub turn: f32,
    /// Forward axis.
    pub fwd: f32,
    /// Strafe axis.
    pub strafe: f32,
}

/// One poll of the controller's input words.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ControllerState {
    /// `[pc + PC_MODE_OFF] & MODE_MASK`.
    pub mode: u32,
    /// `[pc + PC_BUTTONS_OFF] & BUTTON_MASK`.
    pub buttons: u32,
    /// `[pc + 0x4a0]` and `[pc + 0x4a4]`, as `i32`.
    pub drag: [i32; 2],
    /// `[pc + PC_HOVER_OFF]`, as `i32`.
    pub hover_id: i32,
    /// `None` when `PlayerInput*` is 0 or unreadable.
    pub axes: Option<InputAxes>,
}

/// The controller's input state. `None` when any of mode, buttons, drag or
/// hover cannot be read; the axes alone may be `None` and still give a state.
pub fn read_controller(mem: &impl Mem, pc: u32) -> Option<ControllerState> {
    let at = |off: u32| pc.wrapping_add(off);
    let mode = mem.u32_at(at(PC_MODE_OFF))? & MODE_MASK;
    let buttons = mem.u32_at(at(PC_BUTTONS_OFF))? & BUTTON_MASK;
    let drag_x = mem.u32_at(at(PC_DRAG_X_OFF))? as i32;
    let drag_y = mem.u32_at(at(PC_DRAG_Y_OFF))? as i32;
    let hover_id = mem.u32_at(at(PC_HOVER_OFF))? as i32;
    Some(ControllerState {
        mode,
        buttons,
        drag: [drag_x, drag_y],
        hover_id,
        axes: read_axes(mem, pc),
    })
}

/// The axes of the `PlayerInput` that `pc` points to. `None` on a null
/// pointer or any unreadable word.
fn read_axes(mem: &impl Mem, pc: u32) -> Option<InputAxes> {
    let input = mem.u32_at(pc.wrapping_add(PC_INPUT_PTR_OFF))?;
    if input == 0 {
        return None;
    }
    let at = |off: u32| input.wrapping_add(off);
    let axis = |off: u32| mem.u32_at(at(off)).map(f32::from_bits);
    Some(InputAxes {
        turn: axis(INPUT_TURN_OFF)?,
        fwd: axis(INPUT_FWD_OFF)?,
        strafe: axis(INPUT_STRAFE_OFF)?,
    })
}
