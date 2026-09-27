//! The crafting enumerations from `entities/defs/enumerations.xml`.
//!
//! Every value here is pinned to the defs by
//! `tests::constants::every_constant_matches_enumerations_xml`, so a typo or a
//! changed def fails a test instead of a client window.

/// `ECraftTypeFlags` (`INT8`): the four crafting verbs as bits. The station
/// gate (CR-05) grants a mask of these, carried as `CraftRequest::allowed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CraftType {
    /// `CRAFT_TYPE_Craft`.
    Craft = 1,
    /// `CRAFT_TYPE_Research`.
    Research = 2,
    /// `CRAFT_TYPE_ReverseEngineering`.
    ReverseEngineering = 4,
    /// `CRAFT_TYPE_Alloying`.
    Alloying = 8,
}

impl CraftType {
    /// Every verb, in `CraftingOptions` section order (crafting, research,
    /// reverseEngineering, alloying; `entities/defs/alias.xml`).
    pub const ALL: [CraftType; 4] = [
        CraftType::Craft,
        CraftType::Research,
        CraftType::ReverseEngineering,
        CraftType::Alloying,
    ];

    /// The verb's bit in an `ECraftTypeFlags` mask.
    pub const fn bit(self) -> u8 {
        self as u8
    }

    /// Whether `mask` grants this verb.
    pub const fn allowed_by(self, mask: u8) -> bool {
        mask & self.bit() != 0
    }

    /// The `ENTITYFLAG_Craft_*` bit that makes an entity a station for this
    /// verb.
    pub const fn entity_flag(self) -> u32 {
        match self {
            CraftType::Craft => ENTITYFLAG_CRAFT_CRAFT,
            CraftType::Research => ENTITYFLAG_CRAFT_RESEARCH,
            CraftType::ReverseEngineering => ENTITYFLAG_CRAFT_REV_ENG,
            CraftType::Alloying => ENTITYFLAG_CRAFT_ALLOYING,
        }
    }
}

impl TryFrom<u8> for CraftType {
    type Error = u8;

    /// A single verb bit. A mask of several bits, or none, is not a verb.
    fn try_from(value: u8) -> Result<Self, u8> {
        match value {
            1 => Ok(CraftType::Craft),
            2 => Ok(CraftType::Research),
            4 => Ok(CraftType::ReverseEngineering),
            8 => Ok(CraftType::Alloying),
            other => Err(other),
        }
    }
}

/// `EItemFlag` (`UINT32` bitfield), the `resources.items.flags` column.
///
/// The seed's flags are only partly trustworthy (audit C-24):
/// `ELEMENTARY_COMPONENT` is set on every item and `CRAFT_CRAFT` and
/// `NOT_RESEARCHABLE` on none, so no rule may rest on those three.
/// `CRAFT_RESEARCH`, `CRAFT_REV_ENG` and `KICKER` do mark the right items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ItemFlags(pub u32);

impl ItemFlags {
    pub const MINIGAME_INSTRUMENT: u32 = 1;
    pub const MINIGAME_CONSUMABLE: u32 = 2;
    pub const BIND_ON_ACQUIRE: u32 = 4;
    pub const BIND_ON_EQUIP: u32 = 8;
    /// Set on no seeded item (C-24).
    pub const NOT_RESEARCHABLE: u32 = 16;
    /// A research kicker (items 5668-5671).
    pub const KICKER: u32 = 32;
    /// Set on no seeded item (C-24).
    pub const CRAFT_CRAFT: u32 = 64;
    /// Researchable gear (D-CR18).
    pub const CRAFT_RESEARCH: u32 = 128;
    /// Reverse-engineerable gear (D-CR18).
    pub const CRAFT_REV_ENG: u32 = 256;
    pub const CRAFT_ALLOYING: u32 = 512;
    pub const CAN_BE_SOLD: u32 = 1024;
    pub const CAN_BE_DELETED: u32 = 2048;
    pub const UNIQUE: u32 = 4096;
    pub const MUST_EQUIP_TO_USE: u32 = 8192;
    pub const DESTROY_ON_CLEAR: u32 = 16384;
    /// Set on every seeded item, so it distinguishes nothing (C-24).
    pub const ELEMENTARY_COMPONENT: u32 = 32768;

    /// Whether every bit of `flag` is set.
    pub const fn contains(self, flag: u32) -> bool {
        self.0 & flag == flag
    }

    /// Researchable (`ITEM_FLAG_Craft_Research`).
    pub const fn is_researchable(self) -> bool {
        self.contains(Self::CRAFT_RESEARCH)
    }

