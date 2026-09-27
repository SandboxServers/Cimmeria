//! Pure appearance/tint wire-arg builders and the post-burst / resend bundle
//! composers.
//!
//! Extracted from `world_entry_appearance.rs` — these are side-effect-free
//! functions that build the BeingAppearance / onEntityTint wire data and
//! compose the Mercury bundles the visual-resend handlers dispatch. Keeping
//! them together with their byte-exact regression guards isolates the wire
//! shape from the async handler logic in `client_ready.rs` / `cinematic.rs`.

use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};

use crate::mercury::{method_idx, write_wstring, SKIN_TINTS};

// ── Appearance data builders ────────────────────────────────────────────────

/// Build the BeingAppearance wire args: `[wstring bodyset][u32 count][wstring comp]*`.
///
/// Used by `handle_map_loaded` to cache for later resend, and by
/// `handle_on_client_ready` / `handle_cancel_movie` to resend.
pub fn build_appearance_args(bodyset: &str, components: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    write_wstring(&mut buf, bodyset);
    buf.extend_from_slice(&(components.len() as u32).to_le_bytes());
    for comp in components {
        write_wstring(&mut buf, comp);
    }
    buf
}

/// Build the onEntityTint wire args: `[u32 primary=0][u32 secondary=0][u32 skin_tint]`.
///
/// Maps `skin_color_id` (DB index) through the SKIN_TINTS table, matching
/// the C++ `requestCharacterVisuals` flow that sends the mapped tint value.
pub fn build_tint_args(skin_color_id: i32) -> Vec<u8> {
    let skin_tint = if (skin_color_id as usize) < SKIN_TINTS.len() {
        SKIN_TINTS[skin_color_id as usize]
    } else {
        SKIN_TINTS[0]
    };
    let mut buf = Vec::with_capacity(12);
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&skin_tint.to_le_bytes());
    buf
}

/// Compose the post-onClientReady burst into a single Mercury bundle.
///
/// Order matches the original per-message dispatch sequence so a regression
/// that swaps two entries can't slip past via the packet-count check:
///   1. `BeingAppearance` resend  (heals HOLD-FOR-TRANSACTION drop from
///      `handle_map_loaded`'s bundle — see the two-bundle split comment
///      block in `cimmeria_base_world_entry::base::world_entry::map_loaded` for the
///      transaction-state rationale, and `docs/architecture/mercury-bundle.md`
///      for the ADR.)
///   2. `onEntityTint` resend     (same reason)
///   3. `onPlayerCommunication`   (welcome line, `CHAN_FEEDBACK`)
///
/// No `onChatJoined`: the client knows every built-in channel without one,
/// and files any it receives as a user channel (SS-C4; the evidence is in
/// the `world_entry_chat` module doc). Until SS-C4 this burst carried eight.
///
/// Extracted as a pure builder so the burst-shape regression guard
/// [`tests::on_client_ready_burst_bundles_to_single_packet`] can pin
/// `num_messages = 3` and `estimated_packet_count() = 1` against
/// realistic arg sizes — the same composition the handler actually emits
/// (call-site duplication would let the test and the handler drift).
pub fn build_on_client_ready_burst_bundle(
    entity_id: u32,
    appearance_args: &[u8],
    tint_args: &[u8],
    welcome_args: &[u8],
) -> ChannelBundle {
    let mut bundle = ChannelBundle::new(true);
    for (method_index, args) in
        on_client_ready_burst_messages(appearance_args, tint_args, welcome_args)
    {
        bundle.append_entity_method(method_index, IDBASE_SGW_PLAYER, entity_id, args);
    }
    bundle
}

