//! The Before the Patrol Returns (CB-N06) cooked-sync transfer: a stock
//! client that asks for the authored mission and dialog receives exactly
//! the frozen bytes.

use cimmeria_resources::base::resources::ResourceCache;

use super::{decode, rig, server_entries, server_version, ClientModel, Sent};
use crate::mercury::FRAG_FIRST_AND_LAST;

/// Mission 62060 category-3 XML, copied from w5-frozen.md (F1): 1369 bytes,
/// one fragment.
const EXPECTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Before the Patrol Returns" MissionID="62060"><HistoryText>Two prisoners in the cell block are too weak to escape with you. Before the patrol returns there is time for one thing: help them hide, or jam the cell-door panel so the guards cannot seal the block.</HistoryText><Steps StepEnabled="false" StepID="62061" AwardXP="false" Difficulty="1"><StepDisplayLogText>Help the prisoners hide, or jam the cell-door panel.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62062" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Help the prisoners or jam the panel.</DisplayLogText><Tasks TaskType="1" TaskID="62063" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// Dialog 60151 emitted bytes, copied from w5-frozen.md (F3): 392 bytes,
/// one fragment.
const EXPECTED_DIALOG: &str = r#"<?xml version="1.0" encoding="UTF-8"?><COOKED_DIALOG DialogFlags="0" DialogID="60151" KismetEventSetID="0" UIScreenType="2"><Screens SpeakerID="0" ScreenID="96111" Text="The two prisoners are too weak to run, and the patrol is coming back. There is time for one thing: help them hide, or jam the cell-door panel by Prisoner 329 so the guards cannot seal the block."></Screens></COOKED_DIALOG>"#;

/// A stock client asks for category 3 (the shipped table at its metadata):
/// the server resyncs it, and mission 62060 arrives as one fragment whose
/// bytes are the frozen XML.
#[tokio::test]
async fn prisoner_choice_mission_reaches_a_stock_client() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
    let raw = ResourceCache::load_raw_mission_category_for_test(dir).unwrap();
    let stock_version = raw.metadata;
    let mut client = ClientModel::holding(3, stock_version, raw.elements.into_iter().collect());
    let rig = rig(47_110);
    let served = server_version(3);
    assert_ne!(served, stock_version, "the catalog bump is in the cache");

    rig.request(3, stock_version).await;
    rig.pump_until_idle().await;
    let plaintexts = rig.take_plaintexts();
    let sent: Vec<Sent> = plaintexts.iter().map(|pt| decode(pt)).collect();

    let heads: Vec<&Sent> = sent
        .iter()
        .filter(|s| {
            matches!(
                s,
                Sent::Fragment {
                    head: Some((3, 62060)),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        heads.len(),
        1,
        "mission 62060 must be transferred exactly once"
    );
    let Sent::Fragment { flags, body, .. } = heads[0] else {
        unreachable!()
    };
    assert_eq!(*flags, FRAG_FIRST_AND_LAST, "1369 bytes is one fragment");
    assert!(
        body.as_slice() == EXPECTED.as_bytes(),
        "mission 62060 body differs from the frozen F1 bytes"
    );

    client.apply_all(&plaintexts);
    let held = &client.categories[&3];
    assert_eq!(held.entries[&62060], EXPECTED.as_bytes());
    assert_eq!(held.version, served, "category 3: version");
    assert_eq!(
        held.required_updates, 0,
        "category 3: RequiredUpdates drained"
    );
}

/// A stale client with category 5 missing dialog 60151 (version mismatch):
/// the dialog arrives as one fragment whose body is the frozen F3 bytes.
#[tokio::test]
async fn prisoner_choice_dialog_reaches_a_stock_client() {
    let rig = rig(47_111);
    let served = server_version(5);
    let stale_version = served.wrapping_add(7);
    let mut stale = server_entries(5);
    assert!(
        stale.remove(&60151).is_some(),
        "60151 is in the served category 5"
    );
    let mut client = ClientModel::holding(5, stale_version, stale);

    rig.request(5, stale_version).await;
    rig.pump_until_idle().await;
    let plaintexts = rig.take_plaintexts();
    let sent: Vec<Sent> = plaintexts.iter().map(|pt| decode(pt)).collect();

    let heads: Vec<&Sent> = sent
        .iter()
        .filter(|s| {
            matches!(
                s,
                Sent::Fragment {
                    head: Some((5, 60151)),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        heads.len(),
        1,
        "dialog 60151 must be transferred exactly once"
    );
    let Sent::Fragment { flags, body, .. } = heads[0] else {
        unreachable!()
    };
    assert_eq!(*flags, FRAG_FIRST_AND_LAST, "392 bytes is one fragment");
    assert!(
        body.as_slice() == EXPECTED_DIALOG.as_bytes(),
        "dialog 60151 body differs from the frozen F3 bytes"
    );

    client.apply_all(&plaintexts);
    let held = &client.categories[&5];
    assert_eq!(held.entries[&60151], EXPECTED_DIALOG.as_bytes());
    assert_eq!(held.version, served, "category 5: version");
    assert_eq!(
        held.required_updates, 0,
        "category 5: RequiredUpdates drained"
    );
}
