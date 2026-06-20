//! Per-entity sub-slot threshold (`idBase`) constants and the formula that
//! derives them from an entity type's exposed-client-method count.
//!
//! The idbase decides whether a server→client method index direct-encodes
//! (single wire byte `index | 0x80`) or extended-encodes (marker `0xBD` +
//! sub-index byte). See [`ChannelBundle::append_entity_method`] for the
//! encoder that consumes these.
//!
//! [`ChannelBundle::append_entity_method`]: super::ChannelBundle::append_entity_method

/// Per-entity sub-slot threshold (`idBase`) for SGWPlayer.
///
/// SGWPlayer exposes 157 client methods, which plugged into the formula
/// [`idbase_from_exposed_method_count`] gives `idBase = 61`. Every
/// server→player method call uses this value: indices `0..61` direct-
/// encode (msg_id = `index | 0x80`), indices `61..157` extended-encode
/// (marker `0xBD`, then sub_index byte = `index - 61`).
///
/// **The threshold is per-entity-type, NOT a global constant.** Different
/// entity types have different exposed-method counts and therefore
/// different idbases. Pass the correct idbase for the target entity to
/// every encoder call. For entities with `nExposedCount <= 62`, idbase
/// is `62` — see [`idbase_from_exposed_method_count`] for the formula
/// (`EntityDescription_AssignClientMethodIds @
/// ghidra://SGW.exe@0x01590df0`).
///
/// Spec: [docs/drafts/spec/entity-property-sync.md §1.4][1] §1.17 S1,
/// §1.18 R2.
///
/// [1]: ../../../../docs/drafts/spec/entity-property-sync.md
pub const IDBASE_SGW_PLAYER: u8 = 61;

/// Per-entity sub-slot threshold for SGW non-player entities (SGWMob,
/// SGWPet, SGWBeing-derived NPCs, etc.). Every such entity type in the
/// current SGW schema exposes ≤62 client methods, so the formula
/// [`idbase_from_exposed_method_count`] returns `62` (the maximum). Use
/// this for any method call targeting a non-player entity until a
/// per-class lookup is wired up.
pub const IDBASE_NPC_DEFAULT: u8 = 62;

/// Extended-encoding marker byte. Cannot be used as a direct msg_id for
/// SGWPlayer (whose `idbase = 61`) because the direct range stops at
/// `0xBC = 60 | 0x80`. For entities with a higher idbase the byte `0xBD`
/// can mean "direct method 61" rather than "extended marker" — the
/// client decoder distinguishes by knowing the receiving entity's idbase.
pub const EXTENDED_ENCODING_MARKER: u8 = 0xBD;

