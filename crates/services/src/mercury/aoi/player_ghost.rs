//! Phase-2 `createOnClient()` cascade for a **player** entering another
//! player's AoI — the "player ghost".
//!
//! Sibling of the NPC cascade in [`super::create`]. It is a separate composer
//! rather than another branch there because the two read from different
//! sources: an NPC's cascade is template data the cell already holds
//! ([`NpcAoIData`](crate::cell::messages::NpcAoIData)), a player's is the
//! cell's live state joined with the base session's identity + appearance
//! cache. Phase 1 (`CREATE_ENTITY` + `UPDATE_AVATAR`) is shared and unchanged.
//!
//! The method set and order mirror the 2009 server's witness cascade for an
//! `SGWPlayer`, which is the chain `SGWSpawnableEntity.createOnClient` →
//! `SGWBeing.createOnClient` → `SGWPlayer.createOnClient`
//! (`deprecated/python/cell/SGWSpawnableEntity.py:125-140`,
//! `SGWBeing.py:491-514`, `SGWPlayer.py:575-581`).

use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;
use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use crate::cell::messages::PlayerAoIData;
use crate::mercury::{
    append_entity_method, encrypt_packet, method_idx, write_wstring, REPLY_FLAGS,
};

/// `SGWPlayer.kismetEventSetId` (`SGWPlayer.py:440`). Witnesses need it as
/// much as the owning client does: it is the event set the client resolves a
/// player ghost's `onSequence` animations against.
pub(crate) const PLAYER_KISMET_EVENT_SET_ID: i32 = 1025;

/// Faction every player is placed in (`setupPlayer`). Shared with the owning
/// client's `mapLoaded` body so the two views of one player cannot drift.
pub(crate) const PLAYER_FACTION: u8 = 3;

/// `GENERICPROPERTY_AmmoTypeId` from `entities/defs/enumerations.xml`.
const GENERICPROPERTY_AMMO_TYPE_ID: i32 = 3;

/// `ARCHETYPE_Any` — "no archetype"; `SGWBeing.createOnClient` skips
/// `onArchetypeUpdate` for it.
const ARCHETYPE_ANY: i32 = 0;

/// A `StatUpdateList` payload with a zero count: just the `u32` length
/// prefix. Python's `sendStats` sends nothing for an empty list.
const EMPTY_STAT_LIST_LEN: usize = 4;

/// Everything the player-ghost cascade needs, borrowed from its two owners.
///
/// The identity fields come from the observee's base session; `live` is the
/// cell's snapshot carried on `CellToBaseMsg::EnteredAoI`.
#[derive(Debug, Clone, Copy)]
pub struct PlayerGhostCascade<'a> {
    /// Character name for `onBeingNameUpdate` — what the witness's nameplate
    /// shows. Skipped when empty, as the legacy cascade does.
    pub name: &'a str,
    pub level: i32,
    pub archetype: i32,
    pub alignment: u8,
    /// Pre-serialized `BeingAppearance(bodySet, componentList)` args, exactly
    /// as last sent to the player's existing witnesses. `None` when the
    /// session has no cached appearance — the ghost then has no body, which
    /// the caller is responsible for logging.
    pub appearance_args: Option<&'a [u8]>,
    /// Pre-serialized `onEntityTint` args.
    pub tint_args: Option<&'a [u8]>,
    pub live: &'a PlayerAoIData,
}

/// Build and encrypt the player-ghost cascade as a standalone reliable packet.
///
/// Sent after [`super::build_create_entity_base`], in its own packet, for the
/// same HOLD-FOR-TRANSACTION reason the NPC cascade is.
pub fn build_player_ghost_cascade(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    ghost: &PlayerGhostCascade<'_>,
    version: EncryptionVersion,
) -> Vec<u8> {
    let body = compose_player_ghost_cascade_body(entity_id, ghost);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key, version)
}

