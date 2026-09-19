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

/// Result of [`walk_actor_chain`].
///
/// `props` alone is not enough for the caller to act on: an all-`None`
/// result can mean "the chain defined nothing" **or** "the chain could
/// not be read", and those differ by whether `bCollideActors` may
/// safely default to UE3's `true`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActorChainOutcome {
    /// Nearest definition of each inherited property, as far as the
    /// walk got.
    pub props: ActorArchetypeProps,
    /// `None` when every template the walk needed was readable.
    /// `Some(reason)` when one was not — `props` then holds only what
    /// was read *before* the failure.
    pub unreadable: Option<SkipReason>,
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
/// It stops early once every field is pinned, because a nearer
/// template's value wins and the rest of the chain cannot change the
/// answer — that is a *complete* read even though the walk did not
/// reach the root.
///
/// A `fetch` failure is reported rather than swallowed. An earlier
/// revision returned the partially-merged properties and let the caller
/// treat them as authoritative; the effect was that an unreadable
/// template silently became `bCollideActors = true` (UE3's default for
/// an actor that says nothing), and the component chain would then
/// resolve a mesh and emit geometry the level author had switched
/// collision off on.
pub fn walk_actor_chain<F>(chain: &[String], fetch: &mut F) -> ActorChainOutcome
where
    F: FnMut(&[String]) -> Result<ActorProbe, SkipReason>,
{
    let mut visited: Vec<String> = Vec::new();
    let mut current: Vec<String> = chain.to_vec();
    let mut out = ActorChainOutcome::default();

    for _ in 0..MAX_ARCHETYPE_DEPTH {
        if current.len() < 2 {
            out.unreadable = Some(SkipReason::ArchetypeUnrooted);
            return out;
        }
        let key = current.join(".");
        if visited.contains(&key) {
            out.unreadable = Some(SkipReason::ArchetypeChainLoop);
            return out;
        }
        visited.push(key);

        let probe = match fetch(&current) {
            Ok(p) => p,
            Err(reason) => {
                out.unreadable = Some(reason);
                return out;
            }
        };
        out.props.inherit_from(&probe.props);
        if out.props.is_complete() {
            return out;
        }
        match probe.next {
            Some(next) if next.len() >= 2 => current = next,
            // The chain ends here. Whatever it defined is the whole
            // answer, and the rest takes the UE3 default.
            _ => return out,
        }
    }
    out.unreadable = Some(SkipReason::ArchetypeChainLoop);
    out
}
