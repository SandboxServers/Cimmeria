//! Detours on the press chain (AB-C2). Every one forwards its arguments
//! untouched and returns the original's EAX (the functions are `void` in
//! effect, but a pass-through cannot clobber a value a caller might read);
//! the reads happen before or after the original, through
//! `ReadProcessMemory`. Calling conventions and `ret N` were re-checked
//! against the QA image on 2026-10-04 (finding, AB-C1/AB-C2 addendum).

use std::ffi::c_void;
use std::sync::OnceLock;

use super::{guarded, install, report, with_pending, with_press, PressScope};
use crate::hooks::ability_trace::layout::{self, action, record};
use crate::hooks::ability_trace::now_ms;
use crate::hooks::ability_trace::press::Source;
use crate::hooks::entity_trace::map::{LiveMem, Mem};
use crate::queue::Producer;

/// `useAction` tolua thunk (registered at `0x00ad5cf4`).
pub(in crate::hooks::inline_hooks) const ADDR_USE_ACTION_THUNK: usize = 0x00aa_94e0;
/// `useAbility` tolua thunk (registered at `0x00ad4cd8`).
pub(in crate::hooks::inline_hooks) const ADDR_USE_ABILITY_THUNK: usize = 0x00aa_2910;
/// `FUN_00ad9580(actionId, self)`; its only caller is the `useAction`
/// thunk (`0x00aa9559`).
pub(in crate::hooks::inline_hooks) const ADDR_SLOT: usize = 0x00ad_9580;
/// `FUN_00d2afc0(set, abilityId, targetId)`; callers `0x00ad793b`,
/// `0x00ad795c` (the `useAbility` path) and `0x00e3cdee`
/// (`AbilityAction::execute`).
pub(in crate::hooks::inline_hooks) const ADDR_LOOKUP: usize = 0x00d2_afc0;
/// `FUN_00d2ae40(set, record, targetId)`: builds and posts
/// `Event_NetOut_UseAbility`, or opens the ground reticle.
pub(in crate::hooks::inline_hooks) const ADDR_SEND_BUILDER: usize = 0x00d2_ae40;
/// `PetAbilityAction::execute` (vtable slot at `0x019da48c`).
pub(in crate::hooks::inline_hooks) const ADDR_PET_ACTION_EXECUTE: usize = 0x00e3_cf40;
/// The GamePet send (`thiscall(pet, abilityId, targetId)`, `ret 8`);
/// callers `0x00e3cfba` and `0x00ad7bfe`.
pub(in crate::hooks::inline_hooks) const ADDR_GAME_PET_SEND: usize = 0x00d3_a820;

pub(super) static USE_ACTION_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static USE_ABILITY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static SLOT_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static LOOKUP_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static SEND_BUILDER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static PET_ACTION_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static PET_SEND_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

pub(super) unsafe fn install_all(producer: &Producer) {
    let hooks: [(&'static str, usize, *mut c_void, &OnceLock<usize>); 7] = [
        (
            "ability_use_action",
            ADDR_USE_ACTION_THUNK,
            use_action_detour as *mut c_void,
            &USE_ACTION_TRAMPOLINE,
        ),
        (
            "ability_use_ability",
            ADDR_USE_ABILITY_THUNK,
            use_ability_detour as *mut c_void,
            &USE_ABILITY_TRAMPOLINE,
        ),
        (
            "ability_slot",
            ADDR_SLOT,
            slot_detour as *mut c_void,
            &SLOT_TRAMPOLINE,
        ),
        (
            "ability_lookup",
            ADDR_LOOKUP,
            lookup_detour as *mut c_void,
            &LOOKUP_TRAMPOLINE,
        ),
        (
            "ability_send_builder",
            ADDR_SEND_BUILDER,
            send_builder_detour as *mut c_void,
            &SEND_BUILDER_TRAMPOLINE,
        ),
        (
            "ability_pet_action",
            ADDR_PET_ACTION_EXECUTE,
            pet_action_detour as *mut c_void,
            &PET_ACTION_TRAMPOLINE,
        ),
        (
            "ability_pet_send",
            ADDR_GAME_PET_SEND,
            pet_send_detour as *mut c_void,
            &PET_SEND_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        unsafe { install(producer, name, addr, detour, slot) };
    }
}

// ---------------------------------------------------------------------
// The two Lua bindings

type LuaCFunction = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

/// Run a binding inside a press scope. A Lua error unwinds through here as
/// a C++ exception; the scope's drop reports the press on the way out.
unsafe fn binding(l: *mut c_void, source: Source, trampoline: &OnceLock<usize>) -> i32 {
    let Some(&t) = trampoline.get() else {
        return 0;
    };
    let original: LuaCFunction = unsafe { std::mem::transmute(t) };
    let _scope = guarded(|| PressScope::begin(source));
    unsafe { original(l) }
}

pub(super) unsafe extern "C-unwind" fn use_action_detour(l: *mut c_void) -> i32 {
    unsafe { binding(l, Source::Hotbar, &USE_ACTION_TRAMPOLINE) }
}

pub(super) unsafe extern "C-unwind" fn use_ability_detour(l: *mut c_void) -> i32 {
    unsafe { binding(l, Source::Lua, &USE_ABILITY_TRAMPOLINE) }
}

// ---------------------------------------------------------------------
// The chain

type SlotFn = unsafe extern "C-unwind" fn(i32, u32) -> u32;

/// `FUN_00ad9580(actionId, self)`. `self` is a pushed dword whose low byte
/// is the bool (`0x00aa954f`).
pub(super) unsafe extern "C-unwind" fn slot_detour(action_id: i32, self_flag: u32) -> u32 {
    let Some(&t) = SLOT_TRAMPOLINE.get() else {
        return 0;
    };
    let original: SlotFn = unsafe { std::mem::transmute(t) };
    let held = guarded(|| {
        with_press(|p| p.slot_entered(action_id, self_flag & 0xff != 0))?;
        Some(layout::slot_action(&LiveMem, action_id))
    })
    .flatten();
    let r = unsafe { original(action_id, self_flag) };
    if let Some(action) = held {
        report(
            guarded(|| with_press(|p| p.slot_left(action)))
                .flatten()
                .unwrap_or_default(),
        );
    }
    r
}

type LookupFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32, i32) -> u32;

/// `FUN_00d2afc0(set, abilityId, targetId)`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn lookup_detour(
    set: *mut c_void,
    ability_id: i32,
    target_id: i32,
) -> u32 {
    let Some(&t) = LOOKUP_TRAMPOLINE.get() else {
        return 0;
    };
    let original: LookupFn = unsafe { std::mem::transmute(t) };
    report(
        guarded(|| with_press(|p| p.lookup_entered(ability_id, target_id)))
            .flatten()
            .unwrap_or_default(),
    );
    let r = unsafe { original(set, ability_id, target_id) };
    report(
        guarded(|| with_press(|p| p.lookup_left()))
            .flatten()
            .unwrap_or_default(),
    );
    r
}

type SendBuilderFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, i32) -> u32;

/// `FUN_00d2ae40(set, record, targetId)`. Reached from `FUN_00d2afc0` only
/// with a record; also called from `0x00d2b009`, outside any press, which
/// passes straight through.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn send_builder_detour(
    set: *mut c_void,
    rec: *mut c_void,
    target_id: i32,
) -> u32 {
    let Some(&t) = SEND_BUILDER_TRAMPOLINE.get() else {
        return 0;
    };
    let original: SendBuilderFn = unsafe { std::mem::transmute(t) };
    let _ = guarded(|| {
        let ground = LiveMem
            .u32_at((rec as u32).wrapping_add(record::TARGETING))
            .map(|v| v == record::TARGETING_GROUND);
        if let Some(pending) = with_press(|p| p.send_entered(ground, now_ms())) {
            with_pending(|t| t.push(pending));
        }
    });
    unsafe { original(set, rec, target_id) }
}

type PetActionFn = unsafe extern "thiscall-unwind" fn(*mut c_void, u32) -> u32;

/// `PetAbilityAction::execute(self)`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn pet_action_detour(
    this: *mut c_void,
    self_flag: u32,
) -> u32 {
    let Some(&t) = PET_ACTION_TRAMPOLINE.get() else {
        return 0;
    };
    let original: PetActionFn = unsafe { std::mem::transmute(t) };
    report(
        guarded(|| {
            let a = this as u32;
            let ability = LiveMem.u32_at(a.wrapping_add(action::ABILITY_ID))? as i32;
            let pet = LiveMem.u32_at(a.wrapping_add(action::PET_ID))? as i32;
            with_press(|p| p.pet_entered(ability, pet))
        })
        .flatten()
        .unwrap_or_default(),
    );
    let r = unsafe { original(this, self_flag) };
    report(
        guarded(|| with_press(|p| p.pet_left()))
            .flatten()
            .unwrap_or_default(),
    );
    r
}

type PetSendFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32, i32) -> u32;

/// The GamePet send `(abilityId, targetId)`. Its three gates are read
/// before it runs; the second caller (`0x00ad7bfe`) is outside any press
/// and passes straight through.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn pet_send_detour(
    pet: *mut c_void,
    ability_id: i32,
    target_id: i32,
) -> u32 {
    let Some(&t) = PET_SEND_TRAMPOLINE.get() else {
        return 0;
    };
    let original: PetSendFn = unsafe { std::mem::transmute(t) };
    let outs = guarded(|| {
        let gate = layout::pet_gate(&LiveMem, pet as u32, ability_id);
        let (outs, pending) = with_press(|p| p.pet_send_entered(gate, target_id, now_ms()))?;
        if let Some(pending) = pending {
            with_pending(|t| t.push(pending));
        }
        Some(outs)
    })
    .flatten()
    .unwrap_or_default();
    report(outs);
    unsafe { original(pet, ability_id, target_id) }
}
