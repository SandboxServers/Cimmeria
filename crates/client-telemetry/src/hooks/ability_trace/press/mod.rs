//! The press chain as a state machine (AB-C2).
//!
//! ```text
//! useAction thunk 0x00aa94e0 (hotbar)          useAbility thunk 0x00aa2910 (lua)
//!   └─ FUN_00ad9580(actionId, self)               └─ FUN_00ad78e0(abilityId, unitSlot)
//!        └─ action->vtable[9]                          │
//!             ├─ AbilityAction 0x00e3cd90 ─────────────┤
//!             │     └─ FUN_00d2afc0(set, ability, target)   map find; miss = drop 0x00d2afcf
//!             │          └─ FUN_00d2ae40(set, record, target) ─► Event_NetOut_UseAbility posted
//!             └─ PetAbilityAction 0x00e3cf40            pet not found = drop 0x00e3cfb1
//!                   └─ GamePet send 0x00d3a820          three more drops, then petInvokeAbility posted
//! ```
//!
//! One [`Press`] lives in a thread-local for the length of the thunk
//! (the whole chain runs synchronously on the main thread). Each hook in
//! the chain moves it on and may return events. The post is asynchronous:
//! the router (`0x00c6fc40`) runs on a later pump of the event queue, so
//! a press that reaches the post leaves a [`PendingSend`], and the router
//! hook claims it by method and ability id ([`PendingTable::take`]).
//!
//! Reasons are the client's own branches (finding, "Where the press can be
//! dropped", plus the three GamePet branches read for this packet). The
//! client checks no cooldown, range, target or death, so there are no
//! such reasons here.

use serde_json::{json, Value};

use super::layout::PetGate;
use super::{Out, TARGET_DROPPED, TARGET_PRESS};

mod pending;
mod route_outcome;

// Used by the i686 detours (and the tests); the host build has no caller.
#[cfg_attr(not(target_arch = "x86"), allow(unused_imports))]
pub(crate) use pending::{PendingSend, PendingTable, GROUND_TTL_MS, SEND_TTL_MS};
// Used by the i686 detours (and the tests); the host build has no caller.
#[cfg_attr(not(target_arch = "x86"), allow(unused_imports))]
pub(crate) use route_outcome::{route_dropped, route_outcome, RouteOutcome};

/// Which Lua binding started the press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// `useAction(actionId, self)`: `ActionButtonMod.onActionPress`, click
    /// or hotkey.
    Hotbar,
    /// `useAbility(id, unitSlot)`: the Ability window's button
    /// (`Ability.lua:181`) and any other script. Native code cannot tell
    /// the callers apart, so both are `lua`.
    Lua,
}

impl Source {
    /// The `source` field.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Source::Hotbar => "hotbar",
            Source::Lua => "lua",
        }
    }
}

/// Why the client discarded a press or an allowlisted send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DropReason {
    /// Empty action slot, or an action id outside 1..=200 (row 3).
    NoAction,
    /// The Lua call failed the binding's argument check (row 2); the
    /// binding raised a Lua error.
    BadArgs,
    /// The ability is not in the client's `AbilitySet` (row 5), or not in
    /// the pet's.
    NotKnown,
    /// The pet entity does not resolve to a `GamePet` (row 4).
    PetMissing,
    /// `GamePet+0x38` lacks bit `0x400` (GamePet send, `0x00d3a84a`).
    PetStateFlag,
    /// The pet's ability record has bit `0x8` of `+0x98` set
    /// (`0x00d3a875`).
    PetAbilityFlag,
    /// No connection, not connected, or the local player is not in the
    /// world (rows 6 to 8).
    NotConnected,
    /// The entity's type has no description mapping, or its class chain
    /// does not contain the method's class (rows 9 and 10).
    ClassMismatch,
}

impl DropReason {
    /// The `reason` field.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            DropReason::NoAction => "no_action",
            DropReason::BadArgs => "bad_args",
            DropReason::NotKnown => "not_known",
            DropReason::PetMissing => "pet_missing",
            DropReason::PetStateFlag => "pet_state_flag",
            DropReason::PetAbilityFlag => "pet_ability_flag",
            DropReason::NotConnected => "not_connected",
            DropReason::ClassMismatch => "class_mismatch",
        }
    }
}

