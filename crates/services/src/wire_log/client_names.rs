//! Method-name lookup for the wire-log capture surface.
//!
//! Two sides:
//!
//! * [`outbound_method_name`] — server→client method indices (157
//!   methods on SGWPlayer). The flat index encoding is per
//!   `docs/protocol/client-method-dispatch-table.md`: indices 0–60
//!   ship as direct `msg_id = 0x80 + index`; 61+ use the extended
//!   encoding (`msg_id = 0xBD`, sub-byte = `index - 61`). The wire-log
//!   captures at the structured-message layer where `method_index`
//!   is the *flat* index, so this table is keyed by flat index.
//!
//! * [`inbound_msg_name`] — client→server message IDs. Bundled
//!   system-message IDs (0x00–0x0D) get explicit names; entity-method
//!   calls (0xC0+) fall through to the cell-method-name lookup at
//!   `cell::dispatch::names::cell_method_name`, which already covers
//!   the full set.

/// Outbound (server → client) method name for the flat SGWPlayer
/// method index. Source: `docs/protocol/client-method-dispatch-table.md`.
/// Unknown indices return `"unknown"`.
pub fn outbound_method_name(method_index: u16) -> &'static str {
    match method_index {
        0 => "onStaticMeshNameUpdate",
        1 => "onSequence",
        2 => "onEntityMove",
        3 => "InteractionType",
        4 => "onEntityFlags",
        5 => "getInteractions",
        6 => "toggleInteractionDebugging",
        7 => "onEntityProperty",
        8 => "onVisible",
        9 => "onKismetEventSetUpdate",
        10 => "onEntityTint",
        11 => "onBeingNameIDUpdate",
        12 => "onTimerUpdate",
        13 => "onEffectUserData",
        14 => "onEffectResults",
        15 => "onLevelUpdate",
        16 => "onTargetUpdate",
        17 => "onBeingNameUpdate",
        18 => "onTopSpeedUpdate",
        19 => "onStateFieldUpdate",
        20 => "onStatUpdate",
        21 => "onStatBaseUpdate",
        22 => "onMeleeRangeUpdate",
        23 => "onArchetypeUpdate",
        24 => "onAlignmentUpdate",
        25 => "onFactionUpdate",
        26 => "BeingAppearance",
        27 => "onSystemCommunication",
        28 => "onPlayerCommunication",
        29 => "onLocalizedCommunication",
        30 => "onTellSent",
        31 => "onChatJoined",
        32 => "onChatLeft",
        33 => "onNickChanged",
        34 => "onOrganizationInvite",
        35 => "onOrganizationJoined",
        36 => "onOrganizationLeft",
        37 => "onMemberJoinedOrganization",
        38 => "onOrganizationRosterInfo",
        39 => "onMemberLeftOrganization",
        40 => "onMemberRankChangedOrganization",
        41 => "onStrikeTeamUpdate",
        42 => "onPvPOrganizationLeaveRequest",
        43 => "onOrganizationNameUpdate",
        44 => "onOrganizationExperienceUpdate",
        45 => "onOrganizationMOTDUpdate",
        46 => "onOrganizationNoteUpdate",
        47 => "onOrganizationOfficerNoteUpdate",
        48 => "onOrganizationCashUpdate",
        49 => "onOrganizationRankUpdate",
        50 => "onOrganizationRankNameUpdate",
        51 => "onSquadLootType",
        52 => "onStartMinigame",
        53 => "onStartMinigameDialog",
        54 => "onStartMinigameDialogClose",
        55 => "onEndMinigame",
        56 => "onSpectateList",
        57 => "onMinigameRegistrationPrompt",
        58 => "minigameRegistrationInfo",
        59 => "addOrUpdateMinigameHelper",
        60 => "removeMinigameHelper",
        61 => "minigameCallDisplay",
        62 => "minigameCallResult",
        63 => "minigameCallAbort",
        64 => "showMinigameContact",
        65 => "setupStargateInfo",
        66 => "updateStargateAddress",
        67 => "stargateRotationOverride",
        68 => "onStargatePassage",
        69 => "onBagInfo",
        70 => "onActiveSlotUpdate",
        71 => "onRemoveItem",
        72 => "onUpdateItem",
        73 => "onRefreshItem",
        74 => "onClearOrgVaultInventory",
        75 => "onCashChanged",
        76 => "onMailHeaderInfo",
        77 => "onMailHeaderRemove",
        78 => "onMailRead",
        79 => "sendMailResult",
        80 => "onMissionUpdate",
        81 => "onStepUpdate",
        82 => "onObjectiveUpdate",
        83 => "onTaskUpdate",
        84 => "offerSharedMission",
        85 => "onContactListUpdate",
        86 => "onContactListDelete",
        87 => "onContactListAddMembers",
        88 => "onContactListRemoveMembers",
        89 => "onContactListEvent",
        90 => "onBMOpen",
        91 => "onBMError",
        92 => "onBMAuctions",
        93 => "onBMAuctionRemove",
        94 => "onBMAuctionUpdate",
        95 => "onBMWatchedItemsUpdate",
        96 => "onVersionInfo",
        97 => "onCookedDataError",
        98 => "onBeginAidWait",
        99 => "onEndAidWait",
        100 => "onDHDReply",
        101 => "onKnownAbilitiesUpdate",
        102 => "onTimeofDay",
        103 => "onOverridePerfStatsRate",
        104 => "onInitialInteraction",
        105 => "onDialogDisplay",
        106 => "onVaultOpen",
        107 => "onTeamVaultOpen",
        108 => "onCommandVaultOpen",
        109 => "onStoreOpen",
        110 => "onStoreUpdate",
        111 => "onStoreClose",
        112 => "onCraftingRespecPrompt",
        113 => "onTrainerOpen",
        114 => "onLootDisplay",
        115 => "onPlayerDataLoaded",
        116 => "onPlayerTeleport",
        117 => "onClientMapLoad",
        118 => "giveAbility",
        119 => "giveXPForLevel",
        120 => "onDisplayDHD",
        121 => "onErrorCode",
        122 => "setupWorldParameters",
        123 => "onMapInfo",
        124 => "clearClientHintedGenericRegions",
        125 => "addClientHintedGenericRegion",
        126 => "onResetMapInfo",
        127 => "onMissionRewardsDisplay",
        128 => "onMissionOfferDisplay",
        129 => "stargateTriggerFailed",
        130 => "onExtraNameUpdate",
        131 => "onExpUpdate",
        132 => "onMaxExpUpdate",
        133 => "onRingTransporterList",
        134 => "onOrganizationCreationResult",
        135 => "launchOrganizationCreation",
        136 => "onUpdateDiscipline",
        137 => "onDisciplineRespec",
        138 => "onUpdateRacialParadigmLevel",
        139 => "onUpdateKnownCrafts",
        140 => "onUpdateCraftingOptions",
        141 => "onAbilityTreeInfo",
        142 => "onClientChallenge",
        143 => "onDuelChallenge",
        144 => "onTradeState",
        145 => "onTradeResults",
        146 => "onSpaceQueued",
        147 => "onSpaceQueueReady",
        148 => "onRemoteEntityCreate",
        149 => "onRemoteEntityMove",
        150 => "onRemoteEntityRemove",
        151 => "onDuelEntitiesSet",
        152 => "onDuelEntitiesRemove",
        153 => "onDuelEntitiesClear",
        154 => "onThreatenedMobsUpdate",
        155 => "onPlayMovie",
        156 => "onCancelMovie",
        _ => "unknown",
    }
}

