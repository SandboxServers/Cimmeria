//! Carter's lab as seeded, against the numbers read from the cooked SGC_W1
//! map (chunk `0000ffff`) for CS-05. The map is not in the repository, so
//! the numbers are pinned here and explained on spawn row 81 in
//! `db/resources/Worlds/Seed/spawnlist.sql`.
//!
//! These are placement guards: they fail when a later edit moves the desk
//! SMG off the desk, swaps its template for a pressable one, or turns Carter
//! to face a wall.

use sqlx::Row;

use crate::test_support::require_db_or_skip;

/// `resources.worlds.world_id` of SGC_W1.
const SGC_W1: i32 = 58;

/// The centre desk's top: a flat up-facing collision surface at y 2.38 over
/// this XZ footprint. The floor of the lab is y 1.28.
const DESK_X: (f32, f32) = (-50.5, -48.0);
const DESK_Z: (f32, f32) = (36.0, 37.5);
const DESK_TOP_Y: f32 = 2.38;

/// The lab's only doorway (the two `CartersLabDoors` doors), east of Carter.
const DOORWAY: (f32, f32) = (-25.0, 36.48);

/// **Guard: chain 3035's prop exists, on the desk.** One spawn row carries
/// the tag chains 3033, 3035 and 3037 address, in SGC_W1, resting on the
/// desk top (not inside it, not hovering), on the SMG mesh, and on a
/// template with no cursor bit of its own: `interaction_type` 0 is what
/// makes it scenery for anyone chain 3033 has not lit it for.
#[tokio::test]
async fn live_db_the_desk_smg_spawn_lies_on_carters_desk() {
    let pool = require_db_or_skip!();

    let rows = sqlx::query(
        "SELECT s.x, s.y, s.z, s.world_id, t.static_mesh, t.interaction_type, t.class::text AS class \
         FROM resources.spawnlist s \
         JOIN resources.entity_templates t ON t.template_id = s.template_id \
         WHERE s.tag = 'SGC_W1_CarterDeskSMG'",
    )
    .fetch_all(&pool)
    .await
    .expect("the spawnlist query must succeed");
    assert_eq!(rows.len(), 1, "exactly one spawn carries the desk SMG tag");
    let row = &rows[0];
    let (x, y, z): (f32, f32, f32) = (row.get("x"), row.get("y"), row.get("z"));

    assert_eq!(row.get::<i32, _>("world_id"), SGC_W1);
    assert!(
        DESK_X.0 < x && x < DESK_X.1 && DESK_Z.0 < z && z < DESK_Z.1,
        "the SMG at ({x}, {z}) must be inside the desk top's footprint",
    );
    assert!(
        (DESK_TOP_Y..DESK_TOP_Y + 0.1).contains(&y),
        "the SMG at y {y} must rest on the desk top (y {DESK_TOP_Y})",
    );
    assert_eq!(
        row.get::<Option<String>, _>("static_mesh").as_deref(),
        Some("WP-Human.WP_SMG_1A"),
        "the prop is the SMG mesh",
    );
    assert_eq!(
        row.get::<i64, _>("interaction_type"),
        0,
        "no cursor bit of its own: only chain 3033 makes it pressable",
    );
    assert_eq!(row.get::<String, _>("class"), "spawnable");
}

/// **Guard: Carter faces the doorway.** She stands inside her U-shaped lab
/// bench on the doorway's axis; her heading (BigWorld: `atan2(dx, dz)`) must
/// point at the lab's only door, the way a visitor comes in, within a few
/// degrees. The seeded value, 1.59985, was already right; this pins it.
#[tokio::test]
async fn live_db_carter_faces_her_lab_doorway() {
    let pool = require_db_or_skip!();

    let row = sqlx::query(
        "SELECT x, z, heading FROM resources.spawnlist \
         WHERE tag = 'SGC_W1_SamCarter' AND world_id = $1",
    )
    .bind(SGC_W1)
    .fetch_one(&pool)
    .await
    .expect("Carter's spawn row must exist");
    let (x, z, heading): (f32, f32, f32) = (row.get("x"), row.get("z"), row.get("heading"));

    let to_door = (DOORWAY.0 - x).atan2(DOORWAY.1 - z);
    let off = (heading - to_door).sin().atan2((heading - to_door).cos());
    assert!(
        off.abs() < 5.0_f32.to_radians(),
        "Carter's heading {heading} must face the doorway (bearing {to_door}); off by {off} rad",
    );
    assert!(
        x < DOORWAY.0 && x > DESK_X.1,
        "Carter at x {x} stands between the desks and the doorway",
    );
}
