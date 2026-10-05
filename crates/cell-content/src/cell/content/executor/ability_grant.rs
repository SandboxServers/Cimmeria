//! `Action::GrantAbility`: the non-GM content grant (Class Start v6, CS-01a).
//!
//! Tutorials, racial cores, class signatures and mission rewards teach
//! abilities through this action (OD-CS04, OD-CS06). The cell does no write
//! of its own: it forwards `ContentGrantAbilities` to the base, which owns
//! `sgw_player.abilities` and the provenance table and decides id by id
//! (learned, already known, converted from trained, starter). The base's
//! answer, `ContentAbilitiesGranted`, is mirrored in
//! `cell::service::base_messages::content_ability_grant`, which sends
//! `onKnownAbilitiesUpdate` and the "You have learned ..." lines.
//!
//! **Archetype gate (review F2).** A grant with an `archetypes` list fires
//! only for a player whose own archetype is in it, whatever the trigger's
//! conditions said: dialog triggers do not set the `archetype` param yet,
//! so a chain's `archetype` condition can fail open. A mismatch sends
//! nothing, no line, and logs one `refused` row. The base checks again
//! against `sgw_player.archetype`.
//!
//! Not GM-gated, and not debounced: a replay is a no-op at the base, and a
//! chain that grants is not something a player can spam.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::{AbilityGrant, AbilityGrantKind};

use crate::cell::messages::{CellToBaseMsg, ContentGrantAbilities};
use crate::cell::space_manager::SpaceManager;

/// `event` of every row (target `content`).
const EVENT: &str = "content_grant_ability";

/// `ability_ids` as `id:name` pairs for one log field (Rule 6).
fn ability_names(ability_ids: &[i32]) -> String {
    let book = cimmeria_names::book();
    ability_ids
        .iter()
        .map(|&id| match book.ability(id) {
            Some(name) => format!("{id}:{name}"),
            None => id.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `source_id`'s name: the mission's, for a mission reward (Rule 6).
fn source_name(grant: &AbilityGrant) -> Option<String> {
    (grant.source_kind == AbilityGrantKind::Mission)
        .then_some(grant.source_id)
        .flatten()
        .and_then(|id| cimmeria_names::book().mission(id).map(str::to_string))
}

/// Run one `grant_ability` action for `entity_id`.
pub(super) async fn run(
    grant: AbilityGrant,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let entity = space_mgr.get_entity(entity_id);
    let is_player = entity.is_some_and(|e| e.is_player);
    let archetype = entity.and_then(|e| e.archetype_id);
    let source_name = source_name(&grant);
    let Some(player_id) = who.player_id.filter(|&id| id > 0 && is_player) else {
        // The chain fired on an NPC or a player with no loaded character:
        // an authoring or lifecycle bug, never ordinary play.
        tracing::warn!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "not_a_player",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            source_kind = grant.source_kind.as_str(),
            source_id = grant.source_id,
            source_name = source_name.as_deref(),
            ability_ids = ?grant.ability_ids,
            ability_names = %ability_names(&grant.ability_ids),
            "grant_ability fired for an entity that is not a loaded player; nothing granted"
        );
        return;
    };
    if !grant.allows(archetype) {
        // Ordinary play (another class walked into the chain), so INFO, and
        // no line: the player was never meant to get it.
        tracing::info!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "archetype_mismatch",
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            archetype,
            archetype_name = archetype.and_then(cimmeria_names::archetype_name),
            allowed_archetypes = ?grant.archetypes,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            source_kind = grant.source_kind.as_str(),
            source_id = grant.source_id,
            source_name = source_name.as_deref(),
            ability_ids = ?grant.ability_ids,
            ability_names = %ability_names(&grant.ability_ids),
            "grant_ability refused: the player's archetype is not in the grant's list"
        );
        return;
    }
    let ability_ids = grant.ability_ids.clone();
    let msg = CellToBaseMsg::ContentGrantAbilities(ContentGrantAbilities {
        entity_id,
        player_id,
        account_id: who.account_id,
        chain_id,
        ability_ids: grant.ability_ids,
        source_kind: grant.source_kind,
        source_id: grant.source_id,
        archetypes: grant.archetypes,
    });
    if let Err(e) = tx.send(msg).await {
        tracing::error!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "cell_to_base_closed",
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            source_kind = grant.source_kind.as_str(),
            source_id = grant.source_id,
            source_name = source_name.as_deref(),
            ability_ids = ?ability_ids,
            ability_names = %ability_names(&ability_ids),
            error = %e,
            "grant_ability: cell->base send failed; nothing granted"
        );
        return;
    }
    // Module-path target: `content` is exported at INFO only, the
    // `cimmeria_cell_content=debug` OTEL_FILTER row exports this one.
    tracing::debug!(
        event = EVENT,
        decision_outcome = "forwarded",
        entity_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id,
        player_name = who.player_name,
        chain_id,
        chain_name = cimmeria_names::book().chain(chain_id),
        source_kind = grant.source_kind.as_str(),
        source_id = grant.source_id,
        source_name = source_name.as_deref(),
        ability_ids = ?ability_ids,
        ability_names = %ability_names(&ability_ids),
        "grant_ability forwarded to the base"
    );
}
