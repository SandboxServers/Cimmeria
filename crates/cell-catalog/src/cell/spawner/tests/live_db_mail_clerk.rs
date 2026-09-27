//! Live-DB guards for the debug hub's Gate Mail Clerk (social-systems
//! SS-U3): template 390 and dialog 60104 (`docs/content/debug-hub.md`).
//! Spawn 490's placement is guarded with the rest of the hub in
//! [`super::live_db_debug_hub`].
//!
//! The seams, each of which loads without error and is still wrong:
//!
//! * a clerk without a cursor bit: the client never sends the click, so
//!   chain 7010 never fires;
//! * a clerk with a trainer list, a vendor list or loot: the built-in
//!   interaction path answers the click first, and the chain never runs;
//! * a dialog whose button is not on its final screen, or that has no
//!   button: the player cannot ask for the mail;
//! * a dialog whose screens are all speaker 0 (a monologue), which makes
//!   `display_dialog` bind the player as the speaker.
//!
//! The seed rows for dialog 60104 stay while its cooked-data override is
//! quarantined (client map-load crash, 2026-09-27; see
//! `QUARANTINED_DIALOG_OVERRIDES` in `cimmeria-resources`).
mod live_db {
    use sqlx::Row;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const MAIL_CLERK: i32 = 390;
    const MAIL_CLERK_DIALOG: i32 = 60104;
    /// `INT_NonAStoryMissionAvaliable`, the hub dialog NPC's cursor bit.
    const INT_NON_A_STORY_MISSION: i64 = 134_217_728;
    /// `DN_npc_int_Harriman_SGCW1` ('Sgt. Harriman'), shipped in the client.
    const NAME_ID: i32 = 26715;
    /// Speaker 'Sgt. Harriman'.
    const SPEAKER: i32 = 843;

    /// Template 390 is a talk-cursor NPC with a shipped name and speaker, and
    /// nothing that routes its click anywhere but chain 7010.
    #[tokio::test]
    async fn mail_clerk_template_carries_its_role_fields() {
        let pool = require_db_or_skip!();
        let row = sqlx::query(
            "SELECT t.class, t.faction, t.interaction_type, t.name_id, t.speaker_id, \
                    t.buy_item_list, t.sell_item_list, t.repair_item_list, \
                    t.recharge_item_list, t.trainer_ability_list_id, t.loot_table_id, \
                    t.ability_set_id, x.text \
               FROM resources.entity_templates t \
               LEFT JOIN resources.texts x ON x.moniker_id = t.name_id AND x.language = 1033 \
              WHERE t.template_id = $1",
        )
        .bind(MAIL_CLERK)
        .fetch_optional(&pool)
        .await
        .expect("entity_templates query")
        .expect("template 390 must be seeded");

        let got = (
            row.get::<String, _>("class"),
            row.get::<Option<i32>, _>("faction"),
            row.get::<i64, _>("interaction_type"),
            row.get::<Option<i32>, _>("name_id"),
            row.get::<Option<i32>, _>("speaker_id"),
            [
                row.get::<Option<i32>, _>("buy_item_list"),
                row.get::<Option<i32>, _>("sell_item_list"),
                row.get::<Option<i32>, _>("repair_item_list"),
                row.get::<Option<i32>, _>("recharge_item_list"),
            ],
            row.get::<Option<i32>, _>("trainer_ability_list_id"),
            row.get::<Option<i32>, _>("loot_table_id"),
            row.get::<Option<i32>, _>("ability_set_id"),
        );
        assert_eq!(
            got,
            (
                "mob".to_string(),
                Some(1),
                INT_NON_A_STORY_MISSION,
                Some(NAME_ID),
                Some(SPEAKER),
                [None; 4],
                None,
                None,
                None,
            ),
            "template 390 role columns"
        );
        assert_eq!(
            row.get::<Option<String>, _>("text").as_deref(),
            Some("Sgt. Harriman"),
            "name_id {NAME_ID} must resolve to the shipped name"
        );

        let trainers = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        assert!(
            !trainers.contains_key(&MAIL_CLERK),
            "the trainer check runs first and would swallow the clerk's click"
        );
    }

    /// Dialog 60104: one screen spoken by the clerk, carrying one Generic 1
    /// button (type 4, ButtonID 8), and not a monologue.
    #[tokio::test]
    async fn mail_clerk_dialog_has_one_screen_and_one_button() {
        let pool = require_db_or_skip!();
        let rows: Vec<(i32, i32, Option<i32>, Option<i32>, Option<i32>)> = sqlx::query_as(
            "SELECT s.screen_id, s.index, s.speaker_id, b.button_id, b.button_type \
               FROM resources.dialog_screens s \
               LEFT JOIN resources.dialog_screen_buttons b ON b.screen_id = s.screen_id \
              WHERE s.dialog_id = $1 ORDER BY s.index, b.button_id",
        )
        .bind(MAIL_CLERK_DIALOG)
        .fetch_all(&pool)
        .await
        .expect("dialog screens query");
        assert_eq!(
            rows,
            vec![(200005, 0, Some(SPEAKER), Some(8), Some(4))],
            "60104: one clerk screen with one Generic 1 button"
        );

        let monologues = load_monologue_dialog_ids(&pool)
            .await
            .expect("load_monologue_dialog_ids must succeed");
        assert!(
            !monologues.contains(&MAIL_CLERK_DIALOG),
            "dialog {MAIL_CLERK_DIALOG} is spoken by the clerk, not the player"
        );
    }
}
