//! The [`Engine`] in `SGW.exe`: `startEntityMessage` and the bundle's
//! `reserve`, called on the main thread under a structured-exception guard.

use core::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use super::{Engine, Refusal};
use crate::addresses::{
    BUNDLE_RESERVE, CONN_ONLINE, EXTENDED_METHOD_ID, GAME_ENTITY_MANAGER, GEM_LOCAL_PLAYER_ID,
    GEM_SERVER_CONNECTION, LOCAL_AVATAR, SERVER_CONNECTION_START_ENTITY_MESSAGE,
};
use crate::memory::{MemoryReader, ProcessMemory};

/// The game's main thread: the thread `FEngineLoop::Tick` runs on,
/// recorded by the `Tick` detour on its first frame. 0 until then, and a
/// send before it is refused as off the main thread.
pub(crate) static MAIN_THREAD_ID: AtomicU32 = AtomicU32::new(0);

/// Record the calling thread as the main thread, once.
pub(crate) fn note_main_thread() {
    if MAIN_THREAD_ID.load(Ordering::Relaxed) == 0 {
        // SAFETY: no preconditions.
        let id = unsafe { GetCurrentThreadId() };
        let _ = MAIN_THREAD_ID.compare_exchange(0, id, Ordering::Relaxed, Ordering::Relaxed);
    }
}

/// `ServerConnection::startEntityMessage`. It can throw (its prologue sets
/// up a C++ exception frame), so it gets the `-unwind` ABI, per issue
/// #915's rule. The structured-exception guard around the call is what
/// stops an exception: the i686 tests raise one through it.
type StartEntityMessage =
    unsafe extern "thiscall-unwind" fn(conn: *mut c_void, idx: u8, entity_id: u32) -> *mut c_void;

/// Bundle vtable `+0x10`, `BinaryOStream::reserve`.
type Reserve = unsafe extern "thiscall-unwind" fn(bundle: *mut c_void, n: i32) -> *mut u8;

/// Where a guarded send stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// `startEntityMessage` returned null.
    Start,
    /// `reserve` returned null for the sub-index byte.
    ReserveSubIndex,
    /// `reserve` returned null for the payload.
    ReservePayload,
}

impl Step {
    fn describe(self) -> &'static str {
        match self {
            Self::Start => "startEntityMessage returned no bundle",
            Self::ReserveSubIndex => "reserve(1) for the sub-index returned null",
            Self::ReservePayload => "reserve(n) for the payload returned null",
        }
    }
}

/// Sends through the engine at the addresses in [`crate::addresses`]. The
/// two addresses are fields so the i686 tests can point them at a fake
/// connection and a fake `startEntityMessage`.
pub(crate) struct EngineSender {
    /// Holds the `GameEntityManager*`.
    gem_slot: usize,
    /// `startEntityMessage`.
    start_entity_message: usize,
}

impl EngineSender {
    /// The live client.
    pub(crate) const fn live() -> Self {
        Self {
            gem_slot: GAME_ENTITY_MANAGER,
            start_entity_message: SERVER_CONNECTION_START_ENTITY_MESSAGE,
        }
    }

    /// The connection to send on: `[[gem_slot] + 8]`, online, with a local
    /// player. Read without faulting.
    fn connection(&self) -> Result<usize, Refusal> {
        let mem = ProcessMemory;
        let gem = mem
            .read_ptr(self.gem_slot)
            .ok_or(Refusal::Offline("no GameEntityManager"))?;
        let conn = mem
            .read_ptr(gem + GEM_SERVER_CONNECTION)
            .ok_or(Refusal::Offline("no ServerConnection"))?;
        if mem.read_u32(conn + CONN_ONLINE).unwrap_or(0) == 0 {
            return Err(Refusal::Offline("the ServerConnection is not online"));
        }
        if mem.read_u32(gem + GEM_LOCAL_PLAYER_ID).unwrap_or(0) == 0 {
            return Err(Refusal::Offline("no local player entity yet"));
        }
        Ok(conn)
    }
}

impl Engine for EngineSender {
    fn on_main_thread(&self) -> bool {
        let main = MAIN_THREAD_ID.load(Ordering::Relaxed);
        // SAFETY: no preconditions.
        main != 0 && main == unsafe { GetCurrentThreadId() }
    }

    fn send(&mut self, sub_index: u8, payload: &[u8]) -> Result<(), Refusal> {
        let conn = self.connection()?;
        // SAFETY: the address is `startEntityMessage` in this build (the
        // fingerprint gate checked its prologue) or a test's stand-in.
        let start =
            unsafe { core::mem::transmute::<usize, StartEntityMessage>(self.start_entity_message) };
        // Nothing inside the guard allocates, holds a lock or can panic: a
        // fault unwinds past it without running `Drop`.
        let guarded =
            microseh::try_seh(|| unsafe { write_message(start, conn, sub_index, payload) });
        match guarded {
            Ok(Ok(())) => Ok(()),
            Ok(Err(step)) => Err(Refusal::EngineError(step.describe().into())),
            Err(e) => Err(Refusal::EngineError(format!(
                "exception 0x{:08x} at 0x{:08x} ({})",
                e.raw_code(),
                e.address() as usize,
                e.code()
            ))),
        }
    }
}

