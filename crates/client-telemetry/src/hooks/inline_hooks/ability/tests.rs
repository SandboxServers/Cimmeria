//! Patch, call, unpatch: every detour here is installed with MinHook on a
//! stand-in function of the same ABI (the mechanism `install_one` uses on
//! the client), called through the stand-in, then removed.
//!
//! The stand-ins are wired the way the game's chain is (the `useAction`
//! stand-in calls the slot stand-in, which calls the lookup stand-in, and
//! so on), so each test drives the real detours in their real order. A
//! trampoline slot is a `OnceLock`, set once per process, so each detour is
//! installed by exactly one test.

use std::ffi::c_void;
use std::hint::black_box;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use super::press_detours::*;
use super::route::test_access::{
    enter_route, leave_route, START_ENTITY_TRAMPOLINE, START_PROXY_TRAMPOLINE,
};
use super::route::{start_entity_detour, start_proxy_detour};
use super::seq::*;
use super::*;
use crate::hooks::ability_trace::capture;
use crate::hooks::ability_trace::seq_join::SentTag;
use crate::hooks::ability_trace::{field, Out, TARGET_DROPPED, TARGET_PRESS, TARGET_SENT_SEQ};

/// The MinHook tests share the pending and join tables.
static SERIAL: Mutex<()> = Mutex::new(());

/// Install `detour` over `target` with MinHook, publish the trampoline
/// into `slot`, and return a guard that removes the hook.
struct Patched(usize);

unsafe fn patch(target: usize, detour: usize, slot: &OnceLock<usize>) -> Patched {
    unsafe {
        let s = minhook_sys::MH_Initialize();
        assert!(s == minhook_sys::MH_OK || s == minhook_sys::MH_ERROR_ALREADY_INITIALIZED);
        let mut tramp: *mut c_void = std::ptr::null_mut();
        let s =
            minhook_sys::MH_CreateHook(target as *mut c_void, detour as *mut c_void, &mut tramp);
        assert_eq!(s, minhook_sys::MH_OK, "create 0x{target:x}");
        slot.set(tramp as usize)
            .expect("each trampoline slot is set by one test only");
        assert_eq!(
            minhook_sys::MH_EnableHook(target as *mut c_void),
            minhook_sys::MH_OK
        );
    }
    Patched(target)
}

impl Drop for Patched {
    fn drop(&mut self) {
        unsafe {
            minhook_sys::MH_DisableHook(self.0 as *mut c_void);
            minhook_sys::MH_RemoveHook(self.0 as *mut c_void);
        }
    }
}

fn targets(outs: &[Out]) -> Vec<&'static str> {
    outs.iter().map(|o| o.target).collect()
}

fn reason(outs: &[Out]) -> Option<String> {
    outs.iter()
        .find(|o| o.target == TARGET_DROPPED)
        .and_then(|o| field(&o.fields, "reason"))
        .and_then(|v| v.as_str().map(str::to_owned))
}

// ---------------------------------------------------------------------
// The press chain's stand-ins

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

// ---------------------------------------------------------------------
// The router's start*Message stand-ins

#[inline(never)]
unsafe extern "thiscall-unwind" fn start_entity_standin(
    conn: *mut c_void,
    msg: u32,
    id: u32,
) -> u32 {
    black_box(conn as u32 ^ msg ^ id).wrapping_add(1)
}

#[inline(never)]
unsafe extern "thiscall-unwind" fn start_proxy_standin(conn: *mut c_void, msg: u32) -> u32 {
    black_box(conn as u32 ^ msg).wrapping_add(2)
}

#[test]
fn the_start_message_hooks_mark_the_router_call() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let entity: unsafe extern "thiscall-unwind" fn(*mut c_void, u32, u32) -> u32 =
        black_box(start_entity_standin);
    let proxy: unsafe extern "thiscall-unwind" fn(*mut c_void, u32) -> u32 =
        black_box(start_proxy_standin);
    let hooks = unsafe {
        [
            patch(
                start_entity_standin as *const () as usize,
                start_entity_detour as *const () as usize,
                &START_ENTITY_TRAMPOLINE,
            ),
            patch(
                start_proxy_standin as *const () as usize,
                start_proxy_detour as *const () as usize,
                &START_PROXY_TRAMPOLINE,
            ),
        ]
    };
    // Outside the router: passed through, nothing marked.
    assert_eq!(
        unsafe { entity(0x10 as *mut c_void, 0x44, 7) },
        (0x10 ^ 0x44 ^ 7) + 1
    );
    enter_route();
    assert!(!leave_route());
    // Inside: the stream pointer comes back untouched and the call is seen.
    enter_route();
    assert_eq!(
        unsafe { entity(0x10 as *mut c_void, 0x44, 7) },
        (0x10 ^ 0x44 ^ 7) + 1
    );
    assert!(leave_route());
    enter_route();
    assert_eq!(
        unsafe { proxy(0x20 as *mut c_void, 0x12) },
        (0x20 ^ 0x12) + 2
    );
    assert!(leave_route());
    drop(hooks);
    enter_route();
    unsafe { proxy(0x20 as *mut c_void, 0x12) };
    assert!(!leave_route(), "unpatched");
}

