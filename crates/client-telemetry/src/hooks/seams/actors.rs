//! Actor spawn and destroy: `UWorld::SpawnActor` and `UWorld::DestroyActor`.
//!
//! The client spawns an actor for every BigWorld entity that enters view and
//! every particle, pickup and prop a level script creates. An entity that
//! "should be there and is not" is, at this layer, either a `SpawnActor`
//! that returned `NULL` or a `DestroyActor` that ran early.
//!
//! # SpawnActor, `0x00876970`
//!
//! `__thiscall`, `this` = the `UWorld` (`GWorld`), eleven stack arguments,
//! `ret 0x2c` (confirmed at both `RET` sites, `0x00876a4e` and
//! `0x00876db6`):
//!
//! | # | Argument | Evidence |
//! |---|---|---|
//! | 1 | `UClass* Class` | `[esp+0x34]`; first thing checked, `NULL` returns `NULL` |
//! | 2, 3 | `FName Name` (index, number) | forwarded to `StaticConstructObject` |
//! | 4 | `FVector* Location` | copied to the actor at `+0xdc` |
//! | 5 | `FRotator* Rotation` | copied to the actor at `+0xe8` |
//! | 6 | `AActor* Template` | `Class` compared with `Template + 0x34` |
//! | 7 | `bNoCollisionFail` | tested before the collision check |
//! | 8 | `bRemoteOwned` | swaps the actor's `Role` and `RemoteRole` |
//! | 9 | `AActor* Owner` | its outer picks the level |
//! | 10 | `APawn* Instigator` | stored on the actor |
//! | 11 | `bNoFail` | `[esp+0x5c]`, tested on every failure path |
//!
//! It returns the new actor, or `NULL` on: a `NULL`, abstract or deprecated
//! class; a template that is not of the class; a spawn location that fails
//! the collision check (unless `bNoCollisionFail`); a level that is not the
//! current one; and an actor that destroys itself in `PreBeginPlay`. It
//! asserts (through the `check()` reporter, see
//! [`ue3_assert`](crate::hooks::sinks::ue3_assert)) rather than failing when
//! the world is not `GWorld`. Called from `AActor::execSpawn` (`0x006e1640`)
//! and from engine code (`USeqAct_Interp::Activated` spawns its replicated
//! actor).
//!
//! # DestroyActor, `0x00875290`
//!
//! `__thiscall(this = UWorld, AActor* Actor, bNetForce, bShouldModifyLevel)`,
//! `ret 0xc`, returns a `UBOOL`. It returns `0` without destroying when the
//! actor is already pending kill (`flags & 5`) or, on a client, when the
//! actor is replicated from the server and `bNetForce` is not set
//! (`Role != ROLE_Authority`); `1` when it destroyed the actor or it was
//! already destroyed (`flags & 8`). Asserts `ThisActor` is non-null and
//! valid (`UnLevAct.cpp` lines `0x1ac`, `0x1ad`).
//!
//! Both are rate-limited per class name. Failures are never quieter than
//! warnings.
//!
//! Static evidence only (2026-09-28: Ghidra decompile and disassembly of both
//! functions and of `AActor::execSpawn`); not yet seen from a live client.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;

/// Entry of `UWorld::SpawnActor`.
pub const ADDR_SPAWN_ACTOR: usize = 0x0087_6970;
/// Entry of `UWorld::DestroyActor`.
pub const ADDR_DESTROY_ACTOR: usize = 0x0087_5290;

/// Telemetry target of a spawn.
pub const SPAWN_TARGET: &str = "client.engine.spawn_actor";
/// Telemetry target of a destroy.
pub const DESTROY_TARGET: &str = "client.engine.destroy_actor";

/// Rate-limit key of a spawn: one bucket per class, and per outcome so a
/// burst of successes cannot hide the first failure.
pub fn spawn_key(class: &str, ok: bool) -> String {
    format!("{}:{class}", if ok { "ok" } else { "fail" })
}

/// Rate-limit key of a destroy.
pub fn destroy_key(class: &str, done: bool) -> String {
    format!("{}:{class}", if done { "done" } else { "refused" })
}

