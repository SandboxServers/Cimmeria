# Wireclient in CI: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `rust-gameserver-dev` for WC-12, WC-14 and WC-15, and `packet-reviewer`
> reviewers (Sonnet). Ledger, findings (F1 to F18) and decisions (D-WC1 to
> D-WC12): [README.md](README.md).
>
> PowerShell only. Every compiling command goes through the lane, in this
> order, from the worktree root:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p cimmeria-wireclient
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p cimmeria-wireclient --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-wireclient
> ```
>
> The no-DB nextest run skips the live-DB tests. Once WC-03 has merged, the
> live-DB tier for this crate is
> `pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 --wireclient <test-name substring>`;
> before that, a packet that needs it says how. Read the lane's summary and
> its failures file; do not rerun a build to see the output.
>
> Rust rules for every packet: no `unwrap()` outside tests; `#[derive(Debug)]`
> on every wire type; explicit integer widths (`u32`, `i32`, `u8`) for wire
> fields; comments say why, not what; `#[cfg(test)] mod tests` last in a file;
> no new file over 500 lines. Test names say what they prove. Every test that
> needs a database has `live_db` in its fn or module name.

## Contents

- [Contract](#contract)
- [WC-01 Strict bundle decode](#wc-01-strict-bundle-decode)
- [WC-02 Tap fixture loader and client-call builders](#wc-02-tap-fixture-loader-and-client-call-builders)
- [WC-03 Runner: nextest profile and --wireclient mode](#wc-03-runner-nextest-profile-and---wireclient-mode)
- [WC-04 tests/it live-DB isolation](#wc-04-testsit-live-db-isolation)
- [WC-05 Semantic decoder skeleton and dialog family](#wc-05-semantic-decoder-skeleton-and-dialog-family)
- [WC-06 Mission family decoders](#wc-06-mission-family-decoders)
- [WC-07 Ability, sequence and movie decoders](#wc-07-ability-sequence-and-movie-decoders)
- [WC-08 Inventory family decoders](#wc-08-inventory-family-decoders)
- [WC-09 Entity-introduction and chat decoders](#wc-09-entity-introduction-and-chat-decoders)
- [WC-10 Character creation on the wire, world entry recorded](#wc-10-character-creation-on-the-wire-world-entry-recorded)
- [WC-11 Entity mirror and query](#wc-11-entity-mirror-and-query)
- [WC-12 ScriptSession](#wc-12-scriptsession)
- [WC-13 CI workflow](#wc-13-ci-workflow)
- [WC-14 Praxis-start end-to-end test](#wc-14-praxis-start-end-to-end-test)
- [WC-15 First-login flush-shape guard](#wc-15-first-login-flush-shape-guard)
- [WC-16 Shakedown and gate](#wc-16-shakedown-and-gate)
- [WC-17 Close-out](#wc-17-close-out)

## Contract

Parallel packets build against these names. A packet that needs to change
one stops and tells the coordinator.

### Module layout of `crates/wireclient/src/`

```text
lib.rs            + pub mod calls; pub mod mirror; pub mod script; pub mod semantic; pub mod tap_fixture;
bundle.rs         WC-01: strict decode, DecodeError, S2CMessage gains sub_index + offset
calls.rs          WC-02: client-call builders (impl GameSession), write_wstring, CLIENT_CALL_ENTITY_ID
tap_fixture.rs    WC-02: serde types for the lab packet-tap JSON, the Praxis fixture
semantic/
  mod.rs          WC-05: S2CEvent, TargetKind, SemanticError, decode_event dispatcher
  primitives.rs   WC-05: Reader
  dialog.rs       WC-05: onDialogDisplay (105), InteractionType (3)
  mission.rs      WC-06: onMissionUpdate (80), onStepUpdate (81), onObjectiveUpdate (82)
  sequence.rs     WC-07: onSequence (1), onKnownAbilitiesUpdate (101), onPlayMovie (155)
  inventory.rs    WC-08: onActiveSlotUpdate (70), onRemoveItem (71), onUpdateItem (72)
  entity.rs       WC-09: onStaticMeshNameUpdate (0), onEntityProperty (7), onVisible (8),
                         onBeingNameIDUpdate (11), onPlayerCommunication (28)
mirror/
  mod.rs          WC-11: EntityMirror, MirroredEntity, MirrorError
  query.rs        WC-11: EntityQuery
  tests.rs        WC-11
script.rs         WC-12: ScriptSession, Observed
world_entry.rs    WC-10: open_character_list / create_character / play split, entry_bundles
error.rs          WC-01, WC-10, WC-12 each add the variants named below
```

### `bundle.rs` (WC-01)

```rust
pub struct S2CMessage {
    pub msg_id: u8,
    pub entity_id: Option<u32>,
    pub class_id: Option<u8>,
    /// Unchanged meaning: direct index, or sub-slot + 61 for 0xBD.
    pub method_index: Option<u16>,
    /// NEW. The raw 0xBD sub-slot byte; None for every other message.
    pub sub_index: Option<u8>,
    /// NEW. Offset of this message's id byte within the reassembled bundle.
    pub offset: usize,
    pub payload: Bytes,
}
impl S2CMessage {
    /// Entity-method arguments: payload after the 4-byte entity id, and after
    /// the sub-slot byte for 0xBD. None for static (0x00..=0x7F, 0xFF) messages.
    pub fn args(&self) -> Option<&[u8]>;
    /// The method index under the target's idbase (61 player, 62 NPC).
    pub fn method_index_for(&self, idbase: u8) -> Option<u16>;
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("unknown msg id {msg_id:#04x} at offset {offset}")]
    UnknownMsgId { msg_id: u8, offset: usize },
    #[error("msg {msg_id:#04x} at offset {offset} needs {need} bytes, {have} left")]
    Truncated { msg_id: u8, offset: usize, need: usize, have: usize },
}
pub fn decode_bundle_strict(body: &Bytes) -> Result<Vec<S2CMessage>, DecodeError>;
pub fn decode_bundle(body: &Bytes) -> Vec<S2CMessage>; // unchanged behaviour
```

`error.rs` gains `#[error("bundle decode: {0}")] Decode(#[from] crate::bundle::DecodeError)`.

### `tap_fixture.rs` (WC-02)

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TapCapture { pub capacity: u32, pub count: u32, pub dropped: u32,
                        pub entity_id: u32, pub messages: Vec<TapRecord> }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TapDir { In, Out }
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TapRecord {
    pub ts_ms: u64, pub dir: TapDir, pub msg_id: Option<u8>, pub msg_name: String,
    pub method_index: i32, pub target_entity_id: Option<u32>,
    pub args_hex: String, pub args_len: usize,
    #[serde(default)] pub decoded: Option<serde_json::Value>,
}
impl TapCapture {
    pub fn parse(json: &str) -> crate::Result<TapCapture>;   // Error::TraceJson on bad JSON
    /// include_str!("../tests/fixtures/praxis_start_tap.json"), parsed. Panics on a bad fixture.
    pub fn praxis_start() -> TapCapture;
    /// Record `i`, asserting its msg_name is `name` (catches a fixture that moved).
    pub fn expect(&self, i: usize, name: &str) -> &TapRecord;
}
impl TapRecord { pub fn args(&self) -> Vec<u8>; } // hex::decode, panics on bad hex
```

### `calls.rs` (WC-02)

```rust
/// The entity id the real client puts in a cell call's prefix (README F6).
pub const CLIENT_CALL_ENTITY_ID: u32 = 0;
pub const CM_MOVE_ITEM: u16 = 38;
pub const CM_INTERACT: u16 = 74;
pub const CM_DIALOG_BUTTON_CHOICE: u16 = 75;
pub const CM_TRIGGER_CLIENT_HINTED_GENERIC_REGION: u16 = 85;
pub const CM_CANCEL_MOVIE: u16 = 108;
pub const CM_GM_GOTO_XYZ: u16 = 163;
pub const MSG_REQUEST_ENTITY_UPDATE: u8 = 0x07;
pub const BASE_CREATE_CHARACTER: u8 = 0xC3;
/// WSTRING: u32 count of UTF-16 code units, then the units little-endian.
pub fn write_wstring(out: &mut Vec<u8>, s: &str);
impl GameSession {
    pub fn interact(entity_id: u32, override_target: i32) -> Vec<u8>;
    pub fn dialog_button_choice(entity_id: u32, dialog_id: i32, button_id: i32) -> Vec<u8>;
    pub fn gm_goto_xyz(entity_id: u32, pos: [f32; 3]) -> Vec<u8>;
    pub fn trigger_client_hinted_generic_region(entity_id: u32, region_id: i32, entering: bool, pos: [f32; 3]) -> Vec<u8>;
    /// `target_slot_wire` is 1-based, as the client sends it.
    pub fn move_item(entity_id: u32, item_id: i32, target_bag: i32, target_slot_wire: i32, quantity: i32) -> Vec<u8>;
    pub fn cancel_movie(entity_id: u32, movie_name: &str) -> Vec<u8>;
    /// `[u32 entity_id]`, no cache stamps (this client build sends none).
    pub fn request_entity_update(entity_id: u32) -> Vec<u8>;
    pub fn create_character(name: &str, extra_name: &str, char_def_id: i32,
                            visual_choices: &[(i32, i32)], skin_tint_color_id: i32) -> Vec<u8>;
}
```

`session.rs`'s private `word_len_msg` becomes `pub(crate)`.

### `semantic/` (WC-05 defines all of it; WC-06 to WC-09 fill one file each)

```rust
/// Which method table a target uses above index 26 (the shared
/// SGWSpawnableEntity/SGWBeing range is the same for both).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind { Player, Npc }

#[derive(Debug, Clone, PartialEq)]
pub enum S2CEvent {
    // dialog.rs (WC-05)
    DialogDisplay(DialogDisplay),                                   // 105, Player
    InteractionType { type_id: u64 },                               // 3, shared
    // mission.rs (WC-06)
    MissionUpdate { mission_id: i32, status: i8, giver_name_id: i32 }, // 80, Player
    StepUpdate { step_id: i32, status: i8 },                        // 81, Player
    ObjectiveUpdate { objective_id: i32, status: i8, hidden: i8, optional: i8 }, // 82, Player
    // sequence.rs (WC-07)
    Sequence(Sequence),                                             // 1, shared
    KnownAbilitiesUpdate { ability_ids: Vec<i32> },                 // 101, Player
    PlayMovie { movie_name: String, full_screen: u8 },              // 155, Player
    // inventory.rs (WC-08)
    ActiveSlotUpdate { bag_id: i32, slot_id: i32 },                 // 70, Player
    RemoveItem { item_ids: Vec<i32> },                              // 71, Player
    UpdateItem { items: Vec<InvItem> },                             // 72, Player
    // entity.rs (WC-09)
    StaticMeshNameUpdate { static_mesh: String, body_set: String }, // 0, shared
    EntityProperty { property: i32, value: i32 },                   // 7, shared
    Visible { visible: i8 },                                        // 8, shared
    BeingNameIdUpdate { name_id: i32 },                             // 11, shared
    PlayerCommunication { speaker: String, speaker_flags: u8, channel: u8, text: String }, // 28, Player
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogDisplay { pub entity_id: i32, pub dialog_id: i32, pub mission_flags: i32,
                           pub is_immediate: u8, pub mission_id: i32 }
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence { pub kismet_event_set_seq_id: i32, pub source_id: i32, pub target_id: i32,
                      pub primary_target: i8, pub impact_time: f32, pub nvp_count: u32,
                      pub view_type: i8, pub instance_id: i32 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvItem { pub id: i32, pub dbid: i32, pub stack_size: i32, pub slot_id: i32,
                     pub container_id: i32, pub is_bound: bool, pub durability: i32,
                     pub ammo_types: Vec<i32>, pub cur_ammo_type: i32, pub charges: i32 }

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SemanticError {
    #[error("{method}: need {need} bytes at {at}, args are {len}")]
    Short { method: &'static str, at: usize, need: usize, len: usize },
    #[error("{method}: {extra} trailing bytes")]
    Trailing { method: &'static str, extra: usize },
    #[error("{method}: invalid UTF-16 in a WSTRING")]
    BadWstring { method: &'static str },
    #[error("{method}: {what} not decoded")]
    Unsupported { method: &'static str, what: &'static str },
    #[error("{method}: decoder not written yet")]
    NotImplemented { method: &'static str },
}

/// Ok(None): no decoder for this index on this kind (not an error).
/// Err: a decoder exists and the bytes do not fit it exactly (D-WC5).
pub fn decode_event(kind: TargetKind, method_index: u16, args: &[u8])
    -> Result<Option<S2CEvent>, SemanticError>;
```

`semantic/primitives.rs`:

```rust
pub struct Reader<'a> { method: &'static str, buf: &'a [u8], at: usize }
impl<'a> Reader<'a> {
    pub fn new(method: &'static str, buf: &'a [u8]) -> Self;
    pub fn u8(&mut self) -> Result<u8, SemanticError>;
    pub fn i8(&mut self) -> Result<i8, SemanticError>;
    pub fn i32(&mut self) -> Result<i32, SemanticError>;
    pub fn u32(&mut self) -> Result<u32, SemanticError>;
    pub fn u64(&mut self) -> Result<u64, SemanticError>;
    pub fn f32(&mut self) -> Result<f32, SemanticError>;
    pub fn wstring(&mut self) -> Result<String, SemanticError>;   // u32 count + UTF-16LE units
    pub fn array_i32(&mut self) -> Result<Vec<i32>, SemanticError>; // u32 count + i32s
    /// Err(Trailing) unless every byte was read.
    pub fn finish(self) -> Result<(), SemanticError>;
}
```

All little-endian. A count that would read past the end is `Short`, checked
before allocating (a forged count must not allocate gigabytes).

### Wire layouts the decoders implement

From [client-method-dispatch-table.md](../../protocol/client-method-dispatch-table.md);
"Fixture" is the record index in `praxis_start_tap.json` (outbound
`args_hex` holds the arguments only, README F9).

| Index | Method | Kind | Arguments in order | Fixture |
|---|---|---|---|---|
| 0 | `onStaticMeshNameUpdate` | shared | WSTRING StaticMeshName, WSTRING BodySetName | none |
| 1 | `onSequence` | shared | INT32 KismetEventSetSeqID, INT32 SourceID, INT32 TargetID, INT8 PrimaryTarget, FLOAT ImpactTime, ARRAY\<NameValuePair\> (u32 count), INT8 ViewType, INT32 InstanceId | #66, #72, #77 |
| 3 | `InteractionType` | shared | UINT64 TypeId | #21, #22, #44 |
| 7 | `onEntityProperty` | shared | INT32 type, INT32 value | #73 |
| 8 | `onVisible` | shared | INT8 visible | none |
| 11 | `onBeingNameIDUpdate` | shared | INT32 BeingNameID | none |
| 28 | `onPlayerCommunication` | Player | WSTRING Speaker, UINT8 SpeakerFlags, UINT8 Channel, WSTRING Text | #5, #36, #50 to #53 |
| 70 | `onActiveSlotUpdate` | Player | INT32 BagId, INT32 SlotId | none |
| 71 | `onRemoveItem` | Player | ARRAY\<INT32\> ItemIdList | none |
| 72 | `onUpdateItem` | Player | ARRAY\<InvItem\>; InvItem = INT32 id, INT32 dbid, INT32 stackSize, INT32 slotID, INT32 containerID, UINT8 isBound, INT32 durability, ARRAY\<INT32\> ammoTypes, INT32 curAmmoType, INT32 charges (`cimmeria_entity::inventory::InvItem::serialize`) | none |
| 80 | `onMissionUpdate` | Player | INT32 MissionID, INT8 Status, INT32 MissionGiverName | #28, #69 |
| 81 | `onStepUpdate` | Player | INT32 StepID, INT8 Status | #25, #26, #29, #46, #47, #68 |
| 82 | `onObjectiveUpdate` | Player | INT32 ObjectiveID, INT8 Status, INT8 Hidden, INT8 Optional | #23, #24, #27, #30, #45, #48, #67 |
| 101 | `onKnownAbilitiesUpdate` | Player | ARRAY\<INT32\> AbilityData | #49, #71 |
| 105 | `onDialogDisplay` | Player | INT32 EntityId, INT32 DialogID, INT32 MissionFlags, UINT8 IsImmediate, INT32 aMissionId | #20, #43, #54 |
| 155 | `onPlayMovie` | Player | WSTRING MovieName, UINT8 FullScreen | none |

`onSequence` with a non-zero NameValuePair count returns
`Unsupported { what: "NameValuePairs" }`: the pair layout is not needed for
this stage. Fixture #6 (index 27 on NPC 100745) decodes to `Ok(None)` for
`TargetKind::Npc`.

### `mirror/` (WC-11)

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct MirroredEntity {
    pub entity_id: u32, pub class_id: u8,
    pub position: Option<[f32; 3]>,        // from 0x10 only; see WC-11
    pub static_mesh: Option<String>, pub body_set: Option<String>,
    pub name_id: Option<i32>, pub interaction_type: Option<u64>,
    pub properties: std::collections::BTreeMap<i32, i32>,
    pub visible: Option<i8>, pub hidden: bool,
    /// Order of introduction, 0-based, across the session.
    pub introduced: u64,
}
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MirrorError {
    #[error("entity {entity_id} method {method_index}: {source}")]
    Semantic { entity_id: u32, method_index: u16, source: crate::semantic::SemanticError },
    #[error("no entity matches {query}")]
    NotFound { query: String },
    #[error("{query} matches {ids:?}")]
    Ambiguous { query: String, ids: Vec<u32> },
}
#[derive(Debug, Default)]
pub struct EntityMirror { /* own_id, entities: BTreeMap<u32, MirroredEntity>, inventory: BTreeMap<i32, InvItem>, orphans: Vec<S2CMessage>, next_intro: u64 */ }
impl EntityMirror {
    pub fn new() -> Self;
    pub fn set_own_id(&mut self, id: u32);
    pub fn own_id(&self) -> Option<u32>;
    pub fn get(&self, entity_id: u32) -> Option<&MirroredEntity>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    /// The player's inventory, by item instance id.
    pub fn inventory(&self) -> &std::collections::BTreeMap<i32, InvItem>;
    /// Entity methods that arrived for an entity the mirror does not hold.
    pub fn orphans(&self) -> &[S2CMessage];
    /// Apply one message. Returns its semantic event, if any, for the caller's log.
    pub fn apply(&mut self, msg: &S2CMessage) -> Result<Option<S2CEvent>, MirrorError>;
    pub fn find(&self, q: &EntityQuery) -> Vec<&MirroredEntity>;
    /// Exactly one match, or NotFound / Ambiguous.
    pub fn find_one(&self, q: &EntityQuery) -> Result<&MirroredEntity, MirrorError>;
}
// mirror/query.rs
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityQuery {
    pub static_mesh: Option<String>, pub name_id: Option<i32>, pub class_id: Option<u8>,
    pub near: Option<([f32; 3], f32)>, pub interaction_nonzero: bool,
}
impl EntityQuery {
    pub fn static_mesh(mesh: &str) -> Self;
    pub fn name_id(self, id: i32) -> Self;
    pub fn class_id(self, class: u8) -> Self;
    pub fn near(self, pos: [f32; 3], radius: f32) -> Self;
    pub fn interactable(self) -> Self;      // interaction_nonzero = true
    pub fn matches(&self, e: &MirroredEntity) -> bool;
}
impl std::fmt::Display for EntityQuery { /* "mesh=... name_id=... near=(x,y,z)±r" */ }
```

`error.rs` gains `#[error("mirror: {0}")] Mirror(#[from] crate::mirror::MirrorError)`.

### `script.rs` (WC-12)

```rust
#[derive(Debug, Clone)]
pub struct Observed { pub at: std::time::Duration, pub msg: S2CMessage, pub event: Option<S2CEvent> }
pub struct ScriptSession { pub session: GameSession, pub mirror: EntityMirror, /* log, cursor, started, last_keepalive */ }
impl ScriptSession {
    pub const KEEPALIVE: Duration = Duration::from_millis(250);
    /// Feeds `session.entry_bundles` (WC-10) through the mirror into the log first.
    pub fn new(session: GameSession) -> crate::Result<Self>;
    pub fn log(&self) -> &[Observed];
    /// Concatenate `msgs` into one reliable bundle, as the client batches its queue.
    pub async fn send(&mut self, msgs: &[Vec<u8>]) -> crate::Result<()>;
    /// Receive for up to `max`: strict decode, mirror, log; keep-alive as due. Returns messages added.
    pub async fn pump(&mut self, max: Duration) -> crate::Result<usize>;
    /// First logged entry at or after the cursor matching `pred`; the cursor moves past it.
    pub async fn wait_event(&mut self, what: &str, timeout: Duration,
                            pred: impl FnMut(&Observed) -> bool) -> crate::Result<Observed>;
    /// Each step must match, in this order, gaps allowed; one deadline for all.
    pub async fn expect_in_order(&mut self, what: &str, timeout: Duration,
                                 steps: &mut [(&str, &mut dyn FnMut(&Observed) -> bool)])
        -> crate::Result<Vec<Observed>>;
    /// Pump until exactly one entity matches `q`; returns its id.
    pub async fn wait_entity(&mut self, q: &EntityQuery, timeout: Duration) -> crate::Result<u32>;
    /// The last `n` log entries, one line each, for failure messages.
    pub fn trail(&self, n: usize) -> String;
}
```

`error.rs` gains `#[error("script: waited {waited:?} for {what}; last events:\n{trail}")] ScriptTimeout { what: String, waited: Duration, trail: String }`.

### `world_entry.rs` (WC-10)

```rust
impl GameSession {
    /// AUTHENTICATE + ENABLE_ENTITIES, then the character-list bundle.
    pub async fn open_character_list(&mut self, recv_timeout: Duration) -> Result<()>;
    /// createCharacter (0xC3); Ok on onCharacterList (0x82), Err(CharacterCreateFailed) on 0x83.
    pub async fn create_character(&mut self, name: &str, char_def_id: i32,
        visual_choices: &[(i32, i32)], recv_timeout: Duration) -> Result<()>;
    /// playCharacter .. onClientReady; sets player_entity_id.
    pub async fn play(&mut self, player_id: i32, recv_timeout: Duration) -> Result<()>;
    /// Unchanged signature: open_character_list then play.
    pub async fn enter_world(&mut self, player_id: i32, recv_timeout: Duration) -> Result<()>;
}
// session.rs: GameSession gains
/// Every non-tickSync bundle world entry consumed, in arrival order (WC-10).
pub entry_bundles: Vec<Bytes>,
```

`error.rs` gains `#[error("createCharacter refused, code {0}")] CharacterCreateFailed(i32)`.

### Test support in `crates/wireclient/tests/it/support/` (WC-04, WC-10, WC-14)

```rust
pub fn db_url() -> String;                    // WC-04: cimmeria_test_support::database_url(), expect
pub fn init_test_logging();                   // WC-04: once; fmt test writer + EnvFilter
pub async fn insert_sentinel_account_with_level(pool: &PgPool, account_id: i32, name: &str, access_level: i32); // WC-10
pub async fn default_visual_choices(pool: &PgPool, char_def_id: i32) -> Vec<(i32, i32)>;   // WC-10
pub async fn cleanup_account(pool: &PgPool, account_id: i32);                               // WC-10
pub async fn template_query(pool: &PgPool, template_id: i32) -> EntityQuery;                // WC-14
```

Sentinel ids for this campaign: block `0x7000_9C00` to `0x7000_9CFF` (unused
on `main` @ `c05c0638a`). WC-10 uses `0x7000_9C01`, WC-14 `0x7000_9C02`,
WC-15 `0x7000_9C03`.

---

## WC-01 Strict bundle decode

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** none.
**Branch:** `wireclient-ci/wc01-strict-decode`. **Worktree:** `wc01`.
**Subject:** `feat(wireclient): WC-01 strict bundle decode with message offsets and the raw 0xBD sub-slot`

Why: README F1 and F2.

Files:

1. `crates/wireclient/src/bundle.rs`
   - Add `sub_index` and `offset` to `S2CMessage` (contract), with doc
     comments. Set `offset = msg_start` for every message; set `sub_index`
     to `Some(payload[4])` for `0xBD` with a payload of 5 bytes or more,
     else `None`. Keep `method_index` exactly as today.
   - Add `DecodeError` (contract) and move the loop body into
     `fn walk(body: &Bytes) -> (Vec<S2CMessage>, Option<DecodeError>)`, which
     returns what it decoded plus the error that stopped it. Every `break`
     becomes a `DecodeError`: `UnknownMsgId` for the `0x38..=0x7F` gap,
     `Truncated` (with `need` and `have`) for a header or payload that runs
     past the end.
   - `pub fn decode_bundle_strict(body) -> Result<Vec<S2CMessage>, DecodeError>`:
     `walk`, `Err` when it stopped early.
   - `pub fn decode_bundle(body) -> Vec<S2CMessage>`: `walk`, log the error
     with the existing `tracing::warn!` line (keep its fields, add
     `reason = "unknown_msg_id" | "truncated"`), return the prefix. Callers
     that rely on the lenient form (`two_client_*` tests, `sparbot`) keep
     working.
   - `S2CMessage::args()` and `method_index_for(idbase)` (contract):
     `args` is `payload[4..]` for `0x80..=0xFE` except `0xBD`, `payload[5..]`
     for `0xBD`, `None` otherwise or when the payload is shorter.
     `method_index_for` is `msg_id & 0x7F` for direct encoding and
     `sub_index + idbase` for `0xBD`.
2. `crates/wireclient/src/error.rs`: add `Error::Decode` (contract).
3. Struct-literal sites that must add the two fields: `src/sparbot.rs`
   (`method_msg`, about line 342) and `tests/it/sparbot_duel.rs` (about line
   72). Use `sub_index: None, offset: 0` unless the literal builds a `0xBD`
   message (then `Some(<the sub byte>)`). Grep `S2CMessage {` to confirm no
   others.

Tests (unit, in `bundle.rs`'s `mod tests`; TESTING.md type 2, wire-format):

- `strict_decode_rejects_an_unknown_msg_id_with_its_offset`: a valid
  `0x02` message then a `0x40` byte; expect
  `Err(UnknownMsgId { msg_id: 0x40, offset: 2 })`, and `decode_bundle` on the
  same bytes returns 1 message. Fails if the strict path falls back to the
  lenient prefix.
- `strict_decode_rejects_a_word_header_cut_after_its_first_length_byte`:
  `[0x09, 0x08]` (a `CREATE_ENTITY` whose length is cut); expect
  `Truncated { msg_id: 0x09, offset: 0, need: 2, have: 1 }`.
- `strict_decode_rejects_a_payload_past_the_end`: `CREATE_ENTITY` declaring
  8 bytes with 6 present; `Truncated { need: 8, have: 6 }`.
- `offsets_and_args_cover_direct_and_extended_methods`: a bundle of `0x02`
  (2 bytes), a direct method (`0x80 | 12`, entity 123, args `b"hi"`), and a
  `0xBD` method (entity 123, sub 44, args `[1, 2]`); assert offsets
  `[0, 2, 11]`, `args()` `b"hi"` and `[1, 2]`, `sub_index` `None` and
  `Some(44)`, `method_index_for(61) == Some(105)` and
  `method_index_for(62) == Some(106)` for the `0xBD` one.
- The existing seven tests stay green.

Checks: the three lane commands in the header.

Docs owed: none in this packet (WC-17 updates the ADR's `bundle.rs` line).

Reviewer focus: no behaviour change in `decode_bundle` for valid bundles;
`offset` is the id byte's position, not the payload's; `Truncated` values are
right at every exit.

## WC-02 Tap fixture loader and client-call builders

**Implementer:** packet-coder. **Size:** M. **Wave:** 1. **Depends on:** none (D-WC10 proposed).
**Branch:** `wireclient-ci/wc02-calls`. **Worktree:** `wc02`.
**Subject:** `feat(wireclient): WC-02 Praxis tap fixture loader and client-call builders pinned to the capture`

Why: the script needs builders for the ten calls, and the decoder packets
need the fixture's bytes. README F6, F7, F9, F12.

Files:

1. New `crates/wireclient/src/tap_fixture.rs` (contract). Module doc: what
   the fixture is (the lab packet tap for one new Praxis character, colo,
   2026-10-10; ADR § Praxis start) and F9 (inbound keeps the prefix,
   outbound is arguments only). `praxis_start()` uses
   `include_str!("../tests/fixtures/praxis_start_tap.json")`.
2. New `crates/wireclient/src/calls.rs` (contract). Each builder is
   `GameSession::cell_method(<CM_*>, entity_id, &args)` with `args` in `.def`
   order (indices and argument lists from
   [cell-method-dispatch-table.md](../../protocol/cell-method-dispatch-table.md)
   rows 38, 74, 75, 85, 108, 163):
   - `interact`: `INT32 overrideTarget`.
   - `dialog_button_choice`: `INT32 dialogId, INT32 buttonId`.
   - `gm_goto_xyz`: `FLOAT x, y, z`.
   - `trigger_client_hinted_generic_region`: `INT32 id, UINT8 bEntering, VECTOR3 position` (three f32).
   - `move_item`: `INT32 itemId, INT32 targetBag, INT32 targetSlot, INT32 quantity`.
   - `cancel_movie`: `WSTRING movieName`.
   - `request_entity_update`: `word_len_msg(0x07, &entity_id.to_le_bytes())`.
   - `create_character`: `base_method(0xC3, ..)` with
     `[WSTRING Name][WSTRING ExtraName][INT32 CharDefId][u32 count][count x (INT32 VisGroupId, INT32 ChoiceId)][INT32 SkinTintColorID]`,
     the layout `handle_create_character` parses
     (`crates/base/src/base/character_create/mod.rs`).
   A doc comment on `CLIENT_CALL_ENTITY_ID` cites README F6.
3. `crates/wireclient/src/session.rs`: `word_len_msg` becomes `pub(crate)`.
4. `crates/wireclient/src/lib.rs`: `pub mod calls; pub mod tap_fixture;`
   and one line each in the crate doc's scope list.

Tests:

- In `calls.rs` (unit, wire-format, type 2). Each pins a builder against the
  real client's bytes: build with `CLIENT_CALL_ENTITY_ID`, then assert
  `out[0] == rec.msg_id.unwrap()`, `u16::from_le_bytes([out[1], out[2]]) as usize == rec.args_len`
  and `out[3..] == rec.args()[..]`. Take every float from the record's own
  bytes (`f32::from_le_bytes`), never from a decimal literal (F12).
  - `dialog_button_choice_matches_the_capture`: #1 (2982, -1), #33, #59, #61.
  - `gm_goto_xyz_matches_the_capture`: #4, #35.
  - `region_trigger_matches_the_capture`: #8 (region 14, leaving).
  - `interact_matches_the_capture`: #19 (100751), #42 (100748).
  - `move_item_matches_the_capture`: #65 (10345, 3, 1, 1); its msg id is
    166 (`0xA6`, direct cell 38).
  - `request_entity_update_matches_the_capture`: #9 (100745); its msg id is 7.
  - `cancel_movie_encodes_a_wstring`: hand bytes for `"Cine-SGWLogo.SGWLogo"`:
    `0xBD`, length, 4 zero bytes, sub-slot 47, `u32` 20, 20 UTF-16 units.
  - `create_character_layout`: hand bytes for name `"Ab"`, extra `""`,
    char def 3, choices `[(1, 2)]`, tint 0, starting `0xC3`.
  Each fails if an index, argument order or width changes. Use
  `TapCapture::expect(i, "<msg_name>")` so a reshuffled fixture fails loudly.
- In `tap_fixture.rs`: `praxis_fixture_loads_84_records_for_entity_8`
  (`count == 84`, `messages.len() == 84`, `entity_id == 8`, `dropped == 0`).
- New `crates/wireclient/tests/it/client_calls.rs` (declared in
  `tests/it/main.rs`, no DB):
  `client_call_indices_match_the_server_names` asserts
  `cimmeria_wire::names::player_cell_method(CM_X) == Some("<name>")` for all
  six `CM_*` constants. Fails if a constant drifts from the server's table.

Checks: the three lane commands.

Docs owed: none (WC-17).

Reviewer focus: no builder hard-codes the entity id; `create_character`
matches the server parser byte for byte; floats come from fixture bytes.

## WC-03 Runner: nextest profile and --wireclient mode

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** D-WC6.
**Branch:** `wireclient-ci/wc03-runner`. **Worktree:** `wc03`.
**Subject:** `ci(wireclient): WC-03 wireclient-e2e nextest profile and a --wireclient mode for the live-DB runner`

Why: README F15, F16. Written for D-WC6 option (a); if the owner picks (b),
the profile instead sets `threads-required = "num-test-threads"` on the
override and drops the `test-group` line, and nothing else changes.

Files:

1. `.config/nextest.toml`: append, after the `ci-live-db` blocks, keeping the
   existing `live-db = { max-threads = 8 }` line untouched (the scripts parse
   it):

   ```toml
   # The wireclient end-to-end tests (crates/wireclient/tests/it): each spawns
   # a full in-process Orchestrator. They run in the `live-db` group so each
   # running test gets its own database clone (docs/analysis/wireclient-ci,
   # D-WC6). No retries: a flake is a bug (D-WC9).
   [profile.wireclient-e2e]
   fail-fast = false
   retries = 0
   slow-timeout = { period = "60s", terminate-after = 5 }

   [profile.wireclient-e2e.junit]
   path = "junit.xml"
   report-name = "cimmeria-wireclient-e2e"

   [[profile.wireclient-e2e.overrides]]
   filter = "package(cimmeria-wireclient) & kind(test)"
   test-group = "live-db"
   ```
2. `tools/test-live-db.sh` and `tools/test-live-db.ps1` (twins; same change
   in both):
   - Add `cimmeria-wireclient` to `LIVE_DB_CRATES` / `$LiveDbCrates` (its
     lib tests are cheap, and WC-04 adds the `cimmeria-test-support`
     dev-dependency the `live_db_wrapper_lists_every_test_support_crate`
     guard then requires).
   - Accept a third leading flag, `--wireclient`, in the existing flag loop.
     With it, the nextest arguments are
     `--profile=wireclient-e2e -p cimmeria-wireclient --test it` instead of
     `--profile=ci-live-db <every crate> --lib`. Everything else (the
     `DATABASE_URL` check, the clone step, `--build-only`, `--llvm-cov`) is
     unchanged.
   - Update both header comments' usage lines.
3. `tools/build-lane/live-db-test.ps1` and `live-db-test.sh`: no code change
   (they pass arguments through); add one usage line each:
   `live-db-test.ps1 --wireclient <filter>`.

Tests: the existing guards are the tests. Run, through the lane:

```powershell
pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-test-support nextest_profile
pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-services live_db_wrapper
```

`nextest_profile_groups_the_live_db_filter` must stay green (it pins the
`ci-live-db` lines; if it fails because it reads the whole file, extend it to
accept the new profile and say so in the commit body).
`live_db_wrapper_sh_and_ps1_list_the_same_crates` fails if the two lists
differ. Then a live smoke:
`pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 --wireclient login_probe_live_db`
must run 2 tests and pass (they exist today), and the log must show
`--profile=wireclient-e2e`.

Docs owed: `TESTING.md` § Running the test suite, "Locally (live DB)": one
line for `--wireclient`. (This packet owns that row.)

Reviewer focus: the `.sh` and `.ps1` agree flag for flag; `--wireclient`
combined with `--build-only` builds only the `it` binary.

## WC-04 tests/it live-DB isolation

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** WC-03.
**Branch:** `wireclient-ci/wc04-isolation`. **Worktree:** `wc04`.
**Subject:** `test(wireclient): WC-04 resolve the live-DB slot URL and capture logs in tests/it`

Why: README F14; D-WC9 (log capture on failure).

Files:

1. `crates/wireclient/Cargo.toml` `[dev-dependencies]`: add
   `cimmeria-test-support = { workspace = true }` with a comment (per-slot
   database URL and the live-DB gate). `tracing-subscriber` is already a
   normal dependency.
2. `crates/wireclient/tests/it/support/mod.rs`:
   - Delete `live_db_pool_or_skip` and its comment block.
   - Add `pub fn db_url() -> String` returning
     `cimmeria_test_support::database_url().expect("called after require_db_or_skip!")`.
   - Add `pub fn init_test_logging()`: a `std::sync::Once` around
     `tracing_subscriber::fmt().with_test_writer().with_env_filter(filter).try_init()`,
     where `filter` is `EnvFilter::try_from_default_env()` or else
     `"warn,aoi.cinematic_hold=info,cimmeria_wireclient=debug"`. Ignore the
     `try_init` error (another test in the process may have set one).
     nextest prints a test's captured output when it fails.
   - `start_server` and `start_server_with_base_transport` call
     `init_test_logging()` first.
   - Update the module doc: the gate is `cimmeria_test_support`'s.
3. Every live test module in `tests/it/` (`two_client_castle_visibility.rs`,
   `two_client_castle_visibility_chaos.rs`, `two_client_squad.rs`,
   `two_client_command_invite.rs`, `two_client_mail_cod.rs`,
   `two_client_tell.rs`, `duel_two_duelists_and_a_spectator.rs`,
   `sparbot_duel.rs`, `login_probe.rs`): replace
   `let pool = match live_db_pool_or_skip().await { Some(p) => p, None => return };`
   with `let pool = cimmeria_test_support::require_db_or_skip!();`, and every
   `std::env::var("DATABASE_URL")...` with `support::db_url()`. Update each
   header comment's run line to
   `pwsh tools/build-lane/live-db-test.ps1 --wireclient <module>`.
4. `crates/wireclient/tests/it/main.rs`: the doc's run instructions say the
   same.
5. Drop the "until WC-04" caveats WC-03's review added: in
   `.config/nextest.toml` (the `wireclient-e2e` comment) and `TESTING.md`
   (the `--wireclient` paragraph under "Locally (live DB)"), say each test
   runs on its own slot clone.

Tests:

- New `crates/wireclient/tests/it/isolation.rs` (declared in `main.rs`), a
  source guard with no database, `tests_it_never_reads_database_url_directly`:
  read every `.rs` under `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/it")`
  and fail, naming file and line, on any line containing the string literal
  `"DATABASE_URL"` outside a `//` comment. It fails if this packet is reverted
  (the old helper read it), and it keeps a new test from bypassing the slot.
- `isolation_live_db_resolves_the_slot_database`: `require_db_or_skip!()`,
  then `SELECT current_database()` on the pool equals the database name at
  the end of `support::db_url()`, and under the `wireclient-e2e` profile that
  name ends in `_<NEXTEST_TEST_GROUP_SLOT>`. Fails if the support reads the
  template URL.
- Live run: `pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 --wireclient two_client`
  and `... --wireclient isolation`; all pass. Record in the worknote any test
  that fails or takes over 60 s (input for WC-16).

Checks: the three lane commands, then the two live runs.

Docs owed: none (WC-17 rewrites TESTING.md type 11's run instructions).

Reviewer focus: no test lost its skip-without-DB behaviour (the no-DB nextest
run still passes); `init_test_logging` cannot panic.

## WC-05 Semantic decoder skeleton and dialog family

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** WC-02.
**Branch:** `wireclient-ci/wc05-semantic-dialog`. **Worktree:** `wc05`.
**Subject:** `feat(wireclient): WC-05 semantic decoder skeleton and the dialog family`

Why: ADR Phase 3; D-WC4, D-WC5. This packet writes the whole contract so the
four family packets can run in parallel, each in its own file.

Files:

1. New `crates/wireclient/src/semantic/mod.rs`: module doc (typed decoders of
   server-to-client entity methods, written from the dispatch table, D-WC4;
   strict, D-WC5; `TargetKind` and why index 27 differs between a player and
   an NPC, README F8). `TargetKind`, `S2CEvent`, `DialogDisplay`,
   `Sequence`, `InvItem`, `SemanticError` exactly as the contract. Submodules
   `primitives`, `dialog`, `mission`, `sequence`, `inventory`, `entity`.
   `decode_event`:

   ```rust
   pub fn decode_event(kind: TargetKind, method_index: u16, args: &[u8])
       -> Result<Option<S2CEvent>, SemanticError> {
       match (method_index, kind) {
           (0, _) => entity::static_mesh_name_update(args).map(Some),
           (1, _) => sequence::sequence(args).map(Some),
           (3, _) => dialog::interaction_type(args).map(Some),
           (7, _) => entity::entity_property(args).map(Some),
           (8, _) => entity::visible(args).map(Some),
           (11, _) => entity::being_name_id_update(args).map(Some),
           (28, TargetKind::Player) => entity::player_communication(args).map(Some),
           (70, TargetKind::Player) => inventory::active_slot_update(args).map(Some),
           (71, TargetKind::Player) => inventory::remove_item(args).map(Some),
           (72, TargetKind::Player) => inventory::update_item(args).map(Some),
           (80, TargetKind::Player) => mission::mission_update(args).map(Some),
           (81, TargetKind::Player) => mission::step_update(args).map(Some),
           (82, TargetKind::Player) => mission::objective_update(args).map(Some),
           (101, TargetKind::Player) => sequence::known_abilities_update(args).map(Some),
           (105, TargetKind::Player) => dialog::dialog_display(args).map(Some),
           (155, TargetKind::Player) => sequence::play_movie(args).map(Some),
           _ => Ok(None),
       }
   }
   ```
2. New `semantic/primitives.rs`: `Reader` (contract) with unit tests for
   each read, a short read, `finish` with a leftover byte, a WSTRING of
   `"SYSTEM"` (from fixture #5's first 16 bytes), invalid UTF-16 (a lone
   `0xD800`), and an array whose count says 1,000,000 with 4 bytes present
   (`Short`, no allocation).
3. New `semantic/dialog.rs`: `pub(super) fn dialog_display(args)` and
   `pub(super) fn interaction_type(args)`, each a `Reader` then `finish()`.
4. New stub files `semantic/mission.rs`, `sequence.rs`, `inventory.rs`,
   `entity.rs`: every function `decode_event` calls, each
   `pub(super) fn <name>(args: &[u8]) -> Result<S2CEvent, SemanticError>`
   returning `Err(SemanticError::NotImplemented { method: "<wire name>" })`,
   with `let _ = args;` and a `// WC-0n fills this in` comment. The family
   packets replace only the bodies and add tests.
5. `crates/wireclient/src/lib.rs`: `pub mod semantic;` and a crate-doc line.

Tests (in `dialog.rs`, unit, wire-format, type 2):

- `dialog_display_matches_the_capture`: #20 gives
  `DialogDisplay { entity_id: 100751, dialog_id: 3995, mission_flags: 0, is_immediate: 1, mission_id: 0 }`,
  #43 `100748` / `3996`, #54 `8` / `5882`. Cross-check (D-WC4): each equals
  the record's `decoded` JSON (`dialog_id`, `entity_id`, `is_immediate`,
  `mission_flags`, `mission_id`).
- `interaction_type_matches_the_capture`: #21 `0`, #22 `0x4000_0000`, #44 `0`.
- `dialog_display_rejects_a_trailing_byte` (Trailing) and
  `dialog_display_rejects_a_short_body` (Short).
- In `mod.rs`: `index_27_on_an_npc_has_no_decoder` (#6's args, `Npc`, gives
  `Ok(None)`) and `an_unlisted_index_has_no_decoder` (index 50 on `Player`
  gives `Ok(None)`). No test pins the stubs; the family packets replace them.
- Each fixture test fails if a field's width or order changes.

Checks: the three lane commands.

Docs owed: none (WC-17).

Reviewer focus: `decode_event` table matches the contract's layout table
exactly (indices, kinds); every decoder calls `finish()`.

## WC-06 Mission family decoders

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** WC-05.
**Branch:** `wireclient-ci/wc06-mission`. **Worktree:** `wc06`.
**Subject:** `feat(wireclient): WC-06 decode onMissionUpdate, onStepUpdate and onObjectiveUpdate`

Files: `crates/wireclient/src/semantic/mission.rs` only (bodies of
`mission_update`, `step_update`, `objective_update`, layouts per the contract
table).

Tests (unit, wire-format):

- `mission_update_matches_the_capture`: #28 `(1360, 0, 0)`, #69 `(622, 1, 0)`.
  Cross-check `decoded` (`MissionID`, `Status`, `MissionGiverName`).
- `step_update_matches_the_capture`: #25 `(2113, 1)`, #26 `(80623, 0)`,
  #29 `(4037, 0)`, #46 `(80623, 1)`, #47 `(80622, 0)`, #68 `(80622, 1)`.
- `objective_update_matches_the_capture`: #23 `(3238, 1, hidden 1, optional 1)`,
  #24 `(2452, 1, 0, 0)`, #27, #30, #45, #48, #67; field order is Status,
  Hidden, Optional (#23 alone cannot tell them apart; #24 can).
- `mission_update_status_is_one_byte`: 9-byte body decodes; a 12-byte body
  (status widened to INT32) fails with `Trailing`. Fails if `status` is read
  as i32.

Checks: the three lane commands.

## WC-07 Ability, sequence and movie decoders

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** WC-05.
**Branch:** `wireclient-ci/wc07-sequence`. **Worktree:** `wc07`.
**Subject:** `feat(wireclient): WC-07 decode onSequence, onKnownAbilitiesUpdate and onPlayMovie`

Files: `crates/wireclient/src/semantic/sequence.rs` only.

- `sequence`: per the contract table; a non-zero NameValuePair count
  returns `Unsupported { method: "onSequence", what: "NameValuePairs" }`.
  Note in a comment that `cimmeria-wire-log`'s decoder ignores
  `InstanceId` (README F10).
- `known_abilities_update`: `array_i32`.
- `play_movie`: `wstring`, `u8`.

Tests (unit, wire-format):

- `sequence_matches_the_capture`: #66 (10000, source 8, target 8, primary 1,
  impact 0.0, nvp 0, view 0, instance 0), #72 (1872), #77 (1873). Fails if
  `InstanceId` is dropped (then 4 bytes trail).
- `sequence_with_name_value_pairs_is_unsupported`.
- `known_abilities_match_the_capture`: #49 `[597, 1218, 592, 594]`, #71
  `[579, 597, 1218, 592, 594, 708]`, order kept.
- `play_movie_decodes_the_first_login_movie`: hand bytes for
  `"Cine-SGWLogo.SGWLogo"` (the name `client_ready/mod.rs` sends) and
  `full_screen = 1`.

Checks: the three lane commands.

## WC-08 Inventory family decoders

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** WC-05.
**Branch:** `wireclient-ci/wc08-inventory`. **Worktree:** `wc08`.
**Subject:** `feat(wireclient): WC-08 decode onUpdateItem, onRemoveItem and onActiveSlotUpdate`

Files: `crates/wireclient/src/semantic/inventory.rs` only.

- `update_item`: `u32` count, then per item the `InvItem` layout in the
  contract table (the producer is `cimmeria_entity::inventory::InvItem::serialize`,
  `crates/entity/src/inventory.rs`). `is_bound` is `u8 != 0`. Check the
  count against the bytes left (minimum 37 bytes per item) before
  allocating.
- `remove_item`: `array_i32`.
- `active_slot_update`: `i32`, `i32`.

Tests (unit, wire-format; the fixture has no inventory message, F11):

- `update_item_decodes_two_items`: hand-build the bytes for two items, one
  with `ammo_types = [1, 2]` and one with none, field by field in the
  serializer's order; assert both decode equal. Fails if any field's width
  or order differs from `InvItem::serialize`.
- `update_item_rejects_a_forged_count`: count 100,000 with one item's bytes;
  `Short`.
- `remove_item_and_active_slot_round_trip`: hand bytes.
- A comment points at WC-14, which checks these against the real server's
  bytes.

Checks: the three lane commands.

## WC-09 Entity-introduction and chat decoders

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** WC-05.
**Branch:** `wireclient-ci/wc09-entity`. **Worktree:** `wc09`.
**Subject:** `feat(wireclient): WC-09 decode the entity-introduction methods and onPlayerCommunication`

Files: `crates/wireclient/src/semantic/entity.rs` only:
`static_mesh_name_update` (two WSTRINGs), `entity_property` (`i32, i32`),
`visible` (`i8`), `being_name_id_update` (`i32`), `player_communication`
(WSTRING, `u8`, `u8`, WSTRING).

Tests (unit, wire-format):

- `player_communication_matches_the_capture`: #5 gives speaker `"SYSTEM"`,
  flags 0, channel 9, text `"gmGotoXYZ: teleported to (-325, 73.6, -212.8)"`;
  #50 `"You have learned Pistol Shot."`. Cross-check `decoded`.
- `entity_property_matches_the_capture`: #73 `(3, 1)`.
- `static_mesh_name_update_decodes_frosts_corpse`: args built with
  `crate::calls::write_wstring` for `"CA-Props.CA-PrisonerCorpse00"` then
  `"GLB_Components.WorldObject_Small"` (template 14's row; the order
  `append_appearance` in `crates/wire/src/mercury/aoi/create.rs` writes).
  WC-11 checks this decoder against the server's own cascade bytes.
- `visible_and_being_name_id_decode`: hand bytes (`[1]`; `7031`).

Checks: the three lane commands.

## WC-10 Character creation on the wire, world entry recorded

**Implementer:** packet-coder. **Size:** M. **Wave:** 3. **Depends on:** WC-02, WC-04.
**Branch:** `wireclient-ci/wc10-create-character`. **Worktree:** `wc10`.
**Subject:** `feat(wireclient): WC-10 create a character on the wire and keep the world-entry bundles`

Why: README F4, F13; D-WC11.

Files:

1. `crates/wireclient/src/session.rs`: add `pub entry_bundles: Vec<Bytes>`
   to `GameSession` (doc comment per the contract), initialised empty in
   `from_auth_session`.
2. `crates/wireclient/src/world_entry.rs`:
   - Split `enter_world` into `open_character_list` (the
     `AUTHENTICATE + ENABLE_ENTITIES` send and its one bundle) and `play`
     (everything from `playCharacter` on). `enter_world` calls both; its
     signature and behaviour do not change.
   - `expect_bundles` pushes clones of what it returns onto
     `self.entry_bundles`.
   - `create_character(name, char_def_id, visual_choices, recv_timeout)`:
     send `GameSession::create_character(name, "", char_def_id, visual_choices, 0)`
     reliably, then wait (with `recv_meaningful_bundles`, one bundle at a
     time, until the timeout) for a bundle holding msg `0x82`
     (`onCharacterList`) or `0x83` (`onCharacterCreateFailed`, account client
     methods 2 and 3, `docs/protocol/message-dispatch-table.md`). On `0x83`,
     return `Error::CharacterCreateFailed(code)` with the first `i32` of the
     message's args (read the server's `fail_code.rs` to confirm that is the
     code's position; if not, stop and tell the coordinator). Timeout is
     `Error::WorldEntry("no onCharacterList after createCharacter ...")`.
3. `crates/wireclient/src/error.rs`: `CharacterCreateFailed(i32)`.
4. `crates/wireclient/tests/it/support/mod.rs`: add
   `insert_sentinel_account_with_level`, `default_visual_choices` (the SQL in
   `crates/base/src/base/character_create/seed_parity_live_db_tests.rs`
   `default_choices`), and `cleanup_account` (delete, in order, `sgw_mission`,
   `sgw_inventory`, `sgw_player` rows of the account's characters, then the
   account; find any other table with a `player_id` / `character_id` foreign
   key to `sgw_player` that a new character gets rows in, and delete those
   too).
5. New `crates/wireclient/tests/it/character_create_live_db.rs` (declared in
   `main.rs`).

Tests:

- `wire_created_praxis_character_enters_castle_cellblock_live_db` (live-DB,
  type 3 plus wire): account `0x7000_9C01` (`wc10_praxis`, access level 0),
  `cleanup_account` before and after. `connect`, `open_character_list`,
  `create_character("Wirepraxis", 3, &default_visual_choices(&pool, 3).await, 5 s)`.
  Then from the DB: exactly one `sgw_player` row for the account, with
  `first_login = 1` and `world_location = 'Castle_CellBlock'` (adjust the
  column names to the table; see `db/sgw/Players/Tables/sgw_player.sql`).
  Then `play(player_id, 5 s)`: `player_entity_id` is `Some`, and
  `entry_bundles` holds at least 3 bundles, one of them with an entity
  method of index 72 (`onUpdateItem`) on the player (use
  `decode_bundle_strict` and `method_index_for(61)`). Fails if creation is
  refused, if world entry stops recording, or if the starter kit is not sent
  at entry.
- `create_character_refusal_is_an_error_live_db`: create with char def
  `-1`; expect `Err(CharacterCreateFailed(_))`. Fails if a refusal is
  mistaken for success.
- Character names must pass the server's name check
  (`character_create/mod.rs`, "Name validation"); if `"Wirepraxis"` fails it,
  pick a letters-only name that passes and say so in the commit body.

Checks: the three lane commands, then
`pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 --wireclient character_create_live_db`
and `... --wireclient two_client` (the split must not break existing entry).

Docs owed: none (WC-17).

Reviewer focus: `enter_world`'s observable behaviour is unchanged for the
existing tests; cleanup deletes by exact sentinel; no fixed sleeps.

## WC-11 Entity mirror and query

**Implementer:** packet-coder. **Size:** M. **Wave:** 4. **Depends on:** WC-01, WC-05, WC-08, WC-09.
**Branch:** `wireclient-ci/wc11-mirror`. **Worktree:** `wc11`.
**Subject:** `feat(wireclient): WC-11 entity mirror that finds entities by what the client is told`

Why: ADR Phase 3's mirror; D-WC1; README F2, F5.

Files: new `crates/wireclient/src/mirror/mod.rs`, `mirror/query.rs`,
`mirror/tests.rs` (contract); `src/lib.rs` (`pub mod mirror;`);
`src/error.rs` (`Error::Mirror`).

`EntityMirror::apply(msg)`:

| Message | Effect |
|---|---|
| `0x05` `CREATE_BASE_PLAYER` | `set_own_id`; insert the player (class from the message) |
| `0x09` `CREATE_ENTITY` | insert, or replace if the id is already there (a re-introduction), with `introduced = next_intro++`; clear `hidden` |
| `0x0C` `leaveAoI` | remove |
| `0x0B` `entityInvisible` | `hidden = true` |
| `0x10` `avatarUpdateNoAliasFullPosYawPitchRoll` | `position` from payload bytes 4..16 (three f32 after the entity id; `compose_create_entity_base_body`) |
| other `0x11..=0x30` | nothing (`// TODO(WC-11): other avatar-update variants; the Praxis start needs only 0x10`) |
| entity method, `0x80..=0xFE` | find the target; kind is `Player` when the target is the own id or its class is 2 or 3, else `Npc`; index is `msg.method_index_for(61 or 62)`; `semantic::decode_event(kind, index, msg.args())`; on `Err`, `MirrorError::Semantic`; apply the event (below) and return it. A target the mirror does not hold: push the message onto `orphans`, return `Ok(None)` |

Applying events: `StaticMeshNameUpdate` sets `static_mesh` and `body_set`;
`BeingNameIdUpdate` sets `name_id`; `InteractionType` sets
`interaction_type`; `EntityProperty` inserts into `properties`; `Visible`
sets `visible`; `UpdateItem` on the own entity upserts each item into
`inventory` by `id`; `RemoveItem` on the own entity removes. Every other
event changes nothing and is still returned.

`EntityQuery::matches`: every `Some` field must equal; `near` compares
Euclidean distance `<= radius` and fails when `position` is `None`;
`interaction_nonzero` needs `interaction_type` to be `Some(x)` with `x != 0`.

Tests (`mirror/tests.rs`, unit). Build bundles with the server's own
producers so the mirror is tested against real layouts:
`cimmeria_wire::mercury::aoi::create::compose_create_entity_base_body` and
`compose_create_entity_cascade_body` (check the exact public paths; they are
`pub` in `crates/wire/src/mercury/aoi/create.rs`; `cimmeria-wire` is a
dev-dependency). If `NpcAoIData` cannot be built from a test, build the
cascade by hand with `cimmeria_wire::mercury::append_entity_method` and say
so in the commit body.

- `create_then_cascade_identifies_frosts_corpse`: create entity 100751
  class 1 at `(-322.0, 73.6, -211.0)`, then a cascade with static mesh
  `CA-Props.CA-PrisonerCorpse00` and name id 7031;
  `find_one(EntityQuery::static_mesh("CA-Props.CA-PrisonerCorpse00").name_id(7031))`
  returns 100751 with the position set.
- `leave_removes_and_reintroduction_replaces`.
- `an_npc_extended_method_uses_idbase_62`: a `0xBD` method with sub-slot 0
  to an NPC resolves to index 62, not 61 (decodes to `Ok(None)`, no error);
  the same bytes to the own player resolve to 61. Fails if the mirror uses
  `method_index` (always 61).
- `a_method_for_an_unknown_entity_is_an_orphan`.
- `interactable_finds_the_body_after_interaction_type_changes`: two
  entities, one gets `InteractionType 0x4000_0000`; `.interactable()`
  matches only it.
- `find_one_reports_ambiguity_with_ids`.
- `inventory_tracks_update_and_remove_on_the_own_entity`.
- `a_bad_known_method_is_an_error`: `onDialogDisplay` to the player with one
  byte short; `MirrorError::Semantic`.

Checks: the three lane commands.

Reviewer focus: idbase per target, not per message; replace-on-recreate
keeps no stale fields.

## WC-12 ScriptSession

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 5. **Depends on:** WC-10, WC-11.
**Branch:** `wireclient-ci/wc12-script-session`. **Worktree:** `wc12`.
**Subject:** `feat(wireclient): WC-12 ScriptSession: lossless event log, ordered waits, keep-alive`

Why: README F3; D-WC5, D-WC9, D-WC12. The judgment is in the async
plumbing: no message may be lost between waits, the keep-alive must keep
ACKs flowing while the script waits, and every failure must say what
arrived.

Files: new `crates/wireclient/src/script.rs` (contract), `src/lib.rs`,
`src/error.rs` (`ScriptTimeout`).

Shape:

- `new(session)`: take `session.entry_bundles`, decode each strictly, apply
  through the mirror, log. Seed `mirror.set_own_id` from
  `session.player_entity_id` first. The cursor starts at 0, so a script can
  wait for a message world entry already received.
- `pump(max)`: loop until `max`: if `KEEPALIVE` has passed since the last
  send, send `GameSession::authenticate()` unreliably (sparbot's keep-alive,
  `src/sparbot.rs`); then `session.recv_bundles(1, min(remaining, KEEPALIVE))`.
  Each bundle: skip it when every message is `tickSync` (`0x0D`); else
  `decode_bundle_strict` (an error ends the script: `Error::Decode`), apply
  each message to the mirror (`Error::Mirror` ends it), push an `Observed`
  with `at = started.elapsed()`.
- `wait_event`: scan the log from the cursor; if nothing matches, `pump`
  until the deadline, rescanning only the new entries; on a match set the
  cursor past it and return a clone. On timeout,
  `Error::ScriptTimeout { trail: self.trail(30), .. }`.
- `expect_in_order`: the steps in turn through the same scan, one shared
  deadline; the timeout names the first step that did not match.
- `wait_entity`: pump until `mirror.find_one(q)` is `Ok`; an `Ambiguous`
  result at the deadline is reported as such.
- `trail(n)`: one line per entry: `+12.345s msg 0xbd entity 8 m105 DialogDisplay { .. }`.
- Fidelity (D-WC12): after applying a `CREATE_ENTITY` for an entity other
  than the own player, queue `GameSession::request_entity_update(id)`; the
  next `send` or keep-alive carries the queue in the same bundle. The real
  client sends one per entity entering its world (`encrypted/mod.rs`'s
  `0x07` comment).

Tests (unit). A `GameSession` needs a real handshake, so there is no cheap
fake for `pump`; factor the logic out and test it pure:

- Factor the scan into a pure `fn scan(log: &[Observed], from: usize, pred) -> Option<usize>`
  and the trail formatter; unit-test both: a match before the cursor is not
  returned; ordered steps with gaps; the trail lists the last n.
- `keepalive_is_due_after_250ms` on a pure `fn keepalive_due(last: Instant, now: Instant) -> bool`.
- The end-to-end behaviour is proven by WC-14. In the commit body, state
  that a live test of `pump` alone was not added, or add
  `script_session_live_db_sees_the_arrival_dialog` (create a Praxis
  character as in WC-10, `play`, `ScriptSession::new`, `wait_event` for
  `DialogDisplay { dialog_id: 2982 }` within 30 s) if it fits.

Checks: the three lane commands; the live test if added.

Reviewer focus: no path drops a decoded message; the keep-alive never sends
reliably; `pump` returns promptly when `max` is short.

## WC-13 CI workflow

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** WC-03, WC-04, D-WC7.
**Branch:** `wireclient-ci/wc13-workflow`. **Worktree:** `wc13`.
**Subject:** `ci(wireclient): WC-13 run the wireclient end-to-end tests on server changes`

Written for D-WC7 option (a). Not added to `ship.py` yet (WC-16 does that).

Files:

1. New `.github/workflows/wireclient.yml`, modelled on `lab.yml` (header
   comment, triggers, concurrency) and on `test.yml`'s `test-live-db` job
   (service, steps):

   ```yaml
   name: wireclient
   # Path-filtered: the wire client's end-to-end tests (crates/wireclient/tests/it)
   # against a spawned server and a live Postgres. Every server change runs it
   # (docs/analysis/wireclient-ci, D-WC7). It gates merges through ship.py's
   # CONDITIONAL list once WC-16's shakedown has passed.
   on:
     push:
       branches: [main]
       paths:
         - 'crates/**'
         # ... the list below, written out in full
     pull_request:
       paths:
         - 'crates/**'
         # ... the same list again
   ```

   The path list, written out under both `push` and `pull_request` as
   `lab.yml` does (in order, since a `!` pattern only excludes what an
   earlier pattern included):
   `crates/**`, then `!crates/launcher/**`, `!crates/lab/**`,
   `!crates/client-hookgate/**`, `!crates/client-launch/**`,
   `!crates/client-patches/**`, `!crates/client-telemetry/**`,
   `!crates/sgw-testhost/**`, `!crates/start32/**`; then `db/**`,
   `entities/**`, `data/spaces/**`, `Cargo.toml`, `Cargo.lock`,
   `rust-toolchain.toml`, `.config/nextest.toml`, `.cargo/config.toml`,
   `.github/actions/rust-toolchain/**`, `tools/test-live-db.sh`,
   `tools/live-db-schema-load.sh`, `.github/workflows/wireclient.yml`.
   Before committing, confirm none of the excluded crates is in
   `cargo tree -p cimmeria-wireclient -e normal,dev --prefix none` (run it
   through the lane); drop any exclusion that is, and say so.

   One job, `e2e`, `name: wireclient e2e (live DB)`, `runs-on: ubuntu-latest`,
   `timeout-minutes: 60`, the same `postgres:17.9` service and
   `DATABASE_URL` as `test-live-db`, and these steps in order: checkout
   (`persist-credentials: false`); `./.github/actions/rust-toolchain`;
   install clang, mold and psql; `bash tools/live-db-schema-load.sh start`;
   install nextest; hydrate `external/recast` (copy the step); rust-cache with
   `shared-key: wireclient-e2e` and `save-if` on `main`;
   `bash tools/test-live-db.sh --wireclient --build-only`;
   `bash tools/live-db-schema-load.sh wait 300`;
   `bash tools/test-live-db.sh --wireclient`; and, `if: ${{ !cancelled() }}`,
   `actions/upload-artifact` of `target/nextest/wireclient-e2e/junit.xml`
   (pin the action to the version other workflows here use).
2. `docs/agents/pre-pr-checks.md` § CI checks: a new row,
   `wireclient e2e (live DB)` | `wireclient.yml` | "No, reports only, until
   WC-16" | the path list in words. (This packet owns that row.)

Tests: none in Rust. Validate the YAML locally with `python -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/wireclient.yml`
(stock Python with PyYAML; if absent, say so). After the PR is open, the job
must appear on the PR (the workflow file itself is in its path list) and
pass; link the run in the PR body.

Reviewer focus: the job name matches what WC-16 will put in `CONDITIONAL`;
the path list cannot skip a server crate the test links.

## WC-14 Praxis-start end-to-end test

**Implementer:** rust-gameserver-dev. **Size:** L. **Wave:** 6. **Depends on:** WC-06, WC-07, WC-12; D-WC2, D-WC3.
**Branch:** `wireclient-ci/wc14-praxis-start`. **Worktree:** `wc14`.
**Subject:** `test(wireclient): WC-14 Praxis start, mission 622, end to end on the wire`

Why: the campaign's goal. The ADR's [Praxis start](../../architecture/wireclient.md#praxis-start-the-wire-sequence)
table is the script; the fixture is the oracle for order; `sgw_mission` is
the oracle for the result. Written for D-WC2 (a) and D-WC3 (a).

Files:

1. `crates/wireclient/tests/it/support/mod.rs` (or a new
   `support/templates.rs` declared from it): `template_query(pool, template_id)`
   reads `static_mesh` and `name_id` from `resources.entity_templates` and
   returns the `EntityQuery`.
2. New `crates/wireclient/tests/it/praxis_start_live_db.rs` (declared in
   `main.rs`). One test, `praxis_start_completes_mission_622_live_db`:

   | Step | Send | Wait for (in order, `expect_in_order`, 10 s each unless stated) |
   |---|---|---|
   | 0 | account `0x7000_9C02` (`wc14_praxis`, access level 2: GM, D-WC2); `connect`; `open_character_list`; `create_character("Wirescript", 3, default choices)`; player id from the DB; `play` | `ScriptSession::new` |
   | 0a | | `PlayMovie` (movie `Cine-SGWLogo.SGWLogo`), then send `cancel_movie(CLIENT_CALL_ENTITY_ID, <that name>)` (D-WC3) |
   | 0b | | `wait_entity(template_query(14))` (Frost, 20 s) and the arrival `DialogDisplay { dialog_id: 2982 }` |
   | 1 | `dialog_button_choice(0, 2982, -1)` | nothing required |
   | 2 | `gm_goto_xyz(0, P1)`, `P1` from fixture #4's bytes; then `trigger_client_hinted_generic_region(0, 14, false, P1')`, `P1'` from #8's bytes | `PlayerCommunication` whose text starts `"gmGotoXYZ: teleported to"` |
   | 3 | `interact(0, frost_id)` | `DialogDisplay { dialog_id: 3995, entity_id: frost }`; `InteractionType { 0x4000_0000 }` on an entity other than Frost (call it `body`); `StepUpdate { 2113, 1 }`; `StepUpdate { 80623, 0 }`; `MissionUpdate { 1360, 0 }` |
   | 4 | `dialog_button_choice(0, 3995, -1)` | nothing required |
   | 5 | `gm_goto_xyz(0, P2)`, `P2` from #35's bytes | the teleport line |
   | 6 | `interact(0, body)` | `DialogDisplay { 3996, entity body }`; `StepUpdate { 80623, 1 }`; `StepUpdate { 80622, 0 }`; `KnownAbilitiesUpdate [597, 1218, 592, 594]`; `DialogDisplay { 5882, entity own }` |
   | 7 | `dialog_button_choice(0, 5882, -1)` | |
   | 8 | `dialog_button_choice(0, 3996, -1)` | |
   | 9 | `move_item(0, pistol, 3, 1, 1)` | `Sequence { 10000 }`; `StepUpdate { 80622, 1 }`; `MissionUpdate { 622, 1 }`; `Sequence { 1872 }` |
   | 10 | | poll `SELECT status FROM sgw_mission WHERE player_id = $1 AND mission_id = $2` every 100 ms, up to 10 s: 622 is 2, 1360 is 1 (the lab rows' oracle). If persistence is deferred to logout, send `GameSession::disconnect(0)` first and say so in a comment |

   - `pistol`: the instance id of the starter-kit pistol in
     `script.mirror.inventory()` (it arrived in the world-entry
     `onUpdateItem`). Identify it by the kit's design id: read the
     `PRA_OPCORE_COMMANDO` start profile's weapon from the seed (find the
     table `character_create` reads the kit from; `starter_kit.rs`), then the
     item with that `dbid`. Assert it is not already in container 3.
   - After step 9, also wait for the inventory answer: find out from the base
     `MoveInventoryItem` handler what it sends (expected `onUpdateItem` with
     the pistol in container 3; README F11 says the tap did not show it) and
     assert it. If the server sends nothing, that is a finding: write it in
     the worknote and the PR body, do not weaken the test silently.
   - Frost's id comes from the mirror (D-WC1), never a literal. `body` comes
     from the `InteractionType` event in step 3.
   - `cleanup_account` before and after (also on panic: a guard struct whose
     `Drop` spawns nothing; simplest is cleanup at the start of the next run,
     which the per-slot clone makes safe; state which in a comment).
   - Every wait failure prints `script.trail(30)` (built into
     `ScriptTimeout`).
3. `crates/wireclient/tests/it/main.rs`: `mod praxis_start_live_db;`.

Test type: TESTING.md type 11 (wire-level replay) plus a live-DB oracle. It
fails if any server answer is missing or reordered (the ordered waits), if
the mission does not complete (the DB oracle), or if the server sends bytes
the decoders reject (D-WC5). Prove it is a guard: temporarily break one
answer (for example skip the `onStepUpdate` for 80622 in the content chain
step, or make `handle_move_item` drop the call), run, see the test fail with
a readable trail, revert, and describe the experiment in the PR body.

Checks: the three lane commands, then
`pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 --wireclient praxis_start`
three times in a row, all green, each under 60 s. Record the durations in
`worknotes/wc-14.md`.

Docs owed: none in this packet (WC-17 updates the ADR; the worknote lists
any difference between this run and the ADR's table).

Reviewer focus (`packet-reviewer`, `testing-validation-engineer`,
`mission-systems-advisor`): no literal entity ids; no sleeps; the
subsequence assertions match the fixture's order; the DB oracle is not
satisfiable by a row the test inserted.

## WC-15 First-login flush-shape guard

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 7. **Depends on:** WC-14; D-WC8; #1341's mitigation merged or in the same PR.
**Branch:** `wireclient-ci/wc15-flush-shape`. **Worktree:** `wc15`.
**Subject:** `test(wireclient): WC-15 the first-login flush never exposes a quest introduction to the #1341 residue`

Why: README F17. #1341 must decide first; this packet then asserts what its
mitigation guarantees, at the wire.

Files:

1. `crates/mercury/src/test_harness/peer.rs` (feature `test-harness` only):
   an opt-in inbound packet log. `pub fn enable_packet_log(&self)` and
   `pub fn take_packet_log(&self) -> Vec<InboundPacket>`, where
   `#[derive(Debug, Clone, PartialEq, Eq)] pub struct InboundPacket { pub seq: Option<u32>, pub reliable: bool, pub fragment: Option<(u32, u32)>, pub body_len: usize }`
   (`fragment` is `(firstFrag, lastFrag)`). Record in `spawn_recv_pump` right
   after `parse_incoming`, before delivery, when enabled. Unit-test it in the
   harness's own tests. Run `-p cimmeria-mercury` checks too.
2. `crates/wireclient/src/session.rs`: pass-throughs
   `enable_packet_log` / `take_packet_log`.
3. `crates/wireclient/src/bundle.rs` or a new `src/flush_shape.rs`: a pure
   function that, given one reassembled bundle and the body lengths of the
   packets it came in, returns the messages whose header starts in the first
   packet at a cursor other than 1. That set is what a residue can hit
   (client-mercury-receive-path.md, "Only a bundle's first packet is
   exposed" in #1341). Unit-test it on synthetic bundles.
4. New `crates/wireclient/tests/it/first_login_flush_live_db.rs`: create a
   Praxis character on the wire (account `0x7000_9C03`), enable the packet
   log before `play`, send `cancelMovie`, collect the flush bundles (the ones
   carrying `CREATE_ENTITY` after the movie), and assert the invariant
   #1341 chose. Candidates, as #1341 lists them: every single-packet flush
   bundle has exactly one message; or the first fragment of a fragmented
   flush bundle has exactly one message. Assert that the bundle carrying
   Frost's (template 14) introduction satisfies it, and that every flush
   bundle does.

Test type: TESTING.md type 2 at the wire (bundle shape) driven end to end.
It must fail on `main` before #1341's fix (today's 889-byte bundle has about
26 messages in its only packet): show that in the PR body.

If #1341 chooses the client patch (D-WC8), do not write files 1 to 4.
Instead, the coordinator records in the ledger that the hazard is not
testable from the wire (the wire client parses each message from its own
header and has no residue) and WC-17 adds that sentence to the ADR.

Checks: lane commands for `-p cimmeria-mercury` and `-p cimmeria-wireclient`,
then `pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 --wireclient first_login_flush`.

Reviewer focus (`aoi-witness-broadcast`, `bigworld-engine-advisor`): the
packet log records what the client's iterator sees (body length after
footers); the invariant is the one #1341 shipped.

## WC-16 Shakedown and gate

**Implementer:** the coordinator runs the shakedown; packet-coder makes the
change. **Size:** S. **Wave:** 7. **Depends on:** WC-13, WC-14.
**Branch:** `wireclient-ci/wc16-gate`. **Worktree:** `wc16`.
**Subject:** `ci(wireclient): WC-16 wireclient e2e gates merges when it runs`

Shakedown (coordinator, before the packet): re-run the `wireclient e2e
(live DB)` job on `main` until it has passed 10 times in a row
(`gh workflow run wireclient.yml` or re-runs of the latest `main` run).
Any failure is a bug (D-WC9): file it, fix it or `#[ignore]` the test with
the issue number and tell the owner, and restart the count. Record the runs,
durations and any fixes in `worknotes/wc-16.md`. Only then dispatch the
packet.

Files:

1. `tools/build-lane/ship.py`: add `"wireclient e2e (live DB)"` to
   `CONDITIONAL`, and extend the comment above it to name `wireclient.yml`.
2. `tools/build-lane/test_ship.py`: add
   `test_a_path_filtered_wireclient_check_that_ran_gates_the_merge`, a copy of
   the lab one with `ship.CONDITIONAL[<new index>]` and a
   `crates/base/src/x.rs` change. `test_conditional_names_exist_in_workflows`
   already fails if the job name and the workflow disagree.
3. `docs/agents/pre-pr-checks.md`: the row from WC-13 becomes "Yes when it
   runs, enforced by `ship.py merge`", like the lab rows.
4. `.claude/skills/ship-pr/SKILL.md`: if it lists the conditional checks by
   name, add this one; then `python tools/agent-skills/sync.py`.

Tests: `python -m unittest tools/build-lane/test_ship.py` (stock Python).
The new test fails if the name is not in `CONDITIONAL`.

## WC-17 Close-out

**Implementer:** documentation-writer. **Size:** M. **Wave:** 8. **Depends on:** every other packet.
**Branch:** `wireclient-ci/wc17-close-out`. **Worktree:** `wc17`.
**Subject:** `docs(wireclient): WC-17 close out the wireclient CI campaign`

Files (docs are CRLF; keep them CRLF):

1. `docs/architecture/wireclient.md`: Status and TL;DR (F18's wording);
   § 1 crate layout (new modules and test modules); § 7 rewritten for the
   profile D-WC6 chose; phase table: 3 "Partial: mirror and the decoders the
   Praxis start needs", 4 "Partial: the Praxis start (mission 622)", 7
   "Done"; § Praxis start: the corrected `moveItem` row (F7), method 27 (F8),
   the omitted answers (F11), the entity-id-0 prefix (F6), and how the test
   finds Frost (D-WC1); § Test corpora: the fixture is now loaded by tests;
   the #1341 outcome (WC-15 or the not-testable sentence).
2. `TESTING.md` type 11: what works now, the run command
   (`live-db-test.ps1 --wireclient`), the CI job, and the test counts.
3. `crates/README.md`: the `cimmeria-wireclient` row.
4. `docs/gap-analysis.md` / `docs/gap-analysis/` and `docs/project-status.md`:
   once, per the repo rule.
5. `docs/analysis/wireclient-ci/README.md`: every packet's final status, the
   review outcomes, a closing line in the header.
6. Issue #281: a comment listing what this campaign shipped against its
   checklist (1.5, 2 done earlier; 3 and 4 partial; 7 done; 5 and 6 open),
   and leave it open for 5 and 6.
7. Board: a handoff post in `handoffs` (`~/.agent-board/board --as documentation-writer ...`).
8. Retire the campaign's merged worktrees:
   `pwsh tools/build-lane/rm-worktree.ps1 --merged`.

No `docs/guides/unified-uat.md` steps: nothing here needs an owner UAT in the
client.
