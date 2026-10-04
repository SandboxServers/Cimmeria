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

/// `bag_name` (the `bag_name` log field) spells every bag ID the way the
/// client's `EInventoryContainerId` does, and names nothing else.
#[test]
fn bag_names_are_the_client_einventorycontainerid_tokens() {
    use cimmeria_entity::inventory::bag_name;
    let client = tokens("EInventoryContainerId");
    for (id, name) in &client {
        assert_eq!(bag_name(*id as i32), Some(name.as_str()), "bag {id}");
    }
    assert_eq!(bag_name(0), None);
    assert_eq!(bag_name(21), None);
}

/// `stat_name` (the `stat_name` log field) is the client's `EStats` token
/// for every stat, and names nothing else.
#[test]
fn stat_names_are_the_client_estats_tokens() {
    use cimmeria_entity::stats::stat_name;
    let client = tokens("EStats");
    for (id, name) in &client {
        assert_eq!(stat_name(*id as i32), Some(name.as_str()), "stat {id}");
    }
    let declared: Vec<i32> = client.iter().map(|(id, _)| *id as i32).collect();
    for id in -1..200 {
        if !declared.contains(&id) {
            assert_eq!(stat_name(id), None, "stat {id} is not in EStats");
        }
    }
}

/// `target_type_name` spells `ETargetType` the client's way.
#[test]
fn target_type_names_are_the_client_etargettype_tokens() {
    use cimmeria_entity::abilities::target_type_name;
    for (id, name) in tokens("ETargetType") {
        assert_eq!(
            target_type_name(id as i32),
            Some(name.as_str()),
            "target type {id}"
        );
    }
    assert_eq!(target_type_name(4), None);
}

/// `sequence_event_name` spells `ESequenceEventType` the client's way.
#[test]
fn sequence_event_names_are_the_client_tokens() {
    use cimmeria_entity::abilities::sequence_event_name;
    let client = tokens("ESequenceEventType");
    assert!(!client.is_empty());
    for (id, name) in &client {
        assert_eq!(
            sequence_event_name(*id as i32),
            Some(name.as_str()),
            "event {id}"
        );
    }
    assert_eq!(sequence_event_name(-1), None);
}
