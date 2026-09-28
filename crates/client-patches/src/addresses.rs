//! Every `SGW.exe` address and structure offset the DLL relies on.
//!
//! All are for the QA build of `SGW.exe` (image base `0x00400000`, ASLR off
//! via `AtreaFixASLR.bat`). A different build gets different addresses,
//! which is what [`crate::fingerprint`] checks before anything is hooked.
//!
//! Evidence: `docs/reverse-engineering/findings/black-market-client-io.md`
//! §2 (receive contract) and §1 (the `GameEntityManager` fields), and
//! `docs/reverse-engineering/findings/black-market-client-window-patch.md`
//! (the UI `lua_State` chain). The four hooked or called functions' first
//! bytes were also read from the shipped `SGW.exe` when this crate was
//! written; they are the expected prologues in [`crate::fingerprint`], as
//! are four small engine functions whose bytes pin the send side's data
//! offsets.

// ── functions ────────────────────────────────────────────────────────────

/// `Client_NetIn_EntityMethodDispatch`:
/// `__thiscall(void* this, Entity* entity, u32 msgId, BinaryIStream* stream)`,
/// `RET 0xC`. Every inbound entity method on a known entity comes through
/// here, on a Mercury network thread. It decodes the method index, looks
/// up a handler, and on a miss calls [`GET_EXPOSED_CLIENT_METHOD_BY_INDEX`]
/// and returns with the arguments still unread in `stream`. Its prologue
/// sets up an MSVC C++ exception frame.
pub const CLIENT_NET_IN_ENTITY_METHOD_DISPATCH: usize = 0x00c6_f8f0;

/// `EntityDescription_GetExposedClientMethodByIndex`, the drop callee:
/// `__thiscall(void* methodTable /* desc + 0xe0 */, int index) ->
/// MethodDescription*`, `RET 4`. Its only caller is the dispatcher's
/// not-found path, whose result it ignores. The telemetry DLL hooks it as
/// its silent-drop oracle.
pub const GET_EXPOSED_CLIENT_METHOD_BY_INDEX: usize = 0x0159_0f30;

/// `FEngineLoop::Tick`: `__thiscall(FEngineLoop* this)`, no stack
/// arguments. The main game thread's per-frame driver; the telemetry DLL
/// hooks it too.
pub const FENGINE_LOOP_TICK: usize = 0x0041_6ec0;

/// `ServerConnection::startEntityMessage`: `__thiscall(conn, u8 idx, u32
/// entityId) -> Bundle*`, `RET 8`. Called by the send natives with
/// `idx = 0x3D` and `entityId = 0` (the local avatar). It ORs `0x80` into
/// the message id, starts the message on the channel bundle, writes the
/// 4-byte entity id, and returns the bundle. When the connection is offline
/// (`[conn + 0x30c] == 0`) it logs an error and starts the message anyway,
/// so the caller checks first. Its prologue sets up an MSVC C++ exception
/// frame.
pub const SERVER_CONNECTION_START_ENTITY_MESSAGE: usize = 0x00dd_6a60;

/// `ServerConnection::startAvatarMessage` (`FUN_00dd8010`): `mov eax,
/// [esp+4]; push 0; push eax; call startEntityMessage; ret 4`. Not called:
/// fingerprinted because its bytes pin the engine's own use of
/// [`SERVER_CONNECTION_START_ENTITY_MESSAGE`], `(conn, idx, 0)`, which the
/// send natives copy.
pub const SERVER_CONNECTION_START_AVATAR_MESSAGE: usize = 0x00dd_8010;

/// `ServerConnection::isOnline` (`FUN_00dd6130`): `xor eax, eax; cmp
/// [ecx+0x30c], eax; setne al; ret`. Not called: fingerprinted because its
/// bytes pin [`CONN_ONLINE`], which the send natives read directly.
pub const SERVER_CONNECTION_IS_ONLINE: usize = 0x00dd_6130;

/// `GameEntityManager` getter (`FUN_00dd05a0`): `mov eax, [0x01ef244c];
/// ret`. Not called: fingerprinted because its bytes pin
/// [`GAME_ENTITY_MANAGER`].
pub const GAME_ENTITY_MANAGER_GET: usize = 0x00dd_05a0;