// ---------------------------------------------------------------------
// The sequence join's stand-ins

/// The fake bundle the channel stand-in hands to the "network thread".
static QUEUED: AtomicUsize = AtomicUsize::new(0);
/// Whether the channel stand-in detaches its bundle.
static DETACH: AtomicU32 = AtomicU32::new(1);
static CI: AtomicUsize = AtomicUsize::new(0);

#[inline(never)]
unsafe extern "thiscall-unwind" fn channel_send_standin(channel: *mut c_void) -> u32 {
    let slot = (channel as usize + 0x28) as *mut u32;
    if DETACH.load(Ordering::SeqCst) == 1 {
        let b = unsafe { slot.read() };
        QUEUED.store(b as usize, Ordering::SeqCst);
        unsafe { slot.write(0) };
        return 1;
    }
    black_box(0)
}

/// The real counter's body: `eax = [ecx+0x4c]; [ecx+0x4c] = (eax+1) &
/// 0x0fffffff`.
#[inline(never)]
unsafe extern "thiscall-unwind" fn seq_next_standin(ci: *mut c_void) -> u32 {
    let p = (ci as usize + 0x4c) as *mut u32;
    let v = unsafe { p.read() };
    unsafe { p.write(v.wrapping_add(1) & 0x0fff_ffff) };
    black_box(v)
}

#[inline(never)]
unsafe extern "thiscall-unwind" fn nub_send_standin(
    nub: *mut c_void,
    addr: u32,
    bundle: *mut c_void,
    channel: u32,
) -> u32 {
    let next: unsafe extern "thiscall-unwind" fn(*mut c_void) -> u32 = black_box(seq_next_standin);
    let ci = CI.load(Ordering::SeqCst) as *mut c_void;
    for _ in 0..3 {
        unsafe { next(ci) };
    }
    black_box(nub as u32 ^ addr ^ bundle as u32 ^ channel)
}

#[test]
fn the_seq_join_patch_call_unpatch() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut channel = vec![0u32; 0x20];
    let mut ci = vec![0u32; 0x20];
    ci[0x4c / 4] = 0x0fff_fffe; // two before the 28-bit wrap
    CI.store(ci.as_mut_ptr() as usize, Ordering::SeqCst);
    let bundle = 0x00B0_0B00u32;
    let send: unsafe extern "thiscall-unwind" fn(*mut c_void) -> u32 =
        black_box(channel_send_standin);
    let nub: unsafe extern "thiscall-unwind" fn(*mut c_void, u32, *mut c_void, u32) -> u32 =
        black_box(nub_send_standin);

    let hooks = unsafe {
        [
            patch(
                channel_send_standin as *const () as usize,
                channel_send_detour as *const () as usize,
                &CHANNEL_SEND_TRAMPOLINE,
            ),
            patch(
                nub_send_standin as *const () as usize,
                nub_send_detour as *const () as usize,
                &NUB_SEND_TRAMPOLINE,
            ),
            patch(
                seq_next_standin as *const () as usize,
                seq_next_detour as *const () as usize,
                &SEQ_NEXT_TRAMPOLINE,
            ),
        ]
    };
    let _ = capture::take();
    with_joiner(|j| {
        j.note_sent(SentTag {
            send_id: 41,
            press_id: Some(40),
            method: "useAbility",
            ability_id: Some(597),
        })
    });

    // Nothing to send yet: the tag goes back.
    DETACH.store(0, Ordering::SeqCst);
    channel[0x28 / 4] = bundle;
    assert_eq!(unsafe { send(channel.as_mut_ptr() as *mut c_void) }, 0);
    assert!(with_joiner(|j| j.has_unsent()), "untagged");

    // Detached and sent: the network thread's sequence numbers come back
    // as one range, across the wrap, and the counter still counts.
    DETACH.store(1, Ordering::SeqCst);
    assert_eq!(unsafe { send(channel.as_mut_ptr() as *mut c_void) }, 1);
    assert_eq!(QUEUED.load(Ordering::SeqCst), bundle as usize);
    let r = unsafe { nub(0x1111 as *mut c_void, 0x2, bundle as *mut c_void, 0x4) };
    assert_eq!(
        r,
        0x1111 ^ 2 ^ bundle ^ 4,
        "Nub::send's result is untouched"
    );
    assert_eq!(ci[0x4c / 4], 1, "three packets: fffffe, ffffff, 0");
    let outs = capture::take();
    assert_eq!(targets(&outs), vec![TARGET_SENT_SEQ]);
    let f = &outs[0].fields;
    assert_eq!(field(f, "send_id"), Some(&json!(41)));
    assert_eq!(field(f, "press_id"), Some(&json!(40)));
    assert_eq!(field(f, "mercury_seq_first"), Some(&json!(0x0fff_fffe)));
    assert_eq!(field(f, "mercury_seq_last"), Some(&json!(0)));
    assert_eq!(field(f, "packets"), Some(&json!(3)));

    // An untagged bundle records nothing.
    unsafe { nub(0x1111 as *mut c_void, 0x2, 0xDEAD as *mut c_void, 0x4) };
    assert!(capture::take().is_empty());

    drop(hooks);
    with_joiner(|j| {
        j.note_sent(SentTag {
            send_id: 42,
            press_id: None,
            method: "trainAbility",
            ability_id: Some(1),
        })
    });
    channel[0x28 / 4] = bundle;
    unsafe { send(channel.as_mut_ptr() as *mut c_void) };
    unsafe { nub(0x1111 as *mut c_void, 0x2, bundle as *mut c_void, 0x4) };
    assert!(capture::take().is_empty(), "unpatched");
    assert!(with_joiner(|j| j.has_unsent()), "never tagged");
    with_joiner(|j| {
        j.tag_bundle(1);
        j.take_bundle(1);
    });
}

