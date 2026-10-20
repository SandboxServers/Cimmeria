//! Phase: which players can see an entity (Tollana campaign, PH-01a).
//!
//! A phase is a named instance of the world. An entity with an
//! `entity_phase` is visible only to the players who hold that key in their
//! `player_phases` set; an entity without one is visible to everybody. The
//! state lives in `CellEntity::extensions` as a `PhaseState`, so an entity
//! that is not phased pays nothing. Design: `docs/analysis/tollana-continuation/evidence/phasing.md`.
//!
//! This module holds the decision-independent core: the key, the state, and
//! the visibility rule. The AoI filter that applies the rule to witnesses
//! lives in `cimmeria-cell-world` (PH-01b), and nothing here is persisted.

use std::collections::HashSet;
use std::fmt;

use super::CellEntity;

/// A phase key: 1 to 64 characters, each in `[a-z0-9_]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhaseKey(String);

impl PhaseKey {
    /// The longest key, in bytes.
    pub const MAX_LEN: usize = 64;

    /// Validate `name` and build a key from it.
    pub fn new(name: &str) -> Result<PhaseKey, PhaseKeyError> {
        if name.is_empty() {
            return Err(PhaseKeyError::Empty);
        }
        if name.len() > Self::MAX_LEN {
            return Err(PhaseKeyError::TooLong(name.len()));
        }
        if let Some(c) = name
            .chars()
            .find(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_'))
        {
            return Err(PhaseKeyError::BadChar(c));
        }
        Ok(PhaseKey(name.to_owned()))
    }

    /// The key as the string it was built from.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PhaseKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a string is not a valid `PhaseKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseKeyError {
    /// The string is empty.
    Empty,
    /// The string is longer than `PhaseKey::MAX_LEN` bytes; holds its length.
    TooLong(usize),
    /// The first character outside `[a-z0-9_]`.
    BadChar(char),
}

impl fmt::Display for PhaseKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseKeyError::Empty => f.write_str("phase key is empty"),
            PhaseKeyError::TooLong(len) => write!(
                f,
                "phase key is {len} bytes, the limit is {}",
                PhaseKey::MAX_LEN
            ),
            PhaseKeyError::BadChar(c) => {
                write!(f, "phase key has a character outside [a-z0-9_]: {c:?}")
            }
        }
    }
}

impl std::error::Error for PhaseKeyError {}

/// Phase state of one entity, stored in `CellEntity::extensions`.
///
/// `entity_phase` is the phase this entity belongs to (`None` for the whole
/// world). `player_phases` is the set of phases a player holds, so a player
/// can see the entities of every phase in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhaseState {
    /// The phase this entity is in, if any.
    pub entity_phase: Option<PhaseKey>,
    /// The phases this player holds. Empty for an entity that is not a player.
    pub player_phases: HashSet<PhaseKey>,
}

/// The phases a viewer holds: its `PhaseState::player_phases`, or empty when
/// it has no `PhaseState`.
pub fn viewer_phases(entity: &CellEntity) -> HashSet<PhaseKey> {
    entity
        .extensions
        .get::<PhaseState>()
        .map(|s| s.player_phases.clone())
        .unwrap_or_default()
}

/// Whether a viewer holding `viewer_phases` can see `other`.
///
/// Players are always visible. An entity with no phase is visible to
/// everybody; a phased entity is visible only to a viewer that holds its key.
pub fn phase_visible(viewer_phases: &HashSet<PhaseKey>, other: &CellEntity) -> bool {
    // D-PH1 (owner, open): players are never phased in v1.
    if other.is_player {
        return true;
    }
    match other
        .extensions
        .get::<PhaseState>()
        .and_then(|s| s.entity_phase.as_ref())
    {
        None => true,
        Some(key) => viewer_phases.contains(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::{EntityId, SpaceId, Vector3};

    fn entity(id: i32) -> CellEntity {
        CellEntity::new(EntityId(id), SpaceId(100), Vector3::new(10.0, 0.0, 20.0))
    }

    fn key(name: &str) -> PhaseKey {
        PhaseKey::new(name).expect("valid test key")
    }

    fn set(keys: &[&str]) -> HashSet<PhaseKey> {
        keys.iter().map(|k| key(k)).collect()
    }

    fn phased(id: i32, name: &str) -> CellEntity {
        let mut e = entity(id);
        e.extensions.insert(PhaseState {
            entity_phase: Some(key(name)),
            ..Default::default()
        });
        e
    }

    #[test]
    fn key_accepts_lowercase_digits_and_underscore() {
        assert_eq!(key("svarog_vault").as_str(), "svarog_vault");
        assert_eq!(key("p9").to_string(), "p9");
    }

    #[test]
    fn key_rejects_empty() {
        assert_eq!(PhaseKey::new(""), Err(PhaseKeyError::Empty));
    }

    #[test]
    fn key_rejects_length_over_64() {
        let ok = "a".repeat(PhaseKey::MAX_LEN);
        assert!(PhaseKey::new(&ok).is_ok());
        let long = "a".repeat(65);
        assert_eq!(PhaseKey::new(&long), Err(PhaseKeyError::TooLong(65)));
    }

    #[test]
    fn key_rejects_uppercase_and_space() {
        assert_eq!(
            PhaseKey::new("Svarog Vault"),
            Err(PhaseKeyError::BadChar('S'))
        );
    }

    #[test]
    fn unphased_npc_is_visible_to_empty_viewer() {
        let npc = entity(2);
        assert!(phase_visible(&HashSet::new(), &npc));
    }

    #[test]
    fn phased_npc_is_hidden_from_empty_viewer() {
        let npc = phased(2, "svarog_vault");
        assert!(!phase_visible(&HashSet::new(), &npc));
    }

    #[test]
    fn phased_npc_is_visible_to_viewer_holding_its_key() {
        let npc = phased(2, "svarog_vault");
        assert!(phase_visible(&set(&["svarog_vault"]), &npc));
        assert!(phase_visible(&set(&["other", "svarog_vault"]), &npc));
    }

    #[test]
    fn phased_npc_is_hidden_from_viewer_holding_another_key() {
        let npc = phased(2, "svarog_vault");
        assert!(!phase_visible(&set(&["other"]), &npc));
    }

    #[test]
    fn player_with_entity_phase_is_still_visible() {
        let mut player = phased(3, "svarog_vault");
        player.is_player = true;
        assert!(phase_visible(&HashSet::new(), &player));
    }

    #[test]
    fn viewer_phases_is_empty_without_state() {
        assert!(viewer_phases(&entity(4)).is_empty());
    }

    #[test]
    fn viewer_phases_equals_inserted_set() {
        let mut viewer = entity(5);
        let held = set(&["svarog_vault", "tollana_nest"]);
        viewer.extensions.insert(PhaseState {
            entity_phase: None,
            player_phases: held.clone(),
        });
        assert_eq!(viewer_phases(&viewer), held);
    }
}