/// The engine's write-one-byte helper (`FUN_00c701a0`), which calls
/// `bundle->vtable[0x10](1)` and stores the byte. Not called:
/// fingerprinted because its bytes pin [`BUNDLE_RESERVE`], the vtable slot
/// the send natives call.
pub const BUNDLE_WRITE_U8: usize = 0x00c7_01a0;

// ── the local player ─────────────────────────────────────────────────────

/// Pointer to the `GameEntityManager` singleton (`FUN_00dd05a0` is
/// `mov eax, [0x01ef244c]; ret`).
pub const GAME_ENTITY_MANAGER: usize = 0x01ef_244c;

/// `GameEntityManager + 0x14`: the local player's entity id. The engine's
/// own sender compares it with `Entity + 0x0c` to decide whether it is
/// sending as the local player.
pub const GEM_LOCAL_PLAYER_ID: usize = 0x14;

/// `GameEntityManager + 0x08`: the `ServerConnection*`. Null until the
/// client has connected.
pub const GEM_SERVER_CONNECTION: usize = 0x08;

/// `ServerConnection + 0x30c`: non-zero while the connection is up. The
/// engine's own sender (`FUN_00c6fc40`) refuses to send when it is 0.
pub const CONN_ONLINE: usize = 0x30c;

/// `Entity + 0x0c`: the entity id.
pub const ENTITY_ID: usize = 0x0c;

// ── MethodDescription ────────────────────────────────────────────────────

/// `MethodDescription + 0x04`: the name, an MSVC `std::string`. Its
/// 16-byte buffer union starts here: the characters themselves while the
/// capacity is below [`MSVC_SSO_CAPACITY`], a pointer to them otherwise.
pub const METHOD_NAME_BUFFER: usize = 0x04;

/// `MethodDescription + 0x14`: the name's length (`_Mysize`).
pub const METHOD_NAME_LEN: usize = 0x14;

/// `MethodDescription + 0x18`: the name's capacity (`_Myres`).
pub const METHOD_NAME_CAPACITY: usize = 0x18;

/// MSVC's small-string buffer size for `char`: a capacity below this means
/// the characters are stored inline.
pub const MSVC_SSO_CAPACITY: u32 = 16;

// ── BinaryIStream ────────────────────────────────────────────────────────

/// `BinaryIStream` vtable `+0x04`: `const void* retrieve(int n)`,
/// `__thiscall`. Returns a pointer to the next `n` bytes and consumes them.
/// Reading past the end is not a clean failure, so callers check
/// [`ISTREAM_REMAINING_LENGTH`] first.
pub const ISTREAM_RETRIEVE: usize = 0x04;

/// `BinaryIStream` vtable `+0x08`: `int remainingLength()`, `__thiscall`.
pub const ISTREAM_REMAINING_LENGTH: usize = 0x08;

// ── Bundle (BinaryOStream) ───────────────────────────────────────────────

/// Bundle vtable `+0x10`: `void* reserve(int n)`, `__thiscall`. Returns a
/// writable pointer to the next `n` bytes of the message being built.
pub const BUNDLE_RESERVE: usize = 0x10;

/// The message id `startEntityMessage` is given for an extended entity
/// method: `SGWPlayer`'s first extended index, `0x3D`. The engine ORs in
/// `0x80`, so the wire id is `0xBD`, and the sub-index byte follows.
pub const EXTENDED_METHOD_ID: u8 = 0x3D;

/// The entity id `startEntityMessage` takes for the local avatar.
pub const LOCAL_AVATAR: u32 = 0;

// ── the UI Lua state ─────────────────────────────────────────────────────

/// `g_SGWUIManager_ptr`. The UI `lua_State` is
/// `*(*(*(0x01ee2a58) + 0x10))`.
pub const SGW_UI_MANAGER: usize = 0x01ee_2a58;

/// `SGWUIManager + 0x10`: pointer to the slot holding the `lua_State*`.
pub const UI_MANAGER_LUA_SLOT: usize = 0x10;

/// `lua_State + 0x04`: the `tt` type tag of the thread object. Compare the
/// **byte**: the dword at this offset also holds the GC mark bits.
pub const LUA_STATE_TT: usize = 0x04;

/// `LUA_TTHREAD`, the tag a live `lua_State` carries.
pub const LUA_TTHREAD: u8 = 0x08;