/// The post-onClientReady burst as `(method_index, args)`, in send order.
/// [`build_on_client_ready_burst_bundle`] appends exactly these, so a test
/// can read the method list without decoding the bundle body.
fn on_client_ready_burst_messages<'a>(
    appearance_args: &'a [u8],
    tint_args: &'a [u8],
    welcome_args: &'a [u8],
) -> [(u16, &'a [u8]); 3] {
    [
        (method_idx::BEING_APPEARANCE, appearance_args),
        (method_idx::ON_ENTITY_TINT, tint_args),
        (method_idx::ON_PLAYER_COMMUNICATION, welcome_args),
    ]
}

/// Compose the BeingAppearance + onEntityTint resend pair into one bundle.
///
/// Both methods target the player's own entity (long since created in
/// `handle_map_loaded`'s bundle), so the transaction-state rule allows
/// combining them — see the safe-combine catalogue in
/// `docs/architecture/mercury-bundle.md` and the two-bundle split comment in
/// `cimmeria_base_world_entry::base::world_entry::map_loaded` for the rationale.
///
/// Called per-iteration of the cinematic-guard spam loop (every 100 ms for
/// up to 20 s) and also from `handle_cancel_movie` on real client
/// `cancelMovie`. Extracted as a pure builder so
/// [`tests::appearance_resend_bundle_collapses_to_single_packet`] can pin
/// `num_messages == 2` and `estimated_packet_count() == 1` against the same
/// composition the resend path actually emits.
pub fn build_appearance_resend_bundle(
    entity_id: u32,
    appearance_args: &[u8],
    tint_args: &[u8],
) -> ChannelBundle {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(
        method_idx::BEING_APPEARANCE,
        IDBASE_SGW_PLAYER,
        entity_id,
        appearance_args,
    );
    bundle.append_entity_method(
        method_idx::ON_ENTITY_TINT,
        IDBASE_SGW_PLAYER,
        entity_id,
        tint_args,
    );
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::world_entry_chat::build_welcome_message_args;
    use crate::mercury::SKIN_TINTS;

    /// `build_appearance_args` wire layout:
    /// `[wstring bodyset] [u32 LE component_count] [wstring component]*`
    /// where each wstring is `[u32 LE char_count] [UTF-16LE chars]`.
    /// Asserts the COMPLETE byte vector against a hand-computed
    /// expected slice — partial spot-checks would let a broken
    /// implementation that emits the right first byte but wrong
    /// subsequent ones still pass.
    #[test]
    fn build_appearance_args_emits_bodyset_count_components_layout() {
        let buf = build_appearance_args("Body", &["A".to_string(), "BB".to_string()]);
        let expected: &[u8] = &[
            // bodyset wstring: count=4, then 'B' 0 'o' 0 'd' 0 'y' 0
            4, 0, 0, 0, b'B', 0, b'o', 0, b'd', 0, b'y', 0, // component_count = 2
            2, 0, 0, 0, // component "A": count=1, 'A' 0
            1, 0, 0, 0, b'A', 0, // component "BB": count=2, 'B' 0 'B' 0
            2, 0, 0, 0, b'B', 0, b'B', 0,
        ];
        assert_eq!(buf, expected, "byte-exact wire layout");
    }

    /// Non-BMP regression guard: write_wstring must use the
    /// UTF-16 code-unit count, NOT `chars().count()`, for the
    /// length prefix. Pick "🌟" (U+1F31F) — a single Unicode
    /// scalar that requires a UTF-16 surrogate PAIR (D83C DF1F),
    /// so:
    ///   - `chars().count()`           = 1
    ///   - `encode_utf16().count()`    = 2
    ///   - UTF-8 byte length           = 4
    ///
    /// All three values are distinct, so the byte-exact assertion
    /// catches both the "drifted to UTF-8" regression and the
    /// "used chars().count() instead of encode_utf16().count()"
    /// regression. (A simpler character like é wouldn't distinguish
    /// the second case because its chars and UTF-16 counts agree.)
    #[test]
    fn build_appearance_args_emits_utf16_for_non_bmp_components() {
        let buf = build_appearance_args("Body", &["🌟".to_string()]);
        let expected: &[u8] = &[
            // bodyset wstring: "Body"
            4, 0, 0, 0, b'B', 0, b'o', 0, b'd', 0, b'y', 0, // component_count = 1
            1, 0, 0, 0,
            // component "🌟": char_count = 2 (UTF-16 code units),
            // then surrogate pair D83C DF1F as little-endian u16s.
            2, 0, 0, 0, 0x3C, 0xD8, 0x1F, 0xDF,
        ];
        assert_eq!(
            buf, expected,
            "non-BMP string must serialize as UTF-16-LE surrogate pair with code-unit count"
        );
    }

    /// Empty-components case must emit the EXACT bodyset wstring
    /// followed by a u32 zero count, with no trailing bytes. Pin
    /// the full byte sequence (not just a length + count slice)
    /// so a regression that drifts the bodyset payload bytes —
    /// e.g. flipping endianness or emitting UTF-8 in the bodyset
    /// while leaving the count zero — still fails this test.
    #[test]
    fn build_appearance_args_with_no_components_emits_zero_count() {
        let buf = build_appearance_args("X", &[]);
        let expected: &[u8] = &[
            // bodyset "X": char_count = 1, then 'X' 0x00
            1, 0, 0, 0, b'X', 0, // component_count = 0
            0, 0, 0, 0,
        ];
        assert_eq!(buf, expected);
    }

    /// `build_tint_args` wire layout: `[u32 0][u32 0][u32 LE skin_tint]`.
    /// The first two slots are reserved for primary/secondary tints and
    /// must always be zero.
    #[test]
    fn build_tint_args_layout_with_valid_skin_color_id() {
        let buf = build_tint_args(3);
        assert_eq!(buf.len(), 12);
        assert_eq!(buf[0..4], [0, 0, 0, 0], "primary tint must be 0");
        assert_eq!(buf[4..8], [0, 0, 0, 0], "secondary tint must be 0");
        let tint = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        assert_eq!(tint, SKIN_TINTS[3]);
    }

    /// Out-of-range skin_color_id falls back to SKIN_TINTS[0]. Pin so a
    /// future regression that panics on the index path can't crash the
    /// world-entry flow.
    #[test]
    fn build_tint_args_clamps_oob_skin_color_id_to_zero() {
        let buf = build_tint_args(999);
        let tint = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        assert_eq!(tint, SKIN_TINTS[0]);
    }

    /// Negative skin_color_id casts to a huge usize and falls into the
    /// fallback branch. Pin so the cast doesn't accidentally succeed
    /// after a refactor (which would index into garbage).
    #[test]
    fn build_tint_args_negative_id_falls_back() {
        let buf = build_tint_args(-1);
        let tint = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        assert_eq!(tint, SKIN_TINTS[0]);
    }

    /// **Burst-shape regression guard for the `handle_on_client_ready`
    /// bundle migration.**
    ///
    /// Before the migration, `handle_on_client_ready` emitted 11 reliable
    /// packets at world entry (1 BeingAppearance + 1 onEntityTint + 8
    /// onChatJoined + 1 onPlayerCommunication welcome), each consuming a
    /// slot in the per-channel 32-slot reliable TX window. After the
    /// migration, the burst rides one `ChannelBundle` that finalizes to a
    /// single fragment whenever the body fits inside `FRAGMENT_BODY_SIZE`.
    /// SS-C4 then dropped the eight `onChatJoined`, leaving three messages.
    ///
    /// Pin two invariants the migration depends on:
    ///   - `num_messages == 3` — every message expected in the burst is
    ///     appended, and nothing else (the old `onChatJoined` loop would
    ///     make it 11).
    ///   - `estimated_packet_count() == 1` — realistic-sized inputs
    ///     comfortably fit a single fragment. A future regression that
    ///     either bloats one of the appended args past
    ///     `FRAGMENT_BODY_SIZE - other_messages`
    ///     would tip this to 2+ packets and fire here, prompting an
    ///     explicit re-audit of the bundle shape (and potentially a
    ///     transaction-state re-audit if the second fragment lands after
    ///     a same-entity CREATE in the same client frame).
    ///
    /// Args sizes mirror the production wire: appearance is a typical
    /// 8-component humanoid set (~120 B), tint is the fixed 12 B layout
    /// from `build_tint_args`, welcome uses the realistic entity_id 12345
    /// to exercise the multi-digit branch of the welcome text formatter.
    #[test]
    fn on_client_ready_burst_bundles_to_single_packet() {
        use cimmeria_mercury::packet::FRAGMENT_BODY_SIZE;

        const ENTITY_ID: u32 = 12345;

        // Realistic 8-component humanoid appearance (Castle_CellBlock spawn
        // shape — see `build_appearance_args` test cases for the canonical
        // wire layout).
        let appearance = build_appearance_args(
            "MaleBody",
            &[
                "Hair".to_string(),
                "Head".to_string(),
                "Torso".to_string(),
                "Legs".to_string(),
                "Hands".to_string(),
                "Feet".to_string(),
                "Belt".to_string(),
                "Backpack".to_string(),
            ],
        );
        let tint = build_tint_args(3);
        let welcome = build_welcome_message_args("Cadacious", ENTITY_ID);

        let bundle = build_on_client_ready_burst_bundle(ENTITY_ID, &appearance, &tint, &welcome);

        // Count check: every burst message lands in the bundle, and no
        // onChatJoined does (SS-C4).
        assert_eq!(
            bundle.num_messages(),
            3,
            "burst must be BeingAppearance + onEntityTint + welcome, nothing else"
        );

        // Single-fragment shape: the burst is small enough that
        // estimated_packet_count drops to 1 for the current channel set + arg
        // sizes. If realistic args grow past the fragment size, this assert
        // forces an explicit re-audit (the migration's "1 frame == 1 bundle"
        // safety claim hinges on the post-finalize fragment count).
        assert!(
            bundle.body_len() < FRAGMENT_BODY_SIZE,
            "burst body ({} B) must fit one fragment (limit {} B) — a regression here \
             means an arg ballooned, and the migration's \
             single-packet shape needs a re-audit",
            bundle.body_len(),
            FRAGMENT_BODY_SIZE
        );
        assert_eq!(
            bundle.estimated_packet_count(),
            1,
            "post-bundle burst must collapse to a single reliable packet \
             (was 11 pre-bundle, before SS-C4 dropped onChatJoined)"
        );
    }

    /// SS-C4 (D-ORG14): the login burst carries no `onChatJoined` for a
    /// built-in channel. The client hardcodes ids 0-11 and files every
    /// `onChatJoined` as a user channel with a "You have joined channel"
    /// line (`ChatWindow.lua:370-386`); the legacy server sent it only for
    /// ids of 12 and up (`SGWPlayer.py:162-163`). Restoring the old
    /// eight-channel loop fails here and in the count check above.
    #[test]
    fn on_client_ready_burst_registers_no_built_in_channel() {
        let welcome = build_welcome_message_args("Tester", 7);
        let methods: Vec<u16> = on_client_ready_burst_messages(b"app", b"tint", &welcome)
            .iter()
            .map(|(m, _)| *m)
            .collect();
        assert!(
            !methods.contains(&method_idx::ON_CHAT_JOINED),
            "onChatJoined in the login burst: {methods:?}"
        );
        assert_eq!(
            methods,
            vec![
                method_idx::BEING_APPEARANCE,
                method_idx::ON_ENTITY_TINT,
                method_idx::ON_PLAYER_COMMUNICATION,
            ]
        );
        let bundle = build_on_client_ready_burst_bundle(7, b"app", b"tint", &welcome);
        assert_eq!(
            bundle.num_messages(),
            methods.len(),
            "the bundle appends exactly the listed messages"
        );
    }

    /// **Burst-shape regression guard for the `resend_appearance_after_cinematic`
    /// 2-packet collapse.**
    ///
    /// The cinematic-guard spam loop in `send_cinematic` invokes
    /// `resend_appearance_after_cinematic` every 100 ms for up to 20 s
    /// (200 iterations max), each iteration emitting BeingAppearance +
    /// onEntityTint. Pre-bundle that was 2 reliable packets per iteration
    /// (400 TX-window slots over the worst-case spam window); post-bundle
    /// each iteration emits exactly 1 packet (200 slots).
    ///
    /// `handle_cancel_movie` also calls the same resend path on real
    /// client `cancelMovie` — single-shot 2 → 1.
    ///
    /// Pin two invariants:
    ///   - `num_messages == 2` — exactly BeingAppearance + onEntityTint,
    ///     in order. A regression that drops one or appends a stray method
    ///     fails here.
    ///   - `estimated_packet_count() == 1` — the realistic appearance arg
    ///     size (~120 B for an 8-component humanoid) plus the 12-byte tint
    ///     fits comfortably under `FRAGMENT_BODY_SIZE`. If a future regression
    ///     bloats the appearance args past the fragment cutoff, this assert
    ///     forces a re-audit (a fragmented appearance resend defeats the
    ///     spam-loop's per-iter slot savings).
    #[test]
    fn appearance_resend_bundle_collapses_to_single_packet() {
        const ENTITY_ID: u32 = 12345;
        let appearance = build_appearance_args(
            "MaleBody",
            &[
                "Hair".to_string(),
                "Head".to_string(),
                "Torso".to_string(),
                "Legs".to_string(),
                "Hands".to_string(),
                "Feet".to_string(),
                "Belt".to_string(),
                "Backpack".to_string(),
            ],
        );
        let tint = build_tint_args(3);

        let bundle = build_appearance_resend_bundle(ENTITY_ID, &appearance, &tint);

        assert_eq!(
            bundle.num_messages(),
            2,
            "appearance resend must be exactly BeingAppearance + onEntityTint"
        );
        assert_eq!(
            bundle.estimated_packet_count(),
            1,
            "appearance resend must collapse to a single reliable packet \
             (was 2 pre-bundle, called per-iter of the 100 ms × 200 spam loop)"
        );
    }
}
