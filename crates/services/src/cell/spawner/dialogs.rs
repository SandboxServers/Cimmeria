//! Dialog set map cache + monologue dialog id cache.
//!
//! - `load_dialog_set_maps` maps `dialog_set_map_id → (dialog_id, interaction_flags)`
//!   for `add_dialog_set` content actions.
//! - `load_monologue_dialog_ids` returns the set of `dialog_id` values whose
//!   every screen has `speaker_id = 0` — player-narration / inner-thought
//!   dialogs that have no NPC speaker. The executor uses this to decide
//!   whether a `display_dialog` chain with no resolved NPC should bind
//!   the player as the dialog context entity (correct for monologues —
//!   the client's per-screen speaker resolution falls back to the
//!   player's name naturally; portrait shows the player) or bail with
//!   a warn (correct for NPC dialogs whose NPC context was lost).

use sqlx::PgPool;
use std::collections::HashSet;

/// Cached row from `resources.dialog_set_maps`, used by `add_dialog_set` content actions.
#[derive(Debug, Clone)]
pub struct DialogSetMapEntry {
    /// `None` for an interaction-only row (`dialog_id IS NULL` in the seed).
    /// Such a row carries an indicator bit in `interaction_flags` and nothing
    /// else: binding it raises the `!` / `?` / quest glow over an NPC without
    /// putting a dialog behind the click. See [`load_dialog_set_maps`].
    pub dialog_id: Option<i32>,
    pub interaction_flags: i64,
}

/// Load the `dialog_set_maps` lookup table from the database.
///
/// Maps `dialog_set_map_id → (dialog_id, interaction_flags)` so that
/// `add_dialog_set` actions can resolve at runtime without per-action DB queries.
///
/// Rows with a NULL `dialog_id` are **kept**, with `dialog_id: None`. They are
/// the interaction-only rows the original content binds for an indicator with
/// no dialog — `Castle.py` binds seven of them (3062, 3071, 3073, 5828, 5829,
/// 5846, 5863). Dropping them (the pre-CA02 behaviour) made every such bind a
/// cache miss, so the indicator never reached the client.
///
/// This is safe on the wire because a bind's only client-visible effect is
/// `SGWSpawnableEntity.InteractionType(UINT64 TypeId)`
/// (`entities/defs/SGWSpawnableEntity.def:114-116`), a single flags bitfield
/// with no dialog field. The dialog id is consulted server-side only when the
/// player clicks, and the click paths skip entries that have none.
pub async fn load_dialog_set_maps(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, DialogSetMapEntry>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT dialog_set_map_id, dialog_id, interaction_flags \
         FROM resources.dialog_set_maps",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    let mut interaction_only = 0usize;
    for r in &rows {
        let id: i32 = r.get("dialog_set_map_id");
        let dialog_id: Option<i32> = r.get("dialog_id");
        let interaction_flags: i64 = r.get("interaction_flags");
        if dialog_id.is_none() {
            interaction_only += 1;
        }
        map.insert(
            id,
            DialogSetMapEntry {
                dialog_id,
                interaction_flags,
            },
        );
    }

    tracing::info!(
        count = map.len(),
        interaction_only,
        "Loaded dialog_set_maps cache"
    );
    Ok(map)
}

/// Load the set of dialog ids whose screens are *all* player-monologue
/// (every `dialog_screens.speaker_id = 0`).
///
/// Used by the `DisplayDialog` executor to distinguish:
///
/// - A monologue dialog (player inner thought, no NPC) where binding
///   the player as the wire `EntityId` of `onDialogDisplay` is the
///   correct render — the client's per-screen lookup of `speaker_id = 0`
///   falls back to the player's own name (see RE finding
///   `docs/reverse-engineering/findings/dialog-portrait-lookup.md`),
///   and the portrait shows the player's character.
/// - A normal NPC dialog where binding the player would blank the NPC
///   portrait and substitute the player's name everywhere — the bug
///   the existing "warn and bail" path was added to prevent.
///
/// Computed as: dialog_ids where every screen has an explicit
/// `speaker_id = 0`. The `dialog_screens.speaker_id` column is
/// nullable and the seed has NULL rows; treating NULL as "monologue"
/// would misclassify dialogs whose screens have unknown speakers
/// (the very mis-binding this cache exists to prevent), so the
/// predicate counts only explicit zeros and requires that to equal
/// the total row count.
pub async fn load_monologue_dialog_ids(pool: &PgPool) -> Result<HashSet<i32>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT dialog_id \
         FROM resources.dialog_screens \
         GROUP BY dialog_id \
         HAVING COUNT(*) > 0 \
            AND COUNT(*) = COUNT(*) FILTER (WHERE speaker_id = 0)",
    )
    .fetch_all(pool)
    .await?;

    let mut ids = HashSet::with_capacity(rows.len());
    for r in &rows {
        let id: i32 = r.get("dialog_id");
        ids.insert(id);
    }

    tracing::info!(count = ids.len(), "Loaded monologue dialog id cache");
    Ok(ids)
}
