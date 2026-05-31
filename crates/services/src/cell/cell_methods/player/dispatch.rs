use super::constants::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

// Static guard: the crafting sub-range AND the trade sub-range must both
// sit inside the ORG_CREATION..=CANCEL_MOVIE outer arm, with crafting
// preceding trade (95..=100 < 104..=107). A constant renumber that breaks
// either invariant fails the build instead of silently routing methods
// to the wrong sub-dispatcher.
const _: () = assert!(
    ORG_CREATION <= SPEND_APPLIED_SCIENCE_POINTS
        && SPEND_APPLIED_SCIENCE_POINTS <= CRAFT
        && CRAFT <= RESPEC_CRAFTING
        && RESPEC_CRAFTING < TRADE_REQUEST
        && TRADE_REQUEST <= TRADE_REQUEST_CANCEL
        && TRADE_REQUEST_CANCEL <= TRADE_UPDATE_PROPOSAL
        && TRADE_UPDATE_PROPOSAL <= TRADE_LOCK_STATE
        && TRADE_LOCK_STATE <= CANCEL_MOVIE,
    "crafting + trade sub-ranges must satisfy \
     ORG_CREATION ≤ SPEND_APPLIED_SCIENCE_POINTS ≤ CRAFT ≤ RESPEC_CRAFTING < \
     TRADE_REQUEST ≤ TRADE_LOCK_STATE ≤ CANCEL_MOVIE"
);

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
            // The outer arm pins method_index into [ORG_CREATION,
            // CANCEL_MOVIE]. Two sub-ranges live inside it:
            //
            //   - crafting:    SPEND_APPLIED_SCIENCE_POINTS..=RESPEC_CRAFTING (95..=100)
            //   - trade:       TRADE_REQUEST..=TRADE_LOCK_STATE              (104..=107)
            //
            // Everything else in the outer range routes to social.
            // Ordering invariants are pinned by the static assert above —
            // a renumber that violates them fails the build.
            if (SPEND_APPLIED_SCIENCE_POINTS..=RESPEC_CRAFTING).contains(&method_index) {
                super::crafting::dispatch(entity_id, method_index, args, tx, space_mgr).await
            } else if (TRADE_REQUEST..=TRADE_LOCK_STATE).contains(&method_index) {
                super::trade::dispatch(entity_id, method_index, args, tx, space_mgr).await
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
    use crate::test_support::make_space_manager_with_player;

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
        // Crafting sub-range: 95..=100 — includes
        // SPEND_APPLIED_SCIENCE_POINTS (95) since it's the discipline
        // unlock entry point and must reach the crafting handler.
        assert_eq!(SPEND_APPLIED_SCIENCE_POINTS, 95);
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
        let mut mgr = make_space_manager_with_player(1);

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
        let mut mgr = make_space_manager_with_player(1);

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

    /// Regression guard for the outer-router fix that widened the crafting
    /// sub-range from `CRAFT..=RESPEC_CRAFTING` (96..=100) to
    /// `SPEND_APPLIED_SCIENCE_POINTS..=RESPEC_CRAFTING` (95..=100).
    ///
    /// Bug shape: the previous narrow range silently dropped index 95
    /// into the social arm of the outer dispatcher. Social's own
    /// `dispatch` *also* has a `SPEND_APPLIED_SCIENCE_POINTS` arm (a
    /// stub left from an earlier wiring attempt), so the bug is
    /// invisible to a bare `assert!(handled)`: both branches return
    /// `true`. The two arms emit distinguishable log messages —
    /// crafting tags its log with `"(Phase 2)"`, social does not —
    /// so we install `LogCapture` and assert the crafting variant
    /// fired.
    ///
    /// If the outer router is narrowed back to `CRAFT..=RESPEC_CRAFTING`,
    /// index 95 reaches the social arm; the `"(Phase 2)"` log never
    /// fires, and this test fails. This is the assertion the existing
    /// `spend_applied_science_points_routes_to_crafting` test in
    /// `crafting.rs` *intended* to make but cannot, because that test
    /// calls `crafting::dispatch` directly and bypasses the outer
    /// router entirely.
    #[tokio::test]
    async fn outer_dispatch_routes_spend_asp_to_crafting_not_social() {
        use crate::test_support::LogCapture;
        use tracing::Level;

        let capture = LogCapture::install();

        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        // Non-empty args so the parse-and-log path runs (rather than the
        // truncated-args warn path, which both arms emit at different
        // levels but with identical-shape strings).
        let args = 42i32.to_le_bytes();
        let handled = dispatch(
            1,
            SPEND_APPLIED_SCIENCE_POINTS,
            &args,
            &tx,
            &mut mgr,
            &engine,
        )
        .await;
        assert!(handled, "outer dispatch must handle method 95");

        // Crafting's stub uniquely tags its log with "(Phase 2)" — the
        // social-side stub at social.rs:66 emits the same UNIMPLEMENTED
        // prefix but without that suffix. Pinning the suffix is what
        // distinguishes "routed to crafting" from "routed to social".
        let crafting_event = capture.find_message(Level::INFO, "(Phase 2)");
        assert!(
            crafting_event.is_some(),
            "outer dispatch must route method 95 (SPEND_APPLIED_SCIENCE_POINTS) \
             to the crafting submodule. The expected log \
             'UNIMPLEMENTED: spendAppliedSciencePoints (Phase 2)' from \
             cell_methods/player/crafting.rs did not fire. \
             A passing `handled == true` is not sufficient because the \
             social submodule also has a SPEND_APPLIED_SCIENCE_POINTS arm \
             that returns true — the (Phase 2) suffix uniquely identifies \
             the crafting branch.\n\nCaptured events: {:#?}",
            capture.all()
        );
    }

    #[tokio::test]
    async fn out_of_range_methods_return_false() {
        let mut mgr = make_space_manager_with_player(1);

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
