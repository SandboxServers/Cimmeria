//! AB-N2: the native ability-testing GM commands' cell halves
//! (`gmGiveAbility` 136, `gmSetGodMode` 142, `gmResetAbilities` 153,
//! `gmGiveAllAbilities` 154, `gmSetMobAbilitySet` 158).
//!
//! Each index is in the gated tail and refused for a non-GM before any
//! handler runs; each GM call reaches its handler and either changes state
//! or forwards to the base; each refusal answers the GM with a line and
//! writes one `gm_command` row.

use cimmeria_cell_catalog::ability_tree::{AbilityTreeCatalog, TreeNode};

use super::*;
use crate::cell::dispatch::gm_gate::{enforce_gm_gate, requires_gm};
use crate::cell::messages::{GmAbilityBulk, GmAbilityChange};
use crate::test_support::LogCapture;

const GM: u32 = 1;
const MOB: u32 = 50;
/// The GM's archetype (Commando) and its fixture tree.
const ARCHETYPE: i32 = 2;
const TREE: [i32; 4] = [597, 641, 642, 700];

const AB_N2: [u16; 5] = [
    GM_GIVE_ABILITY,
    GM_SET_GOD_MODE,
    GM_RESET_ABILITIES,
    GM_GIVE_ALL_ABILITIES,
    GM_SET_MOB_ABILITY_SET,
];

fn fixture() -> SpaceManager {
    let mut mgr = mgr_with_player(GM, "Castle");
    let gm = mgr.get_entity_mut(GM).unwrap();
    gm.archetype_id = Some(ARCHETYPE);
    gm.abilities.add_ability(597);
    cimmeria_cell_world::test_fixtures::seed_ability_defs(&mut mgr, &[597, 641, 642, 700, 2826]);
    // 641 appears in two branches: give-all must send it once.
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCHETYPE, 0, 597, 1, vec![]),
        TreeNode::with_defaults(ARCHETYPE, 0, 641, 1, vec![597]),
        TreeNode::with_defaults(ARCHETYPE, 1, 642, 1, vec![]),
        TreeNode::with_defaults(ARCHETYPE, 1, 641, 1, vec![]),
        TreeNode::with_defaults(ARCHETYPE, 2, 700, 1, vec![]),
        TreeNode::with_defaults(9, 0, 999, 1, vec![]),
    ]);
    mgr
}

async fn call(mgr: &mut SpaceManager, index: u16, args: &[u8]) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(32);
    assert!(
        dispatch(GM, index, args, &tx, mgr, &test_engine()).await,
        "index {index} must be handled, not fall through"
    );
    drain(&mut rx)
}

fn gm_rows(capture: &crate::test_support::LogCaptureGuard) -> Vec<crate::test_support::Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "gm_command"))
        .collect()
}

/// One `gm_command` row with `outcome` (and `reason`), carrying the
/// caller's identity.
fn one_row(capture: &crate::test_support::LogCaptureGuard, outcome: &str, reason: Option<&str>) {
    let rows = gm_rows(capture);
    assert_eq!(rows.len(), 1, "one gm_command row: {rows:#?}");
    let r = &rows[0];
    assert!(r.has_field("decision_outcome", outcome), "{r:#?}");
    assert!(reason.is_none_or(|x| r.has_field("reason", x)), "{r:#?}");
    assert!(r.has_field("player_id", "100"), "caller identity: {r:#?}");
}

/// **Server authority.** Every AB-N2 index sits in the GM tail and the
/// gate refuses it for a player with an `onErrorCode`, before a handler.
#[tokio::test]
async fn ab_n2_indices_are_gated_for_non_gms() {
    for idx in AB_N2 {
        assert!(requires_gm(idx), "{idx} must be GM-gated");
        let mut mgr = fixture();
        mgr.get_entity_mut(GM).unwrap().access_level = 0;
        let (tx, mut rx) = mpsc::channel(8);
        assert!(!enforce_gm_gate(GM, idx, &tx, &mgr).await, "{idx}: refused");
        let msgs = drain(&mut rx);
        assert!(
            matches!(
                msgs.as_slice(),
                [CellToBaseMsg::EntityMethodCall {
                    method_index: 121,
                    ..
                }]
            ),
            "{idx}: only onErrorCode: {msgs:?}"
        );
        mgr.get_entity_mut(GM).unwrap().access_level = 2;
        assert!(
            enforce_gm_gate(GM, idx, &tx, &mgr).await,
            "{idx}: GM passes"
        );
    }
}

// ── gmGiveAbility (136) ─────────────────────────────────────────────────────

