//! The ability press chain, router and sequence-join sites (AB-C1, AB-C2;
//! `docs/reverse-engineering/findings/ability-client-hook-anchors.md`).
//! Bytes read from the QA `SGW.exe` image on 2026-10-04 and matching the
//! finding's table; the GamePet send and the two `start*Message` functions
//! were read for this packet. Live status: UNVERIFIED.

use cimmeria_client_hookgate::Site;

use super::code_sites::code_site;

/// `useAction` tolua thunk (`client.ability.press`, `source = hotbar`):
/// `sub esp, 0x10; push esi; mov esi, [esp+0x18]; lea eax, [esp+8]; push
/// eax; push 0; push`.
pub const USE_ACTION_THUNK: Site = code_site(
    "useAction tolua thunk",
    0x00aa_94e0,
    b"\x83\xec\x10\x56\x8b\x74\x24\x18\x8d\x44\x24\x08\x50\x6a\x00\x6a",
);

/// `useAbility` tolua thunk (`source = lua`): `sub esp, 0xc; push esi; mov
/// esi, [esp+0x14]; ...`.
pub const USE_ABILITY_THUNK: Site = code_site(
    "useAbility tolua thunk",
    0x00aa_2910,
    b"\x83\xec\x0c\x56\x8b\x74\x24\x14\x8d\x44\x24\x04\x50\x6a\x00\x6a",
);

/// `FUN_00ad9580(actionId, self)`: starts with `call 0x00c66ad0` (pinned by
/// opcode and displacement), then `mov edx, [esp+4]; add eax, 0x8c; mov
/// eax, [eax]`.
pub const ABILITY_SLOT: Site = code_site(
    "useAction slot step",
    0x00ad_9580,
    b"\xe8\x4b\xd5\x18\x00\x8b\x54\x24\x04\x05\x8c\x00\x00\x00\x8b\x00",
);

/// `FUN_00d2afc0(set, abilityId, targetId)`, `ret 8`: `mov eax, [esp+4];
/// push esi; push eax; mov esi, ecx; call 0x00d2a000; test eax, eax; je`.
pub const ABILITY_LOOKUP: Site = code_site(
    "AbilitySet lookup",
    0x00d2_afc0,
    b"\x8b\x44\x24\x04\x56\x50\x8b\xf1\xe8\x33\xf0\xff\xff\x85\xc0\x74",
);

/// `FUN_00d2ae40(set, record, targetId)`, `ret 8`: `mov eax, fs:[0]; push
/// -1; push 0x016fcc9f`.
pub const ABILITY_SEND_BUILDER: Site = code_site(
    "useAbility send builder",
    0x00d2_ae40,
    b"\x64\xa1\x00\x00\x00\x00\x6a\xff\x68\x9f\xcc\x6f\x01\x50\x64\x89",
);

/// `PetAbilityAction::execute(self)`, `ret 4`: `push esi; push edi; mov
/// edi, ecx; call 0x00c66ad0; add eax, 0x8c`.
pub const PET_ABILITY_ACTION_EXECUTE: Site = code_site(
    "PetAbilityAction::execute",
    0x00e3_cf40,
    b"\x56\x57\x8b\xf9\xe8\x87\x9b\xe2\xff\x05\x8c\x00\x00\x00\x80\x7c",
);

/// The GamePet send (`thiscall(pet, abilityId, targetId)`, `ret 8`): `mov
/// eax, fs:[0]; push -1; push 0x016fdce0`.
pub const GAME_PET_SEND: Site = code_site(
    "GamePet ability send",
    0x00d3_a820,
    b"\x64\xa1\x00\x00\x00\x00\x6a\xff\x68\xe0\xdc\x6f\x01\x50\x64\x89",
);

/// `startEntityMessage(msgId, entityId)`, `ret 8`: `mov eax, fs:[0]; push
/// -1; push 0x01705538`.
pub const START_ENTITY_MESSAGE: Site = code_site(
    "startEntityMessage",
    0x00dd_6a60,
    b"\x64\xa1\x00\x00\x00\x00\x6a\xff\x68\x38\x55\x70\x01\x50\x64\x89",
);

