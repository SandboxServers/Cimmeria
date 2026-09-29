//! The x86-only half of the [actor seam](super::actors): the `SpawnActor` and
//! `DestroyActor` detours. Split out of `actors.rs` to keep both under the
//! size cap; the argument tables, the event shapes and the evidence live
//! there.

use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::OnceLock;

use super::actors::*;
use super::objects;
use crate::hooks::name_throttle::Decision;
use crate::hooks::sinks::emit::emit;
use crate::hooks::sinks::mem::process_reader;
use crate::hooks::sinks::throttle::SinkThrottle;

pub(in crate::hooks) static SPAWN_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(in crate::hooks) static DESTROY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

static SPAWN_THROTTLE: SinkThrottle = SinkThrottle::new();
static DESTROY_THROTTLE: SinkThrottle = SinkThrottle::new();

fn name_of_class(class: usize) -> String {
    objects::object_name(&process_reader, class).unwrap_or_else(|| "<unknown>".to_string())
}

/// Report one finished spawn.
fn report_spawn(
    class: *const c_void,
    location: *const c_void,
    ret: *mut c_void,
    flags: (bool, bool),
) {
    let ok = !ret.is_null();
    let class_name = name_of_class(class as usize);
    let Decision::Emit { suppressed } = SPAWN_THROTTLE.check(&spawn_key(&class_name, ok)) else {
        return;
    };
    let info = SpawnInfo {
        actor: ok
            .then(|| objects::object_name(&process_reader, ret as usize))
            .flatten(),
        location: objects::read_vec3(&process_reader, location as usize),
        no_collision_fail: flags.0,
        no_fail: flags.1,
        class: class_name,
        ok,
    };
    emit(
        SPAWN_TARGET,
        spawn_level(ok),
        "engine.spawn_actor",
        spawn_fields(&info, suppressed),
    );
}

/// `UWorld::SpawnActor`: eleven stack arguments, `ret 0x2c`.
#[allow(improper_ctypes_definitions)]
pub(in crate::hooks) unsafe extern "thiscall-unwind" fn spawn_detour(
    this: *mut c_void,
    class: *mut c_void,
    name_index: i32,
    name_number: i32,
    location: *const c_void,
    rotation: *const c_void,
    template: *mut c_void,
    no_collision_fail: i32,
    remote_owned: i32,
    owner: *mut c_void,
    instigator: *mut c_void,
    no_fail: i32,
) -> *mut c_void {
    let Some(t) = SPAWN_TRAMPOLINE.get() else {
        // No trampoline means the install failed and the patch was never
        // applied; returning NULL is SpawnActor's own failure answer.
        return std::ptr::null_mut();
    };
    let original: unsafe extern "thiscall-unwind" fn(
        *mut c_void,
        *mut c_void,
        i32,
        i32,
        *const c_void,
        *const c_void,
        *mut c_void,
        i32,
        i32,
        *mut c_void,
        *mut c_void,
        i32,
    ) -> *mut c_void = unsafe { std::mem::transmute(*t) };
    let ret = original(
        this,
        class,
        name_index,
        name_number,
        location,
        rotation,
        template,
        no_collision_fail,
        remote_owned,
        owner,
        instigator,
        no_fail,
    );
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        report_spawn(class, location, ret, (no_collision_fail != 0, no_fail != 0));
    }));
    ret
}

