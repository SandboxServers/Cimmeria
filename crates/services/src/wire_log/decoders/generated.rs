//! Auto-generated wire-format decoders for SGWPlayer client methods.
//!
//! **DO NOT EDIT BY HAND** — regenerate via:
//!
//! ```sh
//! python tools/wire_decoder_codegen.py > \
//!     crates/services/src/wire_log/decoders/generated.rs
//! cargo fmt -p cimmeria-services
//! ```
//!
//! Source schemas: `docs/protocol/client-method-dispatch-table.md`.
//! Schemas using composite types (StatUpdateList, ClientEffectResultList,
//! NameValuePair, FIXED_DICTs, MAILBOX, etc.) are skipped here — they
//! fall through to the hand-written decoders in
//! [`crate::wire_log::decoders::outbound`], which override the
//! generated table when both define the same method.

// Local variable names mirror the protocol's PascalCase / camelCase
// field names so the codegen stays simple and the generated source
// reads alongside the dispatch table. Clippy / rustc would otherwise
// flag every binding.
#![allow(non_snake_case, clippy::needless_question_mark)]

use serde_json::{json, Value};

use super::primitives::Cursor;

/// Read an ARRAY<primitive> by reading a u32 count then `min(count,
/// 32)` repeats. Truncates at 32 to bound per-event size; `count` and
/// `truncated` fields preserve the original size for SigNoz queries.
macro_rules! read_array {
    ($cursor:expr, $read:expr) => {{
        let count = $cursor.u32_le()?;
        let cap = (count as usize).min(32);
        let mut items: Vec<Value> = Vec::with_capacity(cap);
        for _ in 0..cap {
            items.push(json!($read));
        }
        json!({
            "count": count,
            "items": items,
            "truncated": count > 32,
        })
    }};
}

/// Method 0: `onStaticMeshNameUpdate`. Auto-generated.
pub(super) fn decode_0(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let staticMeshName = c.wstring()?;
    let bodySetName = c.wstring()?;
    Some(json!({
        "StaticMeshName": staticMeshName,
        "BodySetName": bodySetName,
    }))
}

/// Method 2: `onEntityMove`. Auto-generated.
pub(super) fn decode_2(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let locationX = c.f32_le()?;
    let locationY = c.f32_le()?;
    let locationZ = c.f32_le()?;
    let velocityX = c.f32_le()?;
    let velocityY = c.f32_le()?;
    let velocityZ = c.f32_le()?;
    let yaw = c.f32_le()?;
    let pitch = c.f32_le()?;
    let roll = c.f32_le()?;
    Some(json!({
        "locationX": locationX,
        "locationY": locationY,
        "locationZ": locationZ,
        "velocityX": velocityX,
        "velocityY": velocityY,
        "velocityZ": velocityZ,
        "yaw": yaw,
        "pitch": pitch,
        "roll": roll,
    }))
}

/// Method 3: `InteractionType`. Auto-generated.
pub(super) fn decode_3(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let typeId = c.u64_le()?;
    Some(json!({
        "TypeId": typeId,
    }))
}

/// Method 4: `onEntityFlags`. Auto-generated.
pub(super) fn decode_4(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aFlags = c.u64_le()?;
    Some(json!({
        "aFlags": aFlags,
    }))
}

/// Method 6: `toggleInteractionDebugging`. Auto-generated.
pub(super) fn decode_6(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let playerId = c.i32_le()?;
    Some(json!({
        "playerId": playerId,
    }))
}

/// Method 7: `onEntityProperty`. Auto-generated.
pub(super) fn decode_7(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let type_ = c.i32_le()?;
    let value = c.i32_le()?;
    Some(json!({
        "type": type_,
        "value": value,
    }))
}

/// Method 8: `onVisible`. Auto-generated.
pub(super) fn decode_8(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let visible = c.i8()?;
    Some(json!({
        "visible": visible,
    }))
}

/// Method 9: `onKismetEventSetUpdate`. Auto-generated.
pub(super) fn decode_9(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let kismetEventSetId = c.i32_le()?;
    Some(json!({
        "kismetEventSetId": kismetEventSetId,
    }))
}

/// Method 10: `onEntityTint`. Auto-generated.
pub(super) fn decode_10(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let primaryColorId = c.u32_le()?;
    let secondaryColorId = c.u32_le()?;
    let skinColorId = c.u32_le()?;
    Some(json!({
        "primaryColorId": primaryColorId,
        "secondaryColorId": secondaryColorId,
        "skinColorId": skinColorId,
    }))
}

/// Method 11: `onBeingNameIDUpdate`. Auto-generated.
pub(super) fn decode_11(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let beingNameID = c.i32_le()?;
    Some(json!({
        "BeingNameID": beingNameID,
    }))
}

/// Method 12: `onTimerUpdate`. Auto-generated.
pub(super) fn decode_12(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let iD = c.i32_le()?;
    let type_ = c.i8()?;
    let sourceID = c.i32_le()?;
    let totalTime = c.f32_le()?;
    let bigWorldTimeComplete = c.f32_le()?;
    Some(json!({
        "ID": iD,
        "Type": type_,
        "SourceID": sourceID,
        "TotalTime": totalTime,
        "BigWorldTimeComplete": bigWorldTimeComplete,
    }))
}

