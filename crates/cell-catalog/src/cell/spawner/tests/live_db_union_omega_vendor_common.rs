//! UO-25: the vendor checks every Omega buy list shares.
//!
//! Each W8 vendor seeds templates, lists and spawns, then calls these helpers
//! with its own ids. Three guards, written once:
//!
//! * `assert_vendor_template`: the template carries exactly one vendor bit
//!   (D-UO35) and its buy list, plus the shared sell, repair and recharge
//!   list (`SELL_LIST`);
//! * `assert_list`: every row is a real, sellable design in the list's own id
//!   block, priced by the D-UO29 rule, with heavy weapons only at tier 2 or
//!   higher (UO-06 Q4);
//! * `assert_spawns`: each spawn stands once, clear of every other Omega NPC
//!   and of every Omega respawner.
//!
//! Callers: UO-11, UO-12, UO-27..UO-30, UO-39, UO-40.

// The W8 modules use these; a helper unused today must not fail clippy.
#![allow(dead_code)]

use sqlx::{PgPool, Row};

use crate::cell::spawner::*;
use crate::test_support::require_db_or_skip;

pub(super) const OMEGA_WORLD: &str = "Omega_Site";
/// The shared sell, repair and recharge list of every vendor.
pub(super) const SELL_LIST: i32 = 2;
/// The three designs list 2 buys back; never stocked (D-UO29).
pub(super) const ARBITRAGE_DESIGNS: [i32; 3] = [21, 55, 3437];
/// Heavy weapons (UO-06 Q4): stocked only at tier 2 or higher. 4991-4996 are
/// the Human heavy designs (UO-21 survey).
pub(super) const HEAVY_WEAPONS: [i32; 12] = [
    4990, 4991, 4992, 4993, 4994, 4995, 4996, 6491, 6480, 5094, 6617, 4565,
];
/// Heavy-class names (UO-22), matched case-insensitively against `ListRow::name`.
/// Covers the Jaffa Rak'tel and Jaffa Cannon designs, whose ids are not listed.
pub(super) const HEAVY_NAMES: [&str; 3] = ["rak'tel", "jaffa cannon", "grenade launcher"];
/// `items.flags` bit CAN_BE_SOLD.
pub(super) const CAN_BE_SOLD: i32 = 1024;
pub(super) const HOSTILE_FACTION: i32 = 10;
/// Tier-1 consumable prices: Slappack 25, Antidote 50, Mark III Stimpack 100.
pub(super) const CONSUMABLE_T1: [i32; 3] = [25, 50, 100];

/// Minimum x/z gap between two Omega NPCs (matches the services guard).
const MIN_NPC_SPACING: f32 = 2.5;
/// Minimum x/z gap between an Omega NPC and any Omega respawner.
const MIN_RESPAWNER_CLEARANCE: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Price {
    Weapon,
    Armor,
    Crafting,
    Consumable,
}

/// One `item_list_items` row joined to its `items` design.
#[derive(Debug)]
pub(super) struct ListRow {
    pub item_id: i32,
    pub design_id: i32,
    pub quantity: i32,
    pub naquadah: i32,
    /// `None` when the design is not a real item.
    pub name: Option<String>,
    pub tier: Option<i32>,
    pub tech_comp: Option<i32>,
    pub flags: Option<i32>,
}

/// True when a design name belongs to a `HEAVY_NAMES` class.
fn is_heavy_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    HEAVY_NAMES.iter().any(|h| lower.contains(h))
}

/// D-UO29. `None` means "one of CONSUMABLE_T1" (a tier-1 consumable).
pub(super) fn expected_price(kind: Price, tier: i32, tc: i32) -> Option<i32> {
    match kind {
        Price::Weapon => Some(200 * tc),
        Price::Armor => Some(100 * tc),
        Price::Crafting => Some(25 * tc),
        Price::Consumable if tier == 1 => None,
        Price::Consumable => Some(25 * tc),
    }
}

