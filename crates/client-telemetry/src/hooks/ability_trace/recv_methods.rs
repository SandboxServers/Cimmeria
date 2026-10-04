//! The inbound ability methods `client.ability.recv` decodes, with their
//! `.def` argument lists.
//!
//! Indices are the flat client method indices of
//! `docs/protocol/client-method-dispatch-table.md`. Indices 0 to 26 come
//! from the classes every being shares (`SGWSpawnableEntity`, `SGWBeing`,
//! `SGWCombatant`), so they mean the same method on a player, a mob and a
//! pet; 27 and up are `SGWPlayer`'s own and are decoded only for a player
//! receiver (an `SGWMob`'s index 27 is a different method). The argument
//! lists are the `.def` `<Arg>`s in order, with the `alias.xml` layouts
//! of the dictionaries they use; `tests::table_matches_entity_defs` reads
//! both files and fails on any drift, and
//! `tests::indices_match_the_dispatch_table` checks every index.
//!
//! `Ability_Interrupt` is not a method: it is an `onSequence` whose
//! sequence the server picked for Kismet event 1002. The wire carries only
//! the sequence id, so the recv row cannot name it; the client learns the
//! event id from the cooked sequence data, and the `sequence_played` /
//! `client.sequence.dropped` rows carry it as `event_id`.

use super::wire_decode::{Arg, Field, WireType};

/// Which receivers an index means this method for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Indices 0-26: the same method on every being class.
    Shared,
    /// `SGWPlayer`'s own methods (27 and up): player receivers only.
    Player,
}

/// One decoded method.
#[derive(Debug)]
pub(crate) struct RecvMethod {
    /// Flat client method index.
    pub index: u16,
    /// Method name, as in the `.def`.
    pub name: &'static str,
    /// Which receivers it applies to.
    pub scope: Scope,
    /// The `.def` file (under `entities/defs/`) that declares it; read by
    /// the conformance test.
    #[cfg_attr(not(test), allow(dead_code))]
    pub def_file: &'static str,
    /// Its arguments, in wire order.
    pub args: &'static [Arg],
}

const fn arg(field: &'static str, def_name: &'static str, ty: WireType) -> Arg {
    Arg {
        field,
        def_name,
        ty,
    }
}

const fn field(name: &'static str, ty: WireType) -> Field {
    Field { name, ty }
}

/// `alias.xml` `NameValuePair`: `{ WSTRING name, WSTRING value }`.
const NAME_VALUE_PAIR: WireType = WireType::Alias(
    "NameValuePair",
    &WireType::Dict(&[
        field("name", WireType::WString),
        field("value", WireType::WString),
    ]),
);

/// `alias.xml` `ClientEffectResult`:
/// `{ INT8 StatID, INT32 Delta, INT8 DamageCode, INT8 StatResultCode }`.
const CLIENT_EFFECT_RESULT: WireType = WireType::Alias(
    "ClientEffectResult",
    &WireType::Dict(&[
        field("StatID", WireType::I8),
        field("Delta", WireType::I32),
        field("DamageCode", WireType::I8),
        field("StatResultCode", WireType::I8),
    ]),
);

/// `alias.xml` `StatUpdate`: `{ INT32 StatId, Min, Current, Max }`.
const STAT_UPDATE: WireType = WireType::Alias(
    "StatUpdate",
    &WireType::Dict(&[
        field("StatId", WireType::I32),
        field("Min", WireType::I32),
        field("Current", WireType::I32),
        field("Max", WireType::I32),
    ]),
);

const STAT_UPDATE_LIST: WireType =
    WireType::Alias("StatUpdateList", &WireType::Array(&STAT_UPDATE));

/// `onSequence` (1).
pub(crate) const ON_SEQUENCE: u16 = 1;
/// `onTimerUpdate` (12).
pub(crate) const ON_TIMER_UPDATE: u16 = 12;
/// `onEffectResults` (14).
pub(crate) const ON_EFFECT_RESULTS: u16 = 14;
/// `onPlayerCommunication` (28).
pub(crate) const ON_PLAYER_COMMUNICATION: u16 = 28;