/// Method 13: `onEffectUserData`. Auto-generated.
pub(super) fn decode_13(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let instanceId = c.i32_le()?;
    let userDataNames = read_array!(c, c.wstring()?);
    let userDataValues = read_array!(c, c.wstring()?);
    Some(json!({
        "InstanceId": instanceId,
        "UserDataNames": userDataNames,
        "UserDataValues": userDataValues,
    }))
}

/// Method 15: `onLevelUpdate`. Auto-generated.
pub(super) fn decode_15(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let level = c.i32_le()?;
    Some(json!({
        "Level": level,
    }))
}

/// Method 16: `onTargetUpdate`. Auto-generated.
pub(super) fn decode_16(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let targetId = c.i32_le()?;
    Some(json!({
        "TargetId": targetId,
    }))
}

/// Method 17: `onBeingNameUpdate`. Auto-generated.
pub(super) fn decode_17(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let beingName = c.wstring()?;
    Some(json!({
        "BeingName": beingName,
    }))
}

/// Method 18: `onTopSpeedUpdate`. Auto-generated.
pub(super) fn decode_18(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let topSpeed = c.f32_le()?;
    Some(json!({
        "TopSpeed": topSpeed,
    }))
}

/// Method 19: `onStateFieldUpdate`. Auto-generated.
pub(super) fn decode_19(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let bStateField = c.i32_le()?;
    Some(json!({
        "bStateField": bStateField,
    }))
}

/// Method 22: `onMeleeRangeUpdate`. Auto-generated.
pub(super) fn decode_22(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let range = c.i32_le()?;
    Some(json!({
        "range": range,
    }))
}

/// Method 23: `onArchetypeUpdate`. Auto-generated.
pub(super) fn decode_23(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let archetype = c.i32_le()?;
    Some(json!({
        "archetype": archetype,
    }))
}

/// Method 24: `onAlignmentUpdate`. Auto-generated.
pub(super) fn decode_24(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let alignment = c.i8()?;
    Some(json!({
        "alignment": alignment,
    }))
}

/// Method 25: `onFactionUpdate`. Auto-generated.
pub(super) fn decode_25(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let faction = c.i8()?;
    Some(json!({
        "faction": faction,
    }))
}

/// Method 26: `BeingAppearance`. Auto-generated.
pub(super) fn decode_26(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let bodySet = c.wstring()?;
    let components = read_array!(c, c.wstring()?);
    Some(json!({
        "BodySet": bodySet,
        "Components": components,
    }))
}

/// Method 28: `onPlayerCommunication`. Auto-generated.
pub(super) fn decode_28(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let speaker = c.wstring()?;
    let speakerFlags = c.u8()?;
    let channel = c.u8()?;
    let text = c.wstring()?;
    Some(json!({
        "Speaker": speaker,
        "SpeakerFlags": speakerFlags,
        "Channel": channel,
        "Text": text,
    }))
}

/// Method 30: `onTellSent`. Auto-generated.
pub(super) fn decode_30(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aTarget = c.wstring()?;
    let aText = c.wstring()?;
    Some(json!({
        "aTarget": aTarget,
        "aText": aText,
    }))
}

/// Method 31: `onChatJoined`. Auto-generated.
pub(super) fn decode_31(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let channelName = c.wstring()?;
    let channelID = c.u8()?;
    Some(json!({
        "ChannelName": channelName,
        "ChannelID": channelID,
    }))
}

/// Method 32: `onChatLeft`. Auto-generated.
pub(super) fn decode_32(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let channelName = c.wstring()?;
    Some(json!({
        "ChannelName": channelName,
    }))
}

/// Method 33: `onNickChanged`. Auto-generated.
pub(super) fn decode_33(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aPlayerName = c.wstring()?;
    let aPlayerNickname = c.wstring()?;
    let aAddRemoveFlag = c.u8()?;
    Some(json!({
        "aPlayerName": aPlayerName,
        "aPlayerNickname": aPlayerNickname,
        "aAddRemoveFlag": aAddRemoveFlag,
    }))
}

/// Method 34: `onOrganizationInvite`. Auto-generated.
pub(super) fn decode_34(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aInviterName = c.wstring()?;
    let aOrganizationType = c.u8()?;
    let aRequestID = c.i32_le()?;
    let aName = c.wstring()?;
    let aIsStrikeTeam = c.u8()?;
    Some(json!({
        "aInviterName": aInviterName,
        "aOrganizationType": aOrganizationType,
        "aRequestID": aRequestID,
        "aName": aName,
        "aIsStrikeTeam": aIsStrikeTeam,
    }))
}

/// Method 35: `onOrganizationJoined`. Auto-generated.
pub(super) fn decode_35(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aOrganizationType = c.u8()?;
    let aRank = c.u8()?;
    let aNewMember = c.u8()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aOrganizationType": aOrganizationType,
        "aRank": aRank,
        "aNewMember": aNewMember,
    }))
}

/// Method 36: `onOrganizationLeft`. Auto-generated.
pub(super) fn decode_36(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aReason = c.u8()?;
    let aOrganizationId = c.i32_le()?;
    Some(json!({
        "aReason": aReason,
        "aOrganizationId": aOrganizationId,
    }))
}

/// Method 37: `onMemberJoinedOrganization`. Auto-generated.
pub(super) fn decode_37(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aMemberName = c.wstring()?;
    let aMember = c.i32_le()?;
    let aOrganizationId = c.i32_le()?;
    let aRank = c.u8()?;
    let aNewMember = c.u8()?;
    Some(json!({
        "aMemberName": aMemberName,
        "aMember": aMember,
        "aOrganizationId": aOrganizationId,
        "aRank": aRank,
        "aNewMember": aNewMember,
    }))
}

