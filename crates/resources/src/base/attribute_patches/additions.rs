//! Error strings the client never shipped, added whole to `ErrorStrings.pak`
//! (category 11).
//!
//! The base answers a refused character name with `ERROR_InvalidCharacterName`
//! (20001), python's code (`Account.py:createCharacter`). The client shows the
//! served `Text` of whatever id `onCharacterCreateFailed` carries, and the
//! shipped PAK has no `_20001` (its seed row came from the Giza dump), so the
//! entry is added here, as `stargate_overrides` adds gate 29. The PAK on disk
//! is never edited; `bump_for` hashes the generated entries into the
//! category's metadata bump, so a client holding the shipped table resyncs.
//!
//! The XML keeps the shipped `COOKED_ERROR_TEXT` shape (SOAP namespaces,
//! attribute order `Text` ... `ErrorID`, an explicit end tag).

use std::collections::HashMap;

use super::CATEGORY_ERROR_STRINGS;
use crate::base::resources::CategoryData;

/// One `COOKED_ERROR_TEXT` entry the client must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorStringAddition {
    /// Cooked key (`_<id>`) and `resources.error_texts.error_id`.
    pub error_id: u32,
    pub moniker_id: u32,
    pub moniker_name: &'static str,
    /// What the client shows. No `"`, `&`, `<` or `>`: the text is written
    /// into an attribute as is, as the attribute patches are.
    pub text: &'static str,
}

/// Every error string Cimmeria adds. Ids must not collide with a shipped
/// entry; a test reads the PAK to hold that.
pub const ERROR_STRING_ADDITIONS: &[ErrorStringAddition] = &[ErrorStringAddition {
    error_id: 20001,
    moniker_id: 20001,
    moniker_name: "ERROR_InvalidCharacterName",
    text: "That name is taken or not allowed. Names are 3 to 20 letters, digits, spaces, \
           hyphens or apostrophes",
}];

/// The `<COOKED_ERROR_TEXT>` entry for one addition.
pub fn generate_error_text_xml(a: &ErrorStringAddition) -> Vec<u8> {
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<COOKED_ERROR_TEXT",
            " xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\"",
            " xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\"",
            " xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"",
            " xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\"",
            " xmlns:CookedData1=\"SGW\"",
            " Text=\"{}\" Flags=\"0\" Language=\"1033\" MonikerName=\"{}\"",
            " MonikerID=\"{}\" ErrorID=\"{}\">",
            "</COOKED_ERROR_TEXT>",
        ),
        a.text, a.moniker_name, a.moniker_id, a.error_id,
    )
    .into_bytes()
}

/// Insert every addition the PAK does not already ship and return their ids.
/// `None` when the category did not load.
pub(super) fn apply(categories: &mut HashMap<u32, CategoryData>) -> Option<Vec<u32>> {
    let Some(category) = categories.get_mut(&CATEGORY_ERROR_STRINGS) else {
        tracing::warn!(
            category = CATEGORY_ERROR_STRINGS,
            reason = "category_not_loaded",
            "error string additions skipped: ErrorStrings did not load"
        );
        return None;
    };
    let mut added = Vec::new();
    for a in ERROR_STRING_ADDITIONS {
        if category.elements.contains_key(&a.error_id) {
            tracing::warn!(
                error_id = a.error_id,
                error_name = a.moniker_name,
                reason = "already_shipped",
                "error string addition skipped: the PAK already ships this id"
            );
            continue;
        }
        category
            .elements
            .insert(a.error_id, generate_error_text_xml(a));
        added.push(a.error_id);
        tracing::info!(
            error_id = a.error_id,
            error_name = a.moniker_name,
            text = a.text,
            "Added Cimmeria error string"
        );
    }
    Some(added)
}
