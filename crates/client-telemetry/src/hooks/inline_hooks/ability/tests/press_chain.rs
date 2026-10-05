//! The press chain: every AB-C2 detour over same-ABI stand-ins wired like
//! the game's chain, driven through each branch, then removed.

use super::*;

const KNOWN: u32 = 0;
const UNKNOWN: u32 = 1;
const BAD_ARGS: u32 = 2;
const GROUND: u32 = 3;
const PET_MISSING: u32 = 4;
const PET_FOUND: u32 = 5;

static SCENARIO: AtomicU32 = AtomicU32::new(KNOWN);
static SEEN: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
/// Address of the fake ability record (`+0x48` is the targeting mode).
static RECORD: AtomicUsize = AtomicUsize::new(0);
/// Address of the fake pet (`+0x38` flags, `+0x174` ability set).
static PET: AtomicUsize = AtomicUsize::new(0);

// Every stand-in has the ABI read from the image (2026-10-04): the two
// thunks `cdecl int(lua_State*)` with plain `ret`; `FUN_00ad9580` `cdecl`
// (two stack args, plain `ret`); `FUN_00d2afc0`, `FUN_00d2ae40` and the
// GamePet send `thiscall` with two stack args (`ret 8`);
// `PetAbilityAction::execute` `thiscall` with one (`ret 4`). Each returns
// a marker mixed with its callee's EAX, so the tests also prove that the
// detours hand the original's EAX back unchanged.

/// What the slot stand-in returned to the `useAction` stand-in.
static SLOT_RET: AtomicU32 = AtomicU32::new(0);

#[inline(never)]
unsafe extern "C-unwind" fn use_action_standin(l: *mut c_void) -> i32 {
    SEEN[0].store(l as u32, Ordering::SeqCst);
    if SCENARIO.load(Ordering::SeqCst) == BAD_ARGS {
        // tolua_error: a Lua error is a C++ throw through the binding.
        panic!("#ferror in function 'useAction'.");
    }
    let next: unsafe extern "C-unwind" fn(i32, u32) -> u32 = black_box(slot_standin);
    SLOT_RET.store(unsafe { next(3, 0xCC00) }, Ordering::SeqCst);
    SEEN[0].fetch_add(1, Ordering::SeqCst);
    0
}

#[inline(never)]
unsafe extern "C-unwind" fn use_ability_standin(l: *mut c_void) -> i32 {
    SEEN[1].store(l as u32, Ordering::SeqCst);
    let next: unsafe extern "thiscall-unwind" fn(*mut c_void, i32, i32) -> u32 =
        black_box(lookup_standin);
    unsafe { next(0x5E7 as *mut c_void, 4000, 99) };
    SEEN[1].fetch_add(1, Ordering::SeqCst);
    0
}

#[inline(never)]
unsafe extern "C-unwind" fn slot_standin(action_id: i32, self_flag: u32) -> u32 {
    SEEN[2].store(action_id as u32 ^ self_flag, Ordering::SeqCst);
    let child = match SCENARIO.load(Ordering::SeqCst) {
        PET_MISSING | PET_FOUND => {
            let next: unsafe extern "thiscall-unwind" fn(*mut c_void, u32) -> u32 =
                black_box(pet_action_standin);
            let action = [0u32, 0, 0, 11, 900];
            unsafe { next(action.as_ptr() as *mut c_void, 0) }
        }
        _ => {
            let next: unsafe extern "thiscall-unwind" fn(*mut c_void, i32, i32) -> u32 =
                black_box(lookup_standin);
            unsafe { next(0x5E7 as *mut c_void, 597, 1234) }
        }
    };
    SEEN[2].fetch_add(1, Ordering::SeqCst);
    black_box(0x2_0000 ^ child)
}

#[inline(never)]
unsafe extern "thiscall-unwind" fn lookup_standin(
    set: *mut c_void,
    ability: i32,
    target: i32,
) -> u32 {
    SEEN[3].store(
        set as u32 ^ ability as u32 ^ target as u32,
        Ordering::SeqCst,
    );
    let mut child = 0;
    if matches!(SCENARIO.load(Ordering::SeqCst), KNOWN | GROUND) {
        let next: unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, i32) -> u32 =
            black_box(send_builder_standin);
        child = unsafe { next(set, RECORD.load(Ordering::SeqCst) as *mut c_void, target) };
    }
    SEEN[3].fetch_add(1, Ordering::SeqCst);
    black_box(0x1000 ^ child)
}

