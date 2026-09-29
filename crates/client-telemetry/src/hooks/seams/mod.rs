//! The engine layer's subsystem seams: actors, cinematics, level streaming,
//! movies, sound, physics, file I/O, the D3D9 device and the frame clock.
//!
//! [`sinks`](crate::hooks::sinks) capture what the client already says about
//! itself. These are the seams where the client *does* something whose
//! failure it does not say anything about: an actor that failed to spawn, a
//! movie that would not open, a sound FMOD refused, a file the OS would not
//! open, a device the driver lost. Each reports through the same producer,
//! with the same rate-limit discipline, and joins the boot-time
//! `client.hooks.capabilities` record.
//!
//! | Seam | Target | Where | Notes |
//! |---|---|---|---|
//! | Actor spawn | `client.engine.spawn_actor` | inline `0x00876970` | [`actors`]: class, actor, location; a `NULL` return is a warning |
//! | Actor destroy | `client.engine.destroy_actor` | inline `0x00875290` | [`actors`]: class, actor, result |
//! | Matinee | `client.engine.matinee` | vtable `0x018ad5e8`, `0x018ad5ec` | [`matinee`]: activated (inputs) / deactivated (position, length, `cut_short`) |
//! | Level visible / slow step | `client.engine.level_visible`, `client.engine.level_stream_slow` | inside the existing `UpdateLevelStreamingInner` hook | [`level_streaming`] |
//! | Load failure | `client.engine.load_failed` | inside the existing `StaticLoadObject` hook | `NULL` result, name, flags |
//! | Bink | `client.media.bink_open`, `client.media.bink_close` | IAT `0x017effa4`, `0x017effa8` | [`bink`]: handle, header, completed |
//! | FMOD | `client.audio.event` | IAT `0x017f00a4`, `0x017f0080` | [`fmod`]: name, action, `FMOD_RESULT` |
//! | PhysX | `client.physx.error`, `client.physx.assert` | vtable `0x01839d94`, `0x01839d98` | [`physx`]: the SDK's messages the client discards |
//! | File open failure | `client.io.open_failed` | IAT `0x017ef2a8`, `0x017ef2a4` | [`file_io`]: path, Win32 error |
//! | Frame hitch, memory | `client.engine.hitch`, `client.engine.memory` | inside the existing `FEngineLoop::Tick` hook | [`frame_health`] |
//! | D3D9 device | `client.gfx.device_created`, `client.gfx.device_reset`, `client.gfx.device_state` | IAT `0x017effd8` then COM vtables | [`d3d9`] |
//!
//! # Not covered here
//!
//! - **Texture and mesh streaming** (`UTexture2D` mip streaming): no anchor
//!   has been recovered; the level-streaming and `StaticLoadObject` seams are
//!   the nearest.
//! - **Kismet activation in general**: the per-frame `USequence::UpdateOp`
//!   vtable hook samples it; a per-node event would be a hot path (thousands
//!   of nodes) and is deliberately not added. Matinee is the exception because
//!   cinematics are rare and load-bearing.
//! - **Thread creation**, **library loads**: already in `iat_hooks`.
//! - **`AActor::Destroy`** as a virtual: `UWorld::DestroyActor` is the
//!   function every destroy path ends in.
//!
//! # Install failures
//!
//! Every hook here has a fingerprinted address (inline, vtable) or a per-slot
//! import check (IAT); a build that differs installs nothing, and a hook
//! whose module is not loaded (`fmod_event.dll`, `binkw32.dll`, `d3d9.dll`)
//! is recorded as skipped, not failed.

pub mod actors;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod actors_detours;
pub mod bink;
pub mod d3d9;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod d3d9_detours;
pub mod file_io;
pub mod fmod;
pub mod frame_health;
pub mod level_streaming;
pub mod load_failures;
pub mod matinee;
pub mod objects;
pub mod physx;

use crate::queue::Producer;