/// Start the message and write the sub-index and payload into the bundle.
///
/// # Safety
///
/// `start` must be `startEntityMessage` (or a stand-in with its signature)
/// and `conn` an online `ServerConnection` it accepts. Main thread only.
unsafe fn write_message(
    start: StartEntityMessage,
    conn: usize,
    sub_index: u8,
    payload: &[u8],
) -> Result<(), Step> {
    let bundle = unsafe { start(conn as *mut c_void, EXTENDED_METHOD_ID, LOCAL_AVATAR) };
    if bundle.is_null() {
        return Err(Step::Start);
    }
    // SAFETY: a bundle is a C++ object whose first field is its vtable;
    // `reserve` is the slot the engine's own byte writer calls.
    let reserve = unsafe {
        let vtable = *bundle.cast::<*const usize>();
        core::mem::transmute::<usize, Reserve>(*vtable.add(BUNDLE_RESERVE / 4))
    };
    let p = unsafe { reserve(bundle, 1) };
    if p.is_null() {
        return Err(Step::ReserveSubIndex);
    }
    unsafe { p.write(sub_index) };
    if !payload.is_empty() {
        let p = unsafe { reserve(bundle, payload.len() as i32) };
        if p.is_null() {
            return Err(Step::ReservePayload);
        }
        // SAFETY: `reserve(n)` returned `n` writable bytes.
        unsafe { core::ptr::copy_nonoverlapping(payload.as_ptr(), p, payload.len()) };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The real send path on the i686 target, against a fake connection and
    //! a fake bundle in this process's memory.

    use super::*;
    use std::cell::RefCell;

    /// A stand-in for the engine's bundle: a vtable pointer, then the bytes
    /// written so far.
    #[repr(C)]
    struct FakeBundle {
        vtable: *const [usize; 5],
        bytes: Vec<u8>,
    }

    thread_local! {
        static STARTED: RefCell<Vec<(usize, u8, u32)>> = const { RefCell::new(Vec::new()) };
        static BUNDLE: RefCell<Option<Box<FakeBundle>>> = const { RefCell::new(None) };
        static SCRATCH: RefCell<Vec<Box<[u8]>>> = const { RefCell::new(Vec::new()) };
    }

    unsafe extern "thiscall-unwind" fn fake_reserve(this: *mut c_void, n: i32) -> *mut u8 {
        let bundle = unsafe { &mut *this.cast::<FakeBundle>() };
        let start = bundle.bytes.len();
        bundle.bytes.resize(start + n as usize, 0xCC);
        unsafe { bundle.bytes.as_mut_ptr().add(start) }
    }

    static VTABLE: [usize; 5] = [0, 0, 0, 0, 0];

    fn vtable() -> *const [usize; 5] {
        // Slot 4 (+0x10) is `reserve`; the rest must never be called.
        let table = Box::leak(Box::new(VTABLE));
        table[BUNDLE_RESERVE / 4] = fake_reserve as *const () as usize;
        table
    }

    unsafe extern "thiscall-unwind" fn fake_start(
        conn: *mut c_void,
        idx: u8,
        entity_id: u32,
    ) -> *mut c_void {
        STARTED.with(|s| s.borrow_mut().push((conn as usize, idx, entity_id)));
        BUNDLE.with(|b| {
            let mut bundle = Box::new(FakeBundle {
                vtable: vtable(),
                bytes: Vec::new(),
            });
            let p = (&mut *bundle as *mut FakeBundle).cast::<c_void>();
            *b.borrow_mut() = Some(bundle);
            p
        })
    }

    unsafe extern "thiscall-unwind" fn null_start(
        _conn: *mut c_void,
        _idx: u8,
        _entity_id: u32,
    ) -> *mut c_void {
        core::ptr::null_mut()
    }

    /// Raises an access violation, as a bad engine pointer would.
    unsafe extern "thiscall-unwind" fn faulting_start(
        _conn: *mut c_void,
        _idx: u8,
        _entity_id: u32,
    ) -> *mut c_void {
        unsafe { core::ptr::read_volatile(8 as *const *mut c_void) }
    }

    /// Raises the SEH exception MSVC's `throw` raises (`0xE06D7363`), as
    /// the engine does for a UE3 error.
    unsafe extern "thiscall-unwind" fn throwing_start(
        _conn: *mut c_void,
        _idx: u8,
        _entity_id: u32,
    ) -> *mut c_void {
        use windows_sys::Win32::System::Diagnostics::Debug::RaiseException;
        unsafe { RaiseException(0xE06D_7363, 0, 0, core::ptr::null()) };
        core::ptr::null_mut()
    }

    /// A `GameEntityManager` whose connection is `online`, with local
    /// player `player`. Returns the slot to point the sender at.
    fn fake_client(online: u32, player: u32) -> usize {
        let mut conn = vec![0u8; CONN_ONLINE + 4].into_boxed_slice();
        conn[CONN_ONLINE..].copy_from_slice(&online.to_le_bytes());
        let conn_addr = conn.as_ptr() as u32;
        let mut gem = vec![0u8; 0x20].into_boxed_slice();
        gem[GEM_SERVER_CONNECTION..GEM_SERVER_CONNECTION + 4]
            .copy_from_slice(&conn_addr.to_le_bytes());
        gem[GEM_LOCAL_PLAYER_ID..GEM_LOCAL_PLAYER_ID + 4].copy_from_slice(&player.to_le_bytes());
        let gem_addr = gem.as_ptr() as u32;
        let slot = Box::new(gem_addr.to_le_bytes());
        let slot_addr = slot.as_ptr() as usize;
        SCRATCH.with(|s| s.borrow_mut().extend([conn, gem, slot as Box<[u8]>]));
        slot_addr
    }

    fn sender(gem_slot: usize, start: usize) -> EngineSender {
        EngineSender {
            gem_slot,
            start_entity_message: start,
        }
    }

    fn written() -> Vec<u8> {
        BUNDLE.with(|b| {
            b.borrow()
                .as_ref()
                .map(|b| b.bytes.clone())
                .unwrap_or_default()
        })
    }

    /// `startEntityMessage(conn, 0x3D, 0)`, then the sub-index byte, then
    /// the payload, byte for byte.
    #[test]
    fn a_send_starts_an_extended_avatar_message_and_writes_sub_index_then_payload() {
        let slot = fake_client(1, 0x1234);
        let mut engine = sender(slot, fake_start as *const () as usize);
        engine.send(2, &[1, 0, 0, 0, 50, 0, 0, 0]).unwrap();
        let conn = ProcessMemory
            .read_ptr(ProcessMemory.read_ptr(slot).unwrap() + GEM_SERVER_CONNECTION)
            .unwrap();
        STARTED.with(|s| assert_eq!(*s.borrow(), [(conn, 0x3D, 0)]));
        assert_eq!(written(), [2, 1, 0, 0, 0, 50, 0, 0, 0]);
    }

    #[test]
    fn offline_states_are_refused_before_the_engine_is_called() {
        let start = fake_start as *const () as usize;
        let cases = [
            (sender(0, start), "no GameEntityManager"),
            (
                sender(fake_client(0, 7), start),
                "the ServerConnection is not online",
            ),
            (
                sender(fake_client(1, 0), start),
                "no local player entity yet",
            ),
        ];
        for (mut engine, why) in cases {
            assert_eq!(engine.send(0, &[0]), Err(Refusal::Offline(why)));
        }
        STARTED.with(|s| assert!(s.borrow().is_empty(), "the engine was not called"));
    }

    #[test]
    fn a_null_bundle_is_an_engine_error() {
        let mut engine = sender(fake_client(1, 7), null_start as *const () as usize);
        assert_eq!(
            engine.send(0, &[0]),
            Err(Refusal::EngineError(
                "startEntityMessage returned no bundle".into()
            ))
        );
    }

    /// A fault inside the engine is caught by the guard and reported, and
    /// the process carries on.
    #[test]
    fn a_fault_in_the_engine_is_an_engine_error() {
        let mut engine = sender(fake_client(1, 7), faulting_start as *const () as usize);
        let Err(Refusal::EngineError(detail)) = engine.send(0, &[0]) else {
            panic!("expected an engine error");
        };
        assert!(detail.contains("0xc0000005"), "{detail}");
    }

    /// A C++ exception from the engine stops at the guard too, instead of
    /// unwinding into the Lua that called the native.
    #[test]
    fn a_cpp_exception_from_the_engine_is_an_engine_error() {
        let mut engine = sender(fake_client(1, 7), throwing_start as *const () as usize);
        let Err(Refusal::EngineError(detail)) = engine.send(0, &[0]) else {
            panic!("expected an engine error");
        };
        assert!(detail.contains("0xe06d7363"), "{detail}");
    }

    #[test]
    fn the_main_thread_is_the_first_one_noted() {
        let engine = sender(0, 0);
        note_main_thread();
        assert!(engine.on_main_thread());
        let other = std::thread::spawn(|| {
            note_main_thread();
            EngineSender::live().on_main_thread()
        });
        assert!(
            !other.join().unwrap(),
            "a second thread is not the main one"
        );
    }
}
