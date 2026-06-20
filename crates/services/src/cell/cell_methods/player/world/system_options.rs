//! `updateSystemOptions` handling: the NameValuePair wire decoder and the
//! apply-then-persist path for the player's [`SystemOptions`] block.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Parse the `updateSystemOptions(ARRAY <of> NameValuePair)` payload and
/// apply each recognised option to the player's [`SystemOptions`] block.
///
/// Wire layout (BigWorld FIXED_DICT array of NameValuePair):
///
/// ```text
/// [count: u32 LE]
///   for each NameValuePair:
///     [name_char_count: u32 LE][name: u16 LE × char_count]    -- WSTRING
///     [val_char_count : u32 LE][value: u16 LE × char_count]   -- WSTRING
/// ```
///
/// Unrecognised option names are logged at `debug!` so a typo or a
/// not-yet-implemented option (we only honour `autoReload` and
/// `reloadOnActivate` today) doesn't get silently swallowed. The full
/// option surface is in `SGWGame/Content/XML/SystemOptions.xml`; extending
/// is mechanical — add a field on `SystemOptions` and a match arm in
/// [`cimmeria_entity::cell_entity::SystemOptions::apply`].
pub(super) async fn handle_update_system_options(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let pairs = match parse_name_value_pairs(args) {
        Ok(pairs) => pairs,
        Err(e) => {
            // Log loud — a parse failure means the client sent a payload
            // we don't understand, and silently dropping it would leave
            // the user with toggles that look saved but never apply.
            tracing::warn!(
                entity_id,
                error = %e,
                body_len = args.len(),
                "updateSystemOptions: parse failed; options unchanged"
            );
            return;
        }
    };

    // Snapshot post-apply state inside the mutable borrow so we can
    // release the borrow before the `.send()` await — `tx.send` is
    // async and `space_mgr.get_entity_mut` returns a non-Send guard.
    let persist = {
        let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
            tracing::warn!(
                entity_id,
                "updateSystemOptions: entity not found; options dropped"
            );
            return;
        };

        let mut applied = 0usize;
        let mut unknown: Vec<String> = Vec::new();
        for (name, value) in &pairs {
            if entity.system_options.apply(name, value) {
                applied += 1;
            } else {
                unknown.push(name.clone());
            }
        }

        tracing::info!(
            entity_id,
            count = pairs.len(),
            applied,
            auto_reload = entity.system_options.auto_reload,
            reload_on_activate = entity.system_options.reload_on_activate,
            "updateSystemOptions: applied"
        );
        if !unknown.is_empty() {
            tracing::debug!(
                entity_id,
                ?unknown,
                "updateSystemOptions: unknown option names (not yet supported)"
            );
        }

        // Only fire the persist message if (a) we have a player_id (NPCs
        // and unattached entities shouldn't persist) AND (b) at least one
        // known option actually applied (skip the wire round-trip on a
        // pure no-op payload of all-unknown options).
        if applied > 0 {
            entity.player_id.map(|pid| {
                (
                    pid,
                    entity.system_options.auto_reload,
                    entity.system_options.reload_on_activate,
                )
            })
        } else {
            None
        }
    };

    if let Some((player_id, auto_reload, reload_on_activate)) = persist {
        // Fire-and-forget. Mirrors `ActiveSlotUpdate`'s pattern — the
        // base-side handler logs `warn!` on DB write failure; we don't
        // block the cell tick on the round-trip.
        let _ = tx
            .send(CellToBaseMsg::SystemOptionsUpdate {
                player_id,
                auto_reload,
                reload_on_activate,
            })
            .await;
    }
}

/// Decode an `ARRAY <of> NameValuePair`. Visible to tests so the
/// wire-format round-trip can be pinned.
pub(crate) fn parse_name_value_pairs(buf: &[u8]) -> Result<Vec<(String, String)>, String> {
    use crate::mercury::read_wstring;

    if buf.len() < 4 {
        return Err(format!(
            "ARRAY<NameValuePair>: need 4 bytes for count, have {}",
            buf.len()
        ));
    }
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    // Hard cap against a corrupt or hostile count field that would
    // otherwise drive `Vec::with_capacity` into a multi-gigabyte
    // allocation. SystemOptions.xml has ~140 options total (only 2 of
    // them server-synced), so a real client never sends more than a
    // few dozen at once; 256 is well past that with no expectation of
    // tightness. The per-pair `read_wstring` calls below also bounds-
    // check against the buffer, so we don't need a separate
    // remaining-buffer check here — a corrupt count that gets past
    // this cap will still surface as a typed read error.
    if count > 256 {
        return Err(format!(
            "ARRAY<NameValuePair>: implausible count {count} (cap 256)"
        ));
    }
    let mut pairs = Vec::with_capacity(count);
    let mut offset = 4;
    for i in 0..count {
        let (name, n) = read_wstring(buf, offset).map_err(|e| {
            format!(
                "ARRAY<NameValuePair>[{i}].name: {e} (offset {offset}, remaining {})",
                buf.len().saturating_sub(offset)
            )
        })?;
        offset += n;
        let (value, n) = read_wstring(buf, offset).map_err(|e| {
            format!(
                "ARRAY<NameValuePair>[{i}].value: {e} (offset {offset}, remaining {})",
                buf.len().saturating_sub(offset)
            )
        })?;
        offset += n;
        pairs.push((name, value));
    }
    Ok(pairs)
}