/// Every row of `list`, joined to its design, ordered by `item_id`.
pub(super) async fn load_list(pool: &PgPool, list: i32) -> Vec<ListRow> {
    let rows = sqlx::query(
        "SELECT li.item_id, li.design_id, li.quantity, li.naquadah, i.name, i.tier, i.tech_comp, i.flags \
           FROM resources.item_list_items li \
           LEFT JOIN resources.items i ON i.item_id = li.design_id \
          WHERE li.item_list_id = $1 \
          ORDER BY li.item_id",
    )
    .bind(list)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| panic!("item list {list} query must succeed: {e}"));

    rows.iter()
        .map(|row| ListRow {
            item_id: row.get("item_id"),
            design_id: row.get("design_id"),
            quantity: row.get("quantity"),
            naquadah: row.get("naquadah"),
            name: row.get("name"),
            tier: row.get("tier"),
            tech_comp: row.get("tech_comp"),
            flags: row.get("flags"),
        })
        .collect()
}

/// Checks one vendor list against the shared rules. `tier` is the tier the
/// list is stocked for, or `None` for a list that spans tiers.
pub(super) fn assert_list(list: i32, rows: &[ListRow], kind: Price, tier: Option<i32>) {
    assert!(
        (1..=20).contains(&rows.len()),
        "list {list} holds 1 to 20 rows, got {}",
        rows.len()
    );
    let base = 638000 + (list - 63800) * 50;
    for row in rows {
        let id = row.item_id;
        assert!(
            (base..=base + 19).contains(&id),
            "list {list} item_id {id} must lie in {base}..={} (D-UO27)",
            base + 19
        );
        assert!(
            row.name.is_some(),
            "list {list} item {id}: design {} is not a real item",
            row.design_id
        );
        assert!(
            !ARBITRAGE_DESIGNS.contains(&row.design_id),
            "list {list} item {id}: design {} is an arbitrage design, never stocked (D-UO29)",
            row.design_id
        );
        assert!(
            (row.flags.unwrap_or(0) & CAN_BE_SOLD) != 0,
            "list {list} item {id}: design {} lacks CAN_BE_SOLD",
            row.design_id
        );
        let appearances = rows.iter().filter(|r| r.design_id == row.design_id).count();
        assert_eq!(
            appearances, 1,
            "list {list} item {id}: design {} appears once in the list",
            row.design_id
        );
        assert_eq!(row.quantity, 1, "list {list} item {id} quantity");

        let item_tier = row
            .tier
            .unwrap_or_else(|| panic!("list {list} item {id}: design has no tier"));
        if let Some(t) = tier {
            assert_eq!(item_tier, t, "list {list} item {id} tier");
        }
        let heavy = HEAVY_WEAPONS.contains(&row.design_id)
            || row.name.as_deref().is_some_and(is_heavy_name);
        if heavy {
            let t = tier.unwrap_or_else(|| {
                panic!(
                    "list {list} item {id}: heavy weapon {} needs a tier",
                    row.design_id
                )
            });
            assert!(
                t >= 2,
                "list {list} item {id}: heavy weapon {} is stocked only at tier 2 or higher, list tier {t}",
                row.design_id
            );
        }

        let tc = row
            .tech_comp
            .unwrap_or_else(|| panic!("list {list} item {id}: design has no tech_comp"));
        match expected_price(kind, item_tier, tc) {
            Some(p) => assert_eq!(
                row.naquadah, p,
                "list {list} item {id} price (D-UO29, {kind:?})"
            ),
            None => assert!(
                CONSUMABLE_T1.contains(&row.naquadah),
                "list {list} item {id} price {} must be a tier-1 consumable price",
                row.naquadah
            ),
        }
    }
}

