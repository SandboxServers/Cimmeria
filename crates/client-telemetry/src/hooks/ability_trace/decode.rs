//! The ability-method allowlist and the decode of its arguments from the
//! event bag (AB-C1).
//!
//! The arguments of an outbound method are not a struct: they are a
//! name-keyed property bag carried by the `Event_NetOut_*` event, keyed by
//! the `.def` `ArgName`s (finding, "The event bag"). The DLL reads each one
//! with the game's own typed reader (`GetInt` `0x00e3cba0`, `GetFloat`
//! `0x00e3cc20`, `GetByte` `0x00d434d0`), passing the method descriptor's
//! own argument-name string as the key. Here that reader is the
//! [`ArgBag`] trait, so the decode is tested against synthetic bags.

use serde_json::{json, Value};

use super::super::entity_trace::Fields;

/// The `.def` type of one argument, which picks the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArgKind {
    /// `INT32`: `GetInt`.
    Int,
    /// `FLOAT`: `GetFloat`.
    Float,
    /// `INT8` / `UINT8`: `GetByte`.
    Byte,
}

/// Where a decoded argument lands in the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    /// `ability_id`.
    AbilityId,
    /// `target_id` (the `TargetID` the client put on the wire).
    TargetId,
    /// `ground_xyz[0..3]`.
    Ground(usize),
    /// `pet_id`.
    PetId,
    /// `toggle`.
    Toggle,
    /// `effect_id`.
    EffectId,
    /// `accepted`.
    Accepted,
}

/// One argument: its `.def` name, type and destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArgSpec {
    /// The `.def` `ArgName`, trimmed.
    pub name: &'static str,
    /// Its `.def` type.
    pub kind: ArgKind,
    /// The event field it fills.
    pub slot: Slot,
}

/// One allowlisted method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodSpec {
    /// The `.def` method name (the `MethodDescription` name).
    pub name: &'static str,
    /// Flat cell-method index (`docs/protocol/cell-method-dispatch-table.md`).
    pub cell_index: u16,
    /// Its arguments in `.def` order.
    pub args: &'static [ArgSpec],
}

const fn arg(name: &'static str, kind: ArgKind, slot: Slot) -> ArgSpec {
    ArgSpec { name, kind, slot }
}

use ArgKind::{Byte, Float, Int};

/// Every method AB-C1 decodes. The `.def` files are the source for names
/// and types (`SGWPlayer.def`, `SGWAbilityManager.def`, `SGWGmPlayer.def`);
/// note the two spellings `aAbilityId` and `AbilityID`, which are the bag
/// keys as written. `toggleCombatDebug` (cell 2, 3) is absent on purpose:
/// the client has no event bound to it and cannot send it (finding).
pub(crate) const ALLOWLIST: &[MethodSpec] = &[
    MethodSpec {
        name: "useAbility",
        cell_index: 68,
        args: &[
            arg("AbilityID", Int, Slot::AbilityId),
            arg("TargetID", Int, Slot::TargetId),
        ],
    },
    MethodSpec {
        name: "useAbilityOnGroundTarget",
        cell_index: 69,
        args: &[
            arg("AbilityID", Int, Slot::AbilityId),
            arg("LocationX", Float, Slot::Ground(0)),
            arg("LocationY", Float, Slot::Ground(1)),
            arg("LocationZ", Float, Slot::Ground(2)),
        ],
    },
    MethodSpec {
        name: "petInvokeAbility",
        cell_index: 88,
        args: &[
            arg("aEntityId", Int, Slot::PetId),
            arg("aAbilityId", Int, Slot::AbilityId),
            arg("aTargetId", Int, Slot::TargetId),
        ],
    },
    MethodSpec {
        name: "petAbilityToggle",
        cell_index: 89,
        args: &[
            arg("aEntityId", Int, Slot::PetId),
            arg("aAbilityId", Int, Slot::AbilityId),
            arg("aToggle", Byte, Slot::Toggle),
        ],
    },
    MethodSpec {
        name: "confirmationResponse",
        cell_index: 4,
        args: &[
            arg("aEffectId", Int, Slot::EffectId),
            arg("aAccepted", Byte, Slot::Accepted),
        ],
    },
    MethodSpec {
        // Sent as `Event_NetOut_RespecAbility`; the router sees the method.
        name: "resetMyAbilities",
        cell_index: 72,
        args: &[],
    },
    MethodSpec {
        name: "trainAbility",
        cell_index: 77,
        args: &[arg("AbilityID", Int, Slot::AbilityId)],
    },
    MethodSpec {
        name: "gmDebugAbility",
        cell_index: 169,
        args: &[arg("aAbilityId", Int, Slot::AbilityId)],
    },
    MethodSpec {
        name: "gmDebugCombat",
        cell_index: 170,
        args: &[],
    },
    MethodSpec {
        name: "gmDebugCombatVerbose",
        cell_index: 171,
        args: &[],
    },
    MethodSpec {
        name: "gmDebugHeal",
        cell_index: 172,
        args: &[],
    },
    MethodSpec {
        name: "gmDebugAbilityOnMob",
        cell_index: 176,
        args: &[arg("AbilityID", Int, Slot::AbilityId)],
    },
];

