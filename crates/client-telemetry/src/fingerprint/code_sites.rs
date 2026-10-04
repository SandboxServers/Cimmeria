//! The code sites the gate checks: every function this DLL hooks or
//! calls, with the bytes the QA `SGW.exe` build has at its entry.

use cimmeria_client_hookgate::Site;

/// `FFullScreenMovieBink::Tick`: `xorps xmm1, xmm1; sub esp, 8; push esi;
/// mov esi, ecx`.
pub const BINK_TICK: Site = code_site(
    "FFullScreenMovieBink::Tick",
    0x0050_bbc0,
    &[
        0x0F, 0x57, 0xC9, 0x83, 0xEC, 0x08, 0x56, 0x8B, 0xF1, 0xF3, 0x0F, 0x10,
    ],
);

/// `FArchiveAsync::Serialize`: `sub esp, 0x10; push ebx; mov ebx,
/// [esp+0x1c]; push ebp`.
pub const ARCHIVE_ASYNC_SERIALIZE: Site = code_site(
    "FArchiveAsync::Serialize",
    0x004c_7ae0,
    &[
        0x83, 0xEC, 0x10, 0x53, 0x8B, 0x5C, 0x24, 0x1C, 0x55, 0x8B, 0x2D, 0x54,
    ],
);

/// `UWorld::UpdateLevelStreamingInner`: `sub esp, 0x28; push ebx; push
/// ebp; push esi; mov esi, [esp+0x38]`.
pub const UPDATE_LEVEL_STREAMING_INNER: Site = code_site(
    "UWorld::UpdateLevelStreamingInner",
    0x0054_e9c0,
    &[
        0x83, 0xEC, 0x28, 0x53, 0x55, 0x56, 0x8B, 0x74, 0x24, 0x38, 0x57, 0x33,
    ],
);

/// `UObject::StaticLoadObject`: `push ebp; mov ebp, esp; push -1; push
/// 0x01682387; mov eax, fs:[0]`.
pub const STATIC_LOAD_OBJECT: Site = code_site(
    "UObject::StaticLoadObject",
    0x004a_8e10,
    &[
        0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x87, 0x23, 0x68, 0x01, 0x64, 0xA1,
    ],
);