// ---------------------------------------------------------------------
// The router probe, against a hand-built `MethodDescription`

/// An MSVC `std::string` holding `s` inline (`s.len() < 16`).
fn msvc_string(s: &str) -> [u8; 0x1c] {
    let mut o = [0u8; 0x1c];
    o[4..4 + s.len()].copy_from_slice(s.as_bytes());
    o[0x14..0x18].copy_from_slice(&(s.len() as u32).to_le_bytes());
    o[0x18..0x1c].copy_from_slice(&15u32.to_le_bytes());
    o
}

/// A send the router reached claims the waiting press and goes to the
/// join; with no bag (a null `args`) every argument is reported missing
/// rather than read. The bag readers are game code, so this test cannot
/// call them; their decode is covered against synthetic bags.
#[test]
fn a_reached_router_call_is_sent_with_its_press() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let names = [msvc_string("AbilityID"), msvc_string("TargetID")];
    let mut desc = vec![0u8; 0x50];
    desc[..0x1c].copy_from_slice(&msvc_string("useAbility"));
    let begin = names.as_ptr() as u32;
    desc[0x34..0x38].copy_from_slice(&begin.to_le_bytes());
    desc[0x38..0x3c].copy_from_slice(&(begin + 2 * 0x1c).to_le_bytes());
    desc[0x44..0x48].copy_from_slice(&0x44u32.to_le_bytes());
    desc[0x48..0x4c].copy_from_slice(&u32::MAX.to_le_bytes());

    with_pending(|t| {
        t.push(crate::hooks::ability_trace::press::PendingSend {
            press_id: 77,
            source: Source::Hotbar,
            method: "useAbility",
            ability_id: None,
            at_ms: now_ms(),
            ttl_ms: 5_000,
        })
    });
    let _ = capture::take();
    let probe = super::route::RouteProbe::begin(
        std::ptr::null_mut(),
        desc.as_mut_ptr() as *mut c_void,
        std::ptr::null_mut(),
    )
    .expect("useAbility is allowlisted");
    // What startEntityMessage's detour does inside the router.
    super::route::test_access::mark_reached();
    probe.finish();
    let outs = capture::take();
    assert_eq!(outs.len(), 1, "{outs:#?}");
    let f = &outs[0].fields;
    assert_eq!(outs[0].target, crate::hooks::ability_trace::TARGET_SENT);
    assert_eq!(field(f, "method"), Some(&json!("useAbility")));
    assert_eq!(field(f, "press_id"), Some(&json!(77)));
    assert_eq!(field(f, "msg_id"), Some(&json!(0x44)));
    assert_eq!(
        field(f, "args_missing"),
        Some(&json!(["AbilityID", "TargetID"]))
    );
    let send_id = field(f, "send_id").and_then(|v| v.as_u64()).unwrap();
    with_joiner(|j| {
        assert!(j.tag_bundle(0x77));
        let tags = j.take_bundle(0x77).unwrap();
        assert!(tags.iter().any(|t| u64::from(t.send_id) == send_id));
    });

    // A method outside the allowlist costs a name read and nothing else.
    desc[..0x1c].copy_from_slice(&msvc_string("createCharacter"));
    assert!(super::route::RouteProbe::begin(
        std::ptr::null_mut(),
        desc.as_mut_ptr() as *mut c_void,
        std::ptr::null_mut(),
    )
    .is_none());
}

use serde_json::json;