/// Branch addresses, reported as `drop_site`.
pub(crate) mod site {
    /// `useAction` thunk: an argument check failed.
    pub(crate) const USE_ACTION_ARGS: &str = "0x00aa9569";
    /// `useAbility` thunk: an argument check failed.
    pub(crate) const USE_ABILITY_ARGS: &str = "0x00aa2997";
    /// `FUN_00ad9580`: no action in the slot.
    pub(crate) const EMPTY_SLOT: &str = "0x00ad959e";
    /// `PetAbilityAction::execute`: no `GamePet`.
    pub(crate) const NO_PET: &str = "0x00e3cfb1";
    /// `FUN_00d2afc0`: not in the `AbilitySet`.
    pub(crate) const NOT_KNOWN: &str = "0x00d2afcf";
    /// GamePet send: state bit clear.
    pub(crate) const PET_STATE: &str = "0x00d3a84a";
    /// GamePet send: not in the pet's `AbilitySet`.
    pub(crate) const PET_NOT_KNOWN: &str = "0x00d3a862";
    /// GamePet send: record flag set.
    pub(crate) const PET_ABILITY_FLAG: &str = "0x00d3a875";
    /// Router: no `ServerConnection`.
    pub(crate) const NO_CONNECTION: &str = "0x00c6fc68";
    /// Router: not connected.
    pub(crate) const NOT_CONNECTED: &str = "0x00c6fc77";
    /// Router: the local player is not in the world.
    pub(crate) const NO_LOCAL_PLAYER: &str = "0x00c6fca8";
    /// Router: rows 9 and 10 are told apart only by calling game code.
    pub(crate) const CLASS: &str = "0x00c6fcd2|0x00c6fd1d|0x00c6fd41";
}

/// One press in flight on the main thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Press {
    /// Local id linking the press to its `sent` or `press_dropped`.
    pub press_id: u32,
    /// Which binding started it.
    pub source: Source,
    /// The hotbar action id (1-based), when it came from the hotbar.
    pub slot: Option<i32>,
    /// `useAction`'s second argument: target self.
    pub self_cast: Option<bool>,
    /// The ability, once known.
    pub ability_id: Option<i32>,
    /// The `TargetID` the client is about to send.
    pub target_id: Option<i32>,
    /// The pet, for a pet action.
    pub pet_id: Option<i32>,
    /// Pending presses that expired unclaimed since the last press.
    pub pending_expired: u64,
    entered_slot: bool,
    entered_lookup: bool,
    entered_send: bool,
    announced: bool,
    resolved: bool,
}

pub(super) fn opt<T: Into<Value>>(v: Option<T>) -> Value {
    v.map(Into::into).unwrap_or(Value::Null)
}

impl Press {
    /// A new press from `source`.
    pub(crate) fn begin(source: Source, press_id: u32, pending_expired: u64) -> Self {
        Self {
            press_id,
            source,
            slot: None,
            self_cast: None,
            ability_id: None,
            target_id: None,
            pet_id: None,
            pending_expired,
            entered_slot: false,
            entered_lookup: false,
            entered_send: false,
            announced: false,
            resolved: false,
        }
    }

    /// Whether the press has its `sent`-side or `press_dropped` answer.
    #[cfg(test)]
    pub(crate) fn resolved(&self) -> bool {
        self.resolved
    }

    fn press_out(&self) -> Out {
        let mut f = vec![
            ("press_id", json!(self.press_id)),
            ("source", json!(self.source.as_str())),
            ("slot", opt(self.slot)),
            ("ability_id", opt(self.ability_id)),
            ("target_id", opt(self.target_id)),
        ];
        if let Some(s) = self.self_cast {
            f.push(("self_cast", json!(s)));
        }
        if let Some(p) = self.pet_id {
            f.push(("pet_id", json!(p)));
        }
        if self.pending_expired > 0 {
            f.push(("pending_expired", json!(self.pending_expired)));
        }
        Out {
            target: TARGET_PRESS,
            level: "info",
            key: format!("{TARGET_PRESS}:{}", self.source.as_str()),
            fields: f,
        }
    }

    fn announce(&mut self, outs: &mut Vec<Out>) {
        if !self.announced {
            self.announced = true;
            outs.push(self.press_out());
        }
    }

    fn drop_with(&mut self, reason: DropReason, site: &'static str) -> Vec<Out> {
        let mut outs = Vec::new();
        self.announce(&mut outs);
        self.resolved = true;
        let mut f = vec![
            ("press_id", json!(self.press_id)),
            ("source", json!(self.source.as_str())),
            ("slot", opt(self.slot)),
            ("ability_id", opt(self.ability_id)),
            ("reason", json!(reason.as_str())),
            ("drop_site", json!(site)),
        ];
        if let Some(p) = self.pet_id {
            f.push(("pet_id", json!(p)));
        }
        outs.push(Out {
            target: TARGET_DROPPED,
            level: "info",
            key: format!("{TARGET_DROPPED}:{}", reason.as_str()),
            fields: f,
        });
        outs
    }

