//! Complete authored mission entries, admitted before any mission patches.
//! Production entries are added only by a mission packet that sealed its XML (CB-N05: 62050; CB-N06: 62060).

/// One complete authored category-3 entry, validated before batch application.
#[derive(Debug)]
pub struct MissionAddition {
    /// New root key (1–65535), equal to the XML's positive `MissionID`.
    pub mission_id: u32,
    /// Exact UTF-8 XML bytes; admission preserves declaration and child order.
    pub xml: &'static str,
}

/// CB-N05 *The Remaining Pods* (CIMMERIA_AUTHORING): the exact category-3
/// entry for mission 62050, 1678 bytes, frozen in
/// docs/analysis/cellblock-continuation/w3-frozen.md (F1).
pub const REMAINING_PODS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="The Remaining Pods" MissionID="62050"><HistoryText>Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.</HistoryText><Steps StepEnabled="false" StepID="62051" AwardXP="false" Difficulty="1"><StepDisplayLogText>Ask Prisoner 329 about the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62052" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak to Prisoner 329.</DisplayLogText><Tasks TaskType="1" TaskID="62053" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="62054" AwardXP="false" Difficulty="1"><StepDisplayLogText>Tell Col. Marsh there may be prisoners in the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62055" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Warn Col. Marsh.</DisplayLogText><Tasks TaskType="1" TaskID="62056" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// CB-N06 *Before the Patrol Returns* (CIMMERIA_AUTHORING): the exact
/// category-3 entry for mission 62060, 1369 bytes, frozen in
/// docs/analysis/cellblock-continuation/w5-frozen.md (F1).
pub const PRISONER_CHOICE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Before the Patrol Returns" MissionID="62060"><HistoryText>Two prisoners in the cell block are too weak to escape with you. Before the patrol returns there is time for one thing: help them hide, or jam the cell-door panel so the guards cannot seal the block.</HistoryText><Steps StepEnabled="false" StepID="62061" AwardXP="false" Difficulty="1"><StepDisplayLogText>Help the prisoners hide, or jam the cell-door panel.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62062" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Help the prisoners or jam the panel.</DisplayLogText><Tasks TaskType="1" TaskID="62063" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// Production authored entries: CB-N05's 62050 and CB-N06's 62060.
pub const MISSION_ADDITIONS: &[MissionAddition] = &[
    MissionAddition {
        mission_id: 62050,
        xml: REMAINING_PODS_XML,
    },
    MissionAddition {
        mission_id: 62060,
        xml: PRISONER_CHOICE_XML,
    },
];

/// Cooked-sync supports at most 256 fragments of 1390 bytes per entry.
/// Base-session tests cross-check this boundary against its transport constant.
pub const MAX_MISSION_XML_BYTES: usize = 355_840;

/// CIMMERIA_AUTHORING fixture: stock _1624 shape, never seeded or offered.
/// Exact bytes and transform are frozen in cb-n05a-seal.md (1139 bytes).
#[cfg(any(test, feature = "test-support"))]
pub const TEST_MISSION_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="true" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="1" CanAbandon="false" CanRepeatOnFail="true" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="CB-N05a catalog fixture" MissionID="61000"><HistoryText>CB-N05a catalog fixture</HistoryText><Steps StepEnabled="false" StepID="61001" AwardXP="false" Difficulty="1"><StepDisplayLogText>Verify the authored mission catalog.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="61002" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText> </DisplayLogText><Tasks TaskType="1" TaskID="61003" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// Test-only registry for the frozen fixture; never used by production loading.
#[cfg(any(test, feature = "test-support"))]
pub const TEST_MISSION_ADDITIONS: &[MissionAddition] = &[MissionAddition {
    mission_id: 61000,
    xml: TEST_MISSION_XML,
}];