/// The `EChannel` the server's one-player system lines use (`CHAN_FEEDBACK`
/// in `crates/wire/src/cell/chat.rs`). `onPlayerCommunication` is decoded
/// only on this channel: it is where the native combat-debug lines (AB-N1)
/// and every refusal arrive, and other channels are players' chat.
pub(crate) const CHAN_FEEDBACK: u8 = 9;

/// Every method `client.ability.recv` decodes.
pub(crate) const METHODS: &[RecvMethod] = &[
    RecvMethod {
        index: ON_SEQUENCE,
        name: "onSequence",
        scope: Scope::Shared,
        def_file: "SGWSpawnableEntity.def",
        args: &[
            arg("sequence_id", "KismetEventSetSeqID", WireType::I32),
            arg("source_id", "SourceID", WireType::I32),
            arg("target_id", "TargetID", WireType::I32),
            arg("primary_target", "PrimaryTarget", WireType::I8),
            arg("impact_time", "ImpactTime", WireType::F32),
            arg("nvps", "NameValuePairs", WireType::Array(&NAME_VALUE_PAIR)),
            arg("view_type", "ViewType", WireType::I8),
            arg("instance_id", "InstanceId", WireType::I32),
        ],
    },
    RecvMethod {
        index: ON_TIMER_UPDATE,
        name: "onTimerUpdate",
        scope: Scope::Shared,
        def_file: "interfaces/SGWBeing.def",
        args: &[
            arg("timer_id", "ID", WireType::I32),
            arg("timer_type", "Type", WireType::I8),
            arg("source_id", "SourceID", WireType::I32),
            arg("secondary_id", "SecondaryId", WireType::I32),
            arg("total_time", "TotalTime", WireType::F32),
            arg("complete_time", "BigWorldTimeComplete", WireType::F32),
        ],
    },
    RecvMethod {
        index: ON_EFFECT_RESULTS,
        name: "onEffectResults",
        scope: Scope::Shared,
        def_file: "interfaces/SGWBeing.def",
        args: &[
            arg("source_id", "SourceID", WireType::I32),
            arg("ability_id", "AbilityID", WireType::I32),
            arg("effect_id", "EffectID", WireType::I32),
            arg("target_id", "TargetID", WireType::I32),
            arg("result_code", "ResultCode", WireType::U8),
            arg(
                "results",
                "ClientEffectResultList",
                WireType::Alias(
                    "ClientEffectResultList",
                    &WireType::Array(&CLIENT_EFFECT_RESULT),
                ),
            ),
        ],
    },
    RecvMethod {
        index: 19,
        name: "onStateFieldUpdate",
        scope: Scope::Shared,
        def_file: "interfaces/SGWBeing.def",
        args: &[arg("state_field", "bStateField", WireType::I32)],
    },
    RecvMethod {
        index: 20,
        name: "onStatUpdate",
        scope: Scope::Shared,
        def_file: "interfaces/SGWCombatant.def",
        args: &[arg("stats", "Stats", STAT_UPDATE_LIST)],
    },
    RecvMethod {
        index: 21,
        name: "onStatBaseUpdate",
        scope: Scope::Shared,
        def_file: "interfaces/SGWCombatant.def",
        args: &[arg("stats", "Stats", STAT_UPDATE_LIST)],
    },
    RecvMethod {
        index: ON_PLAYER_COMMUNICATION,
        name: "onPlayerCommunication",
        scope: Scope::Player,
        def_file: "interfaces/Communicator.def",
        args: &[
            arg("speaker", "Speaker", WireType::WString),
            arg("speaker_flags", "SpeakerFlags", WireType::U8),
            arg("channel", "Channel", WireType::U8),
            arg("text", "Text", WireType::WString),
        ],
    },
    RecvMethod {
        index: 101,
        name: "onKnownAbilitiesUpdate",
        scope: Scope::Player,
        def_file: "SGWPlayer.def",
        args: &[arg(
            "ability_ids",
            "AbilityData",
            WireType::Array(&WireType::I32),
        )],
    },
    RecvMethod {
        index: 121,
        name: "onErrorCode",
        scope: Scope::Player,
        def_file: "SGWPlayer.def",
        args: &[
            arg("system_id", "SystemID", WireType::U8),
            arg("instance_id", "InstanceID", WireType::I32),
            arg("error_code", "ErrorCodeID", WireType::U16),
        ],
    },
    RecvMethod {
        index: 141,
        name: "onAbilityTreeInfo",
        scope: Scope::Player,
        def_file: "SGWPlayer.def",
        args: &[arg(
            "ability_lists",
            "AbilityLists",
            WireType::Array(&WireType::Array(&WireType::I32)),
        )],
    },
];

