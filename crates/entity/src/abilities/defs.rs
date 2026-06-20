//! Ability/effect definitions and the data-driven constants they reference.
//!
//! `AbilityDef` and `EffectDef` mirror the `resources.abilities` /
//! `resources.effects` rows (plus the `effect_nvps` bag); `AbilityTreeData`
//! is the per-archetype three-branch tree sent to the client. The module-
//! level constants are the ability/effect flag bits, target types, timer
//! types, result codes, and damage/stat-result codes the rest of the
//! ability system dispatches on.

// ── Ability flags (from python/Atrea/enums.py) ────────────────────────────

pub const AF_USE_WEAPON_RANGE: u32 = 4;
pub const AF_RESPONSE: u32 = 16;
pub const AF_DEACTIVATE_AUTO_CYCLE: u32 = 1024;
pub const AF_SPEED_GRENADE: u32 = 2048;
pub const AF_SPEED_DEPLOY: u32 = 4096;
pub const AF_SPEED_ATTACK: u32 = 8192;
/// Channelled abilities normally cancel when the channeller moves more
/// than `CHANNEL_INTERRUPT_DISTANCE` from their channel-start position.
/// Setting this flag exempts the ability — useful for "channels while
/// running" content (sustained beam abilities the player can walk with).
///
/// Default for every authored ability today is 0 (cancel-on-move). Flip
/// the bit per-ability as content lands that should be movement-tolerant.
/// Reserved bit 14 — not in any python reference; original game's
/// canonical name unknown, this is the Cimmeria-side name.
pub const AF_CHANNEL_ALLOWS_MOVEMENT: u32 = 16384;

// ── Target types ──────────────────────────────────────────────────────────

pub const TARGET_NONE: i32 = 0;
pub const TARGET_SELF: i32 = 1;
pub const TARGET_TARGET: i32 = 2;
pub const TARGET_GROUND: i32 = 3;

// ── Target collection methods (TCM) — per-effect dispatch shape ──────────
//
// Authored in the original game's data as string literals. We keep them as
// strings on `EffectDef` (rather than mapping to an enum) so unknown
// values from new content surface as a `tracing::warn!` instead of
// silently coercing to a default. The three values that occur in seed:
//   - TCM_Single    (2795 rows) — primary target only
//   - TCM_AERadius  (300 rows)  — every entity within `Radius` of the
//                                 source/ground point
//   - TCM_AECone    (99 rows)   — every entity inside a cone anchored at
//                                 the source, oriented toward the primary
//                                 target, with length+width tiers from
//                                 `tcm_param1` and `tcm_param2`
pub const TCM_SINGLE: &str = "TCM_Single";
pub const TCM_AE_RADIUS: &str = "TCM_AERadius";
pub const TCM_AE_CONE: &str = "TCM_AECone";

// ── Effect flags (subset of `resources.effects.flags` bitmask) ───────────
//
// Authored as a packed integer per-effect. The bits we honor today:
pub const EF_INTERRUPT_CHANCE: u32 = 16; // category: interrupt roll
pub const EF_STUN: u32 = 12; // category: stun on apply (target loses controls)
pub const EF_DONT_USE_QR: u32 = 32; // category: bypass QR (always-hit)
pub const EF_MENTAL_RESIST_ROLL: u32 = 64; // category: target rolls resist
pub const EF_SUPPRESSION: u32 = 76; // category: movement slow + accuracy debuff
pub const EF_EXTRA_DAMAGE: u32 = 512; // category: bonus damage on second pulse
pub const EF_DOT: u32 = 516; // category: damage-over-time (pulses)

// ── Timer types (sent via onTimerUpdate) ──────────────────────────────────

pub const TIMER_ABILITY_WARMUP: i8 = 1;
pub const TIMER_ABILITY_COOLDOWN: i8 = 2;
pub const TIMER_DURATION_EFFECT: i8 = 5;
pub const TIMER_CATEGORY_COOLDOWN: i8 = 8;

// ── Result codes ──────────────────────────────────────────────────────────

pub const RC_NONE: u8 = 0;
pub const RC_HIT: u8 = 1;
pub const RC_MISS: u8 = 2;
pub const RC_CRITICAL: u8 = 3;
pub const RC_DOUBLE_CRITICAL: u8 = 4;
pub const RC_GLANCING: u8 = 5;

