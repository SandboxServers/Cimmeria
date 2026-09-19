//! Guards on the faction design rules the packet encodes.
//!
//! `faction` cannot be changed at runtime — `useAbility` rejects a player's
//! ability against any target whose faction is not `HOSTILE_FACTION`, and
//! right-click on an alive faction-10 NPC is rerouted from dialog to
//! auto-attack. That makes faction a structural choice at seed time rather than
//! a tunable, and it is why three Harset characters need two templates each.

use super::*;

/// **The talk-vs-kill template split.** Three Harset characters are spoken
/// to in one mission and killed in another, and `faction` cannot be changed
/// at runtime, so each needs two templates.
///
/// This asserts the *design rule*, not the data: for each pair, the talk row
/// must be non-hostile and the kill row must be faction 10, and the two must
/// agree on level (they are the same character). A future edit that flips
/// the talk row hostile silently converts a dialog NPC into an unspeakable
/// one — `interact` reroutes right-click on an alive faction-10 NPC to
/// auto-attack — and nothing else in the suite would notice.
#[tokio::test]
async fn talk_and_kill_template_pairs_have_opposite_factions_and_equal_levels() {
    let pool = require_db_or_skip!();

    // (talk template, kill template, character) — see the seed header.
    const PAIRS: [(i32, i32, &str); 3] = [
        (163, 221, "Petbe (742/1363 talk, 1245 kill)"),
        (216, 222, "Grogan (1580 step 4700 talk, step 4702 kill)"),
        (217, 223, "Dawson (741 talk, 1365 kill)"),
    ];

    // `HOSTILE_FACTION` is a `u8` on the runtime entity; the DB column is
    // `integer`. Widen once rather than casting at every comparison.
    let hostile = i32::from(HOSTILE_FACTION);

    for (talk_id, kill_id, who) in PAIRS {
        let (talk_faction, talk_level): (Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT faction, level FROM resources.entity_templates WHERE template_id = $1",
        )
        .bind(talk_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("talk template {talk_id} for {who} must exist: {e}"));

        let (kill_faction, kill_level): (Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT faction, level FROM resources.entity_templates WHERE template_id = $1",
        )
        .bind(kill_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("kill template {kill_id} for {who} must exist: {e}"));

        assert_ne!(
            talk_faction,
            Some(hostile),
            "{who}: talk template {talk_id} is faction {hostile}, so right-click on it \
             reroutes to auto-attack and the dialog beat is unreachable"
        );
        assert_eq!(
            kill_faction,
            Some(hostile),
            "{who}: kill template {kill_id} must be faction {hostile} or the player's \
             ability is rejected before the damage pipeline is entered"
        );
        assert_eq!(
            talk_level, kill_level,
            "{who}: templates {talk_id} and {kill_id} are the same character and must \
             agree on level (it drives max HP = 200 + 50*level)"
        );
    }
}

/// **H-B8, second half.** Template 163 (Petbe) shipped with NULL level,
/// alignment, faction and name_id.
///
/// `faction` is pinned to 1, not 10, on purpose: the shared-hub Petbe
/// carries mission 742's dialog binding and is the tag target of 1363
/// Prudence, whose spec test M-10 requires that tagging him must not aggro.
/// Faction cannot be changed at runtime — `useAbility` rejects a player
/// ability against any target whose faction is not `HOSTILE_FACTION`, and
/// right-click on an alive faction-10 NPC is rerouted to auto-attack — so
/// mission 1245's hostile ambush uses template 221 instead. Flipping 163 to
/// 10 would make Petbe unspeakable and break 742.
#[tokio::test]
async fn petbe_template_163_has_faction_level_and_alignment() {
    let pool = require_db_or_skip!();

    let (level, alignment, faction, name_id): (Option<i32>, Option<i32>, Option<i32>, Option<i32>) =
        sqlx::query_as(
            "SELECT level, alignment, faction, name_id \
         FROM resources.entity_templates WHERE template_id = 163",
        )
        .fetch_one(&pool)
        .await
        .expect("template 163 (Petbe) must exist");

    assert_eq!(level, Some(42), "Petbe's level (mission 1245 is L42)");
    assert_eq!(alignment, Some(0), "Petbe's alignment");
    assert_eq!(
        faction,
        Some(1),
        "Petbe must stay non-hostile — template 221 is the hostile clone for mission 1245"
    );
    assert_eq!(
        name_id,
        Some(7586),
        "Petbe needs a name_id or onNameIdUpdate is never sent and he renders unnamed"
    );

    // The hostile clone must exist and must actually be hostile, or 1245
    // has no ambush target.
    let (hostile_faction, hostile_set): (Option<i32>, Option<i32>) = sqlx::query_as(
        "SELECT faction, ability_set_id FROM resources.entity_templates WHERE template_id = 221",
    )
    .fetch_one(&pool)
    .await
    .expect("template 221 (Petbe, hostile) must exist");

    assert_eq!(hostile_faction, Some(10), "template 221 must be hostile");
    assert!(
        hostile_set.is_some(),
        "template 221 needs an ability set — it is the one Petbe that fights"
    );
}

/// **H-B8 for the eight plaza guards and four lieutenants.** Templates 159
/// and 160 must resolve to the staff auto-attack, not the pistol fallback.
///
/// `respawn_secs` on these two must stay **NULL**, and that is asserted
/// rather than left unsaid. Both templates are shared with Castle world 8
/// (spawns 119, 121, 123, 124), and the runtime resolves
/// `COALESCE(spawnlist.respawn_secs, entity_templates.respawn_secs)` — so a
/// template default here would silently opt Castle's guards into respawning,
/// which is a Castle-ledger decision. Harset packet H13 puts
/// `respawn_secs = 30` on the Harset *spawn* rows instead. A future packet
/// that "helpfully" adds a template default trips this test.
#[tokio::test]
async fn praxis_jaffa_guard_templates_use_the_staff_ability_set() {
    let pool = require_db_or_skip!();

    for template_id in [159_i32, 160] {
        let (ability_id, respawn_secs): (Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT asa.ability_id, t.respawn_secs \
             FROM resources.entity_templates t \
             LEFT JOIN resources.ability_set_abilities asa \
                    ON asa.ability_set_id = t.ability_set_id \
             WHERE t.template_id = $1",
        )
        .bind(template_id)
        .fetch_one(&pool)
        .await
        .expect("Praxis Jaffa guard template must exist");

        assert_eq!(
            ability_id,
            Some(STAFF_AUTO_ATTACK),
            "template {template_id} must resolve to ability {STAFF_AUTO_ATTACK} (Staff Auto \
             Attack); a NULL means it is back on NPC_DEFAULT_ABILITY \
             ({NPC_DEFAULT_ABILITY}, Pistol Shot) and the Jaffa fire a Tau'ri pistol again"
        );
        assert_eq!(
            respawn_secs, None,
            "template {template_id} is shared with Castle world 8, so a template-level \
             respawn_secs would opt Castle's guards into respawning through the \
             COALESCE(spawn, template) resolution. Harset's value belongs on the H13 \
             spawn rows, not here."
        );
    }
}