/// Method 39: `onMemberLeftOrganization`. Auto-generated.
pub(super) fn decode_39(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aMember = c.i32_le()?;
    let aReason = c.u8()?;
    let aOrganizationId = c.i32_le()?;
    let aMemberName = c.wstring()?;
    Some(json!({
        "aMember": aMember,
        "aReason": aReason,
        "aOrganizationId": aOrganizationId,
        "aMemberName": aMemberName,
    }))
}

/// Method 40: `onMemberRankChangedOrganization`. Auto-generated.
pub(super) fn decode_40(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aMember = c.i32_le()?;
    let aRank = c.u8()?;
    let aOrganizationId = c.i32_le()?;
    let aMemberName = c.wstring()?;
    Some(json!({
        "aMember": aMember,
        "aRank": aRank,
        "aOrganizationId": aOrganizationId,
        "aMemberName": aMemberName,
    }))
}

/// Method 41: `onStrikeTeamUpdate`. Auto-generated.
pub(super) fn decode_41(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aPvPValue = c.u8()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aPvPValue": aPvPValue,
    }))
}

/// Method 42: `onPvPOrganizationLeaveRequest`. Auto-generated.
pub(super) fn decode_42(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aPvPValue = c.u8()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aPvPValue": aPvPValue,
    }))
}

/// Method 43: `onOrganizationNameUpdate`. Auto-generated.
pub(super) fn decode_43(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aName = c.wstring()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aName": aName,
    }))
}

/// Method 44: `onOrganizationExperienceUpdate`. Auto-generated.
pub(super) fn decode_44(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aExperience = c.u64_le()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aExperience": aExperience,
    }))
}

/// Method 45: `onOrganizationMOTDUpdate`. Auto-generated.
pub(super) fn decode_45(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aMOTD = c.wstring()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aMOTD": aMOTD,
    }))
}

/// Method 46: `onOrganizationNoteUpdate`. Auto-generated.
pub(super) fn decode_46(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aName = c.wstring()?;
    let aNote = c.wstring()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aName": aName,
        "aNote": aNote,
    }))
}

/// Method 47: `onOrganizationOfficerNoteUpdate`. Auto-generated.
pub(super) fn decode_47(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aName = c.wstring()?;
    let aNote = c.wstring()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aName": aName,
        "aNote": aNote,
    }))
}

/// Method 48: `onOrganizationCashUpdate`. Auto-generated.
pub(super) fn decode_48(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aCash = c.u64_le()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aCash": aCash,
    }))
}

/// Method 49: `onOrganizationRankUpdate`. Auto-generated.
pub(super) fn decode_49(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aRankIds = read_array!(c, c.i32_le()?);
    let aRankFlags = read_array!(c, c.i32_le()?);
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aRankIds": aRankIds,
        "aRankFlags": aRankFlags,
    }))
}

/// Method 50: `onOrganizationRankNameUpdate`. Auto-generated.
pub(super) fn decode_50(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aRankIds = read_array!(c, c.i32_le()?);
    let aRankNames = read_array!(c, c.wstring()?);
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aRankIds": aRankIds,
        "aRankNames": aRankNames,
    }))
}

/// Method 51: `onSquadLootType`. Auto-generated.
pub(super) fn decode_51(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrganizationId = c.i32_le()?;
    let aLootType = c.i32_le()?;
    Some(json!({
        "aOrganizationId": aOrganizationId,
        "aLootType": aLootType,
    }))
}

/// Method 52: `onStartMinigame`. Auto-generated.
pub(super) fn decode_52(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let uRL = c.wstring()?;
    Some(json!({
        "URL": uRL,
    }))
}

/// Method 53: `onStartMinigameDialog`. Auto-generated.
pub(super) fn decode_53(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let name = c.wstring()?;
    let difficulty = c.wstring()?;
    let tCLevel = c.i32_le()?;
    let verb = c.wstring()?;
    let archetypeBitfield = c.i32_le()?;
    let canPlay = c.u8()?;
    let canCall = c.u8()?;
    let canSpectate = c.u8()?;
    Some(json!({
        "Name": name,
        "Difficulty": difficulty,
        "TCLevel": tCLevel,
        "Verb": verb,
        "ArchetypeBitfield": archetypeBitfield,
        "CanPlay": canPlay,
        "CanCall": canCall,
        "CanSpectate": canSpectate,
    }))
}

/// Method 56: `onSpectateList`. Auto-generated.
pub(super) fn decode_56(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let playerIds = read_array!(c, c.i32_le()?);
    let playerNames = read_array!(c, c.wstring()?);
    Some(json!({
        "playerIds": playerIds,
        "playerNames": playerNames,
    }))
}

/// Method 57: `onMinigameRegistrationPrompt`. Auto-generated.
pub(super) fn decode_57(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let cost = c.i32_le()?;
    Some(json!({
        "Cost": cost,
    }))
}

/// Method 58: `minigameRegistrationInfo`. Auto-generated.
pub(super) fn decode_58(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let registered = c.u8()?;
    let inRangeOnly = c.u8()?;
    let wantsRequests = c.u8()?;
    let note = c.wstring()?;
    Some(json!({
        "Registered": registered,
        "InRangeOnly": inRangeOnly,
        "WantsRequests": wantsRequests,
        "Note": note,
    }))
}

