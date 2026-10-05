//! The content objects the content engine's Discord events name, as ID +
//! name pairs (Rule 6, "Discord"; NT-10). Characters come from
//! `SpaceManager::discord_character`.

use cimmeria_discord::Named;

/// A mission: its ID and `missions.mission_defn` from the NameBook.
pub(super) fn mission(mission_id: i32) -> Named {
    Named::new(
        mission_id,
        cimmeria_names::book()
            .mission(mission_id)
            .map(str::to_string),
    )
}

/// A dialog: its ID and its NameBook name.
pub(super) fn dialog(dialog_id: i32) -> Named {
    Named::new(
        dialog_id,
        cimmeria_names::book().dialog(dialog_id).map(str::to_string),
    )
}

/// A dialog choice: the cooked `ButtonID` the client sent. The button's
/// text lives only in the client's `CookedDataDialogs.pak`, which the
/// server does not index, so a button renders as `#id`. The one choice
/// the server can name is `-1`: the client closed a dialog that has no
/// buttons (`dialog_overrides` module doc).
pub(super) fn choice(button_id: i32) -> Named {
    let name = (button_id == -1).then(|| "closed".to_string());
    Named::new(button_id, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_a_buttonless_dialog_is_named() {
        assert_eq!(choice(-1), Named::new(-1, Some("closed".into())));
        assert_eq!(choice(8), Named::new(8, None));
    }
}
