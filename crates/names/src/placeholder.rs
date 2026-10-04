//! Which seed names are not names.
//!
//! The seed fills a missing name with a stand-in: `NO ITEM NAME`,
//! `NO MISSION LABEL`, `UNUSED DIALOGUE.`, `UNUSED. DELETED.`,
//! `UNUSED ERROR CODE`. Logging one of those as `item_name="NO ITEM NAME"`
//! reads like a real name, so the book loads them as unresolved. This is
//! the one list of shapes; a new placeholder of the same shape is caught
//! without an edit here.

/// One placeholder shape, matched against the trimmed, upper-cased name.
#[derive(Debug, Clone, Copy)]
enum Shape {
    /// The whole name (`UNUSED`).
    Exact(&'static str),
    /// The name starts with this (`UNUSED DIALOGUE.`, `UnusedDialog`,
    /// `UNUSED. DELETED.`).
    Prefix(&'static str),
    /// The name starts with the first part and ends with the second
    /// (`NO ITEM NAME`, `NO MISSION DISPLAY NAME`, `NO MISSION LABEL`).
    Framed(&'static str, &'static str),
}

/// Every placeholder shape, case-insensitive. `UNUSED` alone is not enough:
/// mission 819 is really called "Unused Explosive", so only the seed's own
/// stand-ins (`UNUSED`, `UNUSED.`, `UNUSED DIALOG*`, `UNUSED ERROR*`) count.
/// No `regex` dependency for a handful of shapes.
const SHAPES: &[Shape] = &[
    Shape::Exact("UNUSED"),
    Shape::Prefix("UNUSED."),
    Shape::Prefix("UNUSED DIALOG"),
    Shape::Prefix("UNUSEDDIALOG"),
    Shape::Prefix("UNUSED ERROR"),
    Shape::Framed("NO ", "NAME"),
    Shape::Framed("NO ", "LABEL"),
];

/// True when `name` is a seed placeholder rather than a name.
pub fn is_placeholder(name: &str) -> bool {
    let upper = name.trim().to_ascii_uppercase();
    SHAPES.iter().any(|shape| match *shape {
        Shape::Exact(e) => upper == e,
        Shape::Prefix(p) => upper.starts_with(p),
        Shape::Framed(head, tail) => {
            upper.len() >= head.len() + tail.len()
                && upper.starts_with(head)
                && upper.ends_with(tail)
        }
    })
}

/// Why a seed row has no name, or `None` when it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unresolved {
    /// NULL, empty or whitespace only.
    Blank,
    /// A stand-in such as `NO ITEM NAME` (see [`is_placeholder`]).
    Placeholder,
}

/// Classify a seed name: `Ok(trimmed)` when it is a name.
pub fn classify(name: Option<&str>) -> Result<&str, Unresolved> {
    match name.map(str::trim) {
        None | Some("") => Err(Unresolved::Blank),
        Some(n) if is_placeholder(n) => Err(Unresolved::Placeholder),
        Some(n) => Ok(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every placeholder form the seed has, as of `a679e748c`.
    #[test]
    fn each_seed_placeholder_form_is_unresolved() {
        for form in [
            "NO ITEM NAME",
            "NO MISSION DISPLAY NAME",
            "NO MISSION LABEL",
            "UNUSED DIALOGUE",
            "UNUSED DIALOGUE.",
            "UNUSED DIALOGUES",
            "UNUSED DIALOG",
            "UNUSED DIALOG.",
            "Unused Dialog",
            "Unused dialogue",
            "unused dialogue",
            "uNUSED DIALOGUE",
            "UnusedDialog",
            "UNUSED. DELETED.",
            "UNUSED ERROR CODE",
            "UNUSED DIALOG. Text is a result of a shared minigame",
            "Unused",
            "  NO ITEM NAME  ",
        ] {
            assert!(is_placeholder(form), "{form:?} is a placeholder");
            assert_eq!(classify(Some(form)), Err(Unresolved::Placeholder));
        }
    }

    #[test]
    fn blank_names_are_unresolved() {
        assert_eq!(classify(None), Err(Unresolved::Blank));
        assert_eq!(classify(Some("")), Err(Unresolved::Blank));
        assert_eq!(classify(Some("   ")), Err(Unresolved::Blank));
    }

    /// Real names that share a word with a placeholder stay names.
    #[test]
    fn real_names_resolve() {
        for name in [
            "Staff Blast",
            "Nothing Ventured",
            "No Way Out",
            "Noble Gas",
            "Name of the Game",
            "Reused Parts",
            // Mission 819's real name.
            "Unused Explosive",
            "Unused explosive charge from the modified Unas.",
        ] {
            assert!(!is_placeholder(name), "{name:?} is a name");
        }
        assert_eq!(classify(Some(" Jaffa Guard ")), Ok("Jaffa Guard"));
    }
}