/// Method 59: `addOrUpdateMinigameHelper`. Auto-generated.
pub(super) fn decode_59(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let playerId = c.i32_le()?;
    let name = c.wstring()?;
    let note = c.wstring()?;
    let level = c.u8()?;
    let archetype = c.u8()?;
    let friend = c.u8()?;
    Some(json!({
        "PlayerId": playerId,
        "Name": name,
        "Note": note,
        "Level": level,
        "Archetype": archetype,
        "Friend": friend,
    }))
}

/// Method 60: `removeMinigameHelper`. Auto-generated.
pub(super) fn decode_60(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let playerId = c.i32_le()?;
    Some(json!({
        "PlayerId": playerId,
    }))
}

/// Method 61: `minigameCallDisplay`. Auto-generated.
pub(super) fn decode_61(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let callingPlayerId = c.i32_le()?;
    let name = c.wstring()?;
    let archetype = c.i32_le()?;
    let level = c.i32_le()?;
    let tipAmount = c.i32_le()?;
    let expiresAt = c.i32_le()?;
    let gameName = c.wstring()?;
    let gameDifficulty = c.wstring()?;
    let gameVerb = c.wstring()?;
    let gameTC = c.i32_le()?;
    let nPCTitle = c.wstring()?;
    Some(json!({
        "CallingPlayerId": callingPlayerId,
        "Name": name,
        "Archetype": archetype,
        "Level": level,
        "TipAmount": tipAmount,
        "ExpiresAt": expiresAt,
        "GameName": gameName,
        "GameDifficulty": gameDifficulty,
        "GameVerb": gameVerb,
        "GameTC": gameTC,
        "NPCTitle": nPCTitle,
    }))
}

/// Method 62: `minigameCallResult`. Auto-generated.
pub(super) fn decode_62(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let resultCode = c.i32_le()?;
    let startTime = c.f32_le()?;
    Some(json!({
        "ResultCode": resultCode,
        "StartTime": startTime,
    }))
}

/// Method 63: `minigameCallAbort`. Auto-generated.
pub(super) fn decode_63(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let callingPlayerId = c.i32_le()?;
    Some(json!({
        "CallingPlayerId": callingPlayerId,
    }))
}

/// Method 64: `showMinigameContact`. Auto-generated.
pub(super) fn decode_64(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let id = c.i32_le()?;
    let name = c.wstring()?;
    let title = c.wstring()?;
    let icon = c.wstring()?;
    let time = c.i32_le()?;
    let success = c.i32_le()?;
    let cost = c.i32_le()?;
    Some(json!({
        "Id": id,
        "Name": name,
        "Title": title,
        "Icon": icon,
        "Time": time,
        "Success": success,
        "Cost": cost,
    }))
}

/// Method 65: `setupStargateInfo`. Auto-generated.
pub(super) fn decode_65(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let worldStargateList = read_array!(c, c.i32_le()?);
    let knownStargateList = read_array!(c, c.i32_le()?);
    let hiddenStargateList = read_array!(c, c.i32_le()?);
    Some(json!({
        "worldStargateList": worldStargateList,
        "knownStargateList": knownStargateList,
        "hiddenStargateList": hiddenStargateList,
    }))
}

/// Method 66: `updateStargateAddress`. Auto-generated.
pub(super) fn decode_66(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let addressId = c.i32_le()?;
    let hasAddress = c.u8()?;
    let hidden = c.u8()?;
    Some(json!({
        "addressId": addressId,
        "hasAddress": hasAddress,
        "hidden": hidden,
    }))
}

/// Method 67: `stargateRotationOverride`. Auto-generated.
pub(super) fn decode_67(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let yaw = c.f32_le()?;
    Some(json!({
        "yaw": yaw,
    }))
}

/// Method 68: `onStargatePassage`. Auto-generated.
pub(super) fn decode_68(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let addressId = c.i32_le()?;
    Some(json!({
        "addressId": addressId,
    }))
}

/// Method 70: `onActiveSlotUpdate`. Auto-generated.
pub(super) fn decode_70(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let bagId = c.i32_le()?;
    let slotId = c.i32_le()?;
    Some(json!({
        "BagId": bagId,
        "SlotId": slotId,
    }))
}

/// Method 71: `onRemoveItem`. Auto-generated.
pub(super) fn decode_71(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let itemIdList = read_array!(c, c.i32_le()?);
    Some(json!({
        "ItemIdList": itemIdList,
    }))
}

/// Method 73: `onRefreshItem`. Auto-generated.
pub(super) fn decode_73(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let itemId = c.i32_le()?;
    Some(json!({
        "ItemId": itemId,
    }))
}

/// Method 74: `onClearOrgVaultInventory`. Auto-generated.
pub(super) fn decode_74(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let organizationId = c.i32_le()?;
    Some(json!({
        "OrganizationId": organizationId,
    }))
}

/// Method 75: `onCashChanged`. Auto-generated.
pub(super) fn decode_75(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let cash = c.i32_le()?;
    Some(json!({
        "cash": cash,
    }))
}

/// Method 77: `onMailHeaderRemove`. Auto-generated.
pub(super) fn decode_77(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let mailId = c.i32_le()?;
    Some(json!({
        "MailId": mailId,
    }))
}

/// Method 78: `onMailRead`. Auto-generated.
pub(super) fn decode_78(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let mailId = c.i32_le()?;
    let bodyText = c.wstring()?;
    let bodyId = c.i32_le()?;
    let toText = c.wstring()?;
    Some(json!({
        "MailId": mailId,
        "BodyText": bodyText,
        "BodyId": bodyId,
        "ToText": toText,
    }))
}

