use super::*;
fn result(kind: ResultKind) -> Vec<u8> {
    serde_json::to_vec(&PrepareResult {
        schema_version: 1,
        operation_id: Uuid::from_u128(1),
        prefix_generation: Uuid::from_u128(2),
        result: kind,
    })
    .unwrap()
}
fn read(bytes: &[u8]) -> Result<PrepareResult, &'static str> {
    decode_result(bytes, Uuid::from_u128(1), Uuid::from_u128(2))
}
#[test]
fn installer_failures_and_reboot_codes_cannot_be_laundered_by_probe() {
    for code in [1, 1603, 1641, 3010] {
        let result = after_install(code, || panic!("must not probe after installer failure"));
        assert!(
            matches!(result, ResultKind::InstallerFailed { installer_code } if installer_code == code)
        );
        assert!(read(&self::result(result)).is_ok());
    }
}
#[test]
fn installer_success_still_reports_sdk_failure_without_readiness() {
    let kind = after_install(0, || {
        let mut report = crate::collect(crate::LoadResult::Loaded {}, |_| {
            crate::LoadResult::Loaded {}
        });
        report.physx_sdk = crate::physx::SdkResult::CreateFailed { sdk_error: Some(1) };
        Ok(report)
    });
    let bytes = result(kind);
    let decoded = read(&bytes).unwrap();
    assert!(matches!(decoded.result, ResultKind::Probed { report }
        if report.physx_sdk == (crate::physx::SdkResult::CreateFailed { sdk_error: Some(1) })));
    let value = String::from_utf8(bytes).unwrap();
    assert!(!value.contains("ready") && !value.contains("game_binaries"));
    assert!(matches!(
        after_install(0, || Err(Failure::Probe)),
        ResultKind::Failed {
            reason: Failure::Probe
        }
    ));
}
#[test]
fn host_rejects_stale_identity_forged_success_and_unknown_fields() {
    let bytes = result(ResultKind::InstallerFailed {
        installer_code: 1603,
    });
    assert!(decode_result(&bytes, Uuid::from_u128(3), Uuid::from_u128(2)).is_err());
    assert!(decode_result(&bytes, Uuid::from_u128(1), Uuid::from_u128(3)).is_err());
    assert!(read(&result(ResultKind::InstallerFailed { installer_code: 0 })).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["result"]["path"] = "private".into();
    assert!(read(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(read(&vec![b' '; MAX_RESULT + 1]).is_err());
    let mut report = crate::collect(crate::LoadResult::Loaded {}, |_| {
        crate::LoadResult::Loaded {}
    });
    assert!(read(&result(ResultKind::Probed {
        report: crate::collect(crate::LoadResult::Loaded {}, |_| {
            crate::LoadResult::Loaded {}
        })
    }))
    .is_err());
    report.game_started = true;
    assert!(read(&result(ResultKind::Probed { report })).is_err());
}
#[test]
fn request_rejects_retargeting_unbounded_and_unowned_inputs() {
    let root = std::env::current_dir().unwrap();
    let valid = PrepareRequest {
        schema_version: 1,
        operation_id: Uuid::from_u128(1),
        prefix_generation: Uuid::from_u128(2),
        game_binaries: root.join("game"),
        package: root.join("package.exe"),
        scratch: root.join("scratch"),
    };
    let bytes = serde_json::to_vec(&valid).unwrap();
    assert!(decode_request(&bytes).is_ok());
    for (key, value) in [
        ("executable", serde_json::json!("evil.exe")),
        ("schema_version", 2.into()),
        ("operation_id", Uuid::nil().to_string().into()),
        ("scratch", "relative".into()),
        (
            "game_binaries",
            root.join("../game").to_string_lossy().to_string().into(),
        ),
    ] {
        let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        changed[key] = value;
        assert!(
            decode_request(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "{key}"
        );
    }
    assert!(decode_request(&vec![b' '; MAX_REQUEST + 1]).is_err());
}
