//! Pets PT-06 telemetry: the `pets.credit` rows a kill leaves behind
//! (TESTING.md type 12, negative-log).
//!
//! Each test drives a real kill through `kill_npc_out_of_band` and pins the
//! row that answers "why did / didn't this kill pay XP?" from SigNoz: the
//! success event on a pet kill, and one `reason` per seam that pays nothing.

use tracing::Level;

use super::pet_credit_tests::{kill, spawn_mob, world, MOB_XP, OWNER, OWNER_PLAYER_ID};
use crate::test_support::{Captured, LogCapture};

/// The one row on `target` with `event = <event>`.
fn only_event(logs: &[Captured], target: &str, event: &str) -> Captured {
    let rows: Vec<_> = logs
        .iter()
        .filter(|c| c.target == target && c.has_field("event", event))
        .cloned()
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "exactly one `{event}` row on `{target}`: {logs:#?}"
    );
    rows.into_iter().next().unwrap()
}

/// The owner correlators every pet row carries: `owner_id` and the
/// summoner's `account_id` / `player_id`, recorded as plain values (an
/// `Option` field is omitted when `None`, never logged as `Some(..)` or 0).
fn assert_owner_identity(row: &Captured) {
    assert!(row.has_field("owner_id", &OWNER.to_string()), "{row:?}");
    // `add_pet_owner` sets `account_id = entity_id`.
    assert!(row.has_field("account_id", &OWNER.to_string()), "{row:?}");
    assert!(
        row.has_field("player_id", &OWNER_PLAYER_ID.to_string()),
        "{row:?}"
    );
}

/// Rule 5 correlator: `entity_id` (and `pet_id`) name the pet.
fn assert_pet(row: &Captured, pet: u32) {
    assert!(row.has_field("entity_id", &pet.to_string()), "{row:?}");
    assert!(row.has_field("pet_id", &pet.to_string()), "{row:?}");
}

/// A pet's kill logs `pet_kill_credited` at DEBUG with the pet, the owner,
/// the owner's identity, the victim and the XP maths.
#[tokio::test]
async fn a_pet_kill_logs_pet_kill_credited() {
    let (mut mgr, pet, mob) = world();
    let capture = LogCapture::install();
    let _ = kill(&mut mgr, mob, pet).await;

    let row = only_event(&capture.all(), "pets.credit", "pet_kill_credited");
    assert_eq!(row.level, Level::DEBUG);
    assert_pet(&row, pet);
    assert!(row.has_field("victim_id", &mob.to_string()), "{row:?}");
    assert!(row.has_field("xp_granted", &MOB_XP.to_string()), "{row:?}");
    assert!(row.has_field("transfer_xp", "1.0"), "{row:?}");
    assert_owner_identity(&row);
}

/// Seam: a mob kills a pet. DEBUG, `reason = npc_killed_pet`, carrying the
/// dead pet's owner so the owner's support question is answerable.
#[tokio::test]
async fn a_mob_killing_a_pet_logs_npc_killed_pet() {
    let (mut mgr, pet, mob) = world();
    let capture = LogCapture::install();
    let _ = kill(&mut mgr, pet, mob).await;

    let row = only_event(&capture.all(), "pets.credit", "kill_xp_not_granted");
    assert_eq!(row.level, Level::DEBUG);
    assert!(row.has_field("reason", "npc_killed_pet"), "{row:?}");
    assert_pet(&row, pet);
    assert!(row.has_field("attacker", &mob.to_string()), "{row:?}");
    assert_owner_identity(&row);
}

/// Seam: an ordinary NPC kills an ordinary NPC. DEBUG,
/// `reason = npc_attacker`, on the module target (no pet, no owner).
#[tokio::test]
async fn an_npc_killing_an_npc_logs_npc_attacker() {
    let (mut mgr, _pet, mob) = world();
    let other = spawn_mob(&mut mgr);
    let capture = LogCapture::install();
    let _ = kill(&mut mgr, other, mob).await;

    let logs = capture.all();
    let rows: Vec<_> = logs
        .iter()
        .filter(|c| c.has_field("event", "kill_xp_not_granted"))
        .collect();
    assert_eq!(rows.len(), 1, "{logs:#?}");
    let row = rows[0];
    assert_eq!(row.level, Level::DEBUG);
    assert!(row.has_field("reason", "npc_attacker"), "{row:?}");
    assert_ne!(
        row.target, "pets.credit",
        "a plain NPC fight is not a pet row"
    );
}