/// The allowlist entry for `method`, if it is one.
pub(crate) fn spec_for(method: &str) -> Option<&'static MethodSpec> {
    ALLOWLIST.iter().find(|m| m.name == method)
}

/// The game's typed bag readers. `None` is a miss (the reader returned
/// false, or the method descriptor has no argument of that name).
pub(crate) trait ArgBag {
    /// `GetInt`.
    fn int(&self, name: &str) -> Option<i32>;
    /// `GetFloat`.
    fn float(&self, name: &str) -> Option<f32>;
    /// `GetByte`.
    fn byte(&self, name: &str) -> Option<u8>;
}

/// The decoded arguments of one allowlisted call.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct DecodedCall {
    /// `AbilityID` / `aAbilityId`.
    pub ability_id: Option<i32>,
    /// `TargetID` / `aTargetId`.
    pub target_id: Option<i32>,
    /// `LocationX/Y/Z`.
    pub ground: [Option<f32>; 3],
    /// `aEntityId` of a pet method.
    pub pet_id: Option<i32>,
    /// `aToggle`.
    pub toggle: Option<u8>,
    /// `aEffectId`.
    pub effect_id: Option<i32>,
    /// `aAccepted`.
    pub accepted: Option<u8>,
    /// Arguments the bag did not yield, by `.def` name.
    pub missing: Vec<&'static str>,
}

/// Read every argument of `spec` from `bag`.
pub(crate) fn decode(spec: &MethodSpec, bag: &dyn ArgBag) -> DecodedCall {
    let mut d = DecodedCall::default();
    for a in spec.args {
        let got = match a.kind {
            ArgKind::Int => bag.int(a.name).map(|v| {
                match a.slot {
                    Slot::AbilityId => d.ability_id = Some(v),
                    Slot::TargetId => d.target_id = Some(v),
                    Slot::PetId => d.pet_id = Some(v),
                    Slot::EffectId => d.effect_id = Some(v),
                    // An int argument never lands in a float or byte slot.
                    Slot::Ground(_) | Slot::Toggle | Slot::Accepted => {}
                }
            }),
            ArgKind::Float => bag.float(a.name).map(|v| {
                if let Slot::Ground(i) = a.slot {
                    d.ground[i.min(2)] = Some(v);
                }
            }),
            ArgKind::Byte => bag.byte(a.name).map(|v| match a.slot {
                Slot::Toggle => d.toggle = Some(v),
                Slot::Accepted => d.accepted = Some(v),
                _ => {}
            }),
        };
        if got.is_none() {
            d.missing.push(a.name);
        }
    }
    d
}

/// What the router hook knows about the send besides the arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SendCtx {
    /// This send's id, joined by `client.ability.sent_seq`.
    pub send_id: u32,
    /// The press that led here, when one matched.
    pub press_id: Option<u32>,
    /// Milliseconds from that press to this send, on the client's clock
    /// (AB-C6; `timing`).
    pub press_to_sent_ms: Option<u64>,
    /// `MethodDescription+0x44`.
    pub msg_id: i32,
    /// `MethodDescription+0x48`, reported when non-negative.
    pub sub_index: i32,
    /// `"base"` or `"cell"`.
    pub route: &'static str,
    /// `[[EM+0x8c]+0x4]+0xfc` at send time: what the UI had targeted.
    pub client_target_id: Option<i32>,
}

fn opt<T: Into<Value>>(v: Option<T>) -> Value {
    v.map(Into::into).unwrap_or(Value::Null)
}

