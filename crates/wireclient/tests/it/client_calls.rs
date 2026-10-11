//! The client-call indices against the server's own method table (no DB).
//!
//! A `CM_*` constant that drifts from `cimmeria_wire::names` fails here, so
//! a builder cannot silently call a different server method than its name.

use cimmeria_wire::names::player_cell_method;
use cimmeria_wireclient::calls::{
    CM_CANCEL_MOVIE, CM_DIALOG_BUTTON_CHOICE, CM_GM_GOTO_XYZ, CM_INTERACT, CM_MOVE_ITEM,
    CM_TRIGGER_CLIENT_HINTED_GENERIC_REGION,
};

#[test]
fn client_call_indices_match_the_server_names() {
    assert_eq!(player_cell_method(CM_MOVE_ITEM), Some("moveItem"));
    assert_eq!(player_cell_method(CM_INTERACT), Some("interact"));
    assert_eq!(
        player_cell_method(CM_DIALOG_BUTTON_CHOICE),
        Some("dialogButtonChoice")
    );
    assert_eq!(
        player_cell_method(CM_TRIGGER_CLIENT_HINTED_GENERIC_REGION),
        Some("triggerClientHintedGenericRegion")
    );
    assert_eq!(player_cell_method(CM_CANCEL_MOVIE), Some("cancelMovie"));
    assert_eq!(player_cell_method(CM_GM_GOTO_XYZ), Some("gmGotoXYZ"));
}

/// The wire client keeps its own WSTRING writer (`cimmeria-wire` is a
/// dev-dependency only); this keeps the two encodings identical, including a
/// surrogate pair.
#[test]
fn client_wstring_matches_the_server_encoding() {
    for s in ["", "Ab", "Cine-SGWLogo.SGWLogo", "\u{1F31F}"] {
        let mut ours = Vec::new();
        cimmeria_wireclient::calls::write_wstring(&mut ours, s);
        let mut theirs = Vec::new();
        cimmeria_wire::mercury::write_wstring(&mut theirs, s);
        assert_eq!(ours, theirs, "{s:?}");
    }
}