#[inline(never)]
unsafe extern "thiscall-unwind" fn send_builder_standin(
    set: *mut c_void,
    rec: *mut c_void,
    t: i32,
) -> u32 {
    SEEN[4].store(set as u32 ^ rec as u32 ^ t as u32, Ordering::SeqCst);
    SEEN[5].fetch_add(1, Ordering::SeqCst);
    black_box(0x5B)
}

#[inline(never)]
unsafe extern "thiscall-unwind" fn pet_action_standin(this: *mut c_void, self_flag: u32) -> u32 {
    SEEN[6].store(this as u32 ^ self_flag, Ordering::SeqCst);
    let mut child = 0;
    if SCENARIO.load(Ordering::SeqCst) == PET_FOUND {
        let next: unsafe extern "thiscall-unwind" fn(*mut c_void, i32, i32) -> u32 =
            black_box(pet_send_standin);
        child = unsafe { next(PET.load(Ordering::SeqCst) as *mut c_void, 11, 5) };
    }
    SEEN[6].fetch_add(1, Ordering::SeqCst);
    black_box(0x300 ^ child)
}

#[inline(never)]
unsafe extern "thiscall-unwind" fn pet_send_standin(
    pet: *mut c_void,
    ability: i32,
    target: i32,
) -> u32 {
    SEEN[7].store(
        pet as u32 ^ ability as u32 ^ target as u32,
        Ordering::SeqCst,
    );
    SEEN[7].fetch_add(1, Ordering::SeqCst);
    black_box(0x77)
}

fn run(scenario: u32, f: impl FnOnce()) -> Vec<Out> {
    SCENARIO.store(scenario, Ordering::SeqCst);
    let _ = capture::take();
    f();
    capture::take()
}

fn press_hotbar() {
    let f: unsafe extern "C-unwind" fn(*mut c_void) -> i32 = black_box(use_action_standin);
    assert_eq!(unsafe { f(0x1111u32 as *mut c_void) }, 0);
}

