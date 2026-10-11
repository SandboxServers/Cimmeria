//! Queries over the mirror: which entity is the one the client was told about.
//!
//! A query names only the facts a test knows (a static mesh, a name id, a
//! place in the world). Every `Some` field must match, so adding a field
//! narrows the search and never widens it.

use super::MirroredEntity;

/// A conjunction of facts about an entity. Build one with
/// [`EntityQuery::static_mesh`] and chain the rest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityQuery {
    /// The entity's `static_mesh` must equal this.
    pub static_mesh: Option<String>,
    /// The entity's `name_id` must equal this.
    pub name_id: Option<i32>,
    /// The entity's class must equal this.
    pub class_id: Option<u8>,
    /// The entity's position must lie within `radius` of this point.
    pub near: Option<([f32; 3], f32)>,
    /// The entity's `interaction_type` must be known and non-zero.
    pub interaction_nonzero: bool,
}

impl EntityQuery {
    /// A query for entities whose static mesh is `mesh`.
    pub fn static_mesh(mesh: &str) -> Self {
        Self {
            static_mesh: Some(mesh.to_owned()),
            ..Self::default()
        }
    }

    /// Also require this name id.
    pub fn name_id(self, id: i32) -> Self {
        Self {
            name_id: Some(id),
            ..self
        }
    }

    /// Also require this class id.
    pub fn class_id(self, class: u8) -> Self {
        Self {
            class_id: Some(class),
            ..self
        }
    }

    /// Also require the position to be within `radius` of `pos`.
    pub fn near(self, pos: [f32; 3], radius: f32) -> Self {
        Self {
            near: Some((pos, radius)),
            ..self
        }
    }

    /// Also require a non-zero interaction type: the entity offers something
    /// to click on.
    pub fn interactable(self) -> Self {
        Self {
            interaction_nonzero: true,
            ..self
        }
    }

    /// True when `e` satisfies every field this query sets. An entity with no
    /// position never satisfies `near`.
    pub fn matches(&self, e: &MirroredEntity) -> bool {
        if let Some(mesh) = &self.static_mesh {
            if e.static_mesh.as_deref() != Some(mesh.as_str()) {
                return false;
            }
        }
        if self.name_id.is_some() && e.name_id != self.name_id {
            return false;
        }
        if self.class_id.is_some() && Some(e.class_id) != self.class_id {
            return false;
        }
        if let Some((centre, radius)) = self.near {
            match e.position {
                Some(p) if distance(p, centre) <= radius => {}
                _ => return false,
            }
        }
        if self.interaction_nonzero && !matches!(e.interaction_type, Some(x) if x != 0) {
            return false;
        }
        true
    }
}

impl std::fmt::Display for EntityQuery {
    /// Renders the set fields, e.g. `mesh=CA-Props.X name_id=7031`. A query
    /// with no field set renders as `any`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(mesh) = &self.static_mesh {
            parts.push(format!("mesh={mesh}"));
        }
        if let Some(id) = self.name_id {
            parts.push(format!("name_id={id}"));
        }
        if let Some(class) = self.class_id {
            parts.push(format!("class={class}"));
        }
        if let Some(([x, y, z], r)) = self.near {
            parts.push(format!("near=({x},{y},{z})±{r}"));
        }
        if self.interaction_nonzero {
            parts.push("interactable".to_owned());
        }
        if parts.is_empty() {
            f.write_str("any")
        } else {
            f.write_str(&parts.join(" "))
        }
    }
}

/// Euclidean distance between two points.
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}