// ── Damage types ──────────────────────────────────────────────────────────

pub const DT_UNTYPED: i8 = 0;
pub const DT_ENERGY: i8 = 1;
pub const DT_HAZMAT: i8 = 2;
pub const DT_PHYSICAL: i8 = 3;
pub const DT_PSIONIC: i8 = 4;

// ── Stat result codes ─────────────────────────────────────────────────────

pub const SRC_NONE: i8 = 0;
pub const SRC_ABSORB: i8 = 1;
pub const SRC_IMMUNE: i8 = 2;
pub const SRC_MORTAL: i8 = 3;

// ── AbilityDef ────────────────────────────────────────────────────────────

/// An ability definition loaded from the database.
///
/// Mirrors `python/common/defs/Ability.py` fields needed for validation and
/// invocation. Full damage/effect resolution will use the effect system.
#[derive(Debug, Clone)]
pub struct AbilityDef {
    pub ability_id: i32,
    pub name: String,
    pub cooldown: f32,
    pub warmup: f32,
    pub flags: u32,
    pub is_ranged: bool,
    pub min_range: i32,
    pub max_range: i32,
    pub target_type_id: i32,
    pub effect_ids: Vec<i32>,
    pub moniker_ids: Vec<i64>,
    pub required_ammo: i32,
    pub event_set_id: Option<i32>,
    pub velocity: f32,
}

/// A single effect within an ability (damage, heal, buff, etc).
///
/// Mirrors the columns on `resources.effects` plus the NVP bag from
/// `resources.effect_nvps`. The dispatch-relevant fields (`pulse_count`,
/// `pulse_duration`, `is_channeled`, `target_collection_method`,
/// `tcm_param1/2`, `flags`) were added to support pulsing, cone AoE, and
/// flag-driven script categories.
#[derive(Debug, Clone)]
pub struct EffectDef {
    pub effect_id: i32,
    pub ability_id: i32,
    pub delay: i32,
    pub effect_sequence: i32,
    pub event_set_id: Option<i32>,
    pub script_name: Option<String>,
    /// Total pulses to fire. `1` = single shot (most effects). `> 1` = DoT/HoT.
    /// `0` = pulse-until-removed (channelled holdable abilities).
    pub pulse_count: i32,
    /// Seconds between pulses. Ignored when `pulse_count == 1`.
    pub pulse_duration: f32,
    /// `true` when the source must maintain channel state (cancellable
    /// by movement, interrupt, etc.). Pulse-loop respects this; v1
    /// treats interrupt and channel-cancel as TODO.
    pub is_channeled: bool,
    /// `TCM_Single` / `TCM_AERadius` / `TCM_AECone`. See module-level
    /// `TCM_*` constants. Unknown values fall through to single-target
    /// with a warn log so new content surfaces as a missing dispatch.
    pub target_collection_method: String,
    /// Range tier — one of the string tier names the original content
    /// authoring used (Melee, Short, Medium, Long, Weapon). `Weapon`
    /// means "use the equipped weapon's range". Pulled through to
    /// cone/radius computation via `tcm_range_meters`.
    pub tcm_param1: String,
    /// Width tier for cone (Narrow, Medium, Wide); secondary range
    /// tier for radius. Same tier-string discipline as `tcm_param1`.
    pub tcm_param2: String,
    /// Packed flag bitmask. See `EF_*` constants. Drives category
    /// dispatch (Stun, Suppression, DoT, etc.).
    pub flags: u32,
    /// Name-value pairs: e.g. "HealthDamage" → "15"
    pub params: std::collections::HashMap<String, String>,
}

impl Default for EffectDef {
    /// Single-shot, single-target, no flags — the most common shape.
    /// Test code and synthetic construction sites use this with struct
    /// update syntax to set only the fields they care about, e.g.
    /// `EffectDef { effect_id: 999, ability_id: 1, ..Default::default() }`.
    fn default() -> Self {
        Self {
            effect_id: 0,
            ability_id: 0,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            pulse_count: 1,
            pulse_duration: 0.0,
            is_channeled: false,
            target_collection_method: TCM_SINGLE.to_string(),
            tcm_param1: String::new(),
            tcm_param2: String::new(),
            flags: 0,
            params: std::collections::HashMap::new(),
        }
    }
}

