//! Reading a UE3 `UObject`'s identity out of the client's memory: its name,
//! its class, its outermost package.
//!
//! The layout, recovered from the engine's own code (2026-09-28, Ghidra):
//!
//! | Offset | Field | Evidence |
//! |---|---|---|
//! | `+0x28` | `Outer` (`UObject*`) | `GetOutermost` at `0x0049f090` loops `eax = [eax + 0x28]` |
//! | `+0x2c` | `Name.Index` (`i32`) | `SpawnActor` (`0x00876970`) copies `Class + 0x2c` into the actor's `Tag` |
//! | `+0x30` | `Name.Number` (`i32`) | the same 8-byte copy |
//! | `+0x34` | `Class` (`UClass*`) | `SpawnActor` compares `Template + 0x34` with the class argument |
//!
//! An `FName` prints as its table text, followed by `_<number - 1>` when
//! the number is not zero (`FName -> string` at `0x0049b190`: with a
//! non-zero number it joins the name, a separator and `number - 1`).
//!
//! Everything reads through a [`Reader`], so a stale or garbage object
//! pointer yields `None` and never a fault.

use crate::hooks::sinks::gnames;
use crate::hooks::sinks::mem::{self, Reader};

/// Offset of `Outer`.
pub const OUTER_OFFSET: usize = 0x28;
/// Offset of the `FName` (index, then number).
pub const NAME_OFFSET: usize = 0x2c;
/// Offset of `Class`.
pub const CLASS_OFFSET: usize = 0x34;

/// How far up the `Outer` chain [`outermost`] walks before giving up: a
/// cycle in a corrupt object must not hang the game thread.
const MAX_OUTER_HOPS: usize = 32;

/// `name` with UE3's instance suffix: nothing for number 0, `_<n-1>` after.
pub fn display_name(name: &str, number: i32) -> String {
    if number > 0 {
        format!("{name}_{}", number - 1)
    } else {
        name.to_string()
    }
}

/// The `FName` at `obj + 0x2c`, as text.
pub fn object_name(read: Reader, obj: usize) -> Option<String> {
    let index = mem::read_i32(read, obj + NAME_OFFSET)?;
    let number = mem::read_i32(read, obj + NAME_OFFSET + 4)?;
    let info = gnames::resolve(read, index)?;
    Some(display_name(&info.name, number))
}

/// The class of `obj`: the object's `Class` pointer, then that class's name.
pub fn class_name_of(read: Reader, obj: usize) -> Option<String> {
    let class = mem::read_u32(read, obj + CLASS_OFFSET)? as usize;
    object_name(read, class)
}

/// The outermost `Outer` of `obj` (the package), or `obj` itself when it has
/// none. `None` on an unreadable link or a cycle.
pub fn outermost(read: Reader, obj: usize) -> Option<usize> {
    let mut cur = obj;
    for _ in 0..MAX_OUTER_HOPS {
        let outer = mem::read_u32(read, cur + OUTER_OFFSET)? as usize;
        if outer == 0 {
            return Some(cur);
        }
        cur = outer;
    }
    None
}

/// Read three consecutive `f32`s (a `FVector` or `FRotator`).
pub fn read_vec3(read: Reader, addr: usize) -> Option<[f32; 3]> {
    let mut out = [0f32; 3];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = f32::from_bits(mem::read_u32(read, addr + i * 4)?);
    }
    Some(out)
}

/// A vector rounded to a tenth of a unit, as the JSON array the events use
/// (full `f32` precision is noise in a log line, and `NaN` is not JSON).
pub fn vec3_field(v: [f32; 3]) -> serde_json::Value {
    let r = |x: f32| {
        if x.is_finite() {
            (f64::from(x) * 10.0).round() / 10.0
        } else {
            0.0
        }
    };
    serde_json::json!([r(v[0]), r(v[1]), r(v[2])])
}

#[cfg(test)]
pub(crate) mod fake {
    //! A tiny UE3 object graph in a [`FakeMemory`], for the seam tests.

    use super::*;
    use crate::hooks::sinks::mem::fake::FakeMemory;

    /// Put a name table in `m`: index `i` holds `names[i]`.
    pub fn name_table(m: &mut FakeMemory, names: &[&str]) {
        let array = 0x0100_0000usize;
        m.put(gnames::GNAMES_DATA_ADDR, &(array as u32).to_le_bytes());
        m.put(gnames::GNAMES_NUM_ADDR, &(names.len() as i32).to_le_bytes());
        for (i, name) in names.iter().enumerate() {
            let entry = 0x0200_0000 + i * 0x100;
            m.put(array + i * 4, &(entry as u32).to_le_bytes());
            m.put(entry, &(i as u32).to_le_bytes());
            m.put(entry + gnames::ENTRY_FLAGS_OFFSET, &0u32.to_le_bytes());
            m.put_wide(entry + gnames::ENTRY_NAME_OFFSET, name);
        }
    }

