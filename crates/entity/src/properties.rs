//! Property descriptors and dirty-tracking for entity synchronisation.
//!
//! Each entity type has a fixed set of properties declared in its `.def` file.
//! At runtime, each property is described by a [`PropertyDescriptor`] that
//! records its name, type, distribution flags, and index within the entity's
//! property table.
//!
//! When a property value changes, the corresponding bit in [`DirtyFlags`] is
//! set. The sync system then reads dirty flags to determine which properties
//! need to be sent to ghosts, clients, or the database.

use cimmeria_common::DistributionFlags;

/// Static metadata describing a single property on an entity type.
///
/// Populated at startup from the entity definition registry
/// (`cimmeria-defs`). Immutable once constructed.
#[derive(Clone, Debug)]
pub struct PropertyDescriptor {
    /// Property name as declared in the `.def` file (e.g. "health").
    pub name: String,

    /// Type name from the definition (e.g. "INT32", "STRING", "VECTOR3").
    pub type_name: String,

    /// Distribution flags controlling where this property is synced.
    pub flags: DistributionFlags,

    /// Zero-based index into the entity's property table. Used as the bit
    /// index in `DirtyFlags`.
    pub index: u16,
}

/// Bitfield tracking which properties have been modified since the last sync.
///
/// Supports up to 256 properties per entity type (backed by a `[u64; 4]`
/// array, giving 256 bits). This matches the practical limit of BigWorld
/// entity definitions.
#[derive(Clone, Debug)]
pub struct DirtyFlags {
    /// Four 64-bit words = 256 bits of dirty-tracking capacity.
    bits: [u64; 4],
}

impl DirtyFlags {
    /// Create a new `DirtyFlags` with no properties marked dirty.
    pub fn new() -> Self {
        Self { bits: [0; 4] }
    }

    /// Mark a property as dirty by its table index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= 256`.
    pub fn mark_dirty(&mut self, index: u16) {
        let (word, bit) = Self::word_and_bit(index);
        self.bits[word] |= 1u64 << bit;
    }

    /// Returns `true` if the property at `index` is dirty.
    ///
    /// # Panics
    ///
    /// Panics if `index >= 256`.
    pub fn is_dirty(&self, index: u16) -> bool {
        let (word, bit) = Self::word_and_bit(index);
        (self.bits[word] & (1u64 << bit)) != 0
    }

    /// Clear all dirty bits.
    pub fn clear(&mut self) {
        self.bits = [0; 4];
    }

    /// Returns `true` if any property is currently dirty.
    pub fn any_dirty(&self) -> bool {
        self.bits.iter().any(|&w| w != 0)
    }

    /// Decompose an index into the word offset and bit position within that word.
    fn word_and_bit(index: u16) -> (usize, u32) {
        assert!(
            index < 256,
            "property index {} exceeds maximum of 255",
            index
        );
        let word = (index / 64) as usize;
        let bit = (index % 64) as u32;
        (word, bit)
    }
}

impl Default for DirtyFlags {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_flags_are_clean() {
        let flags = DirtyFlags::new();
        assert!(!flags.any_dirty());
    }

    #[test]
    fn mark_and_check_dirty() {
        let mut flags = DirtyFlags::new();
        flags.mark_dirty(0);
        assert!(flags.is_dirty(0));
        assert!(!flags.is_dirty(1));
        assert!(flags.any_dirty());
    }

    #[test]
    fn mark_multiple_indices() {
        let mut flags = DirtyFlags::new();
        flags.mark_dirty(0);
        flags.mark_dirty(63);
        flags.mark_dirty(64);
        flags.mark_dirty(255);
        assert!(flags.is_dirty(0));
        assert!(flags.is_dirty(63));
        assert!(flags.is_dirty(64));
        assert!(flags.is_dirty(255));
        assert!(!flags.is_dirty(1));
        assert!(!flags.is_dirty(128));
    }

    #[test]
    fn clear_resets_all_bits() {
        let mut flags = DirtyFlags::new();
        flags.mark_dirty(10);
        flags.mark_dirty(200);
        assert!(flags.any_dirty());
        flags.clear();
        assert!(!flags.any_dirty());
        assert!(!flags.is_dirty(10));
        assert!(!flags.is_dirty(200));
    }

    #[test]
    fn mark_dirty_is_idempotent() {
        let mut flags = DirtyFlags::new();
        flags.mark_dirty(42);
        flags.mark_dirty(42);
        assert!(flags.is_dirty(42));
    }

    #[test]
    #[should_panic(expected = "property index 256 exceeds maximum")]
    fn index_out_of_range_panics() {
        let mut flags = DirtyFlags::new();
        flags.mark_dirty(256);
    }

    #[test]
    fn default_is_clean() {
        let flags = DirtyFlags::default();
        assert!(!flags.any_dirty());
    }

    #[test]
    fn property_descriptor_construction() {
        let desc = PropertyDescriptor {
            name: "health".to_string(),
            type_name: "INT32".to_string(),
            flags: DistributionFlags::ALL_CLIENTS,
            index: 0,
        };
        assert_eq!(desc.name, "health");
        assert_eq!(desc.index, 0);
        assert!(desc.flags.contains(DistributionFlags::OWN_CLIENT));
    }
}