/// The table row for flat index `index`.
pub(crate) fn by_index(index: u16) -> Option<&'static RecvMethod> {
    METHODS.iter().find(|m| m.index == index)
}

/// The first extended message id for a class with more than 62 exposed
/// client methods: `FUN_01590bb0` computes `0x3e - (N + 0xc0) / 0xff`,
/// which is 61 for `SGWPlayer` (157) and `SGWGmPlayer` (163).
pub(crate) const PLAYER_FIRST_EXTENDED_ID: u32 = 61;

/// Whether `msg_id` (the `onEntityMethod` argument) can be one of the
/// methods above, before any memory is read. Every direct index here is
/// below 61; the player's three extended ones share id 61.
pub(crate) fn may_be_wanted(msg_id: u32) -> bool {
    let id = wire_id(msg_id);
    id == PLAYER_FIRST_EXTENDED_ID || METHODS.iter().any(|m| u32::from(m.index) == id)
}

/// The message id as the method-index decoder sees it. `onEntityMethod`
/// receives the low six bits already (the property bit `0x40` and the
/// entity-message bit `0x80` are the caller's); a raw byte is masked the
/// same way so a caller that passed one still decodes.
fn wire_id(msg_id: u32) -> u32 {
    if msg_id >= 0x80 {
        msg_id & 0x3f
    } else {
        msg_id
    }
}

/// Who the message is for, as far as the hook can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Receiver {
    /// The local player, or an entity whose type is `SGWPlayer` (2) or
    /// `SGWGmPlayer` (3): extended ids start at 61.
    Player,
    /// A known being of another class (fewer than 63 methods: no extended
    /// ids at all).
    Other,
    /// Not in the world map and not the local player (a queued message for
    /// an entity not yet created).
    Unknown,
}

/// The wire type ids of the player classes (`clientIndex`).
pub(crate) const PLAYER_TYPE_IDS: [u16; 2] = [2, 3];

