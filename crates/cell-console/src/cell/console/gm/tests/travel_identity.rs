use super::*; // shared helpers from tests/mod.rs
use crate::test_support::LogCapture;
use tokio::sync::mpsc;

/// TG-MOV-07: the "teleporting GM" row carries the calling GM's Rule 5 identity.
#[tokio::test]
async fn gm_goto_xyz_teleporting_row_carries_account_identity() {
    let mut mgr = mgr_with_player(1, "Castle");
    {
        let e = mgr.get_entity_mut(1).unwrap();
        e.account_id = Some(7);
        e.stamp_log_names(Some("GmCaller"), Some("gm-account"));
    }
    let (tx, _rx) = mpsc::channel(8);

    let mut args = Vec::new();
    for c in [10.0f32, 20.0, 30.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    let capture = LogCapture::install();
    assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr, &test_engine()).await);

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.message.as_deref() == Some("gmGotoXYZ: teleporting GM"))
        .expect("gmGotoXYZ must log the teleporting GM row");
    assert!(
        row.has_field("account_id", "7"),
        "the GM row must carry account_id 7, got fields: {:?}",
        row.fields
    );
}