/// Method 79: `sendMailResult`. Auto-generated.
pub(super) fn decode_79(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let resultCode = c.u8()?;
    let failedRecipients = read_array!(c, c.wstring()?);
    let failedRecipientFlags = c.i32_le()?;
    Some(json!({
        "ResultCode": resultCode,
        "FailedRecipients": failedRecipients,
        "FailedRecipientFlags": failedRecipientFlags,
    }))
}

/// Method 80: `onMissionUpdate`. Auto-generated.
pub(super) fn decode_80(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let missionID = c.i32_le()?;
    let status = c.i8()?;
    let missionGiverName = c.i32_le()?;
    Some(json!({
        "MissionID": missionID,
        "Status": status,
        "MissionGiverName": missionGiverName,
    }))
}

/// Method 81: `onStepUpdate`. Auto-generated.
pub(super) fn decode_81(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let stepID = c.i32_le()?;
    let status = c.i8()?;
    Some(json!({
        "StepID": stepID,
        "Status": status,
    }))
}

/// Method 82: `onObjectiveUpdate`. Auto-generated.
pub(super) fn decode_82(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let objectiveID = c.i32_le()?;
    let status = c.i8()?;
    let hidden = c.i8()?;
    let optional = c.i8()?;
    Some(json!({
        "ObjectiveID": objectiveID,
        "Status": status,
        "Hidden": hidden,
        "Optional": optional,
    }))
}

/// Method 83: `onTaskUpdate`. Auto-generated.
pub(super) fn decode_83(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let taskID = c.i32_le()?;
    let status = c.i8()?;
    let count = c.i32_le()?;
    Some(json!({
        "TaskID": taskID,
        "Status": status,
        "Count": count,
    }))
}

/// Method 84: `offerSharedMission`. Auto-generated.
pub(super) fn decode_84(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let missionId = c.i32_le()?;
    Some(json!({
        "MissionId": missionId,
    }))
}

/// Method 85: `onContactListUpdate`. Auto-generated.
pub(super) fn decode_85(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aListId = c.i32_le()?;
    let aName = c.wstring()?;
    let aFlags = c.u32_le()?;
    Some(json!({
        "aListId": aListId,
        "aName": aName,
        "aFlags": aFlags,
    }))
}

/// Method 86: `onContactListDelete`. Auto-generated.
pub(super) fn decode_86(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aListId = c.i32_le()?;
    Some(json!({
        "aListId": aListId,
    }))
}

/// Method 87: `onContactListAddMembers`. Auto-generated.
pub(super) fn decode_87(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aListId = c.i32_le()?;
    let aPlayerNames = read_array!(c, c.wstring()?);
    Some(json!({
        "aListId": aListId,
        "aPlayerNames": aPlayerNames,
    }))
}

/// Method 88: `onContactListRemoveMembers`. Auto-generated.
pub(super) fn decode_88(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aListId = c.i32_le()?;
    let aPlayerNames = read_array!(c, c.wstring()?);
    Some(json!({
        "aListId": aListId,
        "aPlayerNames": aPlayerNames,
    }))
}

/// Method 89: `onContactListEvent`. Auto-generated.
pub(super) fn decode_89(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aPlayerName = c.wstring()?;
    let aEventId = c.u32_le()?;
    let aDataValue = c.i32_le()?;
    Some(json!({
        "aPlayerName": aPlayerName,
        "aEventId": aEventId,
        "aDataValue": aDataValue,
    }))
}

/// Method 90: `onBMOpen`. Auto-generated.
pub(super) fn decode_90(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let entityId = c.i32_le()?;
    Some(json!({
        "entityId": entityId,
    }))
}

/// Method 91: `onBMError`. Auto-generated.
pub(super) fn decode_91(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let errorId = c.i32_le()?;
    Some(json!({
        "errorId": errorId,
    }))
}

/// Method 93: `onBMAuctionRemove`. Auto-generated.
pub(super) fn decode_93(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let sequenceId = c.i32_le()?;
    Some(json!({
        "sequenceId": sequenceId,
    }))
}

/// Method 95: `onBMWatchedItemsUpdate`. Auto-generated.
pub(super) fn decode_95(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let itemList = read_array!(c, c.i32_le()?);
    Some(json!({
        "itemList": itemList,
    }))
}

/// Method 96: `onVersionInfo`. Auto-generated.
pub(super) fn decode_96(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let categoryId = c.i32_le()?;
    let version = c.i32_le()?;
    let requiredUpdates = c.i32_le()?;
    let invalidateAll = c.i8()?;
    let invalidKeys = read_array!(c, c.i32_le()?);
    Some(json!({
        "CategoryId": categoryId,
        "Version": version,
        "RequiredUpdates": requiredUpdates,
        "InvalidateAll": invalidateAll,
        "InvalidKeys": invalidKeys,
    }))
}

/// Method 97: `onCookedDataError`. Auto-generated.
pub(super) fn decode_97(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let categoryID = c.i32_le()?;
    let elementKey = c.i32_le()?;
    Some(json!({
        "categoryID": categoryID,
        "elementKey": elementKey,
    }))
}

/// Method 100: `onDHDReply`. Auto-generated.
pub(super) fn decode_100(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aMessage = c.wstring()?;
    Some(json!({
        "aMessage": aMessage,
    }))
}

