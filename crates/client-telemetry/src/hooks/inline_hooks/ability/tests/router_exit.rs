//! The router's exits: `startEntityMessage` / `startProxyMessage` mark an
//! allowlisted router call as sent.

use super::*;

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
