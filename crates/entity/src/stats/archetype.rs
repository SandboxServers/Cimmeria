//! Archetype-specific base stat values used to seed a fresh player.

/// Archetype-specific base stat values passed to [`super::StatList::apply_archetype`].
///
/// Same fields as `ArchetypeStats` in mercury_ext.rs but decoupled from
/// wire format concerns.
#[derive(Debug, Clone)]
pub struct ArchetypeStatValues {
    pub coordination: i32,
    pub engagement: i32,
    pub fortitude: i32,
    pub morale: i32,
    pub perception: i32,
    pub intelligence: i32,
    pub health: i32,
    pub focus: i32,
    pub health_per_level: i32,
    pub focus_per_level: i32,
}