/// Every press-chain detour, installed, driven through each branch the
/// chain can take, then removed.
#[test]
fn the_press_chain_patch_call_unpatch() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut record = vec![0u32; 0x40];
    record[0x48 / 4] = 0; // not a ground ability
    RECORD.store(record.as_mut_ptr() as usize, Ordering::SeqCst);
    let pet = vec![0u32; 0x60]; // +0x38 flags: bit 0x400 clear
    PET.store(pet.as_ptr() as usize, Ordering::SeqCst);

    let hooks = unsafe {
        [
            patch(
                use_action_standin as *const () as usize,
                use_action_detour as *const () as usize,
                &USE_ACTION_TRAMPOLINE,
            ),
            patch(
                use_ability_standin as *const () as usize,
                use_ability_detour as *const () as usize,
                &USE_ABILITY_TRAMPOLINE,
            ),
            patch(
                slot_standin as *const () as usize,
                slot_detour as *const () as usize,
                &SLOT_TRAMPOLINE,
            ),
            patch(
                lookup_standin as *const () as usize,
                lookup_detour as *const () as usize,
                &LOOKUP_TRAMPOLINE,
            ),
            patch(
                send_builder_standin as *const () as usize,
                send_builder_detour as *const () as usize,
                &SEND_BUILDER_TRAMPOLINE,
            ),
            patch(
                pet_action_standin as *const () as usize,
                pet_action_detour as *const () as usize,
                &PET_ACTION_TRAMPOLINE,
            ),
            patch(
                pet_send_standin as *const () as usize,
                pet_send_detour as *const () as usize,
                &PET_SEND_TRAMPOLINE,
            ),
        ]
    };

    // Known ability: one press row, the original chain ran to the post,
    // and a pending useAbility waits for the router.
    let before = SEEN[5].load(Ordering::SeqCst);
    let outs = run(KNOWN, press_hotbar);
    assert_eq!(targets(&outs), vec![TARGET_PRESS], "{outs:#?}");
    assert_eq!(field(&outs[0].fields, "slot"), Some(&json!(3)));
    assert_eq!(field(&outs[0].fields, "ability_id"), Some(&json!(597)));
    assert_eq!(field(&outs[0].fields, "target_id"), Some(&json!(1234)));
    assert_eq!(SEEN[5].load(Ordering::SeqCst), before + 1, "original ran");
    assert_eq!(
        SEEN[2].load(Ordering::SeqCst),
        (3 ^ 0xCC00) + 1,
        "args kept"
    );
    assert_eq!(
        SLOT_RET.load(Ordering::SeqCst),
        0x2_0000 ^ 0x1000 ^ 0x5B,
        "EAX passes back through the slot, lookup and send-builder detours"
    );
    let pending = with_pending(|t| t.take("useAbility", Some(597), now_ms()));
    let press_id = field(&outs[0].fields, "press_id").and_then(|v| v.as_u64());
    assert_eq!(pending.map(|p| u64::from(p.press_id)), press_id);

    // Ground ability: the pending send is the ground method.
    record[0x48 / 4] = 3;
    let outs = run(GROUND, press_hotbar);
    assert_eq!(targets(&outs), vec![TARGET_PRESS]);
    assert!(with_pending(|t| t.take("useAbilityOnGroundTarget", Some(597), now_ms())).is_some());
    record[0x48 / 4] = 0;

    // Row 5: not in the AbilitySet.
    let outs = run(UNKNOWN, press_hotbar);
    assert_eq!(targets(&outs), vec![TARGET_PRESS, TARGET_DROPPED]);
    assert_eq!(reason(&outs).as_deref(), Some("not_known"));

    // Row 2: the binding raised a Lua error before the slot step. The
    // press scope reports on the way out and the error still propagates.
    let outs = run(BAD_ARGS, || {
        let caught = std::panic::catch_unwind(press_hotbar);
        assert!(caught.is_err(), "the Lua error reaches the caller");
    });
    assert_eq!(targets(&outs), vec![TARGET_PRESS, TARGET_DROPPED]);
    assert_eq!(reason(&outs).as_deref(), Some("bad_args"));
    assert!(PRESS.with(|c| c.get()).is_none(), "scope restored");

    // The Lua binding goes straight to the lookup.
    let outs = run(UNKNOWN, || {
        let f: unsafe extern "C-unwind" fn(*mut c_void) -> i32 = black_box(use_ability_standin);
        unsafe { f(0x2222u32 as *mut c_void) };
    });
    assert_eq!(field(&outs[0].fields, "source"), Some(&json!("lua")));
    assert_eq!(field(&outs[0].fields, "ability_id"), Some(&json!(4000)));
    assert_eq!(reason(&outs).as_deref(), Some("not_known"));

    // Row 4: the pet does not resolve.
    let outs = run(PET_MISSING, press_hotbar);
    assert_eq!(targets(&outs), vec![TARGET_PRESS, TARGET_DROPPED]);
    assert_eq!(field(&outs[0].fields, "pet_id"), Some(&json!(900)));
    assert_eq!(reason(&outs).as_deref(), Some("pet_missing"));

    // The GamePet send's first gate (state bit 0x400 clear).
    let outs = run(PET_FOUND, press_hotbar);
    assert_eq!(reason(&outs).as_deref(), Some("pet_state_flag"));
    assert_eq!(
        SLOT_RET.load(Ordering::SeqCst),
        0x2_0000 ^ 0x300 ^ 0x77,
        "EAX passes back through the pet detours"
    );
    assert_eq!(
        SEEN[7].load(Ordering::SeqCst) & 1,
        1,
        "the original pet send ran"
    );

    // Unpatched: the chain runs and nothing is reported.
    drop(hooks);
    let outs = run(UNKNOWN, press_hotbar);
    assert!(outs.is_empty(), "{outs:#?}");
    assert_eq!(SLOT_RET.load(Ordering::SeqCst), 0x2_0000 ^ 0x1000);
}