/// Compose the player-ghost cascade body WITHOUT packet framing or
/// encryption, for standalone sends and for the deferred-AoI phase-2 bundle.
///
/// Same transaction-state contract as
/// [`super::compose_create_entity_cascade_body`]: safe alongside other
/// entities' cascades, never in the same bundle as this entity's phase 1.
pub(crate) fn compose_player_ghost_cascade_body(
    entity_id: u32,
    ghost: &PlayerGhostCascade<'_>,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(512);
    let mut method = |index: u16, args: &[u8]| {
        append_entity_method(&mut body, index, IDBASE_SGW_PLAYER, entity_id, args);
    };

    // ── SGWSpawnableEntity.createOnClient ──
    // No GENERICPROPERTY_DatabaseId (players have no template) and no
    // onBeingNameIDUpdate (beingNameId is 0 — a player's name is a string).
    method(
        method_idx::ON_KISMET_EVENT_SET_UPDATE,
        &PLAYER_KISMET_EVENT_SET_ID.to_le_bytes(),
    );
    if let Some(appearance) = ghost.appearance_args {
        method(method_idx::BEING_APPEARANCE, appearance);
        if let Some(tint) = ghost.tint_args {
            method(method_idx::ON_ENTITY_TINT, tint);
        }
    }
    method(method_idx::INTERACTION_TYPE, &0u64.to_le_bytes());
    method(method_idx::ON_ENTITY_FLAGS, &0u64.to_le_bytes());
    method(method_idx::ON_VISIBLE, &[1u8]);

    // ── SGWBeing.createOnClient ──
    method(method_idx::ON_LEVEL_UPDATE, &ghost.level.to_le_bytes());
    method(
        method_idx::ON_TARGET_UPDATE,
        &ghost.live.target_id.to_le_bytes(),
    );
    if !ghost.name.is_empty() {
        let mut args = Vec::with_capacity(4 + ghost.name.len() * 2);
        write_wstring(&mut args, ghost.name);
        method(method_idx::ON_BEING_NAME_UPDATE, &args);
    }
    method(method_idx::ON_ALIGNMENT_UPDATE, &[ghost.alignment]);
    method(method_idx::ON_FACTION_UPDATE, &[PLAYER_FACTION]);
    method(
        method_idx::ON_STATE_FIELD_UPDATE,
        &ghost.live.state_field.to_le_bytes(),
    );
    if ghost.archetype != ARCHETYPE_ANY {
        method(
            method_idx::ON_ARCHETYPE_UPDATE,
            &ghost.archetype.to_le_bytes(),
        );
    }
    if ghost.live.stat_base_update.len() > EMPTY_STAT_LIST_LEN {
        method(
            method_idx::ON_STAT_BASE_UPDATE,
            &ghost.live.stat_base_update,
        );
    }
    if ghost.live.stat_update.len() > EMPTY_STAT_LIST_LEN {
        method(method_idx::ON_STAT_UPDATE, &ghost.live.stat_update);
    }

    // ── SGWPlayer.createOnClient ──
    // onExtraNameUpdate is deliberately absent: the legacy cascade had it
    // commented out because it overwrote every player's displayed name on
    // the witness (`SGWPlayer.py:577-578`).
    let mut ammo = Vec::with_capacity(8);
    ammo.extend_from_slice(&GENERICPROPERTY_AMMO_TYPE_ID.to_le_bytes());
    ammo.extend_from_slice(&ghost.live.ammo_type_id.to_le_bytes());
    method(method_idx::ON_ENTITY_PROPERTY, &ammo);

    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_mercury::channel_bundle::EXTENDED_ENCODING_MARKER;

    const GHOST_ID: u32 = 0x0000_1234;

    /// Decode a cascade body into `(method_index, args)` records, straight
    /// from the documented wire layout rather than via the encoder under
    /// test: direct = `[idx|0x80][len:u16][entity:u32][args]`, extended =
    /// `[0xBD][len:u16][entity:u32][idx-idbase][args]`. Asserts every record
    /// targets `GHOST_ID`.
    fn decode(body: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < body.len() {
            let msg_id = body[i];
            let len = u16::from_le_bytes([body[i + 1], body[i + 2]]) as usize;
            let payload = &body[i + 3..i + 3 + len];
            assert_eq!(
                u32::from_le_bytes(payload[..4].try_into().unwrap()),
                GHOST_ID,
                "every cascade method must target the ghost entity"
            );
            if msg_id == EXTENDED_ENCODING_MARKER {
                let index = u16::from(IDBASE_SGW_PLAYER) + u16::from(payload[4]);
                out.push((index, payload[5..].to_vec()));
            } else {
                out.push((u16::from(msg_id & 0x7F), payload[4..].to_vec()));
            }
            i += 3 + len;
        }
        out
    }

    fn wstring(s: &str) -> Vec<u8> {
        let mut v = Vec::new();
        write_wstring(&mut v, s);
        v
    }

    fn live() -> PlayerAoIData {
        PlayerAoIData {
            state_field: 0b1000, // BSF_InCombat
            target_id: 77,
            stat_update: vec![
                1, 0, 0, 0, 9, 9, 9, 9, 0, 0, 0, 0, 50, 0, 0, 0, 100, 0, 0, 0,
            ],
            stat_base_update: vec![
                1, 0, 0, 0, 9, 9, 9, 9, 0, 0, 0, 0, 100, 0, 0, 0, 100, 0, 0, 0,
            ],
            ammo_type_id: 12,
        }
    }

    fn ghost<'a>(
        live: &'a PlayerAoIData,
        appearance: &'a [u8],
        tint: &'a [u8],
    ) -> PlayerGhostCascade<'a> {
        PlayerGhostCascade {
            name: "Lomiada",
            level: 9,
            archetype: 4,
            alignment: 2,
            appearance_args: Some(appearance),
            tint_args: Some(tint),
            live,
        }
    }

    /// Wire-format guard: the full player-ghost cascade, method by method,
    /// in legacy `createOnClient` order with the exact arg bytes. This is
    /// the test that fails if the cascade regresses to the bare shape — no
    /// `BeingAppearance`, no `onBeingNameUpdate`, placeholder stats — that
    /// left players invisible and nameless to each other.
    #[test]
    fn cascade_emits_identity_appearance_and_live_state_in_legacy_order() {
        let live = live();
        let appearance = [0xAA, 0xBB, 0xCC];
        let tint = [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
        let body = compose_player_ghost_cascade_body(GHOST_ID, &ghost(&live, &appearance, &tint));

        let mut ammo = GENERICPROPERTY_AMMO_TYPE_ID.to_le_bytes().to_vec();
        ammo.extend_from_slice(&12i32.to_le_bytes());
        let expected: Vec<(u16, Vec<u8>)> = vec![
            (
                method_idx::ON_KISMET_EVENT_SET_UPDATE,
                1025i32.to_le_bytes().to_vec(),
            ),
            (method_idx::BEING_APPEARANCE, appearance.to_vec()),
            (method_idx::ON_ENTITY_TINT, tint.to_vec()),
            (method_idx::INTERACTION_TYPE, 0u64.to_le_bytes().to_vec()),
            (method_idx::ON_ENTITY_FLAGS, 0u64.to_le_bytes().to_vec()),
            (method_idx::ON_VISIBLE, vec![1]),
            (method_idx::ON_LEVEL_UPDATE, 9i32.to_le_bytes().to_vec()),
            (method_idx::ON_TARGET_UPDATE, 77i32.to_le_bytes().to_vec()),
            (method_idx::ON_BEING_NAME_UPDATE, wstring("Lomiada")),
            (method_idx::ON_ALIGNMENT_UPDATE, vec![2]),
            (method_idx::ON_FACTION_UPDATE, vec![3]),
            (
                method_idx::ON_STATE_FIELD_UPDATE,
                0b1000u32.to_le_bytes().to_vec(),
            ),
            (method_idx::ON_ARCHETYPE_UPDATE, 4i32.to_le_bytes().to_vec()),
            (
                method_idx::ON_STAT_BASE_UPDATE,
                live.stat_base_update.clone(),
            ),
            (method_idx::ON_STAT_UPDATE, live.stat_update.clone()),
            (method_idx::ON_ENTITY_PROPERTY, ammo),
        ];
        assert_eq!(decode(&body), expected);
    }

    /// The optional methods are skipped exactly where the legacy cascade
    /// skips them: no appearance cached, empty name, `ARCHETYPE_Any`, and
    /// zero-count stat lists. The ghost must still be made visible.
    #[test]
    fn cascade_skips_absent_optionals_but_still_makes_the_ghost_visible() {
        let live = PlayerAoIData {
            stat_update: 0u32.to_le_bytes().to_vec(),
            stat_base_update: 0u32.to_le_bytes().to_vec(),
            ..PlayerAoIData::default()
        };
        let sparse = PlayerGhostCascade {
            name: "",
            level: 1,
            archetype: ARCHETYPE_ANY,
            alignment: 0,
            appearance_args: None,
            // A tint without a body is meaningless; it must not be sent alone.
            tint_args: Some(&[0; 12]),
            live: &live,
        };
        let indices: Vec<u16> = decode(&compose_player_ghost_cascade_body(GHOST_ID, &sparse))
            .into_iter()
            .map(|(index, _)| index)
            .collect();

        for skipped in [
            method_idx::BEING_APPEARANCE,
            method_idx::ON_ENTITY_TINT,
            method_idx::ON_BEING_NAME_UPDATE,
            method_idx::ON_ARCHETYPE_UPDATE,
            method_idx::ON_STAT_BASE_UPDATE,
            method_idx::ON_STAT_UPDATE,
        ] {
            assert!(
                !indices.contains(&skipped),
                "method {skipped} must be skipped: {indices:?}"
            );
        }
        assert!(indices.contains(&method_idx::ON_VISIBLE));
    }

    /// Bundle/standalone equivalence, same contract as the NPC cascade: the
    /// deferred-AoI flush appends the composed body to a bundle while the
    /// live path frames it standalone, and the two must not diverge.
    #[test]
    fn composed_body_matches_the_standalone_packet_body() {
        use cimmeria_mercury::encryption::MercuryEncryption;
        const KEY: [u8; 32] = [7u8; 32];

        let live = live();
        let ghost = ghost(&live, &[0xAA], &[0; 12]);
        let composed = compose_player_ghost_cascade_body(GHOST_ID, &ghost);

        let pkt = build_player_ghost_cascade(&KEY, 1, &[], GHOST_ID, &ghost, EncryptionVersion::V1);
        let pt = MercuryEncryption::from_session_key(KEY)
            .decrypt(&pkt)
            .unwrap();
        assert_eq!(composed.as_slice(), &pt[1..pt.len() - 4]);
    }
}
