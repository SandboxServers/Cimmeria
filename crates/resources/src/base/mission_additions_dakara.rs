//! Dakara_E1 authored missions (docs/analysis/dakara-e1-rebuild/, D-DK1 F).

/// CIMMERIA_AUTHORING; NEW CONTENT. Mission 63100 "Home to Dakara": the
/// level-1 bridge a Free Jaffa gets on first arrival on Dakara_E1 (chains
/// 8075-8077), finished by talking to Rak'nor, which starts mission 1570.
/// The exact transform of `TEST_MISSION_XML` (1206 bytes) is pinned by
/// `tests/mission_additions_dakara.rs`; the seed rows that match it are in
/// `db/resources/Missions/Seed/missions_dakara_e1.sql`.
pub const DAKARA_HOME_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="true" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="1" CanAbandon="false" CanRepeatOnFail="true" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Home to Dakara" MissionID="63100"><HistoryText>Home to Dakara</HistoryText><Steps StepEnabled="false" StepID="63101" AwardXP="false" Difficulty="1"><StepDisplayLogText>Speak with Rak'nor at the Stargate.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="63102" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Rak'nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.</DisplayLogText><Tasks TaskType="1" TaskID="63103" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;
