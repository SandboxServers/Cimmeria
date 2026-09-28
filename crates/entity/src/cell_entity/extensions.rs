//! Per-entity feature state keyed by type (`CellEntity::extensions`).
//!
//! A feature that needs state on an entity stores one value of its own type
//! here instead of adding a field to `CellEntity`
//! (`docs/architecture/plugin-architecture.md` §3.5). The type is the key, so
//! a feature owns its slot by owning its type: `PetState` is the pet slot.
//!
//! Nothing in the map reaches the client or the database on its own. SGW
//! declares no client-replicated property flag, so every value the client
//! sees is an explicit send by the feature's code, as before the move.
//!
//! An entity carries zero to a few extensions, so the storage is a `Vec`
//! scanned linearly; an empty map allocates nothing.

use std::any::{Any, TypeId};

/// One stored value: its type (the key), the type's name (for `Debug`) and
/// the value itself.
struct Slot {
    type_id: TypeId,
    type_name: &'static str,
    value: Box<dyn Any + Send + Sync>,
}

/// Type-keyed feature state on one entity. At most one value per type.
#[derive(Default)]
pub struct EntityExtensions {
    slots: Vec<Slot>,
}

impl EntityExtensions {
    /// An empty map. Allocates nothing.
    pub const fn new() -> Self {
        Self { slots: Vec::new() }
    }

    fn position<T: Any>(&self) -> Option<usize> {
        let id = TypeId::of::<T>();
        self.slots.iter().position(|s| s.type_id == id)
    }

    /// The stored `T`, if any.
    pub fn get<T: Any>(&self) -> Option<&T> {
        let i = self.position::<T>()?;
        self.slots[i].value.downcast_ref::<T>()
    }

    /// The stored `T`, mutably, if any.
    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        let i = self.position::<T>()?;
        self.slots[i].value.downcast_mut::<T>()
    }

    /// Whether a `T` is stored.
    pub fn contains<T: Any>(&self) -> bool {
        self.position::<T>().is_some()
    }

    /// Store `value`, returning the `T` it replaced.
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> Option<T> {
        match self.position::<T>() {
            Some(i) => {
                let old = std::mem::replace(&mut self.slots[i].value, Box::new(value));
                old.downcast::<T>().ok().map(|b| *b)
            }
            None => {
                self.slots.push(Slot {
                    type_id: TypeId::of::<T>(),
                    type_name: std::any::type_name::<T>(),
                    value: Box::new(value),
                });
                None
            }
        }
    }

    /// Take the stored `T` out, if any.
    pub fn remove<T: Any>(&mut self) -> Option<T> {
        let i = self.position::<T>()?;
        self.slots
            .swap_remove(i)
            .value
            .downcast::<T>()
            .ok()
            .map(|b| *b)
    }

    /// How many values are stored.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether nothing is stored.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The stored types' names, in insertion order (until a removal).
    pub fn type_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.slots.iter().map(|s| s.type_name)
    }
}

impl std::fmt::Debug for EntityExtensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.type_names()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct A(u32);
    #[derive(Debug, PartialEq)]
    struct B(&'static str);

    #[test]
    fn empty_map_has_nothing() {
        let m = EntityExtensions::new();
        assert!(m.is_empty());
        assert_eq!(m.get::<A>(), None);
        assert!(!m.contains::<A>());
    }

    #[test]
    fn one_slot_per_type() {
        let mut m = EntityExtensions::new();
        assert_eq!(m.insert(A(1)), None);
        assert_eq!(m.insert(B("x")), None);
        assert_eq!(m.insert(A(2)), Some(A(1)), "a second A replaces the first");
        assert_eq!(m.len(), 2);
        assert_eq!(m.get::<A>(), Some(&A(2)));
        assert_eq!(m.get::<B>(), Some(&B("x")));
    }

    #[test]
    fn get_mut_and_remove() {
        let mut m = EntityExtensions::new();
        m.insert(A(1));
        m.insert(B("x"));
        m.get_mut::<A>().unwrap().0 = 9;
        assert_eq!(m.remove::<A>(), Some(A(9)));
        assert_eq!(m.remove::<A>(), None);
        assert_eq!(m.get::<B>(), Some(&B("x")), "removing A leaves B");
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn debug_lists_type_names() {
        let mut m = EntityExtensions::new();
        m.insert(A(1));
        let s = format!("{m:?}");
        assert!(s.contains("A"), "{s}");
    }
}
