//! The archetype walk, with the package I/O lifted out.
//!
//! Both chains — component→`StaticMesh` and actor→`bCollideActors` —
//! are the same shape: read a template, decide whether it answered the
//! question, and if not climb to *its* archetype. What differs is only
//! what you read off the template and what "answered" means.
//!
//! Splitting the control flow from the reads is not decoration. The
//! interesting cases are the ones a real package can't easily express:
//! a chain that loops, a chain deeper than the budget, a package the
//! index doesn't know, a template that exists but has neither a mesh
//! nor a further archetype. Against a `fetch` closure each of those is
//! three lines of test; against the cooked client tree none of them is
//! testable at all, and the code would report 0 % coverage in CI.

use crate::coverage::SkipReason;
use crate::staticmesh::mesh_ref::MeshRefResult;

use super::{ActorArchetypeProps, MAX_ARCHETYPE_DEPTH};

/// What reading one template `StaticMeshComponent` contributes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TemplateProbe {
    /// The template's `StaticMesh`, already resolved to the
    /// `(package, object)` key the mesh loader wants. `None` means the
    /// template carries no `StaticMesh` property — it is itself a stub
    /// and the walk must climb.
    pub mesh: Option<MeshRefResult>,
    /// Component-level `CollideActors`, when this template sets it.
    pub collide_actors: Option<bool>,
    /// This template's own archetype chain, root package first.
    pub next: Option<Vec<String>>,
}

/// What reading one template `StaticMeshActor` contributes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActorProbe {
    pub props: ActorArchetypeProps,
    pub next: Option<Vec<String>>,
}

/// Follow a component-archetype chain to a `StaticMesh` key.
///
/// `fetch` maps a rooted chain (`["Em-Props", "Prefab", "Arc",
/// "StaticMeshComponent0"]`) to what that template says. It returns
/// the reason itself when the template cannot be read, because only
/// the I/O side can tell "the index has no such package" from "the
/// package opened but holds no export at that path" — and the whole
/// point of the typed reasons is that those two are different stories.
///
/// Terminates on: a mesh, a collision veto, a chain that repeats a
/// path, [`MAX_ARCHETYPE_DEPTH`] hops, or a template with nothing left
/// to climb to.
pub fn walk_mesh_chain<F>(chain: &[String], fetch: &mut F) -> MeshRefResult
where
    F: FnMut(&[String]) -> Result<TemplateProbe, SkipReason>,
{
    let mut visited: Vec<String> = Vec::new();
    let mut current: Vec<String> = chain.to_vec();

    for _ in 0..MAX_ARCHETYPE_DEPTH {
        if current.len() < 2 {
            return Err(SkipReason::ArchetypeUnrooted);
        }
        let key = current.join(".");
        if visited.contains(&key) {
            return Err(SkipReason::ArchetypeChainLoop);
        }
        visited.push(key);

        let probe = fetch(&current)?;
        if probe.collide_actors == Some(false) {
            return Err(SkipReason::CollisionDisabled);
        }
        if let Some(mesh) = probe.mesh {
            return mesh;
        }
        match probe.next {
            Some(next) if next.len() >= 2 => current = next,
            _ => return Err(SkipReason::ArchetypeNoMesh),
        }
    }
    Err(SkipReason::ArchetypeChainLoop)
}

/// Follow an actor-archetype chain, collecting the nearest definition
/// of each inherited property.
///
/// Unlike the mesh walk this has no failure mode worth reporting: an
/// actor whose archetype cannot be read simply inherits nothing, which
/// is exactly the behaviour that predates this module. It stops early
/// once every field is pinned, because a nearer template's value wins
/// and the rest of the chain cannot change the answer.
pub fn walk_actor_chain<F>(chain: &[String], fetch: &mut F) -> ActorArchetypeProps
where
    F: FnMut(&[String]) -> Option<ActorProbe>,
{
    let mut visited: Vec<String> = Vec::new();
    let mut current: Vec<String> = chain.to_vec();
    let mut merged = ActorArchetypeProps::default();

    for _ in 0..MAX_ARCHETYPE_DEPTH {
        if current.len() < 2 {
            return merged;
        }
        let key = current.join(".");
        if visited.contains(&key) {
            return merged;
        }
        visited.push(key);

        let Some(probe) = fetch(&current) else {
            return merged;
        };
        merged.inherit_from(&probe.props);
        if merged.is_complete() {
            return merged;
        }
        match probe.next {
            Some(next) if next.len() >= 2 => current = next,
            _ => return merged,
        }
    }
    merged
}