/// Seam: the pet's `transfer_xp` is not a positive finite number. WARN (a
/// data fault no client can cause), `reason = transfer_xp_invalid`, with
/// the offending value.
#[tokio::test]
async fn a_bad_transfer_xp_logs_transfer_xp_invalid_at_warn() {
    for bad in [0.0_f32, -1.0, f32::NAN] {
        let (mut mgr, pet, mob) = world();
        mgr.get_entity_mut(pet)
            .unwrap()
            .pet
            .as_mut()
            .unwrap()
            .transfer_xp = bad;
        let capture = LogCapture::install();
        let _ = kill(&mut mgr, mob, pet).await;

        let row = only_event(&capture.all(), "pets.credit", "kill_xp_not_granted");
        assert_eq!(row.level, Level::WARN, "transfer_xp = {bad}");
        assert!(row.has_field("reason", "transfer_xp_invalid"), "{row:?}");
        assert!(row.has_field("transfer_xp", &format!("{bad:?}")), "{row:?}");
        assert_pet(&row, pet);
        assert_owner_identity(&row);
    }
}

/// Seam: the registry already dropped the pet (the teardown gap before the
/// entity goes). DEBUG, `reason = pet_unregistered`, still naming the
/// `owner_id` the pet's state points at, but no `account_id` / `player_id`:
/// `forget_pet` removed the summon-time capture, and the row never guesses.
#[tokio::test]
async fn an_unregistered_pet_kill_logs_pet_unregistered() {
    let (mut mgr, pet, mob) = world();
    mgr.pets.forget_pet(pet);
    let capture = LogCapture::install();
    let _ = kill(&mut mgr, mob, pet).await;

    let row = only_event(&capture.all(), "pets.credit", "kill_xp_not_granted");
    assert_eq!(row.level, Level::DEBUG);
    assert!(row.has_field("reason", "pet_unregistered"), "{row:?}");
    assert_pet(&row, pet);
    assert!(row.has_field("owner_id", &OWNER.to_string()), "{row:?}");
    assert!(!row.fields.contains_key("account_id"), "{row:?}");
    assert!(!row.fields.contains_key("player_id"), "{row:?}");
}

/// Seam: the owner's entity id now belongs to another player (id reuse
/// before the sweep). The kill pays nobody, and the only row is
/// `credit_recipient`'s `credit_refused` WARN: `grant_kill_xp` must not log
/// a second `kill_xp_not_granted` row for the same refusal. The row still
/// names the summoner, not the id's new holder.
#[tokio::test]
async fn a_refused_pet_kill_logs_only_credit_refused() {
    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID + 1);
    let capture = LogCapture::install();
    assert_eq!(kill(&mut mgr, mob, pet).await, vec![], "nobody is paid");

    let logs = capture.all();
    let refused: Vec<_> = logs
        .iter()
        .filter(|c| c.target == "pets.credit" && c.has_field("event", "credit_refused"))
        .collect();
    assert!(!refused.is_empty(), "{logs:#?}");
    let row = refused[0];
    assert_eq!(row.level, Level::WARN);
    assert!(
        row.has_field("reason", "owner_identity_mismatch"),
        "{row:?}"
    );
    assert_pet(row, pet);
    assert_owner_identity(row);
    assert!(
        !logs
            .iter()
            .any(|c| c.has_field("event", "kill_xp_not_granted")),
        "no duplicate row for a refusal credit_recipient already logged: {logs:#?}"
    );
}

/// Seam: the pet's payout rounds to zero (a tiny `transfer_xp`: 50 XP x
/// 0.001 rounds to 0). DEBUG (a template can author it; nothing is broken),
/// `reason = zero_xp`, on `pets.credit` with the pet, the owner and the
/// summoner's identity, and no `GrantXP`.
#[tokio::test]
async fn a_zero_xp_pet_kill_logs_zero_xp() {
    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_mut()
        .unwrap()
        .transfer_xp = 0.001;
    let capture = LogCapture::install();
    assert_eq!(kill(&mut mgr, mob, pet).await, vec![], "nothing is paid");

    let row = only_event(&capture.all(), "pets.credit", "kill_xp_not_granted");
    assert_eq!(row.level, Level::DEBUG);
    assert!(row.has_field("reason", "zero_xp"), "{row:?}");
    assert!(row.has_field("victim_id", &mob.to_string()), "{row:?}");
    assert!(row.has_field("base_xp", &MOB_XP.to_string()), "{row:?}");
    assert_pet(&row, pet);
    assert_owner_identity(&row);
}

