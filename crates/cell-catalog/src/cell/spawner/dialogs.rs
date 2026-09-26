//! Dialog set map cache, monologue dialog id cache, dialog screen text cache.
//!
//! - `load_dialog_set_maps` maps `dialog_set_map_id → (dialog_id, interaction_flags)`
//!   for `add_dialog_set` content actions.
//! - `load_dialog_screen_text` maps `screen_id → text` so the `npc_bark`
//!   content action can speak an original 2009 line without a content
//!   author retyping it into the seed.
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

/// Load `screen_id → text` for every row in `resources.dialog_screens`.
///
/// The `npc_bark` content action names a `screen_id` and nothing else;
/// the executor resolves the line here. Keeping the text server-side is
/// the whole point of the verb: the bark lines are shipped 2009 content
/// (dialog 5019 screens 96351-96354 are Col. Marsh's escort lines), and
/// a `"text"` param would let an author's retype drift from the
/// catalogue with nothing to catch it.
///
/// Loaded once at cell startup, alongside the ~20 other resource caches,
/// because the content executor has no DB pool at action time — a bark
/// cannot afford a cell→base round trip mid-chain (it would break the
/// chain's ordered action list, the same reason `spawn_entity` caches
/// `entity_templates`).
///
/// `screen_id` is globally unique across the 13,467 seeded rows, so this
/// is a flat map rather than keying on `(dialog_id, screen_id)`. A future
/// duplicate would be a seed defect, not a shape the executor should
/// disambiguate, so it is reported rather than silently last-wins.
pub async fn load_dialog_screen_text(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, String>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query("SELECT screen_id, text FROM resources.dialog_screens")
        .fetch_all(pool)
        .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    let mut duplicates = 0usize;
    for r in &rows {
        let screen_id: i32 = r.get("screen_id");
        let text: String = r.get("text");
        if map.insert(screen_id, text).is_some() {
            duplicates += 1;
        }
    }

    if duplicates > 0 {
        // A duplicate means a `npc_bark` naming that screen_id speaks
        // whichever row the query happened to return last — not a
        // failure the executor can detect, so surface it at load.
        tracing::warn!(
            duplicates,
            "dialog_screens has duplicate screen_id values -- npc_bark text \
             resolution for those ids is order-dependent"
        );
    }
    tracing::info!(count = map.len(), "Loaded dialog screen text cache");
    Ok(map)
}
