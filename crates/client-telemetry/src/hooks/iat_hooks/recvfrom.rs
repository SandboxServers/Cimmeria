//! Winsock IAT receive detour and its native x86 fixture.
use super::*;

#[cfg(test)]
static TEST_CLOBBER_WSA_ERROR: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[link(name = "Ws2_32")]
unsafe extern "system" {
    fn WSAGetLastError() -> i32;
    fn WSASetLastError(error: i32);
}

/// Observe the Winsock boundary after the original call; preserve its error.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "stdcall-unwind" fn recvfrom_detour(
    socket: usize,
    buf: *mut u8,
    len: i32,
    flags: i32,
    from: *mut c_void,
    from_len: *mut i32,
) -> i32 {
    let orig_addr = ORIG_RECVFROM.load(Ordering::Acquire);
    if orig_addr == 0 {
        return -1;
    }
    let original: unsafe extern "stdcall-unwind" fn(
        usize,
        *mut u8,
        i32,
        i32,
        *mut c_void,
        *mut i32,
    ) -> i32 = unsafe { std::mem::transmute(orig_addr) };
    let result = unsafe { original(socket, buf, len, flags, from, from_len) };
    let error = (result < 0).then(|| unsafe { WSAGetLastError() });
    let _ = std::panic::catch_unwind(|| {
        let wire = if result >= 0 && result <= len && !buf.is_null() {
            Some(unsafe { std::slice::from_raw_parts(buf, result as usize) })
        } else {
            None
        };
        let peer = if result > 0
            && len == 0x5c0
            && !from.is_null()
            && !from_len.is_null()
            && (8..=16).contains(&unsafe { *from_len })
        {
            let bytes = unsafe { std::slice::from_raw_parts(from.cast::<u8>(), 8) };
            crate::hooks::mercury_recv::wire::ipv4_peer(bytes)
        } else {
            None
        };
        let wire_fp = crate::hooks::mercury_recv::wire::stash_received(len, wire);
        if let Some((level, fields)) = crate::hooks::mercury_recv::wire::recv_event(
            socket,
            len,
            wire,
            wire_fp,
            error,
            peer.as_deref(),
        ) {
            crate::hooks::emit::emit("client.mercury.socket_recv", level, fields);
        }
        #[cfg(feature = "lab-bridge")]
        crate::hooks::mercury_recv::wire::record_proof(socket, len, flags, wire, peer.as_deref());
    });
    // Make the restoration observable in the native fixture even if the
    // telemetry producer happens not to call a Winsock API in that run.
    #[cfg(test)]
    if TEST_CLOBBER_WSA_ERROR.load(Ordering::Relaxed) {
        unsafe { WSASetLastError(7777) };
    }
    if let Some(error) = error {
        unsafe { WSASetLastError(error) };
    }
    result
}

#[cfg(all(test, feature = "lab-bridge"))]
mod tests {
    use super::*;
    use crate::hooks::mercury_recv::wire::PROOF_COUNTERS;
    use std::sync::Mutex;

    static DETOUR_TEST: Mutex<()> = Mutex::new(());

    unsafe extern "stdcall-unwind" fn stub_recvfrom(
        socket: usize,
        buf: *mut u8,
        _len: i32,
        _flags: i32,
        from: *mut c_void,
        from_len: *mut i32,
    ) -> i32 {
        if socket == 8 {
            unsafe { WSASetLastError(10040) };
            return -1;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(b"hello".as_ptr(), buf, 5);
            std::ptr::copy_nonoverlapping(
                [2u8, 0, 0x1f, 0x90, 127, 0, 0, 1].as_ptr(),
                from.cast::<u8>(),
                8,
            );
            *from_len = 8;
            WSASetLastError(4242);
        }
        5
    }

    #[test]
    fn mercury_recv_native_detour_records_candidate_and_restores_wsa_error() {
        let _guard = DETOUR_TEST.lock().unwrap();
        if crate::bridge::bridge_epoch().is_none() {
            let handle = crate::bridge::start(crate::session::LabConfig {
                bind: "127.0.0.1".into(),
                port: 0,
                token: "0".repeat(64),
            })
            .unwrap();
            assert!(crate::bridge::install_handle(handle));
        }
        let previous = ORIG_RECVFROM.swap(stub_recvfrom as *const () as usize, Ordering::AcqRel);
        let received_before = PROOF_COUNTERS.snapshot().received;
        let mut buf = [0u8; 1472];
        let mut from = [0u8; 16];
        let mut from_len = from.len() as i32;
        let result = unsafe {
            recvfrom_detour(
                7,
                buf.as_mut_ptr(),
                buf.len() as i32,
                0,
                from.as_mut_ptr().cast(),
                &mut from_len,
            )
        };
        assert_eq!(result, 5);
        assert_eq!(PROOF_COUNTERS.snapshot().received, received_before + 1);
        assert_eq!(&buf[..5], b"hello");
        TEST_CLOBBER_WSA_ERROR.store(true, Ordering::Relaxed);
        let failed = unsafe {
            recvfrom_detour(
                8,
                buf.as_mut_ptr(),
                buf.len() as i32,
                0,
                from.as_mut_ptr().cast(),
                &mut from_len,
            )
        };
        TEST_CLOBBER_WSA_ERROR.store(false, Ordering::Relaxed);
        assert_eq!(failed, -1);
        assert_eq!(unsafe { WSAGetLastError() }, 10040);
        assert_eq!(PROOF_COUNTERS.snapshot().received, received_before + 1);
        ORIG_RECVFROM.store(previous, Ordering::Release);
    }
}
