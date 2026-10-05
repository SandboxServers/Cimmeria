//! Live-DB guards for the opt-in NPC tint (`entity_templates.send_tint`) on
//! Z10, the Debug Area's Visual NPC Lineup (DA-10, templates 1410-1599):
//!
//! * every lineup actor opts in, and its two colours and skin tint are its
//!   source template's (the one its tag names); a template-less actor's are
//!   all zero;
//! * no template outside the block opts in, so every other NPC still sends
//!   `onEntityTint(0, 0, 0)`;
//! * both loaders (`load_spawns_from_db` and the template prototypes behind a
//!   GM `.spawn`) hand the cell exactly those colours as wire `u32`s, the
//!   negative `bigint`s reinterpreted as two's complement, and `None` for
//!   every other template.
mod live_db {
    use cimmeria_entity::cell_entity::EntityTint;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const TEMPLATES: (i32, i32) = (1410, 1599);
    const SPAWNS: (i32, i32) = (13870, 14099);
    const ACTORS: usize = 161;
    /// Lineup actors whose source has a non-zero colour (of the 115
    /// templates outside the block that carry one, grouped by look).
    const COLOURED_ACTORS: usize = 70;

    /// Each lineup actor, its `send_tint`, its colours, and its source's
    /// colours (NULL for a `NoTemplate_` actor). The source is the template
    /// the spawn's tag names: `DebugArea_VisualLineup_<id>`.
    const ACTOR_COLOURS: &str = "\
        SELECT c.template_id, c.send_tint, \
               c.primary_color_id, c.secondary_color_id, c.skin_tint, \
               src.primary_color_id, src.secondary_color_id, src.skin_tint \
        FROM resources.entity_templates c \
        JOIN resources.spawnlist s ON s.template_id = c.template_id \
        LEFT JOIN resources.entity_templates src \
          ON s.tag ~ '^DebugArea_VisualLineup_[0-9]+$' \
         AND src.template_id = substring(s.tag FROM '[0-9]+$')::int \
        WHERE c.template_id BETWEEN $1 AND $2 \
        ORDER BY 1";

    type ActorRow = (
        i32,
        bool,
        i64,
        i64,
        i64,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    );

    /// Every lineup actor has `send_tint` on and carries its source's
    /// colours; a template-less actor's are zero. No template outside the
    /// block opts in. Revert proof: drop `, true` from a lineup row (or the
    /// `send_tint` column from the seed's INSERTs), change a clone's skin
    /// tint, or set `send_tint` on template 146, and this names it.
    #[tokio::test]
    async fn debug_area_lineup_tint_live_db_every_actor_opts_in_with_its_sources_colours() {
        let pool = require_db_or_skip!();
        let rows: Vec<ActorRow> = sqlx::query_as(ACTOR_COLOURS)
            .bind(TEMPLATES.0)
            .bind(TEMPLATES.1)
            .fetch_all(&pool)
            .await
            .expect("actor colour query must succeed");
        assert_eq!(rows.len(), ACTORS, "every lineup actor has a spawn");
        let mut errors = Vec::new();
        let mut coloured = 0;
        for (id, send, p, s, k, sp, ss, sk) in &rows {
            if !send {
                errors.push(format!("{id}: send_tint is off"));
            }
            let want = (sp.unwrap_or(0), ss.unwrap_or(0), sk.unwrap_or(0));
            if (*p, *s, *k) != want {
                errors.push(format!(
                    "{id}: colours {:?}, source has {want:?}",
                    (p, s, k)
                ));
            }
            if want != (0, 0, 0) {
                coloured += 1;
            }
        }
        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(coloured, COLOURED_ACTORS, "lineup actors with a colour");

        let opted: Vec<(i32, String)> = sqlx::query_as(
            "SELECT template_id, template_name FROM resources.entity_templates \
             WHERE send_tint AND template_id NOT BETWEEN $1 AND $2 ORDER BY 1",
        )
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .fetch_all(&pool)
        .await
        .expect("opt-in query must succeed");
        assert!(
            opted.is_empty(),
            "templates outside the lineup that send their tint (not yet \
             looked at in the lab): {opted:?}"
        );
    }

    /// `load_spawns_from_db` gives every lineup spawn its source's colours
    /// as wire `u32`s and every other spawn `None`. Revert proof: drop
    /// `t.send_tint` from the loader query (the load fails), return `None`
    /// from `decode_tint`, or ignore the flag, and this fails.
    #[tokio::test]
    async fn debug_area_lineup_tint_live_db_spawn_loader_carries_the_tint() {
        let pool = require_db_or_skip!();
        let want = wanted_tints(&pool).await;
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let mut errors = Vec::new();
        let mut lineup = 0;
        for r in &records {
            if (SPAWNS.0..=SPAWNS.1).contains(&r.spawn_id) {
                lineup += 1;
                let w = want.get(&r.template_id).copied();
                if r.tint != w {
                    errors.push(format!("spawn {}: {:?}, want {w:?}", r.spawn_id, r.tint));
                }
            } else if r.tint.is_some() {
                errors.push(format!(
                    "spawn {} outside the lineup: {:?}",
                    r.spawn_id, r.tint
                ));
            }
        }
        assert_eq!(lineup, ACTORS);
        assert!(errors.is_empty(), "{errors:#?}");
    }

    /// The template prototypes (the cell's template cache and a GM
    /// `.spawn`) carry the same tints. Revert proof: drop the colour
    /// columns from `entity_template_select!` and the load fails; ignore
    /// the flag in `decode_tint` and the opted-out templates fail.
    #[tokio::test]
    async fn debug_area_lineup_tint_live_db_template_prototypes_carry_the_tint() {
        let pool = require_db_or_skip!();
        let want = wanted_tints(&pool).await;
        let templates = load_spawn_templates(&pool)
            .await
            .expect("load_spawn_templates must succeed");
        let mut errors = Vec::new();
        for (id, r) in &templates {
            let w = if (TEMPLATES.0..=TEMPLATES.1).contains(id) {
                want.get(id).copied()
            } else {
                None
            };
            if r.tint != w {
                errors.push(format!("template {id}: {:?}, want {w:?}", r.tint));
            }
        }
        assert!(errors.is_empty(), "{errors:#?}");
        // A negative skin column goes out as its two's complement: template
        // 1426 copies template 146, skin -256076032.
        assert_eq!(
            templates[&1426].tint,
            Some(EntityTint {
                primary: 0xFFFF_0000,
                secondary: 0xFF00_0000,
                skin: 0xF0BC_9700,
            })
        );
    }

    /// Each lineup template's expected tint: the low 32 bits of its source
    /// template's columns, masked here independently of
    /// [`EntityTint::from_template_columns`].
    async fn wanted_tints(pool: &sqlx::PgPool) -> std::collections::HashMap<i32, EntityTint> {
        let rows: Vec<ActorRow> = sqlx::query_as(ACTOR_COLOURS)
            .bind(TEMPLATES.0)
            .bind(TEMPLATES.1)
            .fetch_all(pool)
            .await
            .expect("actor colour query must succeed");
        let low32 = |v: Option<i64>| (v.unwrap_or(0) & 0xFFFF_FFFF) as u32;
        rows.into_iter()
            .map(|(id, _, _, _, _, p, s, k)| {
                (
                    id,
                    EntityTint {
                        primary: low32(p),
                        secondary: low32(s),
                        skin: low32(k),
                    },
                )
            })
            .collect()
    }
}
