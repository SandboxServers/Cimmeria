use super::FailureCode;

/// The fixture the frontend tests read too; both sides must list the same codes.
const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../frontend/src/failure-codes.json"
));

fn fixture_codes() -> Vec<String> {
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(fixture["schema_version"], 1);
    fixture["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|code| code.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn all_serialized_in_order_equals_the_fixture_codes() {
    let serialized: Vec<String> = FailureCode::ALL
        .iter()
        .map(|code| {
            serde_json::to_value(code)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(serialized, fixture_codes());
}

#[test]
fn code_is_its_serde_name_and_round_trips() {
    for &code in FailureCode::ALL {
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, format!("\"{}\"", code.code()));
        assert_eq!(serde_json::from_str::<FailureCode>(&json).unwrap(), code);
    }
}

#[test]
fn every_log_message_is_short_and_free_of_paths_and_placeholders() {
    for &code in FailureCode::ALL {
        let message = code.log_message();
        assert!(!message.is_empty(), "{code:?} has no log message");
        assert!(message.len() <= 80, "{code:?} log message is too long");
        for forbidden in ['/', ':', '{'] {
            assert!(
                !message.contains(forbidden),
                "{code:?} log message contains {forbidden:?}"
            );
        }
    }
}