/// `APlayerController::execConsoleCommand`: `push -1; push 0x0168a690;
/// mov eax, fs:[0]`.
pub const CONSOLE_COMMAND: Site = code_site(
    "APlayerController::execConsoleCommand",
    0x0053_9850,
    &[
        0x6A, 0xFF, 0x68, 0x90, 0xA6, 0x68, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `GameBeing::onStateFieldUpdate`: `push -1; push 0x01708552; mov eax,
/// fs:[0]`.
pub const STATE_FIELD_UPDATE: Site = code_site(
    "GameBeing::onStateFieldUpdate",
    0x00e0_1c90,
    &[
        0x6A, 0xFF, 0x68, 0x52, 0x85, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `USGWAnimNotify_Event::Notify` (A): `push -1; push 0x01710399; mov eax,
/// fs:[0]`.
pub const ANIM_NOTIFY_A: Site = code_site(
    "USGWAnimNotify_Event::Notify (A)",
    0x00e9_74b0,
    &[
        0x6A, 0xFF, 0x68, 0x99, 0x03, 0x71, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `USGWAnimNotify_Event::Notify` (B): `mov eax, fs:[0]; push -1; push
/// 0x0171028b`.
pub const ANIM_NOTIFY_B: Site = code_site(
    "USGWAnimNotify_Event::Notify (B)",
    0x00e9_7070,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x8B, 0x02, 0x71,
    ],
);

/// CME event-registry lookup, hooked for `client.cme.event`: `mov eax,
/// [esp+4]; sub esp, 8; push ebx; push ebp; push esi; push edi`. It was
/// fingerprinted before as "EventSignal lookup by name" for the removed
/// CME subscriber install, together with `0x0155f790`, `0x00a5c150` and
/// `0x00e04570`, which nothing calls any more.
pub const CME_EVENT_FACTORY: Site = code_site(
    "CME event registry lookup",
    0x00a5_c0f0,
    &[
        0x8B, 0x44, 0x24, 0x04, 0x83, 0xEC, 0x08, 0x53, 0x55, 0x56, 0x57, 0x8B,
    ],
);

/// `DebugMsgHelper::message`, the BigWorld message sink
/// (`client.bw.message`): `push -1; push 0x016d5d48; mov eax, fs:[0]`.
pub const BW_MESSAGE: Site = code_site(
    "BigWorld DebugMsgHelper::message",
    0x00a3_6460,
    &[
        0x6A, 0xFF, 0x68, 0x48, 0x5D, 0x6D, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `FOutputDeviceRedirector::Serialize` (`GLog`, `client.ue3.log`): `push
/// -1; push 0x016856de; mov eax, fs:[0]`.
pub const REDIRECTOR_SERIALIZE: Site = code_site(
    "FOutputDeviceRedirector::Serialize",
    0x004c_e0b0,
    &[
        0x6A, 0xFF, 0x68, 0xDE, 0x56, 0x68, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `FOutputDeviceWindowsError::Serialize` (`GError`,
/// `client.ue3.fatal_error`): `mov eax, fs:[0]; push -1; push 0x01685724`.
pub const ERROR_SERIALIZE: Site = code_site(
    "FOutputDeviceWindowsError::Serialize",
    0x004c_e3a0,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x24, 0x57, 0x68,
    ],
);

/// The UE3 `check()` reporter (`client.ue3.assert`): `push -1; push
/// 0x0167f195; mov eax, fs:[0]`.
pub const CHECK_FAILED: Site = code_site(
    "UE3 check() reporter",
    0x0048_6000,
    &[
        0x6A, 0xFF, 0x68, 0x95, 0xF1, 0x67, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `UWorld::SpawnActor` (`client.engine.spawn_actor`): `sub esp, 0x20; push
/// ebx; push ebp; push esi; push edi; mov edi, ecx; cmp [edi+0x54], 0`.
pub const SPAWN_ACTOR: Site = code_site(
    "UWorld::SpawnActor",
    0x0087_6970,
    &[
        0x83, 0xEC, 0x20, 0x53, 0x55, 0x56, 0x57, 0x8B, 0xF9, 0x83, 0x7F, 0x54,
    ],
);

/// `UWorld::DestroyActor` (`client.engine.destroy_actor`): `push -1; push
/// 0x016b92b4; mov eax, fs:[0]`.
pub const DESTROY_ACTOR: Site = code_site(
    "UWorld::DestroyActor",
    0x0087_5290,
    &[
        0x6A, 0xFF, 0x68, 0xB4, 0x92, 0x6B, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `EntityManager::onEntityEnter` (`enterAoI`): `mov eax, fs:[0]; push -1;
/// push 0x017052ba`.
pub const ENTER_AOI: Site = code_site(
    "EntityManager::onEntityEnter",
    0x00dd_24f0,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0xBA, 0x52, 0x70,
    ],
);

/// `EntityManager::onEntityCreate`: `sub esp, 0x2c; cmp [0x01ef2224], 0;
/// push ebx; push esi`.
pub const CREATE_ENTITY: Site = code_site(
    "EntityManager::onEntityCreate",
    0x00dd_2270,
    &[
        0x83, 0xEC, 0x2C, 0x83, 0x3D, 0x24, 0x22, 0xEF, 0x01, 0x00, 0x53, 0x56,
    ],
);

/// `EntityManager::enterWorld`: `mov eax, fs:[0]; push -1; push
/// 0x01705296`.
pub const ENTER_WORLD: Site = code_site(
    "EntityManager::enterWorld",
    0x00dd_1d00,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x96, 0x52, 0x70,
    ],
);

/// `EntityManager::onEntityLeave` (`leaveAoI`): `sub esp, 0xc; cmp
/// [0x01ef2224], 0; push ebx; push ebp`.
pub const LEAVE_AOI: Site = code_site(
    "EntityManager::onEntityLeave",
    0x00dd_2800,
    &[
        0x83, 0xEC, 0x0C, 0x83, 0x3D, 0x24, 0x22, 0xEF, 0x01, 0x00, 0x53, 0x55,
    ],
);

/// Entity destroy: `cmp [0x01ef2224], 0; push ebx; push esi; push edi; mov
/// ebx, ecx`.
pub const DESTROY_ENTITY: Site = code_site(
    "EntityManager entity destroy",
    0x00dd_1120,
    &[
        0x83, 0x3D, 0x24, 0x22, 0xEF, 0x01, 0x00, 0x53, 0x56, 0x57, 0x8B, 0xD9,
    ],
);

/// `GameEntity` appearance request: `mov eax, fs:[0]; push -1; push
/// 0x0170de4f`.
pub const APPEARANCE_REQUEST: Site = code_site(
    "GameEntity appearance request",
    0x00e6_9150,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x4F, 0xDE, 0x70,
    ],
);

/// Appearance job scheduler: `push -1; push 0x01710657; mov eax, fs:[0]`.
pub const APPEARANCE_SCHEDULE: Site = code_site(
    "Appearance job scheduler",
    0x00e9_98e0,
    &[
        0x6A, 0xFF, 0x68, 0x57, 0x06, 0x71, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `EntityManager::onEntityMethod`: `push -1; push 0x017052e4; mov eax,
/// fs:[0]`.
pub const ENTITY_METHOD: Site = code_site(
    "EntityManager::onEntityMethod",
    0x00dd_2b80,
    &[
        0x6A, 0xFF, 0x68, 0xE4, 0x52, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `EntityManager::onEntityProperty`: `push -1; push 0x017052cf; mov eax,
/// fs:[0]`.
pub const ENTITY_PROPERTY: Site = code_site(
    "EntityManager::onEntityProperty",
    0x00dd_29d0,
    &[
        0x6A, 0xFF, 0x68, 0xCF, 0x52, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// Queued-message replay: `sub esp, 0x10; mov eax, [esp+0x14]; mov edx,
/// [eax+0xc]; push ebx; push ebp`.
pub const QUEUE_REPLAY: Site = code_site(
    "EntityManager queued-message replay",
    0x00dd_1e40,
    &[
        0x83, 0xEC, 0x10, 0x8B, 0x44, 0x24, 0x14, 0x8B, 0x50, 0x0C, 0x53, 0x55,
    ],
);

/// `RouteOutgoingEntityRpc`: `push -1; push 0x016f50ea; mov eax, fs:[0]`.
pub const ROUTE_OUTGOING_RPC: Site = code_site(
    "RouteOutgoingEntityRpc",
    0x00c6_fc40,
    &[
        0x6A, 0xFF, 0x68, 0xEA, 0x50, 0x6F, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `Nub::processFilteredPacket`: `push ebp; mov ebp, esp; push -1; push
/// 0x01793b8b; mov eax, fs:[0]`.
pub const PROCESS_FILTERED_PACKET: Site = code_site(
    "Nub::processFilteredPacket",
    0x0158_0840,
    &[
        0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x8B, 0x3B, 0x79, 0x01, 0x64, 0xA1,
    ],
);

/// `UnAckedHandler::queueAckForPacket`: `push -1; push 0x01794bc1; mov eax,
/// fs:[0]`.
pub const QUEUE_ACK: Site = code_site(
    "UnAckedHandler::queueAckForPacket",
    0x0158_cba0,
    &[
        0x6A, 0xFF, 0x68, 0xC1, 0x4B, 0x79, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `Nub::processPacket`: `push -1; push 0x01793aed; mov eax, fs:[0]`.
pub const PROCESS_PACKET: Site = code_site(
    "Nub::processPacket",
    0x0157_fd20,
    &[
        0x6A, 0xFF, 0x68, 0xED, 0x3A, 0x79, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `Nub::processOrderedPacket`: `push -1; mov eax, fs:[0]; push 0x01793681`.
pub const PROCESS_ORDERED_PACKET: Site = code_site(
    "Nub::processOrderedPacket",
    0x0157_c820,
    &[
        0x6A, 0xFF, 0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x68, 0x81, 0x36, 0x79,
    ],
);

/// `Bundle::iterator::unpack`: `push -1; push 0x017932b1; mov eax, fs:[0]`.
pub const BUNDLE_UNPACK: Site = code_site(
    "Bundle::iterator::unpack",
    0x0157_9830,
    &[
        0x6A, 0xFF, 0x68, 0xB1, 0x32, 0x79, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `SequenceManager` `Event_Cache_ElementReady` handler: `push -1; push
/// 0x016faa49; mov eax, fs:[0]` (`ret 8`).
pub const SEQUENCE_CACHE_READY: Site = code_site(
    "SequenceManager Event_Cache_ElementReady",
    0x00d0_6f30,
    &[
        0x6A, 0xFF, 0x68, 0x49, 0xAA, 0x6F, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `SequenceManager` play step (view-distance cull, then instantiate):
/// `push -1; push 0x016faa37; mov eax, fs:[0]` (`ret 0xc`).
pub const SEQUENCE_PLAY: Site = code_site(
    "SequenceManager play step",
    0x00d0_6dd0,
    &[
        0x6A, 0xFF, 0x68, 0x37, 0xAA, 0x6F, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `SequenceManager` Kismet instantiate: `push -1; push 0x016fa9ea; mov
/// eax, fs:[0]` (`ret 0xc`).
pub const SEQUENCE_INSTANTIATE: Site = code_site(
    "SequenceManager Kismet instantiate",
    0x00d0_67e0,
    &[
        0x6A, 0xFF, 0x68, 0xEA, 0xA9, 0x6F, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// The shared prologue of the three `ScriptedDebug` tolua bindings
/// (`Debug:log`, `warn`, `error`): `mov eax, fs:[0]; push -1; push
/// 0x01795fb9; push eax; mov fs:[0], esp; sub esp, 0x28; push esi; mov
/// esi, [esp+0x3c]; lea eax, [esp+4]; push eax; push 0; push 0x0193ffa8`.
/// The last push is the wide string `ScriptedDebug` (the usertype the
/// binding checks `self` against), so the 41 bytes pin both the build and
/// what the function is. The three are registered as `log`, `warn` and
/// `error` at `0x00ad4703`, `0x00ad4713` and `0x00ad4723`.
const SCRIPTED_DEBUG_PROLOGUE: &[u8] = &[
    0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0xB9, 0x5F, 0x79, 0x01, 0x50, 0x64, 0x89,
    0x25, 0x00, 0x00, 0x00, 0x00, 0x83, 0xEC, 0x28, 0x56, 0x8B, 0x74, 0x24, 0x3C, 0x8D, 0x44, 0x24,
    0x04, 0x50, 0x6A, 0x00, 0x68, 0xA8, 0xFF, 0x93, 0x01,
];

/// `ScriptedDebug::log` tolua binding (`client.lua.debug_log`).
pub const SCRIPTED_DEBUG_LOG: Site = code_site(
    "ScriptedDebug log binding",
    0x00aa_1620,
    SCRIPTED_DEBUG_PROLOGUE,
);

/// `ScriptedDebug::warn` tolua binding (`client.lua.debug_log`).
pub const SCRIPTED_DEBUG_WARN: Site = code_site(
    "ScriptedDebug warn binding",
    0x00aa_1710,
    SCRIPTED_DEBUG_PROLOGUE,
);

/// `ScriptedDebug::error` tolua binding (`client.lua.debug_log`).
pub const SCRIPTED_DEBUG_ERROR: Site = code_site(
    "ScriptedDebug error binding",
    0x00aa_1800,
    SCRIPTED_DEBUG_PROLOGUE,
);

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

/// The event-bag getters the ability hooks call (not hook):
/// `GetInt` / `GetFloat` / `GetByte`, `bool thiscall(event, const
/// std::string*, T*)`, `ret 8`. All three share the prologue `push -1; push
/// 0x01708250; mov eax, fs:[0]`; the address tells them apart.
const EVENT_GETTER_PROLOGUE: &[u8] = &[
    0x6A, 0xFF, 0x68, 0x50, 0x82, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
];

/// `GetInt`.
pub const EVENT_GET_INT: Site = code_site("CME event GetInt", 0x00e3_cba0, EVENT_GETTER_PROLOGUE);
/// `GetFloat`.
pub const EVENT_GET_FLOAT: Site =
    code_site("CME event GetFloat", 0x00e3_cc20, EVENT_GETTER_PROLOGUE);
/// `GetByte`.
pub const EVENT_GET_BYTE: Site = code_site("CME event GetByte", 0x00d4_34d0, EVENT_GETTER_PROLOGUE);

/// Every function the gate checks.
pub const CODE_SITES: [Site; 54] = [
    cimmeria_client_hookgate::ENGINE_TICK,
    cimmeria_client_hookgate::DROP_CALLEE,
    BINK_TICK,
    ARCHIVE_ASYNC_SERIALIZE,
    UPDATE_LEVEL_STREAMING_INNER,
    STATIC_LOAD_OBJECT,
    CONSOLE_COMMAND,
    STATE_FIELD_UPDATE,
    ANIM_NOTIFY_A,
    ANIM_NOTIFY_B,
    CME_EVENT_FACTORY,
    BW_MESSAGE,
    REDIRECTOR_SERIALIZE,
    ERROR_SERIALIZE,
    CHECK_FAILED,
    SPAWN_ACTOR,
    DESTROY_ACTOR,
    ENTER_AOI,
    CREATE_ENTITY,
    ENTER_WORLD,
    LEAVE_AOI,
    DESTROY_ENTITY,
    APPEARANCE_REQUEST,
    APPEARANCE_SCHEDULE,
    ENTITY_METHOD,
    ENTITY_PROPERTY,
    QUEUE_REPLAY,
    ROUTE_OUTGOING_RPC,
    PROCESS_FILTERED_PACKET,
    QUEUE_ACK,
    PROCESS_PACKET,
    PROCESS_ORDERED_PACKET,
    BUNDLE_UNPACK,
    SEQUENCE_CACHE_READY,
    SEQUENCE_PLAY,
    SEQUENCE_INSTANTIATE,
    SCRIPTED_DEBUG_LOG,
    SCRIPTED_DEBUG_WARN,
    SCRIPTED_DEBUG_ERROR,
    EFFECT_TIMER,
    EFFECT_LOOKUP,
    EFFECT_ANNOUNCE,
    EFFECT_DATA_REQUEST,
    EFFECT_POST,
    COOLDOWN_TIMER,
    COOLDOWN_UI,
    STAT_HANDLER,
    STAT_BASE_HANDLER,
    STAT_FUNCTOR,
    STAT_BASE_FUNCTOR,
    ON_SEQUENCE,
    EVENT_GET_INT,
    EVENT_GET_FLOAT,
    EVENT_GET_BYTE,
];

const fn code_site(name: &'static str, address: usize, expected: &'static [u8]) -> Site {
    Site {
        name,
        address,
        expected,
        may_be_chained: false,
    }
}
