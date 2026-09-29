//! `handle_craft_request` routes a gated `craft` to this verb, not to the
//! "not available yet" answer.

use super::*;
use crate::base::crafting::request::handle_craft_request;
use crate::cell::messages::{CraftRequest, CraftVerb};
use cimmeria_cell_catalog::crafting::CraftType;

/// A `craft` with the crafting station bit reaches the verb, whose first
/// rule refuses quantity 0 with its own line.
#[tokio::test]
async fn a_gated_craft_reaches_the_verb() {
    let f = offline(4297);

    handle_craft_request(
        CraftRequest {
            entity_id: f.entity_id,
            player_id: 4298,
            verb: CraftVerb::Craft {
                blueprint_id: BLUEPRINT,
                items: vec![1],
                quantity: 0,
            },
            allowed: CraftType::Craft.bit(),
        },
        &f.ctx(),
    )
    .await;

    assert_eq!(
        f.lines(),
        vec!["You can craft between 1 and 100 at a time.".to_string()]
    );
}