/// One vendor template: a mob with exactly `bit` in `interaction_type`, a
/// non-hostile faction, its buy list `list`, the shared `SELL_LIST` for sell,
/// repair and recharge, no trainer list, no static sets, no interaction set,
/// and a shipped name `name_id`.
pub(super) async fn assert_vendor_template(
    pool: &PgPool,
    template: i32,
    bit: i64,
    name_id: i32,
    list: i32,
) {
    let row = sqlx::query(
        "SELECT t.class, t.faction, t.interaction_type::bigint AS interaction_type, \
                t.buy_item_list, t.sell_item_list, t.repair_item_list, t.recharge_item_list, \
                t.trainer_ability_list_id, t.name_id, \
                t.static_interaction_sets = '{}' AS no_static, \
                t.interaction_set_id IS NULL AS no_set, x.text \
           FROM resources.entity_templates t \
           LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
          WHERE t.template_id = $1",
    )
    .bind(template)
    .fetch_optional(pool)
    .await
    .expect("entity_templates query must succeed")
    .unwrap_or_else(|| panic!("template {template} must be seeded"));

    assert_eq!(
        row.get::<String, _>("class"),
        "mob",
        "template {template} class"
    );
    let faction: Option<i32> = row.get("faction");
    assert!(
        faction.is_some_and(|f| f != HOSTILE_FACTION),
        "template {template} must not be killable (faction {faction:?})"
    );
    assert_eq!(
        row.get::<i64, _>("interaction_type"),
        bit,
        "template {template} interaction_type is exactly bit {bit} (D-UO35)"
    );
    assert_eq!(
        row.get::<Option<i32>, _>("buy_item_list"),
        Some(list),
        "template {template} buy_item_list"
    );
    for column in ["sell_item_list", "repair_item_list", "recharge_item_list"] {
        assert_eq!(
            row.get::<Option<i32>, _>(column),
            Some(SELL_LIST),
            "template {template} {column}"
        );
    }
    assert_eq!(
        row.get::<Option<i32>, _>("trainer_ability_list_id"),
        None,
        "template {template} trainer list would open the trainer before the vendor arm"
    );
    assert_eq!(
        row.get::<Option<i32>, _>("name_id"),
        Some(name_id),
        "template {template} name_id"
    );
    let text: Option<String> = row.get("text");
    assert!(
        text.as_deref().is_some_and(|n| !n.trim().is_empty()),
        "name_id {name_id} of template {template} must resolve to non-empty text, got {text:?}"
    );
    assert!(
        row.get::<bool, _>("no_static"),
        "template {template} must have no static interaction sets"
    );
    assert!(
        row.get::<bool, _>("no_set"),
        "template {template} must have no interaction_set_id"
    );
}

/// Each `(spawn, template, tag)` stands once in `OMEGA_WORLD`: the one record
/// with that spawn id, holding position, not respawning and not
/// aggression-overridden. It stands at least `MIN_NPC_SPACING` from every
/// other Omega NPC and at least `MIN_RESPAWNER_CLEARANCE` from every Omega
/// respawner.
pub(super) async fn assert_spawns(pool: &PgPool, spawns: &[(i32, i32, &str)]) {
    let records = load_spawns_from_db(pool)
        .await
        .expect("load_spawns_from_db must succeed");
    let respawners = load_respawners(pool)
        .await
        .expect("load_respawners must succeed");

    for &(spawn, template, tag) in spawns {
        let matching: Vec<&SpawnRecord> = records.iter().filter(|r| r.spawn_id == spawn).collect();
        assert_eq!(matching.len(), 1, "spawn {spawn} is seeded exactly once");
        let s = matching[0];
        assert_eq!(s.template_id, template, "spawn {spawn} template");
        assert_eq!(s.tag.as_deref(), Some(tag), "spawn {spawn} tag");
        assert_eq!(s.world_name, OMEGA_WORLD, "spawn {spawn} world");
        assert!(s.is_stationary, "spawn {spawn} holds position");
        assert_eq!(s.respawn_secs, None, "spawn {spawn} does not respawn");
        assert_eq!(
            s.aggression_override, None,
            "spawn {spawn} is not aggression-overridden"
        );

        for other in records
            .iter()
            .filter(|r| r.spawn_id != spawn && r.world_name == OMEGA_WORLD)
        {
            let d = xz_distance(s.x, s.z, other.x, other.z);
            assert!(
                d >= MIN_NPC_SPACING,
                "spawn {spawn} is {d:.2} units from spawn {} ({:?})",
                other.spawn_id,
                other.tag
            );
        }
        for r in respawners.iter().filter(|r| r.world_name == OMEGA_WORLD) {
            let d = xz_distance(s.x, s.z, r.pos[0], r.pos[2]);
            assert!(
                d >= MIN_RESPAWNER_CLEARANCE,
                "spawn {spawn} is {d:.2} units from respawner {} ({})",
                r.respawner_id,
                r.name
            );
        }
    }
}