/// `client.ability.sent`.
pub(crate) fn sent_fields(spec: &MethodSpec, call: &DecodedCall, ctx: &SendCtx) -> Fields {
    let mut f: Fields = vec![
        ("send_id", json!(ctx.send_id)),
        ("press_id", opt(ctx.press_id)),
        ("press_to_sent_ms", opt(ctx.press_to_sent_ms)),
        ("method", json!(spec.name)),
        ("cell_index", json!(spec.cell_index)),
        ("route", json!(ctx.route)),
        ("msg_id", json!(ctx.msg_id)),
    ];
    if ctx.sub_index >= 0 {
        f.push(("sub_index", json!(ctx.sub_index)));
    }
    let has = |s: Slot| spec.args.iter().any(|a| a.slot == s);
    if has(Slot::AbilityId) {
        f.push(("ability_id", opt(call.ability_id)));
    }
    if has(Slot::TargetId) {
        f.push(("target_id", opt(call.target_id)));
    }
    if has(Slot::Ground(0)) {
        let xyz: Vec<Value> = call.ground.iter().map(|v| opt(v.map(f64::from))).collect();
        f.push(("ground_xyz", Value::Array(xyz)));
    }
    if has(Slot::PetId) {
        f.push(("pet_id", opt(call.pet_id)));
    }
    if has(Slot::Toggle) {
        f.push(("toggle", opt(call.toggle)));
    }
    if has(Slot::EffectId) {
        f.push(("effect_id", opt(call.effect_id)));
    }
    if has(Slot::Accepted) {
        f.push(("accepted", opt(call.accepted)));
    }
    if !call.missing.is_empty() {
        f.push(("args_missing", json!(call.missing)));
    }
    f.push(("client_target_id", opt(ctx.client_target_id)));
    // Static anchor (finding, open question 1): the object identity of
    // `[[EM+0x8c]+0x4]` as the `GameBeing` is not live-verified yet.
    f.push(("client_target_inferred", json!(true)));
    f
}

#[cfg(test)]
mod tests {
    use super::super::field;
    use super::*;
    use std::collections::HashMap;

    /// A synthetic event bag: what the game's readers would return.
    #[derive(Default)]
    struct Bag {
        ints: HashMap<&'static str, i32>,
        floats: HashMap<&'static str, f32>,
        bytes: HashMap<&'static str, u8>,
    }

    impl ArgBag for Bag {
        fn int(&self, name: &str) -> Option<i32> {
            self.ints.get(name).copied()
        }
        fn float(&self, name: &str) -> Option<f32> {
            self.floats.get(name).copied()
        }
        fn byte(&self, name: &str) -> Option<u8> {
            self.bytes.get(name).copied()
        }
    }

    fn ctx() -> SendCtx {
        SendCtx {
            send_id: 7,
            press_id: Some(3),
            press_to_sent_ms: Some(12),
            msg_id: 0x44,
            sub_index: -1,
            route: "cell",
            client_target_id: Some(1234),
        }
    }

    #[test]
    fn use_ability_reads_ability_and_target() {
        let mut bag = Bag::default();
        bag.ints.insert("AbilityID", 597);
        bag.ints.insert("TargetID", 1234);
        let spec = spec_for("useAbility").unwrap();
        let d = decode(spec, &bag);
        assert_eq!(d.ability_id, Some(597));
        assert_eq!(d.target_id, Some(1234));
        assert!(d.missing.is_empty());
        let f = sent_fields(spec, &d, &ctx());
        assert_eq!(field(&f, "method"), Some(&json!("useAbility")));
        assert_eq!(field(&f, "cell_index"), Some(&json!(68)));
        assert_eq!(field(&f, "ability_id"), Some(&json!(597)));
        assert_eq!(field(&f, "target_id"), Some(&json!(1234)));
        assert_eq!(field(&f, "press_id"), Some(&json!(3)));
        assert_eq!(field(&f, "client_target_id"), Some(&json!(1234)));
        assert_eq!(field(&f, "client_target_inferred"), Some(&json!(true)));
        assert_eq!(field(&f, "sub_index"), None);
        assert_eq!(field(&f, "ground_xyz"), None);
    }

