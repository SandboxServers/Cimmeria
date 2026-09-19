//! The package name table, as an interner.
//!
//! Every FName in a UE3 package is a `(name-table index, instance
//! number)` pair, so a fixture cannot emit a property, an import or an
//! export without first deciding what index a string has. One interner
//! is shared by the package writer and every payload encoder that needs
//! names.
//!
//! Index 0 is always `"None"`. That is not cosmetic: the tagged-property
//! parser terminates on the `None` FName, and a payload whose property
//! block is "empty" is literally eight zero bytes. Keeping `None` at 0
//! means a zeroed region decodes as an empty property stream, exactly as
//! it does in real cooked data.

/// Interned package name table.
#[derive(Debug, Clone)]
pub struct NameTable {
    names: Vec<String>,
}

impl Default for NameTable {
    fn default() -> Self {
        Self::new()
    }
}

impl NameTable {
    /// A fresh table holding only `None` at index 0.
    pub fn new() -> Self {
        Self {
            names: vec!["None".to_string()],
        }
    }

    /// Index of `name`, adding it if it is not present yet.
    pub fn intern(&mut self, name: &str) -> i32 {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            return i as i32;
        }
        self.names.push(name.to_string());
        (self.names.len() - 1) as i32
    }

    /// Index of `name` if it has already been interned.
    pub fn get(&self, name: &str) -> Option<i32> {
        self.names.iter().position(|n| n == name).map(|i| i as i32)
    }

    /// Number of entries, i.e. the header's `name_count`.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        // Never actually empty — `None` is always present — but clippy
        // asks for the pair and an honest answer is cheaper than an
        // allow.
        self.names.is_empty()
    }

    /// Serialise the table: per entry an FString then a u64 flags word.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for n in &self.names {
            push_fstring(&mut out, n);
            out.extend_from_slice(&0u64.to_le_bytes());
        }
        out
    }
}

/// Append a UE3 `FString`: an `i32` length that **includes** the NUL
/// terminator, then that many ASCII bytes.
///
/// `cimmeria_upk`'s reader truncates at the first NUL, so the
/// terminator has to be inside the counted run — writing `s.len()`
/// instead of `s.len() + 1` shifts every subsequent table entry by one
/// byte and the package stops parsing.
pub fn push_fstring(out: &mut Vec<u8>, s: &str) {
    assert!(
        s.is_ascii(),
        "fixture names must be ASCII; {s:?} would need the UTF-16 encoding"
    );
    out.extend_from_slice(&((s.len() + 1) as i32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}
