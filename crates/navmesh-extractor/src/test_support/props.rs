//! UE3 tagged-property stream builder.
//!
//! Mirrors the encoding [`cimmeria_upk::parse_tagged_properties`]
//! consumes, one tag at a time:
//!
//! ```text
//! FName  name          (i32 name index, i32 instance number)
//! FName  type          ("IntProperty", "StructProperty", ...)
//! i32    size          (bytes of value payload)
//! i32    array index
//! FName  struct type   (StructProperty only)
//! i32    value         (BoolProperty only — and no payload follows)
//! u8     value[size]
//! ```
//!
//! terminated by the `None` FName.
//!
//! `ByteProperty` is the Epic-486 shape: no enum name in the tag, one
//! value byte. (An earlier reader heuristic that skipped 8 bytes after a
//! byte tag made this ambiguous; it is gone — see
//! `cimmeria_upk::properties`' `byte_property_does_not_swallow_the_following_tag`.)

use super::names::NameTable;

/// Builder for one export's property block.
pub struct PropStream<'a> {
    names: &'a mut NameTable,
    buf: Vec<u8>,
}

impl<'a> PropStream<'a> {
    /// Start a stream that interns into `names`.
    pub fn new(names: &'a mut NameTable) -> Self {
        Self {
            names,
            buf: Vec::new(),
        }
    }

    fn i32(&mut self, v: i32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    fn f32(&mut self, v: f32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    fn fname(&mut self, n: &str) -> &mut Self {
        let idx = self.names.intern(n);
        self.i32(idx).i32(0)
    }

    fn tag(&mut self, name: &str, type_name: &str, size: i32) -> &mut Self {
        self.fname(name).fname(type_name).i32(size).i32(0)
    }

    /// `IntProperty`.
    pub fn int(&mut self, name: &str, v: i32) -> &mut Self {
        self.tag(name, "IntProperty", 4).i32(v)
    }

    /// `FloatProperty`.
    pub fn float(&mut self, name: &str, v: f32) -> &mut Self {
        self.tag(name, "FloatProperty", 4).f32(v)
    }

    /// `ObjectProperty` — a package object index (positive export,
    /// negative import, 0 for none).
    pub fn object(&mut self, name: &str, v: i32) -> &mut Self {
        self.tag(name, "ObjectProperty", 4).i32(v)
    }

    /// `ByteProperty` — a raw enum ordinal, as SGW cooks
    /// `CoverHeight`/`CoverQuality`.
    pub fn byte(&mut self, name: &str, v: u8) -> &mut Self {
        self.tag(name, "ByteProperty", 1);
        self.buf.push(v);
        self
    }

    /// `ArrayProperty` of object references: `i32 count` then the
    /// indices, the shape of `StaticMeshActor.CoverNodeArray`.
    pub fn object_array(&mut self, name: &str, refs: &[i32]) -> &mut Self {
        self.tag(name, "ArrayProperty", 4 + 4 * refs.len() as i32)
            .i32(refs.len() as i32);
        for &r in refs {
            self.i32(r);
        }
        self
    }

    /// `StructProperty` of type `Vector`.
    pub fn vector(&mut self, name: &str, v: [f32; 3]) -> &mut Self {
        self.tag(name, "StructProperty", 12)
            .fname("Vector")
            .f32(v[0])
            .f32(v[1])
            .f32(v[2])
    }

    /// `StructProperty` of type `Rotator` — UE3 angle units, 65536 per
    /// turn.
    pub fn rotator(&mut self, name: &str, v: [i32; 3]) -> &mut Self {
        self.tag(name, "StructProperty", 12)
            .fname("Rotator")
            .i32(v[0])
            .i32(v[1])
            .i32(v[2])
    }

    /// `NameProperty`.
    pub fn name_value(&mut self, name: &str, v: &str) -> &mut Self {
        let idx = self.names.intern(v);
        self.tag(name, "NameProperty", 8).i32(idx).i32(0)
    }

    /// `BoolProperty` — the value rides in the tag, no payload follows.
    pub fn boolean(&mut self, name: &str, v: bool) -> &mut Self {
        self.fname(name)
            .fname("BoolProperty")
            .i32(0)
            .i32(0)
            .i32(i32::from(v))
    }

    /// The four placement properties an `AActor` carries, in the order
    /// a cooked actor writes them.
    pub fn placement(
        &mut self,
        location: [f32; 3],
        rotation: [i32; 3],
        draw_scale: f32,
        draw_scale_3d: [f32; 3],
    ) -> &mut Self {
        self.vector("Location", location)
            .rotator("Rotation", rotation)
            .float("DrawScale", draw_scale)
            .vector("DrawScale3D", draw_scale_3d)
    }

    /// Append raw bytes. The escape hatch for malformed-stream
    /// regression fixtures — a truncated tag, a garbage name index —
    /// that no typed method should be able to express by accident.
    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(bytes);
        self
    }

    /// Finish with the `None` terminator.
    pub fn finish(mut self) -> Vec<u8> {
        self.fname("None");
        self.buf
    }

    /// Finish **without** the terminator — a property stream that runs
    /// off the end of the export body, which is what a truncated cooked
    /// object looks like.
    pub fn finish_unterminated(self) -> Vec<u8> {
        self.buf
    }
}