/// Method 101: `onKnownAbilitiesUpdate`. Auto-generated.
pub(super) fn decode_101(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let abilityData = read_array!(c, c.i32_le()?);
    Some(json!({
        "AbilityData": abilityData,
    }))
}

/// Method 102: `onTimeofDay`. Auto-generated.
pub(super) fn decode_102(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let time = c.f32_le()?;
    let wind = c.f32_le()?;
    let weather = c.i8()?;
    Some(json!({
        "Time": time,
        "Wind": wind,
        "Weather": weather,
    }))
}

/// Method 103: `onOverridePerfStatsRate`. Auto-generated.
pub(super) fn decode_103(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let newIntervalMS = c.i32_le()?;
    Some(json!({
        "NewIntervalMS": newIntervalMS,
    }))
}

/// Method 105: `onDialogDisplay`. Auto-generated.
pub(super) fn decode_105(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let entityId = c.i32_le()?;
    let dialogID = c.i32_le()?;
    let missionFlags = c.i32_le()?;
    let isImmediate = c.u8()?;
    let aMissionId = c.i32_le()?;
    Some(json!({
        "EntityId": entityId,
        "DialogID": dialogID,
        "MissionFlags": missionFlags,
        "IsImmediate": isImmediate,
        "aMissionId": aMissionId,
    }))
}

/// Method 106: `onVaultOpen`. Auto-generated.
pub(super) fn decode_106(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let entityId = c.i32_le()?;
    let position_x = c.f32_le()?;
    let position_y = c.f32_le()?;
    let position_z = c.f32_le()?;
    Some(json!({
        "EntityId": entityId,
        "Position_x": position_x,
        "Position_y": position_y,
        "Position_z": position_z,
    }))
}

/// Method 107: `onTeamVaultOpen`. Auto-generated.
pub(super) fn decode_107(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let entityId = c.i32_le()?;
    let position_x = c.f32_le()?;
    let position_y = c.f32_le()?;
    let position_z = c.f32_le()?;
    Some(json!({
        "EntityId": entityId,
        "Position_x": position_x,
        "Position_y": position_y,
        "Position_z": position_z,
    }))
}

/// Method 108: `onCommandVaultOpen`. Auto-generated.
pub(super) fn decode_108(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let entityId = c.i32_le()?;
    let position_x = c.f32_le()?;
    let position_y = c.f32_le()?;
    let position_z = c.f32_le()?;
    Some(json!({
        "EntityId": entityId,
        "Position_x": position_x,
        "Position_y": position_y,
        "Position_z": position_z,
    }))
}

/// Method 112: `onCraftingRespecPrompt`. Auto-generated.
pub(super) fn decode_112(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let costToRespec = c.i32_le()?;
    Some(json!({
        "CostToRespec": costToRespec,
    }))
}

/// Method 116: `onPlayerTeleport`. Auto-generated.
pub(super) fn decode_116(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let location_x = c.f32_le()?;
    let location_y = c.f32_le()?;
    let location_z = c.f32_le()?;
    let direction_x = c.f32_le()?;
    let direction_y = c.f32_le()?;
    let direction_z = c.f32_le()?;
    Some(json!({
        "Location_x": location_x,
        "Location_y": location_y,
        "Location_z": location_z,
        "Direction_x": direction_x,
        "Direction_y": direction_y,
        "Direction_z": direction_z,
    }))
}

/// Method 117: `onClientMapLoad`. Auto-generated.
pub(super) fn decode_117(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let areaName = c.wstring()?;
    let mapPath = c.wstring()?;
    let worldID = c.i32_le()?;
    let location_x = c.f32_le()?;
    let location_y = c.f32_le()?;
    let location_z = c.f32_le()?;
    let direction_x = c.f32_le()?;
    let direction_y = c.f32_le()?;
    let direction_z = c.f32_le()?;
    Some(json!({
        "areaName": areaName,
        "mapPath": mapPath,
        "WorldID": worldID,
        "Location_x": location_x,
        "Location_y": location_y,
        "Location_z": location_z,
        "Direction_x": direction_x,
        "Direction_y": direction_y,
        "Direction_z": direction_z,
    }))
}

/// Method 118: `giveAbility`. Auto-generated.
pub(super) fn decode_118(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let abilityId = c.i32_le()?;
    let persist = c.i8()?;
    Some(json!({
        "abilityId": abilityId,
        "persist": persist,
    }))
}

/// Method 119: `giveXPForLevel`. Auto-generated.
pub(super) fn decode_119(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let level = c.i32_le()?;
    Some(json!({
        "level": level,
    }))
}

/// Method 120: `onDisplayDHD`. Auto-generated.
pub(super) fn decode_120(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let pointOfOrigin = c.u8()?;
    Some(json!({
        "PointOfOrigin": pointOfOrigin,
    }))
}

/// Method 121: `onErrorCode`. Auto-generated.
pub(super) fn decode_121(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let systemID = c.u8()?;
    let instanceID = c.i32_le()?;
    let errorCodeID = c.u16_le()?;
    Some(json!({
        "SystemID": systemID,
        "InstanceID": instanceID,
        "ErrorCodeID": errorCodeID,
    }))
}

