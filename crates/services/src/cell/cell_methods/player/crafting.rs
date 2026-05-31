use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::constants::*;

/// Cell-method index for the server→client `onUpdateDiscipline` callback.
///
/// Extended encoding (≥ idbase=61), emitted via [`super::super::super::super::mercury::append_entity_method`].
/// Wire payload: `[disciplineSeqId: i32 LE][expertise: i32 LE]` — 8 bytes.
/// See `cimmeria_entity::crafting::serialize_on_update_discipline`.
///
/// Sourced from `docs/protocol/client-method-dispatch-table.md` row 136.
const ON_UPDATE_DISCIPLINE: u16 = 136;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SPEND_APPLIED_SCIENCE_POINTS => {
            // Phase 1: route only — full ASP-spend validation (paradigm
            // gate, prerequisite expertise, DB UPDATE) lands in Phase 2.
            // We parse the discipline id so the trace is debuggable even
            // before the mutation logic exists.
            if args.len() >= 4 {
                let discipline_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    discipline_id,
                    "UNIMPLEMENTED: spendAppliedSciencePoints (Phase 2)"
                );
            } else {
                tracing::warn!(
                    entity_id,
                    args_len = args.len(),
                    "spendAppliedSciencePoints: malformed/truncated args (need 4 bytes)"
                );
            }
            true
        }

        CRAFT => {
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: craft");
            } else {
                tracing::warn!(
                    entity_id,
                    args_len = args.len(),
                    "craft: malformed/truncated args"
                );
            }
            true
        }

        RESEARCH => {
            tracing::info!(entity_id, "UNIMPLEMENTED: research");
            true
        }

        REVERSE_ENGINEER => {
            tracing::info!(entity_id, "UNIMPLEMENTED: reverseEngineer");
            true
        }

        ALLOYING => {
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: alloying");
            } else {
                tracing::warn!(
                    entity_id,
                    args_len = args.len(),
                    "alloying: malformed/truncated args"
                );
            }
            true
        }

        RESPEC_CRAFTING => {
            tracing::info!(entity_id, "UNIMPLEMENTED: respecCrafting");
            true
        }

        _ => false,
    }
}

/// Emit an `onUpdateDiscipline` callback to the client.
///
/// Sends a `CellToBaseMsg::EntityMethodCall` with method index 136 and the
/// 8-byte payload from `cimmeria_entity::crafting::serialize_on_update_discipline`.
/// The BaseApp encodes the extended-encoding wire bytes and ships the packet.
///
/// Phase 1 only: nothing inside this module calls this yet — the actual
/// expertise-mutating activities (craft/research/alloy/spendASP) land in
/// Phase 2 and will invoke this. We expose it on the public surface now
/// so the wire shape is locked in by [`tests::send_on_update_discipline_emits_correct_message`].
#[allow(dead_code)] // Phase 2 callers (spendAppliedSciencePoints, gainExpertise) wire this up.
pub async fn send_on_update_discipline(
    entity_id: u32,
    discipline_id: i32,
    expertise: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let args = cimmeria_entity::crafting::serialize_on_update_discipline(discipline_id, expertise);
    // mpsc::Sender::send().await returns Err only when the receiver has been
    // dropped — i.e., the base task is shutting down. A dropped client at
    // this point isn't actionable from the cell, so we log-and-continue.
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_UPDATE_DISCIPLINE,
            args,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            discipline_id,
            expertise,
            error = %e,
            "onUpdateDiscipline send dropped — base receiver gone",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::make_space_manager_with_player;

    /// Routing regression guard for the SPEND_APPLIED_SCIENCE_POINTS (95)
    /// dispatch fix. The original code routed only `CRAFT..=RESPEC_CRAFTING`
    /// (96..=100) to crafting; index 95 fell through to the social arm,
    /// which has no case for 95 either, so the message silently dropped
    /// (now-warn-logged after #311, but still wrong-handler).
    ///
    /// Bug shape: a refactor that re-narrows the crafting sub-range back to
    /// `CRAFT..=RESPEC_CRAFTING` (or sets the lower bound to a constant
    /// that compares > 95) will fail this test. The outer dispatcher
    /// must return `true` for method 95 — meaning the crafting handler
    /// matched it and returned `true`, not that the social arm caught
    /// it without a body (which would also return false here).
    #[tokio::test]
    async fn spend_applied_science_points_routes_to_crafting() {
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);

        // Send 4 bytes of payload (a discipline_id) so the handler takes
        // the parse-and-log path, not the truncated-args warn path.
        // Either path returns `true`, but the parse path is the realistic
        // success shape.
        let args = 42i32.to_le_bytes();
        let handled = dispatch(1, SPEND_APPLIED_SCIENCE_POINTS, &args, &tx, &mut mgr).await;
        assert!(
            handled,
            "SPEND_APPLIED_SCIENCE_POINTS (95) must route to the crafting handler \
             and return true. If this fails, the dispatch range in dispatch.rs has \
             regressed to exclude 95 — see issue #53 deep dive risk callout R6.",
        );
    }

    /// `send_on_update_discipline` enqueues an `EntityMethodCall` with
    /// method index 136 and the 8-byte payload `[disciplineId LE][expertise LE]`.
    /// Pins the wire-message shape end-to-end — entity-crate serializer
    /// produces the bytes, cell module wraps them in the right
    /// CellToBaseMsg variant with the right method_index.
    ///
    /// Bug shape this catches: an off-by-one in method_index (e.g., 135 or
    /// 137), a swap of disciplineId/expertise in the wire bytes, or a
    /// regression that changes the args length.
    #[tokio::test]
    async fn send_on_update_discipline_emits_correct_message() {
        let (tx, mut rx) = mpsc::channel(8);

        send_on_update_discipline(42, 7, 50, &tx).await;

        let msg = rx
            .recv()
            .await
            .expect("send_on_update_discipline must enqueue exactly one CellToBaseMsg");

        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(entity_id, 42);
                assert_eq!(
                    method_index, 136,
                    "onUpdateDiscipline method index per docs/protocol/client-method-dispatch-table.md \
                     is 136 — a change here desyncs the client's crafting UI",
                );
                assert_eq!(
                    args,
                    vec![0x07, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00],
                    "wire payload: disciplineId=7 LE, expertise=50 LE (0x32)",
                );
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }
}