/// `startProxyMessage(msgId)`, `ret 4`: `mov eax, fs:[0]; push -1; push
/// 0x01705520`.
pub const START_PROXY_MESSAGE: Site = code_site(
    "startProxyMessage",
    0x00dd_6980,
    b"\x64\xa1\x00\x00\x00\x00\x6a\xff\x68\x20\x55\x70\x01\x50\x64\x89",
);

/// `Channel::send()`: `push -1; push 0x01792ff8; mov eax, fs:[0]`.
pub const CHANNEL_SEND: Site = code_site(
    "Channel::send",
    0x0157_6f90,
    b"\x6a\xff\x68\xf8\x2f\x79\x01\x64\xa1\x00\x00\x00\x00\x50\x64\x89",
);

/// `Nub::send(addr, bundle, channel)`, `ret 0xc`: `push -1; push
/// 0x01793f0b; mov eax, fs:[0]`.
pub const NUB_SEND: Site = code_site(
    "Nub::send",
    0x0158_2160,
    b"\x6a\xff\x68\x0b\x3f\x79\x01\x64\xa1\x00\x00\x00\x00\x50\x64\x89",
);

/// The reliable sequence counter, whole body: `mov eax, [ecx+0x4c]; lea
/// edx, [eax+1]; and edx, 0x0fffffff; mov [ecx+0x4c], edx; ret`.
pub const SEQ_NEXT: Site = code_site(
    "reliable sequence counter",
    0x0158_bb40,
    b"\x8b\x41\x4c\x8d\x50\x01\x81\xe2\xff\xff\xff\x0f\x89\x51\x4c\xc3",
);

/// `GetInt(event, name, out)`, called (not hooked) to read the ability bag:
/// `push -1; push 0x01708250; mov eax, fs:[0]`. The three readers share
/// these bytes; the address tells them apart.
pub const EVENT_GET_INT: Site = code_site(
    "event bag GetInt",
    0x00e3_cba0,
    b"\x6a\xff\x68\x50\x82\x70\x01\x64\xa1\x00\x00\x00\x00\x50\x64\x89",
);

/// `GetFloat(event, name, out)`, called, same prologue.
pub const EVENT_GET_FLOAT: Site = code_site(
    "event bag GetFloat",
    0x00e3_cc20,
    b"\x6a\xff\x68\x50\x82\x70\x01\x64\xa1\x00\x00\x00\x00\x50\x64\x89",
);

/// `GetByte(event, name, out)`, called, same prologue.
pub const EVENT_GET_BYTE: Site = code_site(
    "event bag GetByte",
    0x00d4_34d0,
    b"\x6a\xff\x68\x50\x82\x70\x01\x64\xa1\x00\x00\x00\x00\x50\x64\x89",
);

// ---------------------------------------------------------------------
// AB-C4 and AB-C5: what the client applied, and the `onSequence` net-in
// drop. Read from the QA `SGW.exe` disassembly on 2026-10-04.

