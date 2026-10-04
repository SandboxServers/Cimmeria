//! The Mercury sequence join: `Channel::send`, `Nub::send` and the reliable
//! counter, over stand-ins with the real counter body.

use super::*;

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