/// Method 123: `onMapInfo`. Auto-generated.
pub(super) fn decode_123(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let sysTypeID = c.u32_le()?;
    let sysID = c.u32_le()?;
    let keyID = c.u32_le()?;
    let worldID = c.i32_le()?;
    let location_x = c.f32_le()?;
    let location_y = c.f32_le()?;
    let location_z = c.f32_le()?;
    let lifetime = c.u32_le()?;
    let delete = c.u8()?;
    Some(json!({
        "SysTypeID": sysTypeID,
        "SysID": sysID,
        "KeyID": keyID,
        "WorldID": worldID,
        "Location_x": location_x,
        "Location_y": location_y,
        "Location_z": location_z,
        "Lifetime": lifetime,
        "Delete": delete,
    }))
}

/// Method 130: `onExtraNameUpdate`. Auto-generated.
pub(super) fn decode_130(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let extraName = c.wstring()?;
    Some(json!({
        "ExtraName": extraName,
    }))
}

/// Method 131: `onExpUpdate`. Auto-generated.
pub(super) fn decode_131(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let exp = c.i32_le()?;
    Some(json!({
        "Exp": exp,
    }))
}

/// Method 132: `onMaxExpUpdate`. Auto-generated.
pub(super) fn decode_132(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let maxExp = c.i32_le()?;
    Some(json!({
        "MaxExp": maxExp,
    }))
}

/// Method 134: `onOrganizationCreationResult`. Auto-generated.
pub(super) fn decode_134(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let result = c.u8()?;
    let retCode = c.u8()?;
    Some(json!({
        "Result": result,
        "RetCode": retCode,
    }))
}

/// Method 135: `launchOrganizationCreation`. Auto-generated.
pub(super) fn decode_135(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aOrgType = c.u8()?;
    Some(json!({
        "aOrgType": aOrgType,
    }))
}

/// Method 136: `onUpdateDiscipline`. Auto-generated.
pub(super) fn decode_136(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aDisciplineSeqId = c.i32_le()?;
    let aExpertise = c.i32_le()?;
    Some(json!({
        "aDisciplineSeqId": aDisciplineSeqId,
        "aExpertise": aExpertise,
    }))
}

/// Method 138: `onUpdateRacialParadigmLevel`. Auto-generated.
pub(super) fn decode_138(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aRacialParadigmId = c.i32_le()?;
    let aLevel = c.i8()?;
    Some(json!({
        "aRacialParadigmId": aRacialParadigmId,
        "aLevel": aLevel,
    }))
}

/// Method 139: `onUpdateKnownCrafts`. Auto-generated.
pub(super) fn decode_139(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aCraftList = read_array!(c, c.i32_le()?);
    Some(json!({
        "aCraftList": aCraftList,
    }))
}

/// Method 142: `onClientChallenge`. Auto-generated.
pub(super) fn decode_142(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aClientChallenge = c.i32_le()?;
    let aChallengeType = c.i32_le()?;
    let aChallengeObject = c.wstring()?;
    let aChallengeID1 = c.i32_le()?;
    let aChallengeID2 = c.i32_le()?;
    Some(json!({
        "aClientChallenge": aClientChallenge,
        "aChallengeType": aChallengeType,
        "aChallengeObject": aChallengeObject,
        "aChallengeID1": aChallengeID1,
        "aChallengeID2": aChallengeID2,
    }))
}

/// Method 143: `onDuelChallenge`. Auto-generated.
pub(super) fn decode_143(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aEntityId = c.i32_le()?;
    let aSquadList = read_array!(c, c.i32_le()?);
    Some(json!({
        "aEntityId": aEntityId,
        "aSquadList": aSquadList,
    }))
}

/// Method 145: `onTradeResults`. Auto-generated.
pub(super) fn decode_145(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let entityId = c.i32_le()?;
    let result = c.i32_le()?;
    Some(json!({
        "EntityId": entityId,
        "Result": result,
    }))
}

/// Method 146: `onSpaceQueued`. Auto-generated.
pub(super) fn decode_146(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aSpaceName = c.wstring()?;
    Some(json!({
        "aSpaceName": aSpaceName,
    }))
}

/// Method 147: `onSpaceQueueReady`. Auto-generated.
pub(super) fn decode_147(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aSpaceName = c.wstring()?;
    Some(json!({
        "aSpaceName": aSpaceName,
    }))
}

/// Method 148: `onRemoteEntityCreate`. Auto-generated.
pub(super) fn decode_148(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aEntityId = c.i32_le()?;
    let aEntityType = c.wstring()?;
    let aPosition_x = c.f32_le()?;
    let aPosition_y = c.f32_le()?;
    let aPosition_z = c.f32_le()?;
    let aWorldId = c.i32_le()?;
    let aSpaceId = c.i32_le()?;
    Some(json!({
        "aEntityId": aEntityId,
        "aEntityType": aEntityType,
        "aPosition_x": aPosition_x,
        "aPosition_y": aPosition_y,
        "aPosition_z": aPosition_z,
        "aWorldId": aWorldId,
        "aSpaceId": aSpaceId,
    }))
}

/// Method 149: `onRemoteEntityMove`. Auto-generated.
pub(super) fn decode_149(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aEntityId = c.i32_le()?;
    let aPosition_x = c.f32_le()?;
    let aPosition_y = c.f32_le()?;
    let aPosition_z = c.f32_le()?;
    let aWorldId = c.i32_le()?;
    let aSpaceId = c.i32_le()?;
    Some(json!({
        "aEntityId": aEntityId,
        "aPosition_x": aPosition_x,
        "aPosition_y": aPosition_y,
        "aPosition_z": aPosition_z,
        "aWorldId": aWorldId,
        "aSpaceId": aSpaceId,
    }))
}

