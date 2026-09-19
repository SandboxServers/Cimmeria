//! Stargate travel handler for the CellService.
//!
//! Handles `onDialGate` cell method calls, validating the target stargate
//! address and initiating a world transition via the BaseApp.
//!
//! Stargate destinations are loaded from `resources.stargates` at startup
//! and cached in `SpaceManager.stargates`.
//!
//! Reference: `python/cell/SGWPlayer.py:onDialGate()` — validates target,
//! begins dialing sequence. `python/cell/GateTravel.py:stargatePassed()` —
//! calls `moveTo()` to transition the entity to the destination world.

use tokio::sync::mpsc;

use super::arrival::validate_gate_arrival;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Handler ──────────────────────────────────────────────────────────────────

/// Handle the `onDialGate` cell method call.
///
/// Validates the target stargate address, removes the entity from the current
/// space, and sends a `GateTravel` message to BaseApp to initiate the world
/// transition.
///
/// Reference: `python/cell/SGWPlayer.py:onDialGate()` — the Python version
/// starts a 4-second dial timer; we skip the timer and travel immediately
/// for simplicity.
#[tracing::instrument(
    name = "gate_travel.dial",
    level = "info",
    skip_all,
    fields(entity_id, target_address_id)
)]
pub async fn handle_dial_gate(
    entity_id: u32,
    target_address_id: i32,
    _source_address_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // target_address_id == -1 means cancel dialing (no-op for us)
    if target_address_id == -1 {
        tracing::debug!(entity_id, "onDialGate: cancel dial (no-op)");
        return;
    }

    // Address-book gate (CAT-O-01). `target_address_id` is a raw client
    // integer; without this, any client could dial any of the 28 seeded gates
    // and cross-world teleport itself into content it never unlocked.
    //
    // First thing after the cancel, deliberately, for two reasons. It is the
    // dial *request* gate — Castle CA10 arms a 4-second pending dial further
    // down this function, and the refusal has to land before anything is
    // armed, not after. And answering before the `stargates` lookup below
    // stops the pair of refusals being an existence oracle: an address that
    // does not exist and an address that is not yours now look identical to
    // a client probing the id space.
    //
    // 2009: `deprecated/python/cell/SGWPlayer.py:2060-2064`, which also
    // accepted `hiddenStargates`. Cimmeria has no hidden list — neither a
    // column nor a wire slot; `mercury::world_data::map_loaded` always
    // serialises an empty hidden array — so "known" is the whole address
    // book here.
    //
    // The check belongs here and *only* here: passage through an open
    // wormhole is transit, not a dial (2009 gates `onDialGate`, never
    // `GateTravel.stargatePassed`), so CA10's volume-entry route should
    // inherit this decision from the pending-dial record it already holds
    // rather than re-running it — otherwise a player cannot walk through a
    // gate somebody else opened.
    if !player_knows_stargate(entity_id, target_address_id, tx, space_mgr).await {
        return;
    }

    // Look up the destination stargate from the DB cache
    let gate = match space_mgr.stargates.get(&target_address_id) {
        Some(g) => g.clone(),
        None => {
            tracing::warn!(
                entity_id,
                target_address_id,
                "onDialGate: invalid stargate address"
            );
            return;
        }
    };

    // Validate the entity exists and get its current world
    let current_world = match space_mgr.get_entity_world_name(entity_id) {
        Some(w) => w,
        None => {
            tracing::warn!(entity_id, "onDialGate: entity not found");
            return;
        }
    };

    // Don't travel to the same world (Python also checks this implicitly)
    if gate.world_name == current_world {
        tracing::debug!(
            entity_id, target_address_id, world = %gate.world_name,
            "onDialGate: already in destination world"
        );
        return;
    }

    tracing::info!(
        entity_id, target_address_id,
        from = %current_world, to = %gate.world_name,
        "Gate travel: initiating world transition"
    );

    // Resolve a standable arrival before anything destructive happens. Prefers
    // the gate's authored `arrival_*` pin, falls back to the gate row, and
    // replaces either with the destination world's respawner when the
    // destination has a navmesh that rejects it — see `cell::arrival`.
    //
    // **This one line is the whole arrival contract.** Castle CA10 moves the
    // placement from here to the gate-volume entry path; carry this call with
    // it so the walk-through-the-event-horizon arrival gets the same
    // validation the dial arrival does.
    let arrival = validate_gate_arrival(space_mgr, &gate);

    // Stage D: world transition destroys the cell entity and re-creates it on
    // the destination world. Flush any pending bandolier ammo writes before
    // teardown — anything still in `bandolier_ammo_dirty` after this is lost
    // to the cross-world re-spawn.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            super::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx).await;
        }
    }

    // Tell BaseApp to perform the world transition (RESET_ENTITIES + new world
    // entry) BEFORE removing the entity locally. A closed base channel must
    // not leave the player destroyed cell-side with no transfer in flight —
    // that is an "un-spaced" player who can only recover by relogging. Same
    // ordering the native `gmGotoLocation` handler already uses.
    if let Err(e) = tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: gate.world_name.clone(),
            position: arrival.position,
            rotation: [0.0, 0.0, arrival.yaw],
            destination_ring_id: None,
            // Stargate travel resolves the destination by world name.
            destination_space_id: None,
        })
        .await
    {
        tracing::error!(
            entity_id, world = %gate.world_name, error = %e,
            "onDialGate: base channel closed — entity left in place, no transfer"
        );
        return;
    }

    // Cancel any open trade before the entity goes. `destroy_entity` doesn't
    // clean trade state, and this helper early-returns once `get_entity`
    // misses — so gating on it afterwards would be a silent no-op, leaving the
    // traveller's partner with a dangling `trade_partner_entity_id` and no
    // `onTradeResults(Cancelled)`. Both lifecycle arms call it for the same
    // reason; stargate travel is just as much a departure.
    super::cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx, space_mgr).await;

    // Remove entity from current space (CellService side)
    space_mgr.destroy_entity(entity_id);
}