/// The grant goes to the base as the `.giveability` grant, for the caller,
/// with no training-point message anywhere.
#[tokio::test]
async fn gm_give_ability_forwards_a_no_debit_grant_for_the_caller() {
    let mut mgr = fixture();
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_GIVE_ABILITY, &2826i32.to_le_bytes()).await;
    let grants: Vec<_> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GmGrantAbility {
                entity_id,
                player_id,
                ability_id,
                gm_entity_id,
                ..
            } => Some((*entity_id, *player_id, *ability_id, *gm_entity_id)),
            _ => None,
        })
        .collect();
    assert_eq!(grants, vec![(GM, 100, 2826, GM)]);
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::TrainAbility { .. } | CellToBaseMsg::GrantTrainingPoints { .. }
        )),
        "a GM grant never touches training points: {msgs:?}"
    );
    one_row(&capture, "forwarded", None);
}

#[tokio::test]
async fn gm_give_ability_refuses_unknown_known_and_truncated() {
    for (args, reason) in [
        (99_999i32.to_le_bytes().to_vec(), "unknown_ability"),
        (597i32.to_le_bytes().to_vec(), "already_known"),
        (vec![1, 2], "bad_args"),
    ] {
        let mut mgr = fixture();
        let capture = LogCapture::install();
        let msgs = call(&mut mgr, GM_GIVE_ABILITY, &args).await;
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::GmGrantAbility { .. })),
            "{reason}: nothing to the base"
        );
        let text = feedback_text(&msgs, GM).expect("refusal feedback");
        assert!(text.starts_with("gmGiveAbility:"), "{reason}: {text}");
        one_row(&capture, "refused", Some(reason));
    }
}

// ── gmSetGodMode (142) ──────────────────────────────────────────────────────

#[tokio::test]
async fn gm_set_god_mode_toggles_the_callers_flag_with_feedback() {
    let mut mgr = fixture();
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_SET_GOD_MODE, &[1]).await;
    assert!(mgr.get_entity(GM).unwrap().god_mode, "on");
    assert!(feedback_text(&msgs, GM).unwrap().contains("on"));
    one_row(&capture, "applied", None);

    let msgs = call(&mut mgr, GM_SET_GOD_MODE, &[0]).await;
    assert!(!mgr.get_entity(GM).unwrap().god_mode, "off");
    assert!(feedback_text(&msgs, GM).unwrap().contains("off"));
}

/// A short packet changes nothing: god mode is a security-relevant flag.
#[tokio::test]
async fn gm_set_god_mode_truncated_args_change_nothing() {
    let mut mgr = fixture();
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_SET_GOD_MODE, &[]).await;
    assert!(!mgr.get_entity(GM).unwrap().god_mode);
    assert!(feedback_text(&msgs, GM).is_some());
    one_row(&capture, "refused", Some("bad_args"));
}

// ── gmGiveAllAbilities (154) / gmResetAbilities (153) ───────────────────────

/// **Give-all count.** The fixture tree holds 597, 641 (twice), 642 and
/// 700; the GM knows 597. Exactly 641, 642, 700 go to the base, once each
/// and in tree order, and nothing of another archetype's tree.
#[tokio::test]
async fn gm_give_all_sends_the_missing_tree_abilities_once() {
    let mut mgr = fixture();
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_GIVE_ALL_ABILITIES, &[]).await;
    let bulk: Vec<&GmAbilityBulk> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GmAbilityBulk(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(bulk.len(), 1, "one bulk write: {msgs:?}");
    assert_eq!(bulk[0].change, GmAbilityChange::GrantAll);
    assert_eq!((bulk[0].entity_id, bulk[0].player_id), (GM, 100));
    assert_eq!(bulk[0].ability_ids, TREE[1..].to_vec());
    one_row(&capture, "forwarded", None);
}

#[tokio::test]
async fn gm_give_all_with_everything_known_refuses_with_feedback() {
    let mut mgr = fixture();
    for id in TREE {
        mgr.get_entity_mut(GM).unwrap().abilities.add_ability(id);
    }
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_GIVE_ALL_ABILITIES, &[]).await;
    assert!(!msgs
        .iter()
        .any(|m| matches!(m, CellToBaseMsg::GmAbilityBulk(_))));
    assert!(feedback_text(&msgs, GM)
        .unwrap()
        .contains("already know all"));
    one_row(&capture, "refused", Some("nothing_to_grant"));
}

#[tokio::test]
async fn gm_give_all_without_a_tree_refuses_with_feedback() {
    let mut mgr = fixture();
    mgr.get_entity_mut(GM).unwrap().archetype_id = Some(7);
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_GIVE_ALL_ABILITIES, &[]).await;
    assert!(feedback_text(&msgs, GM).is_some());
    one_row(&capture, "refused", Some("no_tree"));
}

/// The reset needs no trainer: no pin, no range, straight to the base.
#[tokio::test]
async fn gm_reset_abilities_forwards_without_a_trainer() {
    let mut mgr = fixture();
    assert!(mgr
        .get_entity(GM)
        .unwrap()
        .last_interaction_target
        .is_none());
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_RESET_ABILITIES, &[]).await;
    assert!(
        matches!(
            msgs.as_slice(),
            [CellToBaseMsg::GmAbilityBulk(GmAbilityBulk {
                entity_id: GM,
                player_id: 100,
                change: GmAbilityChange::Reset,
                ..
            })]
        ),
        "{msgs:?}"
    );
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::ResetAbilities { .. })),
        "not the paid trainer respec"
    );
    one_row(&capture, "forwarded", None);
}