/// `EffectSet` `onTimerUpdate` handler (`client.ability.applied`): `push
/// -1; push 0x017088ab; mov eax, fs:[0]` (`ret 8`).
pub const EFFECT_TIMER: Site = code_site(
    "EffectSet timer handler",
    0x00e0_9160,
    &[
        0x6A, 0xFF, 0x68, 0xAB, 0x88, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `EffectSet` entry lookup by `SecondaryId`: `sub esp, 8; mov eax,
/// [esp+0xc]; push ebx; push ebp; push esi; lea esi, [ecx+0x10]` (`ret 4`).
pub const EFFECT_LOOKUP: Site = code_site(
    "EffectSet entry lookup",
    0x00e0_8570,
    &[
        0x83, 0xEC, 0x08, 0x8B, 0x44, 0x24, 0x0C, 0x53, 0x55, 0x56, 0x8D, 0x71,
    ],
);

/// Effect-bar announce to the UI: `push -1; push 0x016d9938; mov eax,
/// fs:[0]` (`ret 4`).
pub const EFFECT_ANNOUNCE: Site = code_site(
    "Effect bar announce",
    0x00e0_a9e0,
    &[
        0x6A, 0xFF, 0x68, 0x38, 0x99, 0x6D, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// Effect display-data request (`Event_NetOut_elementDataRequest`,
/// category 9): `mov eax, fs:[0]; push -1; push 0x016f9fc3` (`ret 4`).
pub const EFFECT_DATA_REQUEST: Site = code_site(
    "Effect display-data request",
    0x00e0_a810,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0xC3, 0x9F, 0x6F,
    ],
);

/// Effect-bar add posted to the UI: `mov eax, fs:[0]; push -1; push
/// 0x016fcd46` (`ret 8`).
pub const EFFECT_POST: Site = code_site(
    "Effect bar post",
    0x00e0_a2d0,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x46, 0xCD, 0x6F,
    ],
);

/// `CooldownManager` `onTimerUpdate` handler: `push -1; push 0x0171154f;
/// mov eax, fs:[0]` (`ret 8`).
pub const COOLDOWN_TIMER: Site = code_site(
    "CooldownManager timer handler",
    0x00ea_6af0,
    &[
        0x6A, 0xFF, 0x68, 0x4F, 0x15, 0x71, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// Cooldown button callback: `sub esp, 0x14; mov eax, [esp+0x18]; mov edx,
/// [esp+0x1c]; push ebx` (`ret 0x10`).
pub const COOLDOWN_UI: Site = code_site(
    "CooldownManager button callback",
    0x00ea_62b0,
    &[
        0x83, 0xEC, 0x14, 0x8B, 0x44, 0x24, 0x18, 0x8B, 0x54, 0x24, 0x1C, 0x53,
    ],
);

/// `GameBeing` stat handler: `push -1; push 0x01708574; mov eax, fs:[0]`
/// (`ret 8`).
pub const STAT_HANDLER: Site = code_site(
    "GameBeing stat handler",
    0x00e0_1f40,
    &[
        0x6A, 0xFF, 0x68, 0x74, 0x85, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `GameBeing` base-stat handler: `push -1; push 0x01708596; mov eax,
/// fs:[0]` (`ret 8`).
pub const STAT_BASE_HANDLER: Site = code_site(
    "GameBeing base-stat handler",
    0x00e0_2060,
    &[
        0x6A, 0xFF, 0x68, 0x96, 0x85, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// Current-stat functor: `push esi; push edi; mov edi, ecx; lea eax,
/// [esp+0xc]; push eax; lea ecx, [edi+0x160]` (`ret 0x10`).
pub const STAT_FUNCTOR: Site = code_site(
    "GameBeing current-stat functor",
    0x00e0_04e0,
    &[
        0x56, 0x57, 0x8B, 0xF9, 0x8D, 0x44, 0x24, 0x0C, 0x50, 0x8D, 0x8F, 0x60,
    ],
);

/// Base-stat functor: `push esi; mov esi, ecx; lea eax, [esp+8]; push eax;
/// lea ecx, [esi+0x160]` (`ret 0x10`).
pub const STAT_BASE_FUNCTOR: Site = code_site(
    "GameBeing base-stat functor",
    0x00e0_05b0,
    &[
        0x56, 0x8B, 0xF1, 0x8D, 0x44, 0x24, 0x08, 0x50, 0x8D, 0x8E, 0x60, 0x01,
    ],
);

/// `SequenceManager::onSequence`, the `Event_NetIn_onSequence` handler:
/// `push -1; push 0x016fa823; mov eax, fs:[0]` (`ret 8`).
pub const ON_SEQUENCE: Site = code_site(
    "SequenceManager::onSequence",
    0x00d0_5790,
    &[
        0x6A, 0xFF, 0x68, 0x23, 0xA8, 0x6F, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);
