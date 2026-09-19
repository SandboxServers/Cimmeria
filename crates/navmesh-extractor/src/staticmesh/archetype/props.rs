//! What an actor inherits from its archetype chain, and what it means
//! when the chain could not be read.
//!
//! Split out of `mod.rs` at the semantics seam: nothing here opens a
//! package or follows a reference. It answers two questions about
//! already-gathered properties — *does this actor collide?* and *where
//! does it sit?* — and carries the distinction between "the chain said
//! nothing" and "the chain could not be read", which is the whole
//! reason [`ActorArchetypeResolution`] exists.

use cimmeria_upk::TaggedProperty;

use crate::coverage::SkipReason;
use crate::staticmesh::mesh_ref::{find_float, find_rotator, find_vector};
use crate::transform::ActorTransform;

use super::find_bool;

/// The subset of a prefab template actor's properties the extractor
/// inherits. Every field is `Option`: `None` means "the chain never
/// defined it", which is distinct from "it defined the UE3 default".
///
/// No `location` field, deliberately — see the module doc on [`super`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ActorArchetypeProps {
    /// `AActor::bCollideActors`. `Some(false)` suppresses emission.
    pub collide_actors: Option<bool>,
    pub rotation: Option<[i32; 3]>,
    pub draw_scale: Option<f32>,
    pub draw_scale_3d: Option<[f32; 3]>,
}

impl ActorArchetypeProps {
    /// Does an actor with these archetype properties and `instance`'s
    /// own properties collide?
    ///
    /// Instance wins; then the archetype; then UE3's `AActor` default,
    /// which is `true` — the cooker omits default values, so an actor
    /// that says nothing anywhere is solid.
    ///
    /// That last fallback is why an *unreadable* chain must never reach
    /// this function as an all-`None` value: it would silently assert
    /// "solid" about a template that may have said otherwise. See
    /// [`ActorArchetypeResolution`].
    pub fn collides(&self, instance: &[TaggedProperty]) -> bool {
        find_bool(instance, "bCollideActors")
            .or(self.collide_actors)
            .unwrap_or(true)
    }

    /// Merge into the transform the instance's own properties give.
    /// Any component the instance omits falls through to the archetype,
    /// then to the UE3 cooked default.
    pub fn merge_transform(&self, instance: &[TaggedProperty]) -> ActorTransform {
        ActorTransform {
            location: find_vector(instance, "Location").unwrap_or([0.0; 3]),
            rotation: find_rotator(instance, "Rotation")
                .or(self.rotation)
                .unwrap_or([0; 3]),
            draw_scale: find_float(instance, "DrawScale")
                .or(self.draw_scale)
                .unwrap_or(1.0),
            draw_scale_3d: find_vector(instance, "DrawScale3D")
                .or(self.draw_scale_3d)
                .unwrap_or([1.0; 3]),
        }
    }

    /// Fold `later` (an ancestor further up the chain) underneath
    /// `self` (nearer the instance): nearest definition wins.
    pub(super) fn inherit_from(&mut self, later: &ActorArchetypeProps) {
        self.collide_actors = self.collide_actors.or(later.collide_actors);
        self.rotation = self.rotation.or(later.rotation);
        self.draw_scale = self.draw_scale.or(later.draw_scale);
        self.draw_scale_3d = self.draw_scale_3d.or(later.draw_scale_3d);
    }

    pub(super) fn is_complete(&self) -> bool {
        self.collide_actors.is_some()
            && self.rotation.is_some()
            && self.draw_scale.is_some()
            && self.draw_scale_3d.is_some()
    }

    pub(super) fn from_props(props: &[TaggedProperty]) -> Self {
        Self {
            collide_actors: find_bool(props, "bCollideActors"),
            rotation: find_rotator(props, "Rotation"),
            draw_scale: find_float(props, "DrawScale"),
            draw_scale_3d: find_vector(props, "DrawScale3D"),
        }
    }
}

/// What [`super::resolve_actor_archetype`] concluded.
///
/// The three cases are deliberately distinct. `NotInstanced` and
/// `Resolved(default)` both mean "inherit nothing", but `Unresolved`
/// does **not**: it means the chain that would have told us whether the
/// actor collides could not be read, and treating that as "inherit
/// nothing" silently applies UE3's `bCollideActors = true` default to a
/// template that may have said `false`. The component chain resolves a
/// mesh independently, so such an actor sails through and puts a solid
/// obstacle in the navmesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActorArchetypeResolution {
    /// `Archetype == 0`: the instance's own properties are the whole
    /// story, which is a fact about the export rather than an
    /// assumption.
    NotInstanced,
    /// The chain was followed to a definite end.
    Resolved(ActorArchetypeProps),
    /// The actor is archetype-instanced and the chain could not be
    /// followed. The caller must skip the actor.
    Unresolved(SkipReason),
}

impl ActorArchetypeResolution {
    /// The inherited properties, or `None` when the chain could not be
    /// followed.
    pub fn props(self) -> Option<ActorArchetypeProps> {
        match self {
            ActorArchetypeResolution::NotInstanced => Some(ActorArchetypeProps::default()),
            ActorArchetypeResolution::Resolved(p) => Some(p),
            ActorArchetypeResolution::Unresolved(_) => None,
        }
    }

    /// The reason the chain could not be followed, if it could not.
    pub fn skip_reason(self) -> Option<SkipReason> {
        match self {
            ActorArchetypeResolution::Unresolved(r) => Some(r),
            _ => None,
        }
    }
}