    #[test]
    fn a_ground_cast_carries_xyz() {
        let mut bag = Bag::default();
        bag.ints.insert("AbilityID", 42);
        bag.floats.insert("LocationX", 1.5);
        bag.floats.insert("LocationY", -2.0);
        bag.floats.insert("LocationZ", 300.25);
        let spec = spec_for("useAbilityOnGroundTarget").unwrap();
        let f = sent_fields(spec, &decode(spec, &bag), &ctx());
        assert_eq!(field(&f, "ground_xyz"), Some(&json!([1.5, -2.0, 300.25])));
        assert_eq!(field(&f, "target_id"), None, "no TargetID on a ground cast");
    }

    /// `confirmationResponse` is `INT32 aEffectId, UINT8 aAccepted` (the
    /// finding corrected the dispatch table's `INT8 choice`).
    #[test]
    fn confirmation_response_reads_an_int_and_a_byte() {
        let mut bag = Bag::default();
        bag.ints.insert("aEffectId", 0x0102_0304);
        bag.bytes.insert("aAccepted", 1);
        let spec = spec_for("confirmationResponse").unwrap();
        let d = decode(spec, &bag);
        assert_eq!(d.effect_id, Some(0x0102_0304));
        assert_eq!(d.accepted, Some(1));
        let f = sent_fields(spec, &d, &ctx());
        assert_eq!(field(&f, "effect_id"), Some(&json!(0x0102_0304)));
        assert_eq!(field(&f, "accepted"), Some(&json!(1)));
        assert_eq!(field(&f, "ability_id"), None);
    }

    #[test]
    fn pet_methods_read_the_pet_and_the_toggle_byte() {
        let mut bag = Bag::default();
        bag.ints.insert("aEntityId", 900);
        bag.ints.insert("aAbilityId", 11);
        bag.bytes.insert("aToggle", 0);
        let spec = spec_for("petAbilityToggle").unwrap();
        let f = sent_fields(spec, &decode(spec, &bag), &ctx());
        assert_eq!(field(&f, "pet_id"), Some(&json!(900)));
        assert_eq!(field(&f, "ability_id"), Some(&json!(11)));
        assert_eq!(field(&f, "toggle"), Some(&json!(0)));
    }

    /// A key the bag does not hold is reported, not invented: the field is
    /// null and the name is listed.
    #[test]
    fn a_missing_argument_is_listed_and_null() {
        let mut bag = Bag::default();
        bag.ints.insert("AbilityID", 5);
        let spec = spec_for("useAbility").unwrap();
        let d = decode(spec, &bag);
        assert_eq!(d.missing, vec!["TargetID"]);
        let f = sent_fields(spec, &d, &ctx());
        assert_eq!(field(&f, "target_id"), Some(&Value::Null));
        assert_eq!(field(&f, "args_missing"), Some(&json!(["TargetID"])));
    }

    /// The spelling matters: `gmDebugAbility` keys `aAbilityId`,
    /// `gmDebugAbilityOnMob` keys `AbilityID` (`SGWGmPlayer.def`).
    #[test]
    fn gm_debug_methods_use_their_own_spellings() {
        let mut bag = Bag::default();
        bag.ints.insert("aAbilityId", 1);
        bag.ints.insert("AbilityID", 2);
        let a = spec_for("gmDebugAbility").unwrap();
        let m = spec_for("gmDebugAbilityOnMob").unwrap();
        assert_eq!(decode(a, &bag).ability_id, Some(1));
        assert_eq!(decode(m, &bag).ability_id, Some(2));
        for name in ["gmDebugCombat", "gmDebugCombatVerbose", "gmDebugHeal"] {
            let s = spec_for(name).unwrap();
            assert!(s.args.is_empty(), "{name}");
            assert!(decode(s, &Bag::default()).missing.is_empty());
        }
    }

    #[test]
    fn the_allowlist_matches_the_plan() {
        let names: Vec<&str> = ALLOWLIST.iter().map(|m| m.name).collect();
        for n in [
            "useAbility",
            "useAbilityOnGroundTarget",
            "petInvokeAbility",
            "petAbilityToggle",
            "confirmationResponse",
            "resetMyAbilities",
            "trainAbility",
        ] {
            assert!(names.contains(&n), "{n}");
        }
        let gm: Vec<u16> = ALLOWLIST
            .iter()
            .filter(|m| m.name.starts_with("gmDebug"))
            .map(|m| m.cell_index)
            .collect();
        assert_eq!(gm, vec![169, 170, 171, 172, 176]);
        assert!(spec_for("toggleCombatDebug").is_none());
        assert!(spec_for("createCharacter").is_none());
    }
}