/// Telemetry level of a spawn: a failure is a warning.
pub fn spawn_level(ok: bool) -> &'static str {
    if ok {
        "debug"
    } else {
        "warn"
    }
}

/// Telemetry level of a destroy: a refusal is routine on a client (the
/// server owns replicated actors), so it is `debug` too.
pub fn destroy_level(_done: bool) -> &'static str {
    "debug"
}

/// What a spawn call carried and returned.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnInfo {
    /// Class name.
    pub class: String,
    /// The new actor's name, when it spawned.
    pub actor: Option<String>,
    /// The requested location.
    pub location: Option<[f32; 3]>,
    /// `bNoCollisionFail`.
    pub no_collision_fail: bool,
    /// `bNoFail`.
    pub no_fail: bool,
    /// Whether an actor came back.
    pub ok: bool,
}

/// The fields of one `client.engine.spawn_actor`.
pub fn spawn_fields(s: &SpawnInfo, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("class", json!(s.class)),
        ("ok", json!(s.ok)),
        ("no_collision_fail", json!(s.no_collision_fail)),
        ("no_fail", json!(s.no_fail)),
    ];
    if let Some(actor) = &s.actor {
        f.push(("actor", json!(actor)));
    }
    if let Some(loc) = s.location {
        f.push(("location", super::objects::vec3_field(loc)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// The fields of one `client.engine.destroy_actor`.
pub fn destroy_fields(
    class: &str,
    actor: &str,
    done: bool,
    net_force: bool,
    suppressed: u64,
) -> Fields {
    let mut f: Fields = vec![
        ("class", json!(class)),
        ("actor", json!(actor)),
        ("destroyed", json!(done)),
        ("net_force", json!(net_force)),
    ];
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_spawn_is_a_warning_and_has_its_own_bucket() {
        assert_eq!(spawn_level(true), "debug");
        assert_eq!(spawn_level(false), "warn");
        assert_ne!(spawn_key("SGWPawn", true), spawn_key("SGWPawn", false));
        assert_ne!(spawn_key("SGWPawn", false), spawn_key("Emitter", false));
    }

    #[test]
    fn destroy_refusals_have_their_own_bucket() {
        assert_ne!(destroy_key("SGWPawn", true), destroy_key("SGWPawn", false));
        assert_eq!(destroy_level(false), "debug");
    }

    #[test]
    fn spawn_fields_carry_the_class_actor_and_location() {
        let info = SpawnInfo {
            class: "SGWPawn".into(),
            actor: Some("SGWPawn_3".into()),
            location: Some([10.04, 20.0, -5.0]),
            no_collision_fail: false,
            no_fail: true,
            ok: true,
        };
        let f = spawn_fields(&info, 0);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("class"), Some(json!("SGWPawn")));
        assert_eq!(get("actor"), Some(json!("SGWPawn_3")));
        assert_eq!(get("location"), Some(json!([10.0, 20.0, -5.0])));
        assert_eq!(get("no_fail"), Some(json!(true)));
        assert_eq!(get("suppressed"), None);

        let failed = SpawnInfo {
            actor: None,
            location: None,
            ok: false,
            ..info
        };
        let f = spawn_fields(&failed, 4);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("actor"), None);
        assert_eq!(get("location"), None);
        assert_eq!(get("ok"), Some(json!(false)));
        assert_eq!(get("suppressed"), Some(json!(4)));
    }

    #[test]
    fn destroy_fields_carry_the_outcome() {
        let f = destroy_fields("SGWPawn", "SGWPawn_3", false, true, 2);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("destroyed"), Some(json!(false)));
        assert_eq!(get("net_force"), Some(json!(true)));
        assert_eq!(get("suppressed"), Some(json!(2)));
    }

    #[test]
    fn the_hooked_addresses_are_the_ghidra_ones() {
        assert_eq!(ADDR_SPAWN_ACTOR, 0x0087_6970);
        assert_eq!(ADDR_DESTROY_ACTOR, 0x0087_5290);
    }
}
