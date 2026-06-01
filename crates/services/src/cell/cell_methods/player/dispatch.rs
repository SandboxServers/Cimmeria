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
    /// With the social-side `SPEND_APPLIED_SCIENCE_POINTS` shadow arm
    /// removed, no arm in the social submodule handles index 95 — so
    /// `assert!(handled)` against the outer dispatcher is now sufficient
    /// proof of correct routing. (Before the shadow was removed, both
    /// submodules' arms returned `true`, making this assertion ambiguous
    /// — that's the trap [`route_index_95_must_go_to_crafting_not_social`]
    /// guards against re-introducing.)
    #[tokio::test]
    async fn outer_dispatch_routes_spend_asp_to_crafting() {
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        // Non-empty args so the crafting handler takes the parse-and-log
        // path, not the truncated-args warn path. Either path returns
        // `true`, but the parse path is the realistic success shape. The
        // numeric payload itself is irrelevant — this test pins WHERE the
        // dispatch lands, not what the handler computes from the value.
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
        assert!(
            handled,
            "outer dispatch must route method 95 (SPEND_APPLIED_SCIENCE_POINTS) \
             to the crafting submodule and return true. A `false` here means \
             the crafting sub-range in `dispatch` regressed and 95 fell into \
             the social arm — which (after the shadow-arm removal) has no \
             case for 95 and returns false.",
        );
    }

    /// Shadow-arm regression guard. Asserts that the
    /// `SPEND_APPLIED_SCIENCE_POINTS` arm in
    /// `crates/services/src/cell/cell_methods/player/social.rs` stays
    /// deleted: dispatching index 95 must produce **exactly one** info-
    /// level log carrying the `spendAppliedSciencePoints` substring, and
    /// that log must be the crafting submodule's `"(Phase 2)"` variant.
    ///
    /// Bug shape this catches: if a future PR reintroduces the social
    /// shadow arm (e.g., by reverting this PR or by an unrelated copy-
    /// paste), both the crafting and social handlers fire for index 95
    /// — the count goes from 1 to 2 — and routing tests that assert only
    /// `handled == true` cannot distinguish "routed to the right place"
    /// from "routed to two places". That's the shadow-arm trap this
    /// guard exists to prevent.
    ///
    /// We pin BOTH the count (exactly one) and the suffix `(Phase 2)`
    /// because:
    /// 1. Count alone would miss a future shadow that adopts a different
    ///    log message but still returns `true`.
    /// 2. Suffix alone would miss the same crafting log firing twice
    ///    (unlikely, but cheap to guard).
    #[tokio::test]
    async fn route_index_95_must_go_to_crafting_not_social() {
        use crate::test_support::LogCapture;
        use tracing::Level;

        let capture = LogCapture::install();

        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        // Payload value is irrelevant — only the routing target matters here.
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

        // Count INFO events that mention spendAppliedSciencePoints —
        // the substring is shared by both submodules' historical log
        // shapes, so a re-emerged social shadow shows up as count == 2.
        let asp_logs: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.level == Level::INFO && c.message_contains("spendAppliedSciencePoints"))
            .collect();

        assert_eq!(
            asp_logs.len(),
            1,
            "expected exactly one `spendAppliedSciencePoints` log for method 95; \
             got {}. More than one log indicates the social-submodule shadow \
             arm at cell_methods/player/social.rs has been reintroduced. \
             All captured events: {:#?}",
            asp_logs.len(),
            capture.all(),
        );

        // The single log must be the crafting submodule's `(Phase 2)`
        // variant — proves it's the right handler.
        let only = &asp_logs[0];
        assert!(
            only.message_contains("(Phase 2)"),
            "the single spendAppliedSciencePoints log must be the crafting \
             handler's `(Phase 2)` variant — its message was {:?}. A log \
             without `(Phase 2)` is the social-submodule shape and means \
             index 95 is being routed to social instead of crafting.",
            only,
        );
    }

    /// Companion guard to [`route_index_95_must_go_to_crafting_not_social`].
    ///
    /// The outer-dispatcher test catches a *combined* regression (shadow
    /// arm restored AND outer router narrowed). It can't catch a lone
    /// shadow restoration on its own: with the outer router still
    /// correctly routing 95 to crafting, a re-added social arm never
    /// fires through `dispatch`, the captured-log count stays at 1, and
    /// the test happily passes.
    ///
    /// This test calls `social::dispatch` **directly** with method 95
    /// and asserts the social submodule does NOT claim it. If a future
    /// PR re-introduces the shadow arm at `social.rs:66`, this assertion
    /// catches it at the source — before it can pair with an outer-
    /// router regression to silently re-route real traffic.
    #[tokio::test]
    async fn social_submodule_must_not_handle_index_95() {
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);

        // Payload value is irrelevant — only the routing target matters here.
        let args = 42i32.to_le_bytes();
        // `super` inside this `mod tests` is `dispatch`; the sibling
        // `social` module lives one level up under `player`.
        let handled = crate::cell::cell_methods::player::social::dispatch(
            1,
            SPEND_APPLIED_SCIENCE_POINTS,
            &args,
            &tx,
            &mut mgr,
        )
        .await;

        assert!(
            !handled,
            "social::dispatch must NOT handle method 95 (SPEND_APPLIED_SCIENCE_POINTS). \
             A `true` here means a SPEND_APPLIED_SCIENCE_POINTS arm has been \
             re-added to crates/services/src/cell/cell_methods/player/social.rs \
             — that's the shadow-arm trap this regression guard exists to \
             catch. Index 95 belongs to the crafting submodule; the outer \
             dispatcher already routes it there. Delete the social-side arm.",
        );
    }

    /// Regression guard for the outer-router trade sub-range. The
    /// crafting sub-range guard (`outer_dispatch_routes_spend_asp_to_crafting_not_social`)
    /// pins the crafting branch; this pins the *trade* branch.
    ///
    /// Bug shape: if the `TRADE_REQUEST..=TRADE_LOCK_STATE` else-if arm
    /// regresses (typo'd range, wrong sub-dispatcher, or moved below the
    /// fallback `social` arm), tradeRequest would route to `social`
    /// instead. Social's dispatch does not know about trade methods —
    /// the request would unhandled-warn and the trade UI on the client
    /// would never open. Pinning the routing via `handled == true`
    /// alongside the absence of a "social fallback" log proves the
    /// trade arm fired.
    ///
    /// We send TRADE_REQUEST (104) with truncated args (4 bytes — enough
    /// to read the partner entity id, not enough for a full proposal)
    /// so the trade handler returns early with its own truncation warn.
    /// That warn is what proves the trade sub-dispatcher saw the call.
    ///
    /// Revert-verifier: deleting the trade else-if arm in the outer
    /// `dispatch` (so trade methods fall through to social) causes the
    /// "tradeRequest: malformed LocalTradeProposal" warn to never
    /// fire — the social sub-dispatcher doesn't emit any such warn.
    #[tokio::test]
    async fn outer_dispatch_routes_trade_request_to_trade_not_social() {
        use crate::test_support::LogCapture;
        use tracing::Level;

        let capture = LogCapture::install();

        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        // 4 bytes — partner entity id only; LocalTradeProposal parse
        // will fail with a malformed-proposal warn (proves we reached
        // the trade handler).
        let args = 2i32.to_le_bytes();
        let handled = dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr, &engine).await;
        assert!(
            handled,
            "outer dispatch must handle TRADE_REQUEST (104) — fall-through \
             to social or the unhandled arm would return false here"
        );

        let event = capture.find_message(Level::WARN, "tradeRequest: malformed LocalTradeProposal");
        assert!(
            event.is_some(),
            "outer dispatch must route TRADE_REQUEST to the trade submodule. \
             The expected warn 'tradeRequest: malformed LocalTradeProposal' \
             from cell_methods/player/trade/handlers.rs did not fire. \
             A passing `handled == true` alone is not sufficient because \
             a fall-through could land in social.\n\nCaptured events: {:#?}",
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