/// Compute the per-entity sub-slot threshold (`idBase`) from the entity
/// type's exposed-client-method count, per `EntityDescription_AssignClient
/// MethodIds @ ghidra://SGW.exe@0x01590df0`:
///
/// ```text
///   idBase = 0x3E - (nExposedCount + 0xC0) / 0xFF
/// ```
///
/// Method indices `0..idbase` use single-byte direct encoding (wire byte
/// `(index | 0x80)`). Method indices `idbase..` use the two-byte extended
/// encoding (marker `0xBD` + sub_index byte = `index - idbase`).
///
/// - `nExposedCount = 157` (SGWPlayer) → `idBase = 61`. The constant
///   [`IDBASE_SGW_PLAYER`] pre-computes this for the common case.
/// - `nExposedCount <= 62` → `idBase = 62`. The whole namespace fits in
///   single-byte direct encoding; the wildcard `0xBD` byte is then a
///   valid direct msg_id for method 61 on that entity rather than the
///   extended marker.
/// - `nExposedCount = 318` → `idBase = 60`. Method 60 starts extended-
///   encoding territory.
///
/// # Input range
///
/// `n_exposed_count` is the count of `<Exposed/>` client methods on a
/// flattened entity type — realistically 0–500. The formula's domain is
/// `0..=15_807`: above that, `(n + 0xC0) / 0xFF > 62` and the subtraction
/// would underflow. Out-of-range inputs **saturate to 0** rather than
/// wrap. Such a count would imply ~12_500 method indices in the extended
/// range alone, which exceeds the sub-index byte (`u8`, max 255) — so an
/// entity that large cannot actually be encoded by this wire format and
/// the saturated `0` is a defensive sentinel, not a usable idbase.
///
/// Spec: `docs/drafts/spec/entity-property-sync.md` §1.4, §1.17 S1,
/// §1.18 R2.
pub const fn idbase_from_exposed_method_count(n_exposed_count: usize) -> u8 {
    let i_var2 = (n_exposed_count + 0xC0) / 0xFF;
    if i_var2 > 0x3E {
        0
    } else {
        (0x3E - i_var2) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── idbase formula tests ────────────────────────────────────────────

    /// SGWPlayer exposes 157 client methods; formula must reduce to the
    /// observed wire idbase of 61. Audit Appendix C.6 wire-captured 35
    /// `0xBD` sub-slot bytes for SGWPlayer-targeted methods — those only
    /// align if the threshold is 61.
    #[test]
    fn idbase_formula_sgw_player_157_methods() {
        assert_eq!(idbase_from_exposed_method_count(157), 61);
        // Constant must match the formula for the documented input — drift
        // would mean the SGWPlayer-targeted wire encoder uses a different
        // boundary than the formula advertises.
        assert_eq!(IDBASE_SGW_PLAYER, idbase_from_exposed_method_count(157));
    }

    /// Entities with ≤62 exposed methods all land on `idbase = 62` (the
    /// maximum). Covers the integer-division corner: `iVar2 = (n + 0xC0) /
    /// 0xFF` is 0 for `n ≤ 62` because `n + 192 ≤ 254 < 255`.
    #[test]
    fn idbase_formula_small_method_count_is_62() {
        for n in 0..=62 {
            assert_eq!(
                idbase_from_exposed_method_count(n),
                62,
                "n={n} expected idbase=62, formula returned otherwise"
            );
        }
        // Sanity-check the documented `IDBASE_NPC_DEFAULT` matches.
        assert_eq!(IDBASE_NPC_DEFAULT, 62);
    }

    /// The boundary lives between 62 and 63 exposed methods: `n = 63`
    /// already drops idbase to 61 because `(63 + 192) / 255 = 1`.
    #[test]
    fn idbase_formula_drops_to_61_at_63_methods() {
        assert_eq!(idbase_from_exposed_method_count(62), 62);
        assert_eq!(idbase_from_exposed_method_count(63), 61);
    }

    /// Step at every multiple of 255 exposed methods + 63. Documents the
    /// staircase shape of the formula — each step subtracts 1 from idbase.
    #[test]
    fn idbase_formula_staircase_at_each_255_step() {
        // (n+192)/255 transitions: at n=63 → 1, at n=318 → 2, at n=573 → 3.
        assert_eq!(idbase_from_exposed_method_count(317), 61); // (317+192)/255 = 509/255 = 1
        assert_eq!(idbase_from_exposed_method_count(318), 60); // (318+192)/255 = 510/255 = 2
        assert_eq!(idbase_from_exposed_method_count(572), 60); // (572+192)/255 = 764/255 = 2
        assert_eq!(idbase_from_exposed_method_count(573), 59); // (573+192)/255 = 765/255 = 3
    }

    /// Out-of-domain inputs saturate to `0` rather than wrapping. The
    /// formula's domain is `0..=15_807`; above that, `(n+0xC0)/0xFF`
    /// exceeds `0x3E` and the subtraction would underflow. The guard
    /// returns `0` as a defensive sentinel — `idbase = 0` means every
    /// method index uses extended encoding, which the sub_index byte
    /// can only address up to method 255, so an entity that hits this
    /// branch is unencodable anyway. The point of the test is to pin
    /// the no-panic / no-wrap behaviour on out-of-range input.
    #[test]
    fn idbase_formula_saturates_to_zero_on_overflow() {
        assert_eq!(idbase_from_exposed_method_count(15_807), 0);
        assert_eq!(idbase_from_exposed_method_count(16_000), 0);
        assert_eq!(idbase_from_exposed_method_count(usize::MAX / 2), 0);
        assert_eq!(idbase_from_exposed_method_count(usize::MAX - 0xC0), 0);
    }
}