impl EffectDef {
    /// Get a param as i32, returning 0 if missing or unparseable.
    pub fn param_i32(&self, name: &str) -> i32 {
        self.params
            .get(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    /// Get a param as f32, returning 0.0 if missing or unparseable.
    pub fn param_f32(&self, name: &str) -> f32 {
        self.params
            .get(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0)
    }

    /// `true` when the effect should pulse over time (DoT/HoT/channelled).
    /// Single-shot effects (`pulse_count == 1`) return `false`.
    pub fn is_pulsing(&self) -> bool {
        self.pulse_count == 0 || self.pulse_count > 1
    }

    /// Total duration of a pulsing effect in seconds. `0.0` for
    /// single-shot or channelled-until-released (`pulse_count == 0`).
    pub fn total_duration(&self) -> f32 {
        if self.pulse_count <= 1 {
            0.0
        } else {
            self.pulse_count as f32 * self.pulse_duration.max(0.0)
        }
    }

    /// Translate a TCM range/width tier name to meters. The fan-server's
    /// tier names come from the original 2009 content authoring — we map
    /// them to concrete distances tuned against the radius defaults
    /// already used elsewhere in combat (e.g. `DEFAULT_GROUND_TARGET_RADIUS = 5.0`).
    ///
    /// Unknown tiers fall back to `Medium = 8.0m` with a warn so new
    /// content surfaces as a tier we need to calibrate.
    pub fn tcm_range_meters(tier: &str) -> f32 {
        match tier {
            "Melee" => 2.5,
            "Short" => 5.0,
            "Medium" => 8.0,
            "Long" => 14.0,
            "Weapon" => 20.0, // matches the canonical weapon-range cap
            other => {
                tracing::warn!(
                    target: "abilities",
                    event = "tcm_unknown_range_tier",
                    tier = other,
                    "Unknown TCM range tier; defaulting to Medium=8.0m"
                );
                8.0
            }
        }
    }

    /// Translate a TCM width tier to a cone half-angle in radians.
    /// Narrow / Medium / Wide are the only width tiers in seed.
    pub fn tcm_half_angle_radians(tier: &str) -> f32 {
        // Half-angles chosen so:
        //   Narrow = 22.5° half-angle (45° full cone) — shotgun spread
        //   Medium = 45°               (90° full)     — grenade arc
        //   Wide   = 67.5°             (135° full)    — flamethrower-class
        match tier {
            "Narrow" => std::f32::consts::FRAC_PI_8,     // 22.5° = π/8
            "Medium" => std::f32::consts::FRAC_PI_4,     // 45° = π/4
            "Wide" => 3.0 * std::f32::consts::FRAC_PI_8, // 67.5° = 3π/8
            other => {
                tracing::warn!(
                    target: "abilities",
                    event = "tcm_unknown_width_tier",
                    tier = other,
                    "Unknown TCM width tier; defaulting to Medium=45°"
                );
                std::f32::consts::FRAC_PI_4
            }
        }
    }
}

// ── AbilityTreeData ───────────────────────────────────────────────────────

/// Archetype ability tree: 3 branches of ability IDs.
///
/// Sent to the client via `onAbilityTreeInfo(ARRAY<ARRAY<INT32>>)`.
/// Source: `resources.archetype_ability_tree` table, grouped by `tree_index` (0-2).
#[derive(Debug, Clone, Default)]
pub struct AbilityTreeData {
    pub trees: [Vec<i32>; 3],
}

impl AbilityTreeData {
    /// Serialize for `onAbilityTreeInfo(ARRAY<ARRAY<INT32>>)`.
    ///
    /// The .def declares a single nested-array `<Arg>` —
    /// `ARRAY <of> ARRAY <of> INT32 </of> </of>` — so BigWorld's wire
    /// encoding is `[outer_count:u32][inner_count:u32 ids...] × N`. Both
    /// counts are required; the outer count is what tells the client how
    /// many inner arrays to expect. Without it the client reads the first
    /// inner count as the outer count, tries to parse that many inner
    /// arrays, the parser fails, and the handler aborts silently — the
    /// abilities window stays empty.
    ///
    /// SGW's authored content always sends exactly 3 trees, but the
    /// outer count is on the wire either way. The Python
    /// `self.client.onAbilityTreeInfo([a, b, c])` call passes a 3-element
    /// list; the BigWorld serializer on the Python client side still
    /// prepends the outer length prefix.
    ///
    /// Compare with the working `setupStargateInfo` shape: that .def has
    /// THREE separate `<Arg>` entries, each `ARRAY<of>INT32</of>`. Three
    /// top-level args, each its own length-prefixed array, no outer
    /// wrapper. Coincidentally the same byte layout up to the outer
    /// prefix — which is what masked this bug.
    pub fn serialize(&self) -> Vec<u8> {
        let total_ids: usize = self.trees.iter().map(|t| t.len()).sum();
        let mut buf = Vec::with_capacity(4 + 4 * self.trees.len() + total_ids * 4);
        buf.extend_from_slice(&(self.trees.len() as u32).to_le_bytes());
        for tree in &self.trees {
            buf.extend_from_slice(&(tree.len() as u32).to_le_bytes());
            for &id in tree {
                buf.extend_from_slice(&id.to_le_bytes());
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_tree_empty() {
        let tree = AbilityTreeData::default();
        let data = tree.serialize();
        // outer count(4) + 3 inner counts(4) = 16
        assert_eq!(data.len(), 16);
        // Outer count = 3 (number of inner arrays)
        let outer = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(outer, 3);
        // Each of the 3 inner counts = 0
        for i in 0..3 {
            let off = 4 + i * 4;
            let count =
                u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
            assert_eq!(count, 0, "inner array {i} count must be 0");
        }
    }

    #[test]
    fn ability_tree_with_data() {
        let tree = AbilityTreeData {
            trees: [vec![592, 1005, 642], vec![597, 646, 641], vec![]],
        };
        let data = tree.serialize();
        // outer(4) + tree0:count(4)+3*4=12=16 + tree1:16 + tree2:count(4)=4 = 40
        assert_eq!(data.len(), 40);

        // Outer count = 3 inner arrays
        let outer = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(outer, 3);
        // First inner count = 3 ids
        let c0 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(c0, 3);
        // First ability id in tree 0 = 592
        let a0 = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        assert_eq!(a0, 592);
    }

    /// **Regression guard: the outer ARRAY count prefix is on the wire.**
    ///
    /// Bug shape: the wire encoding for `ARRAY<ARRAY<INT32>>` is
    /// `[outer:u32][inner:u32 ids...] × N`. Skipping the outer prefix
    /// is what the original Rust impl did — based on the (wrong) theory
    /// that "the Python code always sends exactly 3 arrays so the count
    /// is implicit." It isn't; BigWorld's serializer still emits it.
    /// The client decoder reads the first u32 as the outer count, gets
    /// some inner-tree size like 29, tries to parse 29 inner arrays,
    /// parser fails, handler aborts silently → abilities window stays
    /// empty for every player.
    ///
    /// Reverting [`AbilityTreeData::serialize`] to skip the outer count
    /// must fail this assertion.
    #[test]
    fn ability_tree_outer_count_prefix_is_first_four_bytes() {
        let tree = AbilityTreeData {
            trees: [
                // Real shape: Soldier tree 0 has 29 abilities; if we read
                // the first u32 as the outer count we'd get 29, not 3 —
                // the exact decode error the client makes on unprefixed
                // bytes.
                (0..29).collect(),
                vec![],
                vec![],
            ],
        };
        let data = tree.serialize();
        let outer = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(
            outer, 3,
            "first 4 bytes MUST be the outer ARRAY count (=3, the number of \
             trees). Reverting serialize() to skip the outer prefix would \
             surface this as outer=29 here — the exact byte the client \
             misreads as outer-count on the wire."
        );
    }

    // ── EffectDef helpers ────────────────────────────────────────────────

    #[test]
    fn effect_def_default_is_single_target_single_pulse() {
        let e = EffectDef::default();
        assert_eq!(e.pulse_count, 1, "default pulse_count=1");
        assert_eq!(e.target_collection_method, TCM_SINGLE);
        assert!(!e.is_pulsing(), "single-pulse is not pulsing");
    }

    #[test]
    fn effect_def_is_pulsing_matches_pulse_count() {
        // Channelled (pulse_count=0): is_pulsing → true
        let channelled = EffectDef {
            pulse_count: 0,
            ..Default::default()
        };
        assert!(channelled.is_pulsing());
        // Single shot (pulse_count=1): is_pulsing → false
        let single = EffectDef {
            pulse_count: 1,
            ..Default::default()
        };
        assert!(!single.is_pulsing());
        // DoT (pulse_count>1): is_pulsing → true
        let dot = EffectDef {
            pulse_count: 5,
            ..Default::default()
        };
        assert!(dot.is_pulsing());
    }

    #[test]
    fn effect_def_total_duration_skips_single_shot_and_channelled() {
        let single = EffectDef {
            pulse_count: 1,
            pulse_duration: 5.0,
            ..Default::default()
        };
        assert_eq!(single.total_duration(), 0.0, "single-shot has 0 duration");

        let channelled = EffectDef {
            pulse_count: 0,
            pulse_duration: 5.0,
            ..Default::default()
        };
        assert_eq!(
            channelled.total_duration(),
            0.0,
            "channelled has 0 duration (unbounded)"
        );

        let dot = EffectDef {
            pulse_count: 5,
            pulse_duration: 1.5,
            ..Default::default()
        };
        assert_eq!(dot.total_duration(), 7.5, "DoT: 5 * 1.5 = 7.5");
    }

    #[test]
    fn tcm_range_meters_known_tiers_resolve_to_expected_distances() {
        assert_eq!(EffectDef::tcm_range_meters("Melee"), 2.5);
        assert_eq!(EffectDef::tcm_range_meters("Short"), 5.0);
        assert_eq!(EffectDef::tcm_range_meters("Medium"), 8.0);
        assert_eq!(EffectDef::tcm_range_meters("Long"), 14.0);
        assert_eq!(EffectDef::tcm_range_meters("Weapon"), 20.0);
    }

    #[test]
    fn tcm_range_meters_unknown_tier_falls_back_to_medium() {
        // Unknown tier defaults to Medium=8.0 with a warn log.
        assert_eq!(EffectDef::tcm_range_meters("NoSuchTier"), 8.0);
        assert_eq!(EffectDef::tcm_range_meters(""), 8.0);
    }

    #[test]
    fn tcm_half_angle_radians_known_tiers_resolve() {
        use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};
        assert!((EffectDef::tcm_half_angle_radians("Narrow") - FRAC_PI_8).abs() < 1e-6);
        assert!((EffectDef::tcm_half_angle_radians("Medium") - FRAC_PI_4).abs() < 1e-6);
        assert!((EffectDef::tcm_half_angle_radians("Wide") - 3.0 * FRAC_PI_8).abs() < 1e-6);
    }

    #[test]
    fn tcm_half_angle_radians_unknown_tier_falls_back_to_medium() {
        use std::f32::consts::FRAC_PI_4;
        assert!((EffectDef::tcm_half_angle_radians("NoSuchTier") - FRAC_PI_4).abs() < 1e-6);
    }

    /// `param_i32` / `param_f32` read the NVP bag and fall back to a
    /// zero default when the key is missing or the value won't parse.
    /// Net-new coverage for the two accessors (previously exercised
    /// only indirectly): present-and-parseable, missing, and present-
    /// but-garbage must all behave as documented.
    #[test]
    fn param_accessors_parse_present_and_default_on_missing_or_garbage() {
        let e = EffectDef {
            params: [
                ("HealthDamage".to_string(), "15".to_string()),
                ("Radius".to_string(), "5.5".to_string()),
                ("Garbage".to_string(), "not-a-number".to_string()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        // Present + parseable.
        assert_eq!(e.param_i32("HealthDamage"), 15);
        assert_eq!(e.param_f32("Radius"), 5.5);

        // Missing key → zero default.
        assert_eq!(e.param_i32("NoSuchKey"), 0);
        assert_eq!(e.param_f32("NoSuchKey"), 0.0);

        // Present but unparseable → zero default (not a panic).
        assert_eq!(e.param_i32("Garbage"), 0);
        assert_eq!(e.param_f32("Garbage"), 0.0);

        // A float string is not a valid i32 → zero default.
        assert_eq!(e.param_i32("Radius"), 0);
    }
}
