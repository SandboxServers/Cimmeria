//! The helper's display arguments: the closed allow-list and the wire shape.
use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|a| (*a).to_owned()).collect()
}

#[test]
fn only_the_three_display_arguments_are_allowed() {
    assert!(valid_args(&[]));
    assert!(valid_args(&args(&["-windowed", "ResX=1280", "ResY=720"])));
    assert!(!valid_args(&args(&["-windowed", "-windowed"])));
    // Five digits: the size check bounds each value to 3-4 digits.
    assert!(!valid_args(&args(&["ResX=12800"])));
    assert!(!valid_args(&args(&["-log"])));
    assert!(!valid_args(&args(&["ResX=12a4"])));
    assert!(!valid_args(&args(&[
        "-windowed",
        "ResX=1280",
        "ResY=720",
        "ResY=721"
    ])));
}

fn request_json(args: Option<serde_json::Value>) -> Vec<u8> {
    let mut value = serde_json::json!({
        "schema_version": 1,
        "operation_id": Uuid::from_u128(1),
        "exe": "C:\\Games\\SGW\\Working\\Binaries\\SGW.exe",
        "directory": "C:\\Games\\SGW\\Working\\Binaries",
        "dlls": [],
    });
    if let Some(args) = args {
        value["args"] = args;
    }
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn a_request_without_args_decodes_with_an_empty_list() {
    let request = Request::decode(&request_json(None)).unwrap();
    assert!(request.args.is_empty());
    let request = Request::decode(&request_json(Some(serde_json::json!(["-windowed"])))).unwrap();
    assert_eq!(request.args, args(&["-windowed"]));
}

#[test]
fn an_empty_list_serializes_without_the_key() {
    let mut request = Request::decode(&request_json(None)).unwrap();
    let json = serde_json::to_value(&request).unwrap();
    assert!(json.get("args").is_none(), "{json}");
    request.args = args(&["-windowed", "ResX=1024", "ResY=768"]);
    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(
        json["args"],
        serde_json::json!(["-windowed", "ResX=1024", "ResY=768"])
    );
}