/// Method 150: `onRemoteEntityRemove`. Auto-generated.
pub(super) fn decode_150(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aEntityId = c.i32_le()?;
    Some(json!({
        "aEntityId": aEntityId,
    }))
}

/// Method 151: `onDuelEntitiesSet`. Auto-generated.
pub(super) fn decode_151(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aEntityList = read_array!(c, c.i32_le()?);
    Some(json!({
        "aEntityList": aEntityList,
    }))
}

/// Method 152: `onDuelEntitiesRemove`. Auto-generated.
pub(super) fn decode_152(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let aEntityId = c.i32_le()?;
    Some(json!({
        "aEntityId": aEntityId,
    }))
}

/// Method 154: `onThreatenedMobsUpdate`. Auto-generated.
pub(super) fn decode_154(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let entityId = c.i32_le()?;
    let hasThreat = c.u8()?;
    Some(json!({
        "EntityId": entityId,
        "HasThreat": hasThreat,
    }))
}

/// Method 155: `onPlayMovie`. Auto-generated.
pub(super) fn decode_155(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let movieName = c.wstring()?;
    let fullScreen = c.u8()?;
    Some(json!({
        "MovieName": movieName,
        "FullScreen": fullScreen,
    }))
}

/// Method 156: `onCancelMovie`. Auto-generated.
pub(super) fn decode_156(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let movieName = c.wstring()?;
    let entityId = c.i32_le()?;
    Some(json!({
        "MovieName": movieName,
        "EntityId": entityId,
    }))
}

/// Dispatch — returns `Some` when the codegen has a decoder for this
/// method index. Methods missing here fall through to the hand-
/// written decoders in [`crate::wire_log::decoders::outbound`], then
/// to the args_hex fallback.
pub(super) fn decode_generated(method_index: u16, args: &[u8]) -> Option<Value> {
    match method_index {
        0 => decode_0(args),
        2 => decode_2(args),
        3 => decode_3(args),
        4 => decode_4(args),
        6 => decode_6(args),
        7 => decode_7(args),
        8 => decode_8(args),
        9 => decode_9(args),
        10 => decode_10(args),
        11 => decode_11(args),
        12 => decode_12(args),
        13 => decode_13(args),
        15 => decode_15(args),
        16 => decode_16(args),
        17 => decode_17(args),
        18 => decode_18(args),
        19 => decode_19(args),
        22 => decode_22(args),
        23 => decode_23(args),
        24 => decode_24(args),
        25 => decode_25(args),
        26 => decode_26(args),
        28 => decode_28(args),
        30 => decode_30(args),
        31 => decode_31(args),
        32 => decode_32(args),
        33 => decode_33(args),
        34 => decode_34(args),
        35 => decode_35(args),
        36 => decode_36(args),
        37 => decode_37(args),
        39 => decode_39(args),
        40 => decode_40(args),
        41 => decode_41(args),
        42 => decode_42(args),
        43 => decode_43(args),
        44 => decode_44(args),
        45 => decode_45(args),
        46 => decode_46(args),
        47 => decode_47(args),
        48 => decode_48(args),
        49 => decode_49(args),
        50 => decode_50(args),
        51 => decode_51(args),
        52 => decode_52(args),
        53 => decode_53(args),
        56 => decode_56(args),
        57 => decode_57(args),
        58 => decode_58(args),
        59 => decode_59(args),
        60 => decode_60(args),
        61 => decode_61(args),
        62 => decode_62(args),
        63 => decode_63(args),
        64 => decode_64(args),
        65 => decode_65(args),
        66 => decode_66(args),
        67 => decode_67(args),
        68 => decode_68(args),
        70 => decode_70(args),
        71 => decode_71(args),
        73 => decode_73(args),
        74 => decode_74(args),
        75 => decode_75(args),
        77 => decode_77(args),
        78 => decode_78(args),
        79 => decode_79(args),
        80 => decode_80(args),
        81 => decode_81(args),
        82 => decode_82(args),
        83 => decode_83(args),
        84 => decode_84(args),
        85 => decode_85(args),
        86 => decode_86(args),
        87 => decode_87(args),
        88 => decode_88(args),
        89 => decode_89(args),
        90 => decode_90(args),
        91 => decode_91(args),
        93 => decode_93(args),
        95 => decode_95(args),
        96 => decode_96(args),
        97 => decode_97(args),
        100 => decode_100(args),
        101 => decode_101(args),
        102 => decode_102(args),
        103 => decode_103(args),
        105 => decode_105(args),
        106 => decode_106(args),
        107 => decode_107(args),
        108 => decode_108(args),
        112 => decode_112(args),
        116 => decode_116(args),
        117 => decode_117(args),
        118 => decode_118(args),
        119 => decode_119(args),
        120 => decode_120(args),
        121 => decode_121(args),
        123 => decode_123(args),
        130 => decode_130(args),
        131 => decode_131(args),
        132 => decode_132(args),
        134 => decode_134(args),
        135 => decode_135(args),
        136 => decode_136(args),
        138 => decode_138(args),
        139 => decode_139(args),
        142 => decode_142(args),
        143 => decode_143(args),
        145 => decode_145(args),
        146 => decode_146(args),
        147 => decode_147(args),
        148 => decode_148(args),
        149 => decode_149(args),
        150 => decode_150(args),
        151 => decode_151(args),
        152 => decode_152(args),
        154 => decode_154(args),
        155 => decode_155(args),
        156 => decode_156(args),
        _ => None,
    }
}