    /// Reverse-engineerable (`ITEM_FLAG_Craft_RevEng`).
    pub const fn is_reverse_engineerable(self) -> bool {
        self.contains(Self::CRAFT_REV_ENG)
    }

    /// A research kicker (`ITEM_FLAG_Kicker`).
    pub const fn is_kicker(self) -> bool {
        self.contains(Self::KICKER)
    }
}

/// `EItemQuality` (`INT32`). The database stores the token name
/// (`resources."EItemQuality"`); the client and the alloy counts use the
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum ItemQuality {
    Poor = 1000,
    Normal = 2000,
    Good = 3000,
    Great = 4000,
    Fantastic = 5000,
}

impl ItemQuality {
    /// The `EItemQuality` value the client uses.
    pub const fn value(self) -> i32 {
        self as i32
    }

    /// Parse the database label (`ITEM_QUALITY_Good`, ...).
    pub fn from_db_label(label: &str) -> Option<Self> {
        match label {
            "ITEM_QUALITY_Poor" => Some(ItemQuality::Poor),
            "ITEM_QUALITY_Normal" => Some(ItemQuality::Normal),
            "ITEM_QUALITY_Good" => Some(ItemQuality::Good),
            "ITEM_QUALITY_Great" => Some(ItemQuality::Great),
            "ITEM_QUALITY_Fantastic" => Some(ItemQuality::Fantastic),
            _ => None,
        }
    }
}

// `EEntityFlags`: an entity carrying one of these is a crafting station for
// that verb (CR-05). No seeded template sets any of them yet (C-25).

/// `ENTITYFLAG_Craft_Craft`.
pub const ENTITYFLAG_CRAFT_CRAFT: u32 = 2048;
/// `ENTITYFLAG_Craft_Research`.
pub const ENTITYFLAG_CRAFT_RESEARCH: u32 = 4096;
/// `ENTITYFLAG_Craft_RevEng`.
pub const ENTITYFLAG_CRAFT_REV_ENG: u32 = 8192;
/// `ENTITYFLAG_Craft_Alloying`.
pub const ENTITYFLAG_CRAFT_ALLOYING: u32 = 16384;

/// `ETimerUpdateType::CraftInductionTimer`: the only `onTimerUpdate` type the
/// client draws the crafting induction bar from (audit C-32).
pub const TIMER_CRAFT_INDUCTION: u8 = 16;

// `EConditionHandlerFeedback` values for crafting. `HasCraft` (224) and
// `DoesNotHaveCraft` (225) are deliberately absent: the defs give them the
// same values as `AbilityPulseCheckFailed` / `AbilityPulseCheckPassed`, so
// the client cannot tell them apart. Use a text line for those cases.

/// `CONDITION_FEEDBACK_EnoughAppliedSciencePoints`.
pub const CONDITION_FEEDBACK_ENOUGH_APPLIED_SCIENCE_POINTS: u16 = 213;
/// `CONDITION_FEEDBACK_NotEnoughAppliedSciencePoints`.
pub const CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS: u16 = 214;
/// `CONDITION_FEEDBACK_ExpertiseValueNotEqual`.
pub const CONDITION_FEEDBACK_EXPERTISE_VALUE_NOT_EQUAL: u16 = 228;
/// `CONDITION_FEEDBACK_ExpertiseValueEqual`.
pub const CONDITION_FEEDBACK_EXPERTISE_VALUE_EQUAL: u16 = 229;
/// `CONDITION_FEEDBACK_ExpertiseValueGreaterThan`.
pub const CONDITION_FEEDBACK_EXPERTISE_VALUE_GREATER_THAN: u16 = 230;
/// `CONDITION_FEEDBACK_ExpertiseValueGreaterThanOrEqual`.
pub const CONDITION_FEEDBACK_EXPERTISE_VALUE_GREATER_THAN_OR_EQUAL: u16 = 231;
/// `CONDITION_FEEDBACK_ExpertiseValueLessThanOrEqual`.
pub const CONDITION_FEEDBACK_EXPERTISE_VALUE_LESS_THAN_OR_EQUAL: u16 = 232;
/// `CONDITION_FEEDBACK_ExpertiseValueLessThan`.
pub const CONDITION_FEEDBACK_EXPERTISE_VALUE_LESS_THAN: u16 = 233;
/// `CONDITION_FEEDBACK_ExpertiseNoCraft`.
pub const CONDITION_FEEDBACK_EXPERTISE_NO_CRAFT: u16 = 234;
