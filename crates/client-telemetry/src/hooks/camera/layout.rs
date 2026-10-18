//! Offsets and addresses for the player camera and the pawn pose.
//!
//! Every value is copied from the lab's readers in
//! `crates/lab/src/supervisor/world/memory.rs`, which are proven live
//! (findings `cegui-mouse-input-feed.md` section 10). The DLL runs in the
//! client process, so these are the virtual addresses as they are: there is
//! no slide.

/// `g_pGLevel`, the global `UWorld*` pointer (lab `GLEVEL_PTR_VA`).
pub const GLEVEL_PTR_VA: u32 = 0x01ee_2684;

/// `UWorld::Level` (lab `find_player_camera`, the `+0x50` read).
pub const WORLD_LEVEL_OFF: u32 = 0x50;

/// `ULevel::Actors`: the data pointer, with the count at `+4` (lab
/// `find_player_camera`, the `+0x3C` read).
pub const LEVEL_ACTORS_OFF: u32 = 0x3c;

/// The scan stops after this many actors (lab `find_player_camera`).
pub const MAX_LEVEL_ACTORS: u32 = 8192;

/// Vtable of `ASGWController_Player` (lab `PLAYER_CONTROLLER_VTABLE_VA`).
pub const PLAYER_CONTROLLER_VTABLE_VA: u32 = 0x019e_2b2c;

/// Vtable of `ASGWCamera_Player` (lab `PLAYER_CAMERA_VTABLE_VA`).
pub const PLAYER_CAMERA_VTABLE_VA: u32 = 0x019e_1134;

/// The controller's camera pointer (lab `CONTROLLER_CAMERA_OFF`).
pub const CONTROLLER_CAMERA_OFF: u32 = 0x2b0;

/// Camera invert flags: bit 2 inverts yaw, bit 3 inverts pitch (lab `CAMERA_FLAGS_OFF`).
pub const CAMERA_FLAGS_OFF: u32 = 0x348;

/// Look gain, rotator units per mouse count (lab `CAMERA_GAIN_OFF`).
pub const CAMERA_GAIN_OFF: u32 = 0x358;

/// Third-person distance, the zoom (lab `CAMERA_DIST_OFF`).
pub const CAMERA_DIST_OFF: u32 = 0x360;

/// Pitch offset from the pawn, rotator units (lab `CAMERA_PITCH_OFF`).
pub const CAMERA_PITCH_OFF: u32 = 0x368;

/// Yaw offset from the pawn, rotator units (lab `CAMERA_YAW_OFF`).
pub const CAMERA_YAW_OFF: u32 = 0x36c;

/// Bytes from [`CAMERA_FLAGS_OFF`] through the end of the yaw field (lab
/// `camera_state`, which reads `CAMERA_YAW_OFF + 4 - CAMERA_FLAGS_OFF`).
pub const CAMERA_BLOCK_LEN: usize = (0x370 - 0x348) as usize;

/// `AActor::Location` plus the rotator, relative to an actor (lab `ACTOR_LOCATION_OFF`).
pub const ACTOR_LOCATION_OFF: u32 = 0xdc;

/// Bytes of one pose read: three `f32` location words, then pitch and yaw (lab `POSE_LEN`).
pub const POSE_LEN: usize = 0x18;

/// `GameEntityManager::instance_`, the entity manager global (the same VA as
/// `ability_trace::layout::ENTITY_MANAGER`).
pub const ENTITY_MANAGER_VA: u32 = 0x01ef_244c;

/// The manager's local entity slot. Pawn path: `[[ENTITY_MANAGER_VA] + 0x0c] + 0x08`,
/// per the PC-15 packet.
pub const MANAGER_LOCAL_ENTITY_OFF: u32 = 0x0c;

/// The local entity's pawn pointer (pawn path, second hop).
pub const ENTITY_PAWN_OFF: u32 = 0x08;
