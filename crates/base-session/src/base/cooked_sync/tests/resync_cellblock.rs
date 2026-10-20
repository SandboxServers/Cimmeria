//! The Remaining Pods (CB-N05) cooked-sync transfer: a stock client that
//! asks for the authored entries receives exactly the frozen bytes.

use cimmeria_resources::base::resources::ResourceCache;

use super::{decode, rig, server_entries, server_version, ClientModel, Sent};
use crate::mercury::{FRAG_FIRST, FRAG_FIRST_AND_LAST, FRAG_LAST};

/// Mission 62050 category-3 XML, copied from w3-frozen.md (F1): 1678 bytes,
/// two fragments.
const EXPECTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="The Remaining Pods" MissionID="62050"><HistoryText>Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.</HistoryText><Steps StepEnabled="false" StepID="62051" AwardXP="false" Difficulty="1"><StepDisplayLogText>Ask Prisoner 329 about the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62052" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak to Prisoner 329.</DisplayLogText><Tasks TaskType="1" TaskID="62053" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="62054" AwardXP="false" Difficulty="1"><StepDisplayLogText>Tell Col. Marsh there may be prisoners in the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62055" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Warn Col. Marsh.</DisplayLogText><Tasks TaskType="1" TaskID="62056" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// Dialog 60150 emitted bytes, copied from w3-frozen.md (F3): 305 bytes,
/// one fragment.
const EXPECTED_DIALOG: &str = r#"<?xml version="1.0" encoding="UTF-8"?><COOKED_DIALOG DialogFlags="0" DialogID="60150" KismetEventSetID="0" UIScreenType="2"><Screens SpeakerID="0" ScreenID="96110" Text="Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave."></Screens></COOKED_DIALOG>"#;

/// A stock client asks for category 3 (the shipped table at its metadata):
/// the server resyncs it, and mission 62050 arrives as one two-fragment
/// transfer whose reassembled bytes are the frozen XML.
#[tokio::test]
async fn remaining_pods_mission_reaches_a_stock_client() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
    let raw = ResourceCache::load_raw_mission_category_for_test(dir).unwrap();
    let stock_version = raw.metadata;
    let mut client = ClientModel::holding(3, stock_version, raw.elements.into_iter().collect());
    let rig = rig(47_108);
    let served = server_version(3);
    assert_ne!(served, stock_version, "the catalog bump is in the cache");

    rig.request(3, stock_version).await;
    rig.pump_until_idle().await;
    let plaintexts = rig.take_plaintexts();
    let sent: Vec<Sent> = plaintexts.iter().map(|pt| decode(pt)).collect();

    let heads: Vec<usize> = sent
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            matches!(
                s,
                Sent::Fragment {
                    head: Some((3, 62050)),
                    ..
                }
            )
        })
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        heads.len(),
        1,
        "mission 62050 must be transferred exactly once"
    );
    let Sent::Fragment {
        data_id,
        flags: head_flags,
        ..
    } = &sent[heads[0]]
    else {
        unreachable!()
    };
    assert_eq!(
        *head_flags, FRAG_FIRST,
        "1678 bytes is two fragments: the head is FIRST, not FIRST_AND_LAST"
    );

    // Reassemble the transfer: every fragment carrying the head's data id,
    // from the head up to the one flagged LAST.
    let mut xml = Vec::new();
    for s in &sent[heads[0]..] {
        if let Sent::Fragment {
            data_id: id,
            flags,
            body,
            ..
        } = s
        {
            if id != data_id {
                continue;
            }
            xml.extend_from_slice(body);
            if *flags == FRAG_LAST {
                break;
            }
        }
    }
    assert!(
        xml == EXPECTED.as_bytes(),
        "reassembled mission 62050 differs from the frozen F1 bytes ({} vs {} bytes)",
        xml.len(),
        EXPECTED.len()
    );

    client.apply_all(&plaintexts);
    let held = &client.categories[&3];
    assert_eq!(held.entries[&62050], EXPECTED.as_bytes());
    assert_eq!(held.version, served, "category 3: version");
    assert_eq!(
        held.required_updates, 0,
        "category 3: RequiredUpdates drained"
    );
}

/// A stale client with category 5 missing dialog 60150 (version mismatch):
/// the dialog arrives as one fragment whose body is the frozen F3 bytes.
#[tokio::test]
async fn remaining_pods_dialog_reaches_a_stock_client() {
    let rig = rig(47_109);
    let served = server_version(5);
    let stale_version = served.wrapping_add(7);
    let mut stale = server_entries(5);
    assert!(
        stale.remove(&60150).is_some(),
        "60150 is in the served category 5"
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
                    head: Some((5, 60150)),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        heads.len(),
        1,
        "dialog 60150 must be transferred exactly once"
    );
    let Sent::Fragment { flags, body, .. } = heads[0] else {
        unreachable!()
    };
    assert_eq!(*flags, FRAG_FIRST_AND_LAST, "305 bytes is one fragment");
    assert!(
        body.as_slice() == EXPECTED_DIALOG.as_bytes(),
        "dialog 60150 body differs from the frozen F3 bytes"
    );

    client.apply_all(&plaintexts);
    let held = &client.categories[&5];
    assert_eq!(held.entries[&60150], EXPECTED_DIALOG.as_bytes());
    assert_eq!(held.version, served, "category 5: version");
    assert_eq!(
        held.required_updates, 0,
        "category 5: RequiredUpdates drained"
    );
}
