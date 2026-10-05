//! [`Named`]: an object an event names, carried as its ID and its name.

/// An object a Discord event names — a character, an account, a mission,
/// an item, a world — as the pair Rule 6 asks for: its ID and its
/// human-readable name.
///
/// The embed renders it through one renderer: `Name (#id)`, `#id` when
/// the name is missing, the bare name when only the name is known, and
/// `?` when both are missing (`instrumentation-discipline.md` Rule 6,
/// "Discord"). An empty name counts as missing.
///
/// The ID is whatever ID space the object lives in: `player_id` for a
/// character, `account_id` for an account, the seed ID for content, and
/// the cell `entity_id` for an NPC. `i64` holds every one of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Named {
    pub id: Option<i64>,
    pub name: Option<String>,
}

impl Named {
    /// An object whose ID is known and whose name may be.
    pub fn new(id: impl Into<i64>, name: Option<String>) -> Self {
        Self {
            id: Some(id.into()),
            name,
        }
    }

    /// An object where either half may be missing.
    pub fn from_parts(id: Option<i64>, name: Option<String>) -> Self {
        Self { id, name }
    }

    /// An object known only by name (no ID exists or none reached the
    /// emit site).
    pub fn name_only(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: Some(name.into()),
        }
    }

    /// When neither half is known, label the object by the cell entity
    /// it was found on, `entity:<id>`, instead of rendering `?`. For a
    /// character whose `player_id` and name the cell has not cached yet.
    pub fn or_entity(self, entity_id: u32) -> Self {
        if self.id.is_none() && self.name().is_none() {
            Self::name_only(format!("entity:{entity_id}"))
        } else {
            self
        }
    }

    /// The name, if it is known and not empty.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref().filter(|n| !n.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn or_entity_labels_only_a_fully_unknown_object() {
        assert_eq!(
            Named::default().or_entity(42),
            Named::name_only("entity:42")
        );
        assert_eq!(Named::new(7, None).or_entity(42), Named::new(7, None));
        assert_eq!(
            Named::name_only("alice").or_entity(42),
            Named::name_only("alice")
        );
    }
}