/// Inbound (client → server) message name for the bundle msg_id byte.
///
/// System messages 0x00–0x0D have fixed wire formats per the
/// ClientMessageList table in `messages.cpp` (legacy reference);
/// entity-method calls land at 0xC0+ and delegate to the existing
/// cell-method-name lookup, which already covers the full set.
pub fn inbound_msg_name(msg_id: u8) -> &'static str {
    match msg_id {
        0x00 => "loginRequest",
        0x01 => "authenticate",
        0x02 => "avatarUpdateImplicit",
        0x03 => "avatarUpdateExplicit",
        0x04 => "avatarUpdwImplicit",
        0x05 => "avatarUpdwExplicit",
        0x06 => "loggedOff",
        0x07 => "requestEntityUpdate",
        0x08 => "enableEntities",
        0x09 => "viewportAck",
        0x0A => "createCellPlayer",
        0x0B => "restoreClientAck",
        0x0C => "disconnect",
        0x0D => "channelSetup",
        // Entity-method calls (0xC0+) delegate to the cell-method
        // table — that's the canonical naming the dispatch code
        // already uses for these.
        0xC0..=0xFF => crate::cell::dispatch::cell_method_name(msg_id as u16),
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the 157-method coverage. A regression that drops or
    /// renumbers a row in the dispatch table will trip this guard.
    #[test]
    fn outbound_method_table_covers_full_range() {
        // Indices 0..157 should all resolve to a non-"unknown" name.
        for idx in 0..157u16 {
            let name = outbound_method_name(idx);
            assert_ne!(
                name, "unknown",
                "client method index {idx} missing from table"
            );
        }
        // Index 157+ should fall through to "unknown".
        assert_eq!(outbound_method_name(157), "unknown");
        assert_eq!(outbound_method_name(999), "unknown");
    }

    #[test]
    fn outbound_method_names_load_bearing_examples() {
        assert_eq!(outbound_method_name(1), "onSequence");
        assert_eq!(outbound_method_name(26), "BeingAppearance");
        assert_eq!(outbound_method_name(105), "onDialogDisplay");
        assert_eq!(outbound_method_name(154), "onThreatenedMobsUpdate");
    }

    #[test]
    fn inbound_system_messages_named() {
        assert_eq!(inbound_msg_name(0x01), "authenticate");
        assert_eq!(inbound_msg_name(0x03), "avatarUpdateExplicit");
        assert_eq!(inbound_msg_name(0x08), "enableEntities");
        assert_eq!(inbound_msg_name(0x0C), "disconnect");
    }
}