    /// `FUN_00ad9580(actionId, self)` entered.
    pub(crate) fn slot_entered(&mut self, action_id: i32, self_flag: bool) {
        self.entered_slot = true;
        self.slot = Some(action_id);
        self.self_cast = Some(self_flag);
    }

    /// `FUN_00ad9580` returned. `action` is what the slot held (read
    /// before the call); an empty slot with no executor reached is row 3.
    /// A slot holding something that is not an ability action (an item, a
    /// macro) reaches no ability executor and is not an ability press, so
    /// it reports nothing.
    pub(crate) fn slot_left(&mut self, action: Option<u32>) -> Vec<Out> {
        if !self.entered_lookup && !self.resolved && action == Some(0) {
            return self.drop_with(DropReason::NoAction, site::EMPTY_SLOT);
        }
        Vec::new()
    }

    /// `FUN_00d2afc0(set, ability, target)` entered: the press is an
    /// ability press, and this is the target the client will send.
    pub(crate) fn lookup_entered(&mut self, ability_id: i32, target_id: i32) -> Vec<Out> {
        self.entered_lookup = true;
        self.ability_id = Some(ability_id);
        self.target_id = Some(target_id);
        let mut outs = Vec::new();
        self.announce(&mut outs);
        outs
    }

    /// `FUN_00d2afc0` returned without reaching `FUN_00d2ae40`: row 5.
    pub(crate) fn lookup_left(&mut self) -> Vec<Out> {
        if !self.entered_send && !self.resolved {
            return self.drop_with(DropReason::NotKnown, site::NOT_KNOWN);
        }
        Vec::new()
    }

    /// `FUN_00d2ae40` entered: the event will be posted, or the ground
    /// reticle opened (`record+0x48 == 3`), whose `useAbilityOnGroundTarget`
    /// is sent when the player places it.
    pub(crate) fn send_entered(&mut self, ground: Option<bool>, now_ms: u64) -> PendingSend {
        self.entered_send = true;
        self.resolved = true;
        let ground = ground == Some(true);
        PendingSend {
            press_id: self.press_id,
            source: self.source,
            method: if ground {
                "useAbilityOnGroundTarget"
            } else {
                "useAbility"
            },
            ability_id: self.ability_id,
            at_ms: now_ms,
            ttl_ms: if ground { GROUND_TTL_MS } else { SEND_TTL_MS },
        }
    }

    /// `PetAbilityAction::execute` entered.
    pub(crate) fn pet_entered(&mut self, ability_id: i32, pet_id: i32) -> Vec<Out> {
        self.entered_lookup = true;
        self.ability_id = Some(ability_id);
        self.pet_id = Some(pet_id);
        let mut outs = Vec::new();
        self.announce(&mut outs);
        outs
    }

    /// `PetAbilityAction::execute` returned without reaching the GamePet
    /// send: row 4.
    pub(crate) fn pet_left(&mut self) -> Vec<Out> {
        if !self.entered_send && !self.resolved {
            return self.drop_with(DropReason::PetMissing, site::NO_PET);
        }
        Vec::new()
    }

    /// The GamePet send entered, with its three gates read from memory. An
    /// unreadable gate is taken as passed: the router settles it.
    pub(crate) fn pet_send_entered(
        &mut self,
        gate: PetGate,
        target_id: i32,
        now_ms: u64,
    ) -> (Vec<Out>, Option<PendingSend>) {
        self.entered_send = true;
        self.target_id = Some(target_id);
        if gate.ready == Some(false) {
            return (
                self.drop_with(DropReason::PetStateFlag, site::PET_STATE),
                None,
            );
        }
        if gate.known == Some(false) {
            return (
                self.drop_with(DropReason::NotKnown, site::PET_NOT_KNOWN),
                None,
            );
        }
        if gate.allowed == Some(false) {
            return (
                self.drop_with(DropReason::PetAbilityFlag, site::PET_ABILITY_FLAG),
                None,
            );
        }
        self.resolved = true;
        let pending = PendingSend {
            press_id: self.press_id,
            source: self.source,
            method: "petInvokeAbility",
            ability_id: self.ability_id,
            at_ms: now_ms,
            ttl_ms: SEND_TTL_MS,
        };
        (Vec::new(), Some(pending))
    }

    /// The thunk returned, or unwound with a Lua error. A press that never
    /// reached the next step failed the binding's argument check (row 2).
    pub(crate) fn thunk_left(&mut self) -> Vec<Out> {
        if self.resolved {
            return Vec::new();
        }
        match self.source {
            Source::Hotbar if !self.entered_slot => {
                self.drop_with(DropReason::BadArgs, site::USE_ACTION_ARGS)
            }
            Source::Lua if !self.entered_lookup => {
                self.drop_with(DropReason::BadArgs, site::USE_ABILITY_ARGS)
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
