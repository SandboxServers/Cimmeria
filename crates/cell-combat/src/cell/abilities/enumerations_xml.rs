//! Test-only reader for the client's `entities/defs/enumerations.xml`, so
//! enum constants are pinned to the client's declaration and never to a
//! copy of themselves (the repo convention, e.g.
//! `docs/analysis/organizations/work-packets.md`).

/// Every `(value, name)` token of the `<enum_name>` block, in file order.
/// The defs pad names and values with spaces, so both are trimmed.
pub(crate) fn tokens(enum_name: &str) -> Vec<(i64, String)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../entities/defs/enumerations.xml");
    let xml = std::fs::read_to_string(&path).expect("read enumerations.xml");
    let open = format!("<{enum_name}>");
    let close = format!("</{enum_name}>");
    let start = xml
        .find(&open)
        .unwrap_or_else(|| panic!("{enum_name} block"));
    let end = start + xml[start..].find(&close).expect("closing tag");
    xml[start..end]
        .split("<Token>")
        .skip(1)
        .map(|chunk| {
            let field = |tag: &str| {
                let (o, c) = (format!("<{tag}>"), format!("</{tag}>"));
                let from = chunk.find(&o).unwrap_or_else(|| panic!("{o}")) + o.len();
                chunk[from..chunk.find(&c).unwrap_or_else(|| panic!("{c}"))]
                    .trim()
                    .to_string()
            };
            let value = field("Value").parse().expect("numeric value");
            (value, field("Name"))
        })
        .collect()
}

/// The value of one token; panics if it is missing or declared twice.
pub(crate) fn token(enum_name: &str, name: &str) -> i64 {
    let found: Vec<i64> = tokens(enum_name)
        .into_iter()
        .filter(|(_, n)| n == name)
        .map(|(v, _)| v)
        .collect();
    match found.as_slice() {
        [v] => *v,
        [] => panic!("{enum_name}::{name} not in enumerations.xml"),
        _ => panic!("{enum_name}::{name} is declared twice"),
    }
}
