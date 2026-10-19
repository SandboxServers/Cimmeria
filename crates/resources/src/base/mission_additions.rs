//! Complete authored mission entries, admitted before any mission patches.
//! Production entries are added here only once a content packet seals their XML.

/// One complete authored category-3 entry, validated before batch application.
#[derive(Debug)]
pub struct MissionAddition {
    /// New root key (1–65535), equal to the XML's positive `MissionID`.
    pub mission_id: u32,
    /// Exact UTF-8 XML bytes; admission preserves declaration and child order.
    pub xml: &'static str,
}

/// Production authored entries. Each XML lives in its zone's own file.
pub const MISSION_ADDITIONS: &[MissionAddition] = &[
    // Dakara_E1 rebuild, D-DK1 F (DK-10fs).
    MissionAddition {
        mission_id: 63100,
        xml: crate::base::mission_additions_dakara::DAKARA_HOME_XML,
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
