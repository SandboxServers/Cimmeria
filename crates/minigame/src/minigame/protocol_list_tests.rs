use super::*;

use crate::minigame::protocol::encode_extension;

#[test]
fn list_keys_are_indices() {
    let value = SfsValue::list(vec![
        SfsValue::Number(1.0),
        SfsValue::String("two".to_string()),
        SfsValue::Bool(true),
    ]);
    let SfsValue::Object(map) = value else {
        panic!("list must be an object");
    };
    assert_eq!(map.len(), 3);
    assert_eq!(map["0"].as_f64(), Some(1.0));
    assert_eq!(map["1"].as_str(), Some("two"));
    assert!(matches!(map["2"], SfsValue::Bool(true)));
}

#[test]
fn list_encodes_as_sfs_array() {
    let mut vars = HashMap::new();
    vars.insert(
        "wireNames".to_string(),
        SfsValue::list(vec![
            SfsValue::String("a".to_string()),
            SfsValue::String("b".to_string()),
            SfsValue::String("c".to_string()),
        ]),
    );
    let xml = encode_extension(&vars);
    assert!(xml.contains("t='a'"));
    assert!(xml.contains("n='2'"));
}
