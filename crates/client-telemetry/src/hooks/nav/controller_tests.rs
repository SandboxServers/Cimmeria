//! Unit tests for the controller input readers, over a fake address space.

use super::controller::{
    read_controller, InputAxes, BUTTON_RMB, INPUT_FWD_OFF, INPUT_STRAFE_OFF, INPUT_TURN_OFF,
    MODE_AUTORUN, MODE_JUMP, MODE_MOUSELOOK, PC_BUTTONS_OFF, PC_DRAG_X_OFF, PC_DRAG_Y_OFF,
    PC_HOVER_OFF, PC_INPUT_PTR_OFF, PC_MODE_OFF,
};
use crate::hooks::camera::layout::{
    GLEVEL_PTR_VA, LEVEL_ACTORS_OFF, PLAYER_CONTROLLER_VTABLE_VA, WORLD_LEVEL_OFF,
};
use crate::hooks::camera::read::find_player_controller;
use crate::hooks::entity_trace::map::fake::FakeMem;

/// Virtual addresses for the fake world: a `UWorld`, its level, the Actors array, and actors.
const WORLD: u32 = 0x2000_0000;
const LEVEL: u32 = 0x2000_1000;
const ACTORS: u32 = 0x2000_2000;
const PC: u32 = 0x2100_0000;
const OTHER: u32 = 0x2400_0000;
const INPUT: u32 = 0x2300_0000;

/// Lay out the actors in the level's Actors array, and point the global at the world.
fn build_world(mem: &mut FakeMem, actors: &[u32]) {
    mem.set(GLEVEL_PTR_VA, WORLD);
    mem.set(WORLD + WORLD_LEVEL_OFF, LEVEL);
    mem.set(LEVEL + LEVEL_ACTORS_OFF, ACTORS);
    mem.set(LEVEL + LEVEL_ACTORS_OFF + 4, actors.len() as u32);
    for (i, a) in actors.iter().enumerate() {
        mem.set(ACTORS + 4 * i as u32, *a);
    }
}

/// Every controller word except the mode, with a null `PlayerInput`.
fn put_words(mem: &mut FakeMem) {
    mem.set(PC + PC_BUTTONS_OFF, 0xffff_fffe);
    mem.set(PC + PC_DRAG_X_OFF, 0xffff_fff6);
    mem.set(PC + PC_DRAG_Y_OFF, 5);
    mem.set(PC + PC_HOVER_OFF, 0x42);
    mem.set(PC + PC_INPUT_PTR_OFF, 0);
}

/// A controller with every word readable: mode `0xfff3`, so the masked mode is `0x3`.
fn full_controller(mem: &mut FakeMem) {
    mem.set(PC + PC_MODE_OFF, 0xfff3);
    put_words(mem);
}

/// `find_player_controller` finds the controller among other actors, and gives `None` without one.
#[test]
fn find_player_controller_finds_the_controller() {
    let mut mem = FakeMem::default();
    mem.set(OTHER, 0x1234);
    build_world(&mut mem, &[OTHER, PC]);
    mem.set(PC, PLAYER_CONTROLLER_VTABLE_VA);
    assert_eq!(find_player_controller(&mem), Some(PC));

    let mut none = FakeMem::default();
    mem_without_controller(&mut none);
    assert_eq!(find_player_controller(&none), None);
    assert_eq!(find_player_controller(&FakeMem::default()), None);
}

/// A level with one actor that is not the controller.
fn mem_without_controller(mem: &mut FakeMem) {
    mem.set(OTHER, 0x1234);
    build_world(mem, &[OTHER]);
}

/// The mode is masked to its four bits, and the buttons to their two.
#[test]
fn read_controller_masks_mode_and_buttons() {
    let mut mem = FakeMem::default();
    full_controller(&mut mem);
    let state = read_controller(&mem, PC).expect("every word is readable");
    assert_eq!(state.mode, 0x3);
    assert_eq!(state.buttons, 0x2);
}

/// The drag words are read as signed counts, and the hover as a signed id.
#[test]
fn read_controller_reads_drag_and_hover() {
    let mut mem = FakeMem::default();
    full_controller(&mut mem);
    let state = read_controller(&mem, PC).expect("every word is readable");
    assert_eq!(state.drag, [-10, 5]);
    assert_eq!(state.hover_id, 0x42);
}

/// The axes are read as `f32` bits through the `PlayerInput` pointer.
#[test]
fn read_controller_reads_axes_from_f32_bits() {
    let mut mem = FakeMem::default();
    full_controller(&mut mem);
    mem.set(PC + PC_INPUT_PTR_OFF, INPUT);
    mem.set(INPUT + INPUT_TURN_OFF, 0.5f32.to_bits());
    mem.set(INPUT + INPUT_FWD_OFF, (-1.0f32).to_bits());
    mem.set(INPUT + INPUT_STRAFE_OFF, 0.25f32.to_bits());
    let state = read_controller(&mem, PC).expect("every word is readable");
    assert_eq!(
        state.axes,
        Some(InputAxes {
            turn: 0.5,
            fwd: -1.0,
            strafe: 0.25,
        })
    );
}

/// A null `PlayerInput` gives `axes: None`, and the rest of the state still reads.
#[test]
fn read_controller_null_player_input_gives_no_axes() {
    let mut mem = FakeMem::default();
    full_controller(&mut mem);
    let state = read_controller(&mem, PC).expect("every word is readable");
    assert_eq!(state.axes, None);
    assert_eq!(state.hover_id, 0x42);
}

/// One unreadable mode word gives `None` for the whole state.
#[test]
fn read_controller_none_when_mode_is_unreadable() {
    let mut mem = FakeMem::default();
    put_words(&mut mem);
    assert_eq!(read_controller(&mem, PC), None);
}

/// The mode constants name the bits the mask keeps.
#[test]
fn mode_bits_fall_inside_the_mask() {
    let mut mem = FakeMem::default();
    full_controller(&mut mem);
    mem.set(PC + PC_MODE_OFF, MODE_MOUSELOOK | MODE_JUMP | MODE_AUTORUN);
    let state = read_controller(&mem, PC).expect("every word is readable");
    assert_eq!(state.mode, MODE_MOUSELOOK | MODE_JUMP | MODE_AUTORUN);
    mem.set(PC + PC_BUTTONS_OFF, BUTTON_RMB);
    assert_eq!(
        read_controller(&mem, PC).map(|s| s.buttons),
        Some(BUTTON_RMB)
    );
}