// ── Address book ─────────────────────────────────────────────────────────────

/// `EErrorCodeSystem` value carried in `onErrorCode`'s `SystemID`.
///
/// `ERRORCODE_SYSTEM_Ability = 0` is the *only* token the enum ever defines
/// (`deprecated/entities-editor/editor/enumerations.xml:1219`), so every
/// `onErrorCode` in the game — including the existing out-of-range feedback
/// in `abilities::use_ability` — ships a 0 here.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// `EConditionHandlerFeedback::CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress`
/// (`deprecated/entities-editor/editor/enumerations.xml:1404`). The client
/// renders the feedback string for this code; it is the only one in the enum
/// that names the address book, and 2009's own free-text
/// `onError("Failed to dial: not a known stargate address")` has no Cimmeria
/// equivalent (`SGWPlayer.def` exposes `onErrorCode` and nothing else).
const FEEDBACK_ENTITY_DOES_NOT_HAVE_STARGATE_ADDRESS: u16 = 180;

/// Does this player's address book contain `target_address_id`?
///
/// On refusal, emits the client-visible `onErrorCode` (121) and returns
/// `false`; the caller must return without touching any state.
///
/// No GM exemption, deliberately. 2009 had none either, and a GM who needs an
/// address has `.gotolocation` (`cell_methods::gm::travel`, which builds its
/// own `GateTravel` and never passes through here) rather than the dial UI.
/// Adding one would put an `access_level` branch on the security gate for no
/// operational gain.
async fn player_knows_stargate(
    entity_id: u32,
    target_address_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let entity = match space_mgr.get_entity(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(
                entity_id,
                target_address_id,
                reason = "dial_entity_missing",
                "onDialGate: no cell entity for the caller — refusing the dial; \
                 the client gets no travel and no feedback"
            );
            return false;
        }
    };
    if entity.known_stargates.contains(&target_address_id) {
        return true;
    }

    // `known_count = 0` has two causes worth telling apart when triaging a
    // "my gate is greyed out" report: a character that genuinely holds no
    // addresses (the column defaults to `'{}'` and `base::character_create`
    // does not seed it), or a dial sent inside the window between
    // `CreateEntity` and `InitPlayerState`, where the entity exists but its
    // address book has not arrived. Both refuse, which is the right
    // direction; the count is in the fields so the log can distinguish them
    // from "holds addresses, just not this one".
    tracing::warn!(
        entity_id,
        player_id = entity.player_id,
        target_address_id,
        known_count = entity.known_stargates.len(),
        reason = "unknown_stargate_address",
        "onDialGate: address is not in the player's known list — refusing the dial; \
         the traveller stays put and the client is told why"
    );

    // onErrorCode(UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID).
    // `InstanceID` is 0, not the stargate id: the client keys the field on
    // `SystemID`, and system 0 is the ability subsystem — handing it a
    // stargate id there invites a lookup against an unrelated ability row.
    let mut args = Vec::with_capacity(7);
    args.push(ERRORCODE_SYSTEM_ABILITY);
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&FEEDBACK_ENTITY_DOES_NOT_HAVE_STARGATE_ADDRESS.to_le_bytes());
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: super::client_methods::player::ON_ERROR_CODE,
            args,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            target_address_id,
            reason = "error_code_send_failed",
            "onDialGate: refusal onErrorCode could not be enqueued ({e}) — the dial is \
             still refused, but the client gets no feedback and may look hung"
        );
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::spawner::StargateEntry;
    use super::*;

    fn make_manager_with_stargates() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" />
            <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        </Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" />
            <Space WorldName="Castle" />
        </Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Populate stargates cache (simulates DB load)
        mgr.stargates.insert(
            1,
            StargateEntry {
                world_name: "Agnos".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw: 0.0,
                address_origin: 15,
                arrival: None,
            },
        );
        mgr.stargates.insert(
            2,
            StargateEntry {
                world_name: "Castle".to_string(),
                x: 761.677,
                y: 63.466,
                z: 551.716,
                yaw: 2.152,
                address_origin: 18,
                arrival: None,
            },
        );
        mgr.stargates.insert(
            15,
            StargateEntry {
                world_name: "Agnos".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw: 0.0,
                address_origin: 15,
                arrival: None,
            },
        );
        // Same Castle destination, but with an authored arrival pin. Synthetic
        // — this packet seeds no arrival coordinate for any gate; Harset's is
        // pinned in-game during milestone M0.
        mgr.stargates.insert(
            99,
            StargateEntry {
                world_name: "Castle".to_string(),
                x: 761.677,
                y: 63.466,
                z: 551.716,
                yaw: 2.152,
                address_origin: 18,
                arrival: Some(([700.5, 60.25, 540.75], 1.0)),
            },
        );

        mgr
    }

    /// Every seeded address plus the two synthetic ones the fixture adds.
    /// Tests that expect a dial to *succeed* must grant the address first —
    /// `handle_dial_gate` refuses an address the player does not hold
    /// (CAT-O-01), and a `CellEntity` is born with an empty book.
    const ALL_FIXTURE_ADDRESSES: [i32; 6] = [1, 2, 15, 98, 99, 999];

    fn grant_addresses(mgr: &mut SpaceManager, entity_id: u32, ids: &[i32]) {
        mgr.get_entity_mut(entity_id)
            .expect("entity must exist before it can be granted addresses")
            .known_stargates = ids.to_vec();
    }

    /// 999 is in the player's address book but not in `stargates`, so this
    /// still exercises the "address does not exist" arm it always did — the
    /// address-book gate above it has been satisfied.
    #[tokio::test]
    async fn dial_gate_to_unknown_address_is_noop() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        grant_addresses(&mut mgr, 1, &ALL_FIXTURE_ADDRESSES);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 999, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
    }

    /// CAT-O-01: `target_address_id` arrives as a raw client integer. A
    /// player who does not hold the address must not travel, must stay in
    /// their space, and must be told why.
    ///
    /// Deleting the `player_knows_stargate` call from `handle_dial_gate`
    /// fails this on all three counts.
    #[tokio::test]
    async fn dial_gate_to_an_address_the_player_does_not_know_is_refused() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        // Holds an unrelated address — proves the check is per-address, not
        // "has any address at all".
        grant_addresses(&mut mgr, 1, &[15]);
        let space_before = mgr.get_entity_space_id(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        assert!(
            mgr.get_entity(1).is_some(),
            "a refused dial must not tear the traveller out of their space"
        );
        assert_eq!(mgr.get_entity_space_id(1), space_before);

        let mut saw_error = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::GateTravel { .. } => {
                    panic!("a refused dial must never enqueue a GateTravel")
                }
                CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index == crate::cell::client_methods::player::ON_ERROR_CODE =>
                {
                    saw_error = true
                }
                _ => {}
            }
        }
        assert!(saw_error, "the client must be told the dial was refused");
    }

    /// Byte-exact wire check on the refusal, against
    /// `entities/defs/SGWPlayer.def:1240-1244`:
    /// `UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID`, little-endian,
    /// seven bytes. `InstanceID` is 0 and not the stargate id: `SystemID = 0`
    /// is `ERRORCODE_SYSTEM_Ability`, the only token the enum defines, and
    /// the client reads `InstanceID` as an ability id under it.
    #[tokio::test]
    async fn the_refusal_emits_the_stargate_address_feedback_code() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        let args = loop {
            match rx.try_recv() {
                Ok(CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                }) if method_index == 121 => break args,
                Ok(_) => continue,
                Err(_) => panic!("expected an onErrorCode (121) for the refused dial"),
            }
        };
        assert_eq!(
            args,
            vec![0u8, 0, 0, 0, 0, 180, 0],
            "SystemID=0 (ERRORCODE_SYSTEM_Ability), InstanceID=0, \
             ErrorCodeID=180 (CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress)"
        );
    }

    /// The window between `CreateEntity` and `InitPlayerState`: the entity
    /// exists but its address book has not arrived. Refusing is correct —
    /// pinned here so a future "I dialled right after loading and it was
    /// refused" report is recognised as designed behaviour, not a
    /// regression.
    #[tokio::test]
    async fn a_dial_before_init_player_state_is_refused() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        assert!(
            mgr.get_entity(1).unwrap().known_stargates.is_empty(),
            "a freshly created cell entity starts with an empty address book"
        );

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        assert!(mgr.get_entity(1).is_some());
        while let Ok(msg) = rx.try_recv() {
            assert!(
                !matches!(msg, CellToBaseMsg::GateTravel { .. }),
                "no travel may happen before the address book has loaded"
            );
        }
    }

    #[tokio::test]
    async fn dial_gate_cancel_is_noop() {
        let mut mgr = make_manager_with_stargates();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, -1, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dial_gate_same_world_is_noop() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        grant_addresses(&mut mgr, 1, &ALL_FIXTURE_ADDRESSES);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 1, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
        assert!(mgr.get_entity(1).is_some());
    }

    /// A closed base channel must leave the traveller in place. Destroying
    /// the entity first and *then* discovering the send failed produces a
    /// player who is in no space with no transfer in flight — recoverable
    /// only by relogging.
    #[tokio::test]
    async fn dial_gate_with_closed_base_channel_leaves_the_entity_in_place() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        grant_addresses(&mut mgr, 1, &ALL_FIXTURE_ADDRESSES);
        let space_before = mgr.get_entity_space_id(1);

        let (tx, rx) = tokio::sync::mpsc::channel(16);
        drop(rx); // base side is gone

        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        assert!(
            mgr.get_entity(1).is_some(),
            "a failed GateTravel enqueue must not tear the traveller out of their space"
        );
        assert_eq!(
            mgr.get_entity_space_id(1),
            space_before,
            "the traveller must still be in their origin space"
        );
    }

    #[tokio::test]
    async fn dial_gate_valid_sends_gate_travel() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        grant_addresses(&mut mgr, 1, &ALL_FIXTURE_ADDRESSES);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        assert!(mgr.get_entity(1).is_none());

        let msg = rx.try_recv().expect("Expected GateTravel message");
        match msg {
            CellToBaseMsg::GateTravel {
                entity_id,
                target_world_name,
                position,
                ..
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(target_world_name, "Castle");
                assert!((position[0] - 761.677).abs() < 0.01);
            }
            _ => panic!("Expected GateTravel message, got {:?}", msg),
        }
    }

    /// An unpinned gate arrives on the gate row, facing the row's authored
    /// yaw — the pre-H01 behaviour, pinned here so the arrival change can't
    /// silently alter it for the 28 gates that have no pin.
    #[tokio::test]
    async fn dial_gate_without_an_arrival_pin_uses_the_gate_row() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        grant_addresses(&mut mgr, 1, &ALL_FIXTURE_ADDRESSES);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        let (position, rotation) = expect_gate_travel(&mut rx);
        assert!((position[0] - 761.677).abs() < 0.01);
        assert!((position[1] - 63.466).abs() < 0.01);
        assert!((position[2] - 551.716).abs() < 0.01);
        assert!((rotation[2] - 2.152).abs() < 0.01);
    }

    /// A pinned gate arrives on the pin, yaw included. Reverting
    /// `handle_dial_gate` to `[gate.x, gate.y, gate.z]` / `gate.yaw` — the
    /// pre-H01 line — fails this.
    #[tokio::test]
    async fn dial_gate_with_an_arrival_pin_uses_the_pin() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        grant_addresses(&mut mgr, 1, &ALL_FIXTURE_ADDRESSES);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 99, 0, &tx, &mut mgr).await;

        let (position, rotation) = expect_gate_travel(&mut rx);
        // Exact, not tolerance-based: these literals are copied verbatim
        // through the call chain with no arithmetic applied.
        assert_eq!(position, [700.5, 60.25, 540.75]);
        assert_eq!(rotation[2], 1.0);
        assert_ne!(
            position,
            [761.677, 63.466, 551.716],
            "the prefab-origin gate row must not win over an authored pin"
        );
    }

    /// End to end, on the wire the base actually receives: a gate whose
    /// arrival is off the destination world's navmesh must hand BaseApp the
    /// respawner position, not the authored one.
    ///
    /// `cell::arrival`'s own tests prove the *decision*; the two tests above
    /// prove the *pin plumbing*. Without this one the defect the packet
    /// exists to prevent — an off-mesh coordinate reaching
    /// `CellToBaseMsg::GateTravel` — is only covered in two disjoint halves.
    #[tokio::test]
    async fn dial_gate_to_an_off_mesh_arrival_sends_the_respawner_position() {
        use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
        use crate::cell::spawner::RespawnerDef;

        let Some(mesh) = test_fixture_mesh() else {
            return;
        };
        // Same fixture coordinates as `cell::arrival::tests`.
        const ON_MESH: [f32; 3] = [-289.465, 68.542, -154.276];
        const OFF_MESH: [f32; 3] = [-289.465, 268.542, -154.276];

        let mut mgr = make_manager_with_stargates();
        test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
        mgr.respawners.push(RespawnerDef {
            respawner_id: 1,
            world_name: "Castle_CellBlock".to_string(),
            name: "test".to_string(),
            pos: ON_MESH,
        });
        mgr.stargates.insert(
            98,
            StargateEntry {
                world_name: "Castle_CellBlock".to_string(),
                x: OFF_MESH[0],
                y: OFF_MESH[1],
                z: OFF_MESH[2],
                yaw: 1.75,
                address_origin: 18,
                arrival: None,
            },
        );
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        grant_addresses(&mut mgr, 1, &ALL_FIXTURE_ADDRESSES);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 98, 0, &tx, &mut mgr).await;

        let (position, rotation) = expect_gate_travel(&mut rx);
        assert_eq!(position, ON_MESH, "the off-mesh gate row must be replaced");
        assert_ne!(
            position, OFF_MESH,
            "an off-navmesh arrival must never reach the base — that is the \
             silent-freeze bug"
        );
        assert_eq!(
            rotation[2], 1.75,
            "a respawner fallback carries the authored facing through"
        );
    }

    fn expect_gate_travel(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> ([f32; 3], [f32; 3]) {
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::GateTravel {
                position, rotation, ..
            } = msg
            {
                return (position, rotation);
            }
        }
        panic!("Expected GateTravel message");
    }
}