    /// Put a `UObject` header at `addr`: outer, name, class.
    pub fn object(m: &mut FakeMemory, addr: usize, outer: usize, name: (i32, i32), class: usize) {
        m.put(addr + OUTER_OFFSET, &(outer as u32).to_le_bytes());
        m.put(addr + NAME_OFFSET, &name.0.to_le_bytes());
        m.put(addr + NAME_OFFSET + 4, &name.1.to_le_bytes());
        m.put(addr + CLASS_OFFSET, &(class as u32).to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{name_table, object};
    use super::*;
    use crate::hooks::sinks::mem::fake::FakeMemory;

    #[test]
    fn numbered_names_use_the_engines_suffix_rule() {
        assert_eq!(display_name("SGWPawn", 0), "SGWPawn");
        assert_eq!(display_name("SGWPawn", 1), "SGWPawn_0");
        assert_eq!(display_name("SGWPawn", 6), "SGWPawn_5");
    }

    /// An actor `SGWPawn_5` of class `SGWPawn`, in package `sg1_p9q`.
    #[test]
    fn an_object_resolves_its_name_class_and_outermost() {
        let mut m = FakeMemory::new();
        name_table(
            &mut m,
            &["None", "SGWPawn", "sg1_p9q", "Class", "PersistentLevel"],
        );
        // Class object `SGWPawn` (named index 1), whose own class is `Class`.
        object(&mut m, 0x1000, 0, (1, 0), 0x2000);
        object(&mut m, 0x2000, 0, (3, 0), 0);
        // Package.
        object(&mut m, 0x3000, 0, (2, 0), 0x2000);
        // Level inside the package, actor inside the level.
        object(&mut m, 0x4000, 0x3000, (4, 0), 0x2000);
        object(&mut m, 0x5000, 0x4000, (1, 6), 0x1000);
        let r = m.reader();
        assert_eq!(object_name(&r, 0x5000).as_deref(), Some("SGWPawn_5"));
        assert_eq!(class_name_of(&r, 0x5000).as_deref(), Some("SGWPawn"));
        assert_eq!(outermost(&r, 0x5000), Some(0x3000));
        assert_eq!(
            object_name(&r, outermost(&r, 0x5000).unwrap()).as_deref(),
            Some("sg1_p9q")
        );
        // A package is its own outermost.
        assert_eq!(outermost(&r, 0x3000), Some(0x3000));
    }

    #[test]
    fn unreadable_or_unnamed_objects_resolve_to_nothing() {
        let mut m = FakeMemory::new();
        name_table(&mut m, &["None"]);
        object(&mut m, 0x1000, 0, (99, 0), 0x2000); // index past the table
        let r = m.reader();
        assert_eq!(object_name(&r, 0x1000), None);
        assert_eq!(object_name(&r, 0x9000_0000), None);
        assert_eq!(class_name_of(&r, 0x9000_0000), None);
        assert_eq!(outermost(&r, 0x9000_0000), None);
    }

    /// A corrupt `Outer` cycle must not hang a game thread.
    #[test]
    fn an_outer_cycle_is_cut_off() {
        let mut m = FakeMemory::new();
        object(&mut m, 0x1000, 0x2000, (0, 0), 0);
        object(&mut m, 0x2000, 0x1000, (0, 0), 0);
        assert_eq!(outermost(&m.reader(), 0x1000), None);
    }

    #[test]
    fn vectors_read_and_round() {
        let mut m = FakeMemory::new();
        for (i, v) in [1.26f32, -2.0, 3000.04].iter().enumerate() {
            m.put(0x100 + i * 4, &v.to_bits().to_le_bytes());
        }
        let v = read_vec3(&m.reader(), 0x100).unwrap();
        assert_eq!(v, [1.26, -2.0, 3000.04]);
        assert_eq!(vec3_field(v), serde_json::json!([1.3, -2.0, 3000.0]));
        // Non-finite components do not poison the JSON.
        assert_eq!(
            vec3_field([f32::NAN, f32::INFINITY, 1.0]),
            serde_json::json!([0.0, 0.0, 1.0])
        );
        assert_eq!(read_vec3(&m.reader(), 0x9000), None);
    }

    #[test]
    fn the_layout_constants_are_the_recovered_ones() {
        assert_eq!(OUTER_OFFSET, 0x28);
        assert_eq!(NAME_OFFSET, 0x2c);
        assert_eq!(CLASS_OFFSET, 0x34);
    }
}