// ── gmSetMobAbilitySet (158) ────────────────────────────────────────────────

fn with_mob(mgr: &mut SpaceManager) {
    mgr.spawn_npc(MOB, "Castle", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(MOB).unwrap().abilities.add_ability(592);
    mgr.ability_sets.insert(350, vec![221, 1156]);
    mgr.get_entity_mut(GM).unwrap().current_target_id = Some(MOB as i32);
}

#[tokio::test]
async fn gm_set_mob_ability_set_replaces_the_selected_mobs_abilities() {
    let mut mgr = fixture();
    with_mob(&mut mgr);
    let capture = LogCapture::install();
    let msgs = call(&mut mgr, GM_SET_MOB_ABILITY_SET, &350i32.to_le_bytes()).await;
    let mut known = mgr.get_entity(MOB).unwrap().abilities.known_ability_ids();
    known.sort_unstable();
    assert_eq!(known, vec![221, 1156], "exactly the set; 592 is gone");
    assert!(feedback_text(&msgs, GM).unwrap().contains("set 350"));
    one_row(&capture, "applied", None);
}

#[tokio::test]
async fn gm_set_mob_ability_set_refusals_leave_the_mob_alone() {
    type Setup = fn(&mut SpaceManager);
    let cases: [(&str, i32, Setup); 4] = [
        ("no_target", 350, |m| {
            m.get_entity_mut(GM).unwrap().current_target_id = None
        }),
        ("target_is_player", 350, |m| {
            m.get_entity_mut(GM).unwrap().current_target_id = Some(GM as i32)
        }),
        ("no_ability_set_data", 350, |m| m.ability_sets.clear()),
        ("unknown_set", 4242, |_| {}),
    ];
    for (reason, set_id, setup) in cases {
        let mut mgr = fixture();
        with_mob(&mut mgr);
        setup(&mut mgr);
        let gm_known = mgr.get_entity(GM).unwrap().abilities.known_ability_ids();
        let capture = LogCapture::install();
        let msgs = call(&mut mgr, GM_SET_MOB_ABILITY_SET, &set_id.to_le_bytes()).await;
        assert_eq!(
            mgr.get_entity(MOB).unwrap().abilities.known_ability_ids(),
            vec![592],
            "{reason}: the mob keeps its set"
        );
        assert_eq!(
            mgr.get_entity(GM).unwrap().abilities.known_ability_ids(),
            gm_known,
            "{reason}: a player's abilities are never rewritten"
        );
        assert!(feedback_text(&msgs, GM).is_some(), "{reason}: feedback");
        one_row(&capture, "refused", Some(reason));
    }
}

/// Park a warming cast of `ability_id` on the mob.
fn warming(mgr: &mut SpaceManager, ability_id: i32) {
    use cimmeria_entity::cell_entity::PendingCast;
    let (anchor, space_id) = {
        let m = mgr.get_entity(MOB).unwrap();
        (m.position, m.space_id)
    };
    mgr.get_entity_mut(MOB).unwrap().pending_cast = Some(PendingCast {
        ability_id,
        target_id: GM as i32,
        wire_target_id: GM as i32,
        ground: None,
        effect_seq: 1,
        fire_at: std::time::Instant::now() + std::time::Duration::from_secs(5),
        warmup_secs: 5.0,
        anchor,
        space_id,
        weapon_instance: None,
    });
    mgr.pending_casts.insert(MOB);
}

/// **Guard: a swap cancels a warmup of an ability it removed.** The mob is
/// warming 592; set 350 does not hold it, so the cast is interrupted
/// instead of firing an ability the mob no longer knows when the warmup
/// ends. On revert the pending cast survives.
#[tokio::test]
async fn gm_set_mob_ability_set_interrupts_a_warmup_of_a_removed_ability() {
    let mut mgr = fixture();
    with_mob(&mut mgr);
    warming(&mut mgr, 592);
    call(&mut mgr, GM_SET_MOB_ABILITY_SET, &350i32.to_le_bytes()).await;
    assert!(
        mgr.get_entity(MOB).unwrap().pending_cast.is_none(),
        "the warming 592 is interrupted"
    );
}

/// A warmup of an ability the new set keeps is left to fire.
#[tokio::test]
async fn gm_set_mob_ability_set_keeps_a_warmup_of_a_kept_ability() {
    let mut mgr = fixture();
    with_mob(&mut mgr);
    mgr.get_entity_mut(MOB).unwrap().abilities.add_ability(221);
    warming(&mut mgr, 221);
    call(&mut mgr, GM_SET_MOB_ABILITY_SET, &350i32.to_le_bytes()).await;
    assert_eq!(
        mgr.get_entity(MOB)
            .unwrap()
            .pending_cast
            .as_ref()
            .map(|c| c.ability_id),
        Some(221)
    );
}