/// Install every seam. Called from `hooks::install_all`, after the older
/// hooks, MinHook's init and the sinks.
pub fn install(_producer: Producer) {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        install_inner(&_producer);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_inner(producer: &Producer) {
    use crate::fingerprint;
    use crate::hooks::sinks::caps::{self, Outcome};
    use crate::hooks::sinks::install::{iat, inline, vtable};

    inline(
        producer,
        "spawn_actor",
        actors::ADDR_SPAWN_ACTOR,
        actors_detours::spawn_detour as *mut std::ffi::c_void,
        &actors_detours::SPAWN_TRAMPOLINE,
    );
    inline(
        producer,
        "destroy_actor",
        actors::ADDR_DESTROY_ACTOR,
        actors_detours::destroy_detour as *mut std::ffi::c_void,
        &actors_detours::DESTROY_TRAMPOLINE,
    );

    vtable(
        producer,
        "matinee_activated",
        fingerprint::MATINEE_ACTIVATED_SLOT,
        matinee::activated_detour as *const () as usize,
        &matinee::ORIG_ACTIVATED,
    );
    vtable(
        producer,
        "matinee_deactivated",
        fingerprint::MATINEE_DEACTIVATED_SLOT,
        matinee::deactivated_detour as *const () as usize,
        &matinee::ORIG_DEACTIVATED,
    );
    vtable(
        producer,
        "physx_report_error",
        fingerprint::PHYSX_REPORT_ERROR_SLOT,
        physx::error_detour as *const () as usize,
        &physx::ORIG_ERROR,
    );
    vtable(
        producer,
        "physx_report_assert",
        fingerprint::PHYSX_REPORT_ASSERT_SLOT,
        physx::assert_detour as *const () as usize,
        &physx::ORIG_ASSERT,
    );

    iat(
        producer,
        "bink_open",
        bink::IMPORT_OPEN,
        bink::open_detour as *const () as usize,
        &bink::ORIG_OPEN,
    );
    iat(
        producer,
        "bink_close",
        bink::IMPORT_CLOSE,
        bink::close_detour as *const () as usize,
        &bink::ORIG_CLOSE,
    );

    // FMOD's start/stop report the event name through `Event::getInfo`,
    // called through the client's own import of it. Without it the hooks
    // still report, with an unknown name.
    let import = fmod::IMPORT_GET_INFO;
    match import.resolved() {
        Some(addr) => {
            fmod::GET_INFO.store(addr, std::sync::atomic::Ordering::Release);
            caps::record("fmod_get_info", Outcome::Installed);
        }
        None => caps::record(
            "fmod_get_info",
            Outcome::Skipped("fmod_event.dll not loaded"),
        ),
    }
    iat(
        producer,
        "fmod_event_start",
        fmod::IMPORT_START,
        fmod::start_detour as *const () as usize,
        &fmod::ORIG_START,
    );
    iat(
        producer,
        "fmod_event_stop",
        fmod::IMPORT_STOP,
        fmod::stop_detour as *const () as usize,
        &fmod::ORIG_STOP,
    );

    iat(
        producer,
        "create_file_w",
        file_io::IMPORT_W,
        file_io::create_file_w_detour as *const () as usize,
        &file_io::ORIG_W,
    );
    iat(
        producer,
        "create_file_a",
        file_io::IMPORT_A,
        file_io::create_file_a_detour as *const () as usize,
        &file_io::ORIG_A,
    );

    iat(
        producer,
        "d3d9_create",
        d3d9_detours::IMPORT_CREATE9,
        d3d9_detours::create9_detour as *const () as usize,
        &d3d9_detours::ORIG_CREATE9,
    );

    // The tick, level-streaming and load hooks are the older inline hooks;
    // their additions (hitch and memory, level visibility, load failure) are
    // live exactly when those hooks are.
    use crate::hooks::inline_hooks;
    let rides = |live: bool, why: &'static str| {
        if live {
            Outcome::Installed
        } else {
            Outcome::Skipped(why)
        }
    };
    caps::record(
        "frame_health",
        rides(
            inline_hooks::tick_installed(),
            "FEngineLoop::Tick hook not installed",
        ),
    );
    caps::record(
        "level_streaming_state",
        rides(
            inline_hooks::level_streaming_installed(),
            "UpdateLevelStreamingInner hook not installed",
        ),
    );
    caps::record(
        "static_load_object_failures",
        rides(
            inline_hooks::static_load_object_installed(),
            "StaticLoadObject hook not installed",
        ),
    );
}

#[cfg(test)]
mod tests {
    /// Every inline-hooked address is a fingerprinted site, and every
    /// swapped vtable slot is a fingerprinted slot, so a build whose bytes
    /// differ installs nothing.
    #[test]
    fn every_seam_site_is_fingerprinted() {
        for addr in [
            super::actors::ADDR_SPAWN_ACTOR,
            super::actors::ADDR_DESTROY_ACTOR,
        ] {
            assert!(
                crate::fingerprint::CODE_SITES
                    .iter()
                    .any(|s| s.address == addr),
                "0x{addr:08x} is hooked but not fingerprinted"
            );
        }
        for (addr, expected) in [
            (super::matinee::SLOT_ACTIVATED, 0x007b_06a0u32),
            (super::matinee::SLOT_DEACTIVATED, 0x007a_6730),
            (super::physx::SLOT_REPORT_ERROR, 0x0055_c5e0),
            (super::physx::SLOT_REPORT_ASSERT, 0x0055_c5d0),
        ] {
            assert!(
                crate::fingerprint::SLOT_SITES
                    .iter()
                    .any(|s| s.address == addr && s.expected == expected),
                "slot 0x{addr:08x} is swapped but not fingerprinted with 0x{expected:08x}"
            );
        }
    }
}