/// **Guard (#889).** Seam: a finite but huge `transfer_xp` (`f32::MAX`)
/// would overflow the payout, and the `as u64` cast would saturate to
/// `u64::MAX` XP. The kill pays nothing and logs one WARN
/// (`reason = xp_overflow`, bad seed data) with the offending scale and the
/// same identity fields as `zero_xp`. Reverted (no range check before the
/// cast), a `GrantXP` of `u64::MAX` goes to the owner.
#[tokio::test]
async fn an_overflowing_transfer_xp_logs_xp_overflow_at_warn() {
    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_mut()
        .unwrap()
        .transfer_xp = f32::MAX;
    let capture = LogCapture::install();
    assert_eq!(kill(&mut mgr, mob, pet).await, vec![], "no GrantXP");

    let row = only_event(&capture.all(), "pets.credit", "kill_xp_not_granted");
    assert_eq!(row.level, Level::WARN);
    assert!(row.has_field("reason", "xp_overflow"), "{row:?}");
    // Recorded through `f64`, so compare the value, not the f32 spelling.
    assert_eq!(
        row.fields
            .get("transfer_xp")
            .and_then(|v| v.parse::<f64>().ok()),
        Some(f64::from(f32::MAX)),
        "{row:?}"
    );
    assert!(row.has_field("victim_id", &mob.to_string()), "{row:?}");
    assert!(row.has_field("base_xp", &MOB_XP.to_string()), "{row:?}");
    assert_pet(&row, pet);
    assert_owner_identity(&row);
}

/// **Guard (#889).** A payout that fits `u32` but not `i32` (50 XP x 6e7 =
/// 3e9) is also `xp_overflow`: `sgw_player.exp` and the wire payload are
/// `INT32`, so `MAX_KILL_XP` is `i32::MAX`. Reverted to a `u32::MAX` cap,
/// a 3e9 `GrantXP` goes to the owner and the base clamps it silently.
#[tokio::test]
async fn a_payout_past_i32_max_logs_xp_overflow() {
    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_mut()
        .unwrap()
        .transfer_xp = 6.0e7;
    let capture = LogCapture::install();
    assert_eq!(kill(&mut mgr, mob, pet).await, vec![], "no GrantXP");
    let row = only_event(&capture.all(), "pets.credit", "kill_xp_not_granted");
    assert_eq!(row.level, Level::WARN);
    assert!(row.has_field("reason", "xp_overflow"), "{row:?}");
    assert!(
        row.has_field("max_kill_xp", &(i32::MAX as u64).to_string()),
        "{row:?}"
    );
}

/// **Guard (#889).** The registry dropped the pet AND the owner's entity id
/// now belongs to another player. The `pet_unregistered` row keeps the pet
/// and the `owner_id`, but logs no `account_id` / `player_id`: the
/// summon-time capture is gone, and the id's current holder is not the
/// player who summoned the pet. Reverted (fall back to the live holder of
/// `owner_id`), the row names the impostor's ids 4242 / 4243.
#[tokio::test]
async fn an_unregistered_pet_kill_after_owner_id_reuse_logs_no_identity() {
    let (mut mgr, pet, mob) = world();
    mgr.pets.forget_pet(pet);
    {
        let owner = mgr.get_entity_mut(OWNER).unwrap();
        owner.account_id = Some(4242);
        owner.player_id = Some(4243);
    }
    let capture = LogCapture::install();
    let _ = kill(&mut mgr, mob, pet).await;

    let row = only_event(&capture.all(), "pets.credit", "kill_xp_not_granted");
    assert!(row.has_field("reason", "pet_unregistered"), "{row:?}");
    assert_pet(&row, pet);
    assert!(row.has_field("owner_id", &OWNER.to_string()), "{row:?}");
    assert!(!row.fields.contains_key("account_id"), "{row:?}");
    assert!(!row.fields.contains_key("player_id"), "{row:?}");
}

/// `victim_template_id` is recorded as its value, never as a `Some(..)`
/// debug string (#889): an `Option` field is the value when present and
/// omitted when `None`.
#[tokio::test]
async fn pet_kill_credited_records_victim_template_id_as_a_value() {
    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(mob).unwrap().template_id = Some(0x7000_0608);
    let capture = LogCapture::install();
    let _ = kill(&mut mgr, mob, pet).await;
    let row = only_event(&capture.all(), "pets.credit", "pet_kill_credited");
    assert!(
        row.has_field("victim_template_id", &0x7000_0608.to_string()),
        "{row:?}"
    );

    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(mob).unwrap().template_id = None;
    let capture = LogCapture::install();
    let _ = kill(&mut mgr, mob, pet).await;
    let row = only_event(&capture.all(), "pets.credit", "pet_kill_credited");
    assert!(!row.fields.contains_key("victim_template_id"), "{row:?}");
}