fn xz_distance(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    ((ax - bx).powi(2) + (az - bz).powi(2)).sqrt()
}

mod tests {
    use super::*;

    /// D-UO29 by kind: weapons 200 per tech comp, armor 100, crafting and
    /// consumables 25; a tier-1 consumable has no fixed price.
    #[test]
    fn expected_price_follows_d_uo29() {
        assert_eq!(expected_price(Price::Weapon, 1, 5), Some(1000));
        assert_eq!(expected_price(Price::Armor, 2, 22), Some(2200));
        assert_eq!(expected_price(Price::Crafting, 3, 40), Some(1000));
        assert_eq!(expected_price(Price::Consumable, 1, 3), None);
        assert_eq!(expected_price(Price::Consumable, 2, 20), Some(500));
    }

    /// The helpers accept the rows the seeds already ship: the Debug Area
    /// munitions vendor (two bits, list 1300), the Omega banker, and list
    /// 63800, which must pass `assert_list` once UO-11 fills it (empty before
    /// then).
    #[tokio::test]
    async fn live_db_vendor_helpers_accept_known_rows() {
        let pool = require_db_or_skip!();
        assert_vendor_template(&pool, 1302, 49152, 27264, 1300).await;
        assert_spawns(&pool, &[(63820, 63810, "UnionOmega_Banker")]).await;
        let rows = load_list(&pool, 63800).await;
        if !rows.is_empty() {
            assert_list(63800, &rows, Price::Weapon, Some(1));
        }
    }

    fn list_row(design: i32, naquadah: i32) -> ListRow {
        ListRow {
            item_id: 638000,
            design_id: design,
            quantity: 1,
            naquadah,
            name: Some("x".to_string()),
            tier: Some(1),
            tech_comp: Some(2),
            flags: Some(CAN_BE_SOLD),
        }
    }

    #[test]
    fn assert_list_accepts_a_priced_row() {
        assert_list(63800, &[list_row(100, 400)], Price::Weapon, Some(1));
    }

    #[test]
    #[should_panic(expected = "price")]
    fn assert_list_rejects_a_wrong_price() {
        assert_list(63800, &[list_row(100, 401)], Price::Weapon, Some(1));
    }

    #[test]
    #[should_panic(expected = "arbitrage design")]
    fn assert_list_rejects_an_arbitrage_design() {
        assert_list(63800, &[list_row(21, 400)], Price::Weapon, Some(1));
    }

    #[test]
    #[should_panic(expected = "heavy weapon")]
    fn assert_list_rejects_tier_one_heavy() {
        assert_list(63800, &[list_row(4990, 400)], Price::Weapon, Some(1));
    }

    #[test]
    #[should_panic(expected = "heavy weapon")]
    fn assert_list_rejects_tier_one_named_heavy() {
        let mut row = list_row(4000, 400);
        row.name = Some("Rak'tel".to_string());
        assert_list(63800, &[row], Price::Weapon, Some(1));
    }
}
