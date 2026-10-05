//! BigWorld's method-table flattening, replayed over `entities/defs/`.
//!
//! The rule (`entity_description.cpp:parseInterface()`, restated in
//! `docs/protocol/client-method-dispatch-table.md` "BigWorld Flattening
//! Rule"): walk the `<Parent>` chain from root to leaf; at each level first
//! flatten that level's `<Implements>` interfaces, recursively and in
//! document order, then append the level's own methods. ClientMethods count
//! every method; CellMethods and BaseMethods count only `<Exposed/>` ones,
//! because only those have a client-to-server wire index.
//!
//! Two lookups keep the rule honest:
//!
//! - a `<Parent>` resolves only against `defs/<Name>.def`, and a missing file
//!   is the chain root (`Account`'s parent `GamePawn` is an engine class,
//!   not an entity def);
//! - an `<Interface>` resolves only against `defs/interfaces/<Name>.def`.
//!   `SGWBeing` exists as both, and mixing them up shifts every index.
//!
//! The parser is a small hand-rolled element walker: the `.def` files are
//! plain XML with comments and no CDATA, and this crate has no XML
//! dependency.

use std::path::PathBuf;

/// Which method section to flatten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Client,
    Cell,
    Base,
}

impl Section {
    fn tag(self) -> &'static str {
        match self {
            Section::Client => "ClientMethods",
            Section::Cell => "CellMethods",
            Section::Base => "BaseMethods",
        }
    }

    /// Only `<Exposed/>` cell and base methods have a wire index.
    fn exposed_only(self) -> bool {
        !matches!(self, Section::Client)
    }
}

fn defs_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../entities/defs"))
}

/// `text` with every `<!-- ... -->` span removed.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = rest[start..]
            .find("-->")
            .map_or("", |end| &rest[start + end + 3..]);
    }
    out.push_str(rest);
    out
}

/// A def file with comments stripped, or `None` if it does not exist.
fn read_def(path: PathBuf) -> Option<String> {
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(strip_comments(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => panic!("reading {}: {e}", path.display()),
    }
}

/// The body between the first `<name>` and its `</name>`.
fn section<'a>(def: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let start = def.find(&open)? + open.len();
    let end = def[start..].find(&format!("</{name}>"))? + start;
    Some(&def[start..end])
}

/// Trimmed text of every `<tag>...</tag>` in `body`.
fn tag_texts(body: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    body.split(&open)
        .skip(1)
        .filter_map(|s| s.split(&close).next())
        .map(|s| s.trim().to_string())
        .collect()
}

/// The direct child elements of a method section, in document order, each
/// with whether it carries an `<Exposed/>` child.
fn methods_in(body: &str) -> Vec<(String, bool)> {
    let mut methods: Vec<(String, bool)> = Vec::new();
    let mut depth = 0usize;
    for tag in body.split('<').skip(1) {
        let tag = tag.split('>').next().unwrap_or("").trim();
        if tag.starts_with('/') {
            depth = depth.saturating_sub(1);
            continue;
        }
        let (name, self_closing) = match tag.strip_suffix('/') {
            Some(name) => (name.trim(), true),
            None => (tag, false),
        };
        let name = name.split_whitespace().next().unwrap_or("");
        match depth {
            0 => methods.push((name.to_string(), false)),
            1 if name == "Exposed" => {
                if let Some(last) = methods.last_mut() {
                    last.1 = true;
                }
            }
            _ => {}
        }
        if !self_closing {
            depth += 1;
        }
    }
    methods
}

/// Append one def level: its interfaces first, then its own methods.
fn push_level(def: &str, section_kind: Section, out: &mut Vec<String>) {
    if let Some(body) = section(def, "Implements") {
        for iface in tag_texts(body, "Interface") {
            let path = defs_dir().join("interfaces").join(format!("{iface}.def"));
            let iface_def = read_def(path)
                .unwrap_or_else(|| panic!("interface {iface} has no defs/interfaces/{iface}.def"));
            push_level(&iface_def, section_kind, out);
        }
    }
    if let Some(body) = section(def, section_kind.tag()) {
        out.extend(
            methods_in(body)
                .into_iter()
                .filter(|(_, exposed)| !section_kind.exposed_only() || *exposed)
                .map(|(name, _)| name),
        );
    }
}

/// The flattened method table of entity `entity`: the wire index of a method
/// is its position in the returned list.
pub(crate) fn flatten(entity: &str, section_kind: Section) -> Vec<String> {
    let def = read_def(defs_dir().join(format!("{entity}.def")))
        .unwrap_or_else(|| panic!("entity {entity} has no defs/{entity}.def"));
    flatten_level(&def, section_kind)
}

/// Every client-visible entity type with its clientIndex (the wire typeID),
/// in `entities/entities.xml` document order. The client hands the next
/// index only to entries whose `.def` is not `<ServerOnly/>`
/// (`EntityDescriptionMap_parse @ ghidra://SGW.exe@0x01590520`, the second
/// counter at `desc+0x1e`), so `Account` is 0x07, not its row 8.
pub(crate) fn client_classes() -> Vec<(u8, String)> {
    let xml_path = defs_dir().join("../entities.xml");
    let xml =
        read_def(xml_path.clone()).unwrap_or_else(|| panic!("{} is missing", xml_path.display()));
    let mut out = Vec::new();
    for tag in xml.split('<').skip(1) {
        // Only self-closing `<Name/>` entries; skips `<root>` / `</root>`.
        let Some((name, _)) = tag.split_once("/>") else {
            continue;
        };
        let name = name.trim();
        let def = read_def(defs_dir().join(format!("{name}.def")))
            .unwrap_or_else(|| panic!("entities.xml lists {name}, which has no defs/{name}.def"));
        if def.contains("<ServerOnly") {
            continue;
        }
        let index = u8::try_from(out.len()).expect("fewer than 256 client entity types");
        out.push((index, name.to_string()));
    }
    out
}

fn flatten_level(def: &str, section_kind: Section) -> Vec<String> {
    let mut out = match tag_texts(def, "Parent").first() {
        // A parent with no entity def is the chain root (`GamePawn`).
        Some(parent) => read_def(defs_dir().join(format!("{parent}.def")))
            .map(|parent_def| flatten_level(&parent_def, section_kind))
            .unwrap_or_default(),
        None => Vec::new(),
    };
    push_level(def, section_kind, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods_in_reads_direct_children_and_their_exposed_flag() {
        let body = "
            <first> <Exposed/> <Arg> INT32 </Arg> </first>
            <second> <Arg> WSTRING </Arg> </second>
            <third/>
            <fourth> <Exposed/> </fourth>";
        assert_eq!(
            methods_in(body),
            vec![
                ("first".to_string(), true),
                ("second".to_string(), false),
                ("third".to_string(), false),
                ("fourth".to_string(), true),
            ]
        );
    }

    #[test]
    fn comments_are_stripped_before_parsing() {
        assert_eq!(strip_comments("<a/><!-- <b/> --><c/><!-- tail"), "<a/><c/>");
    }
}