/// Resolve `msg_id` to a method for `receiver`. `first` is the first
/// argument byte, which is the sub-index of an extended id. Returns the
/// row and how many leading bytes are the sub-index (0 or 1).
pub(crate) fn resolve(
    msg_id: u32,
    receiver: Receiver,
    first: Option<u8>,
) -> Option<(&'static RecvMethod, usize)> {
    let id = wire_id(msg_id);
    let (index, skip) = if id >= PLAYER_FIRST_EXTENDED_ID {
        // Extended ids exist only on the player classes; for any other
        // class (first extended id 62) this id is not one of ours.
        if receiver != Receiver::Player {
            return None;
        }
        let sub = u32::from(first?);
        let index = PLAYER_FIRST_EXTENDED_ID + 0x100 * (id - PLAYER_FIRST_EXTENDED_ID) + sub;
        (u16::try_from(index).ok()?, 1)
    } else {
        (id as u16, 0)
    };
    let m = by_index(index)?;
    match (m.scope, receiver) {
        (Scope::Shared, _) | (Scope::Player, Receiver::Player) => Some((m, skip)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::wire_decode::def_spelling;
    use super::*;

    #[test]
    fn direct_ids_resolve_for_any_being() {
        for r in [Receiver::Player, Receiver::Other, Receiver::Unknown] {
            let (m, skip) = resolve(14, r, None).unwrap();
            assert_eq!((m.name, skip), ("onEffectResults", 0));
        }
        // A raw entity-message byte (0x80 | 14) decodes the same way.
        assert_eq!(resolve(0x8e, Receiver::Other, None).unwrap().0.index, 14);
    }

    /// 101 is `0xBD` + sub-byte 40 for a player; the same id means nothing
    /// for a mob, and nothing without the sub-byte.
    #[test]
    fn extended_ids_need_a_player_and_the_sub_byte() {
        let (m, skip) = resolve(61, Receiver::Player, Some(40)).unwrap();
        assert_eq!((m.name, skip), ("onKnownAbilitiesUpdate", 1));
        assert_eq!(
            resolve(61, Receiver::Player, Some(60)).unwrap().0.name,
            "onErrorCode"
        );
        assert_eq!(
            resolve(61, Receiver::Player, Some(80)).unwrap().0.name,
            "onAbilityTreeInfo"
        );
        assert!(resolve(61, Receiver::Other, Some(40)).is_none());
        assert!(resolve(61, Receiver::Unknown, Some(40)).is_none());
        assert!(resolve(61, Receiver::Player, None).is_none());
        // A sub-index that is not one of ours.
        assert!(resolve(61, Receiver::Player, Some(0)).is_none());
    }

    /// Index 28 is `onPlayerCommunication` on a player and something else
    /// on a mob (`onAggressionOverrideCleared`).
    #[test]
    fn player_scoped_direct_ids_need_a_player() {
        assert!(resolve(28, Receiver::Player, None).is_some());
        assert!(resolve(28, Receiver::Other, None).is_none());
        assert!(resolve(28, Receiver::Unknown, None).is_none());
    }

    #[test]
    fn the_prefilter_admits_exactly_our_ids() {
        for id in [1, 12, 14, 19, 20, 21, 28, 61, 0x81] {
            assert!(may_be_wanted(id), "{id}");
        }
        for id in [0, 2, 13, 16, 26, 27, 60, 62, 63] {
            assert!(!may_be_wanted(id), "{id}");
        }
    }

    // -----------------------------------------------------------------
    // Conformance with the checked-in definitions.

    fn repo_file(rel: &str) -> String {
        let path = format!("{}/../../{rel}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    /// The text with XML comments and all whitespace removed.
    fn squash(xml: &str) -> String {
        let mut out = String::with_capacity(xml.len());
        let mut rest = xml;
        while let Some(start) = rest.find("<!--") {
            out.push_str(&rest[..start]);
            rest = rest[start..]
                .find("-->")
                .map_or("", |end| &rest[start + end + 3..]);
        }
        out.push_str(rest);
        out.retain(|c| !c.is_whitespace());
        out
    }

    /// The text between `<tag>` and the matching `</tag>` in `s`, from
    /// `from` on.
    fn element<'a>(s: &'a str, tag: &str, from: usize) -> Option<&'a str> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = s[from..].find(&open)? + from + open.len();
        let end = s[start..].find(&close)? + start;
        Some(&s[start..end])
    }

    /// `(type spelling, ArgName)` for each `<Arg>` of `method` in the
    /// `<ClientMethods>` block of `def`.
    fn def_args(def: &str, method: &str) -> Vec<(String, String)> {
        let s = squash(def);
        let client = element(&s, "ClientMethods", 0).expect("a ClientMethods block");
        let body = element(client, method, 0).unwrap_or_else(|| panic!("{method} not declared"));
        body.split("<Arg>")
            .skip(1)
            .map(|a| {
                let a = a.split("</Arg>").next().unwrap();
                let (ty, rest) = a.split_once("<ArgName>").expect("an ArgName");
                let name = rest.split("</ArgName>").next().unwrap();
                (ty.to_string(), name.to_string())
            })
            .collect()
    }

    /// The spelling of an `alias.xml` type: its body, whitespace removed.
    fn alias_body(alias: &str, name: &str) -> String {
        let s = squash(alias);
        element(&s, name, 0)
            .unwrap_or_else(|| panic!("alias {name} not found"))
            .to_string()
    }

    /// `alias.xml`'s `FIXED_DICT` properties, `(name, type)` in order.
    fn dict_fields(body: &str) -> Vec<(String, String)> {
        let props = element(body, "Properties", 0).expect("Properties");
        let mut out = Vec::new();
        let mut rest = props;
        while let Some(open) = rest.strip_prefix('<') {
            let name = &open[..open.find('>').unwrap()];
            let inner = element(rest, name, 0).unwrap();
            let ty = element(inner, "Type", 0).unwrap();
            out.push((name.to_string(), ty.to_string()));
            let close = format!("</{name}>");
            rest = &rest[rest.find(&close).unwrap() + close.len()..];
        }
        out
    }

    /// Check a table type against the `.def` / `alias.xml` spelling,
    /// expanding aliases and dictionaries.
    fn check_type(ty: &WireType, spelled: &str, alias: &str, at: &str) {
        match ty {
            WireType::Alias(name, inner) => {
                assert_eq!(spelled, *name, "{at}");
                let body = alias_body(alias, name);
                check_type(inner, &body, alias, &format!("{at} -> {name}"));
            }
            WireType::Dict(fields) => {
                assert!(spelled.starts_with("FIXED_DICT"), "{at}: {spelled}");
                let got = dict_fields(spelled);
                assert_eq!(got.len(), fields.len(), "{at}: {got:?}");
                for (f, (name, t)) in fields.iter().zip(&got) {
                    assert_eq!(f.name, name, "{at}");
                    check_type(&f.ty, t, alias, &format!("{at}.{name}"));
                }
            }
            WireType::Array(inner) => {
                let inner_spelled = spelled
                    .strip_prefix("ARRAY<of>")
                    .and_then(|s| s.strip_suffix("</of>"))
                    .unwrap_or_else(|| panic!("{at}: {spelled} is not an array"));
                check_type(inner, inner_spelled, alias, at);
            }
            _ => assert_eq!(spelled, def_spelling(ty), "{at}"),
        }
    }

    /// Every row's argument list is the `.def`'s, name for name and type
    /// for type, with each alias expanded through `alias.xml`. A changed
    /// definition fails here instead of misdecoding in a player's client.
    #[test]
    fn table_matches_entity_defs() {
        let alias = repo_file("entities/defs/alias.xml");
        for m in METHODS {
            let def = repo_file(&format!("entities/defs/{}", m.def_file));
            let declared = def_args(&def, m.name);
            assert_eq!(declared.len(), m.args.len(), "{}: {declared:?}", m.name);
            for (a, (ty, name)) in m.args.iter().zip(&declared) {
                assert_eq!(a.def_name, name, "{}", m.name);
                check_type(&a.ty, ty, &alias, &format!("{}.{name}", m.name));
            }
        }
    }

    /// Every index is the dispatch table's, which `cimmeria-wire`'s
    /// `def_conformance` keeps in step with the flattening rule.
    #[test]
    fn indices_match_the_dispatch_table() {
        let doc = repo_file("docs/protocol/client-method-dispatch-table.md");
        // The SGWPlayer table comes first; the SGWMob section reuses
        // small indices, so stop there.
        let player = doc.split("## SGWMob").next().unwrap();
        for m in METHODS {
            let row = format!("| {} | `{}` |", m.index, m.name);
            assert!(player.contains(&row), "missing row {row}");
        }
    }

    /// Shared rows are below the first `SGWPlayer`-only index (27), player
    /// rows at or above it, and indices are unique.
    #[test]
    fn scopes_follow_the_class_boundary() {
        for m in METHODS {
            assert_eq!(m.scope == Scope::Shared, m.index < 27, "{}", m.name);
            assert_eq!(
                METHODS.iter().filter(|o| o.index == m.index).count(),
                1,
                "{}",
                m.name
            );
        }
    }
}