/// `UWorld::DestroyActor`: three stack arguments, `ret 0xc`.
#[allow(improper_ctypes_definitions)]
pub(in crate::hooks) unsafe extern "thiscall-unwind" fn destroy_detour(
    this: *mut c_void,
    actor: *mut c_void,
    net_force: i32,
    should_modify_level: i32,
) -> u32 {
    // Identify the actor before the call: it is marked pending kill by
    // it, and the names are read while the object is certainly intact.
    let ident = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let read = &process_reader;
        let a = actor as usize;
        (
            objects::class_name_of(read, a).unwrap_or_else(|| "<unknown>".to_string()),
            objects::object_name(read, a).unwrap_or_else(|| "<unknown>".to_string()),
        )
    }))
    .ok();

    let Some(t) = DESTROY_TRAMPOLINE.get() else {
        return 0;
    };
    let original: unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, i32, i32) -> u32 =
        unsafe { std::mem::transmute(*t) };
    let ret = original(this, actor, net_force, should_modify_level);

    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let Some((class, name)) = ident else {
            return;
        };
        let done = ret & 0xff != 0;
        let Decision::Emit { suppressed } = DESTROY_THROTTLE.check(&destroy_key(&class, done))
        else {
            return;
        };
        emit(
            DESTROY_TARGET,
            destroy_level(done),
            "engine.destroy_actor",
            destroy_fields(&class, &name, done, net_force != 0, suppressed),
        );
    }));
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::sinks::emit::take_captured;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEEN: [AtomicUsize; 12] = [const { AtomicUsize::new(0) }; 12];
    static DSEEN: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

    unsafe extern "thiscall-unwind" fn fake_spawn(
        this: *mut c_void,
        class: *mut c_void,
        ni: i32,
        nn: i32,
        loc: *const c_void,
        rot: *const c_void,
        tpl: *mut c_void,
        ncf: i32,
        ro: i32,
        owner: *mut c_void,
        inst: *mut c_void,
        nf: i32,
    ) -> *mut c_void {
        let vals = [
            this as usize,
            class as usize,
            ni as usize,
            nn as usize,
            loc as usize,
            rot as usize,
            tpl as usize,
            ncf as usize,
            ro as usize,
            owner as usize,
            inst as usize,
            nf as usize,
        ];
        for (slot, v) in SEEN.iter().zip(vals) {
            slot.store(v, Ordering::SeqCst);
        }
        // Fail when the class argument is odd.
        if class as usize & 1 == 1 {
            std::ptr::null_mut()
        } else {
            0x7777 as *mut c_void
        }
    }

    unsafe extern "thiscall-unwind" fn fake_destroy(
        this: *mut c_void,
        actor: *mut c_void,
        nf: i32,
        sml: i32,
    ) -> u32 {
        DSEEN[0].store(this as usize, Ordering::SeqCst);
        DSEEN[1].store(actor as usize, Ordering::SeqCst);
        DSEEN[2].store(nf as usize, Ordering::SeqCst);
        DSEEN[3].store(sml as usize, Ordering::SeqCst);
        // Only the low byte is a UBOOL; garbage above it must be ignored.
        0xAB00_0001
    }

    /// One test owns both trampolines. All eleven arguments reach the
    /// original in order, the return value comes back untouched, and a
    /// failed spawn is reported as a warning.
    #[test]
    fn spawn_and_destroy_forward_every_argument_and_report() {
        SPAWN_TRAMPOLINE
            .set(fake_spawn as *const () as usize)
            .expect("only this test sets it");
        DESTROY_TRAMPOLINE
            .set(fake_destroy as *const () as usize)
            .expect("only this test sets it");
        let _ = take_captured();

        let loc = [1.0f32, 2.0, 3.0];
        let ret = unsafe {
            spawn_detour(
                0x10 as *mut c_void,
                0x2000 as *mut c_void, // even: succeeds
                5,
                6,
                loc.as_ptr().cast(),
                0x40 as *const c_void,
                0x50 as *mut c_void,
                1,
                0,
                0x70 as *mut c_void,
                0x80 as *mut c_void,
                1,
            )
        };
        assert_eq!(ret as usize, 0x7777);
        let want = [
            0x10,
            0x2000,
            5,
            6,
            loc.as_ptr() as usize,
            0x40,
            0x50,
            1,
            0,
            0x70,
            0x80,
            1,
        ];
        for (i, w) in want.iter().enumerate() {
            assert_eq!(SEEN[i].load(Ordering::SeqCst), *w, "argument {i}");
        }
        let events = take_captured();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, SPAWN_TARGET);
        assert_eq!(events[0].level, "debug");
        assert_eq!(events[0].get("ok"), Some(&json!(true)));
        assert_eq!(events[0].get("no_collision_fail"), Some(&json!(true)));
        assert_eq!(events[0].get("location"), Some(&json!([1.0, 2.0, 3.0])));

        // Odd class pointer: the fake returns NULL.
        let null = unsafe {
            spawn_detour(
                std::ptr::null_mut(),
                0x2001 as *mut c_void,
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        };
        assert!(null.is_null());
        let events = take_captured();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, "warn");
        assert_eq!(events[0].get("ok"), Some(&json!(false)));

        let r = unsafe { destroy_detour(0x10 as *mut c_void, 0x99 as *mut c_void, 1, 0) };
        assert_eq!(r, 0xAB00_0001, "the return value is passed through");
        assert_eq!(DSEEN[1].load(Ordering::SeqCst), 0x99);
        assert_eq!(DSEEN[2].load(Ordering::SeqCst), 1);
        let events = take_captured();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, DESTROY_TARGET);
        // Low byte 1 = destroyed.
        assert_eq!(events[0].get("destroyed"), Some(&json!(true)));
        assert_eq!(events[0].get("net_force"), Some(&json!(true)));
    }
}
