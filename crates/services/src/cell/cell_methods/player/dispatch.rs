use super::constants::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        CALL_FOR_AID..=RESET_MY_ABILITIES => {
            super::combat::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await
        }
        WHO..=INITIAL_RESPONSE => {
            super::interaction::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await
        }
        TRAIN_ABILITY..=RECHARGE_ITEMS => {
            super::vendor::dispatch(entity_id, method_index, args, tx, space_mgr).await
        }
        PET_INVOKE_ABILITY..=PET_CHANGE_STANCE => {
            super::social::dispatch(entity_id, method_index, args, tx, space_mgr).await
        }
        SET_AUTO_CYCLE..=UPDATE_SYSTEM_OPTIONS => {
            super::world::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await
        }
        ORG_CREATION..=CANCEL_MOVIE => {
            // The outer arm already pins method_index into [ORG_CREATION,
            // CANCEL_MOVIE]. Only the [CRAFT, RESPEC_CRAFTING] sub-range
            // routes to crafting; everything else in the outer range is
            // social. Implicit constant ordering:
            //   ORG_CREATION ≤ SPEND_APPLIED_SCIENCE_POINTS < CRAFT
            //   ≤ RESPEC_CRAFTING ≤ CANCEL_MOVIE
            if (CRAFT..=RESPEC_CRAFTING).contains(&method_index) {
                super::crafting::dispatch(entity_id, method_index, args, tx, space_mgr).await
            } else {
                super::social::dispatch(entity_id, method_index, args, tx, space_mgr).await
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the SGWPlayer cell-method constant values. The dispatch routing in
    /// `dispatch` above uses `..=` ranges over these constants — if a future
    /// renumber shifts a constant, the range arms silently start covering
    /// different methods. A failure here forces the renumber to be deliberate.
    #[test]
    fn cell_method_constants_pin_expected_values() {
        // Combat: 67..=72
        assert_eq!(CALL_FOR_AID, 67);
        assert_eq!(RESET_MY_ABILITIES, 72);
        // Interaction: 73..=76
        assert_eq!(WHO, 73);
        assert_eq!(INITIAL_RESPONSE, 76);
        // Vendor: 77..=82
        assert_eq!(TRAIN_ABILITY, 77);
        assert_eq!(RECHARGE_ITEMS, 82);
        // World outer: 83..=93 — contains the pet sub-range
        assert_eq!(SET_AUTO_CYCLE, 83);
        assert_eq!(UPDATE_SYSTEM_OPTIONS, 93);
        // Pet sub-range: 88..=90 (lives inside the world outer range)
        assert_eq!(PET_INVOKE_ABILITY, 88);
        assert_eq!(PET_CHANGE_STANCE, 90);
        // Outer 94..=108 — contains the crafting sub-range
        assert_eq!(ORG_CREATION, 94);
        assert_eq!(CANCEL_MOVIE, 108);
        // Crafting sub-range: 96..=100
        assert_eq!(CRAFT, 96);
        assert_eq!(RESPEC_CRAFTING, 100);
    }

    /// The pet sub-range (88..=90) is fully inside the world outer range
    /// (83..=93). Routing pet methods correctly to social depends on the
    /// pet match arm being checked *before* the world arm in `dispatch`.
    /// If that order regresses, world::dispatch (which has no case for
    /// 88..=90) returns false, and so does the outer dispatch.
    #[tokio::test]
    async fn pet_methods_route_to_social_not_world() {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        for &pet_method in &[PET_INVOKE_ABILITY, PET_ABILITY_TOGGLE, PET_CHANGE_STANCE] {
            let handled = dispatch(1, pet_method, &[], &tx, &mut mgr, &engine).await;
            assert!(
                handled,
                "method {pet_method} (pet) must route to social and return true; \
                 a false here means the arm order regressed: world::dispatch \
                 (which has no case for 88..=90) was reached first and \
                 returned false because nothing in its match handled it",
            );
        }
    }

    /// One method per outer routing arm — proves each range is wired up to a
    /// sub-dispatcher that handles at least its first method. A regression
    /// that broke an arm (typo'd range, wrong sub-dispatcher) would show up
    /// as `dispatch` returning false for one of these.
    #[tokio::test]
    async fn each_outer_range_routes_to_a_handler() {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel(64);
        let engine = ChainEngine::new();

        // (method_index, label) pairs — one per outer range. Each module
        // handles its own first method as a stub that returns true regardless
        // of args, so empty args is enough to probe routing.
        for &(method, label) in &[
            (CALL_FOR_AID, "combat"),
            (WHO, "interaction"),
            (TRAIN_ABILITY, "vendor"),
            (SET_AUTO_CYCLE, "world (low half)"),
            (PET_INVOKE_ABILITY, "social/pet"),
            (CRAFT, "crafting"),
            (CLIENT_CHALLENGE_RESPONSE, "social (high half)"),
        ] {
            let handled = dispatch(1, method, &[], &tx, &mut mgr, &engine).await;
            assert!(
                handled,
                "{label} arm must route method {method} and return true"
            );
        }
    }

    #[tokio::test]
    async fn out_of_range_methods_return_false() {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        // Below CALL_FOR_AID (67) and above CANCEL_MOVIE (108) are outside
        // every routing range. Both must surface as unhandled.
        for &method in &[0u16, 1, 50, 66, 109, 200, u16::MAX] {
            let handled = dispatch(1, method, &[], &tx, &mut mgr, &engine).await;
            assert!(
                !handled,
                "method {method} is outside all routing ranges and must return false",
            );
        }
    }
}
