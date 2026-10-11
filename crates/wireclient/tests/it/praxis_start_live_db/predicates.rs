//! What the Praxis-start script matches in its log, and what it reads from
//! the capture's call records. Pure functions of one log entry or one
//! fixture record.

use cimmeria_wireclient::script::Observed;
use cimmeria_wireclient::semantic::{DialogDisplay, InvItem, S2CEvent};
use cimmeria_wireclient::tap_fixture::TapRecord;

pub(super) fn dialog(o: &Observed, dialog_id: i32) -> Option<&DialogDisplay> {
    match &o.event {
        Some(S2CEvent::DialogDisplay(d)) if d.dialog_id == dialog_id => Some(d),
        _ => None,
    }
}

pub(super) fn step(o: &Observed, step_id: i32, status: i8) -> bool {
    matches!(o.event, Some(S2CEvent::StepUpdate { step_id: s, status: st }) if s == step_id && st == status)
}

pub(super) fn mission(o: &Observed, mission_id: i32, status: i8) -> bool {
    matches!(o.event, Some(S2CEvent::MissionUpdate { mission_id: m, status: st, .. })
        if m == mission_id && st == status)
}

pub(super) fn sequence(o: &Observed, seq_id: i32) -> bool {
    matches!(&o.event, Some(S2CEvent::Sequence(s)) if s.kismet_event_set_seq_id == seq_id)
}

/// An `onKnownAbilitiesUpdate` whose list holds every id in `want`. The list
/// is the player's whole known set, so it is checked by containment: an
/// ability the character already had (a future `char_creation_abilities`
/// row for its char def) must not break the check.
pub(super) fn knows_all(o: &Observed, want: &[i32]) -> bool {
    matches!(&o.event, Some(S2CEvent::KnownAbilitiesUpdate { ability_ids })
        if want.iter().all(|a| ability_ids.contains(a)))
}

/// An `InteractionType` of exactly `type_id` on `entity_id`.
pub(super) fn interaction_on(o: &Observed, entity_id: u32, type_id: u64) -> bool {
    is_interaction(o, type_id) && o.msg.entity_id == Some(entity_id)
}

pub(super) fn is_interaction(o: &Observed, type_id: u64) -> bool {
    matches!(o.event, Some(S2CEvent::InteractionType { type_id: t }) if t == type_id)
}

/// The SYSTEM line `gmGotoXYZ` answers with (`cell-console` `gm/travel.rs`).
pub(super) fn is_teleport_line(o: &Observed) -> bool {
    matches!(&o.event, Some(S2CEvent::PlayerCommunication { text, .. })
        if text.starts_with("gmGotoXYZ: teleported to"))
}

pub(super) fn update_has(o: &Observed, pred: impl Fn(&InvItem) -> bool) -> bool {
    matches!(&o.event, Some(S2CEvent::UpdateItem { items }) if items.iter().any(pred))
}

/// `gmGotoXYZ`'s position: three `f32` after the 4-byte entity id and the
/// `0xBD` sub-slot byte.
pub(super) fn goto_pos(rec: &TapRecord) -> [f32; 3] {
    let a = rec.args();
    [f32_at(&a, 5), f32_at(&a, 9), f32_at(&a, 13)]
}

/// The region trigger's INT32 region id, after the 4-byte prefix and the
/// sub-slot byte.
pub(super) fn region_id(rec: &TapRecord) -> i32 {
    i32::from_le_bytes(bytes_at(&rec.args(), 5))
}

/// The region trigger's position: after the prefix, the sub-slot byte, the
/// INT32 region id and the UINT8 entering flag.
pub(super) fn region_pos(rec: &TapRecord) -> [f32; 3] {
    let a = rec.args();
    [f32_at(&a, 10), f32_at(&a, 14), f32_at(&a, 18)]
}

fn f32_at(args: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(bytes_at(args, at))
}

fn bytes_at(args: &[u8], at: usize) -> [u8; 4] {
    args[at..at + 4]
        .try_into()
        .expect("the fixture record holds four bytes here")
}
