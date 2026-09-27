//! `CraftingCatalog::from_rows`: component rows become alternative sets.

use super::super::*;

fn blueprint(id: i32) -> Blueprint {
    Blueprint {
        blueprint_id: id,
        discipline_id: Some(21),
        is_alloy: false,
        product_id: Some(5401),
        quantity: 1,
        requires_elementary_components: false,
        component_sets: Vec::new(),
    }
}

fn row(blueprint_id: i32, set_id: i32, item_id: i32, quantity: i32) -> ComponentRow {
    ComponentRow {
        blueprint_id,
        set_id,
        item_id,
        quantity,
    }
}

/// Rows arrive in any order; each set collects only its own rows, sets are
/// ordered by id and components by item id. A loader that merged the sets
/// into one list (treating alternatives as steps) fails here.
#[test]
fn component_rows_group_into_sorted_alternative_sets() {
    let catalog = CraftingCatalog::from_rows(
        [],
        [blueprint(412), blueprint(21)],
        [
            row(412, 2, 5256, 5),
            row(412, 1, 5254, 14),
            row(412, 4, 5254, 1),
            row(412, 2, 5254, 1),
            row(412, 3, 5405, 1),
            row(412, 3, 5254, 7),
            row(412, 4, 5188, 1),
            row(412, 4, 5366, 1),
        ],
        [],
    );

    let bp = catalog.blueprint(412).expect("blueprint 412");
    assert_eq!(
        bp.component_sets
            .iter()
            .map(|s| s.set_id)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    let set = |id| {
        bp.component_set(id)
            .unwrap()
            .components
            .iter()
            .map(|c| (c.item_id, c.quantity))
            .collect::<Vec<_>>()
    };
    assert_eq!(set(1), [(5254, 14)]);
    assert_eq!(set(2), [(5254, 1), (5256, 5)]);
    assert_eq!(set(3), [(5254, 7), (5405, 1)]);
    assert_eq!(set(4), [(5188, 1), (5254, 1), (5366, 1)]);
    assert_eq!(catalog.component_count(), 8);

    assert!(
        catalog.blueprint(21).unwrap().component_sets.is_empty(),
        "a blueprint no row feeds has no sets"
    );
}

#[test]
fn orphan_component_rows_are_dropped() {
    let catalog = CraftingCatalog::from_rows([], [blueprint(1)], [row(999, 1, 5254, 1)], []);
    assert_eq!(catalog.component_count(), 0);
    assert!(catalog.blueprint(999).is_none());
}
