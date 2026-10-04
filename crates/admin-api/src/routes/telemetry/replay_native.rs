//! Replaying one injected-DLL event (`client.native`), with its IDs named.
//!
//! The DLL sends raw values; the ingest names them before re-emitting
//! (named telemetry NT-40, instrumentation-discipline Rule 6), so the
//! launcher's installed DLLs never need updating for a new name:
//!
//! | DLL key | Re-emitted pair | Source |
//! |---|---|---|
//! | `entity_id`, `target_id`, `source_id`, `pet_id` | `<prefix>_name` | The cell: whoever held the slot at the row's server time ([`super::entity_labels`]) |
//! | `ability_id` | `ability_name` | NameBook |
//! | `item_type_id` | `item_name` | NameBook |
//! | `type_id` (the entity type) | `class_id` + `class_name` | NT-30 class table (clientIndex) |
//! | `method_index` | `method_name` | NT-30 ClientMethod table, by `type_id` when the row has it |
//! | `msg_id` | `msg_name` | NT-30 message table, by direction |
//! | `address` | `address_name` | `client_symbols.tsv`, exact entry points only |
//!
//! A name that doesn't resolve is left out, never `""` (Rule 6).
//!
//! **`method_index` is a flat ClientMethod index** (server to client) in
//! every row that carries one: `client.dispatch.method_dropped` reports the
//! index `EntityDescription_GetExposedClientMethodByIndex` failed to find,
//! and `client.ability.recv` / `.recv_skipped` report the index of the
//! method being received (the DLL checks its table against
//! `docs/protocol/client-method-dispatch-table.md`). With the row's
//! `type_id` the index is looked up in that type's table; without one,
//! [`names::any_entity_client_method`] names it when every in-world type
//! agrees, which leaves only SGWPlayer's 27-31 versus SGWMob's and
//! SGWPet's unnamed.
//!
//! `effect_id` is not named: in `client.ability.recv` it is
//! `onEffectResults.EffectID`, a per-cast instance, not an effect design ID.
//! `item_id` is not named either: on the client it can be an instance. The
//! DLL sends neither item key today; `item_type_id` is named when it does.

use serde_json::{Map, Value};

use cimmeria_names::NameBook;
use cimmeria_wire::names;

use crate::routes::dev_session::TokenClaims;

use super::client_symbols::symbol_name;
use super::dto::ClientNativeEvent;
use super::entity_labels::entity_id;

/// Targets whose `msg_id` is one the client sent; every other target's is
/// one the client received.
const OUTBOUND_TARGETS: &[&str] = &[
    "client.net.out",
    "client.ability.sent",
    "client.ability.sent_seq",
];

/// DLL targets replayed in the status shape (boot, hooks, streaming, the
/// governor's rollups, the CEGUI log): the families whose status keys
/// (`level_name`, `dll_version`, `usable`, the rollup pair) are lifted.
/// Every other target is replayed in the game shape. The shape follows the
/// target, never the row's fields, so a row can't hide its status keys by
/// carrying a game ID.
const STATUS_TARGET_PREFIXES: &[&str] = &[
    "client.telemetry.",
    "client.dll.",
    "client.hooks.",
    "client.streaming.",
    "client.ui.",
];

/// Whether DLL event `target` is replayed in the status shape.
pub(super) fn is_status_target(target: &str) -> bool {
    STATUS_TARGET_PREFIXES.iter().any(|p| target.starts_with(p))
}

/// What every replayed `client.native` row says about its names: the IDs,
/// spaces and times they were resolved from are the client's own claims.
const NAMES_SOURCE: &str = "client_claimed";

/// Correlation keys lifted out of a DLL event's `fields` bag into
/// attributes of their own, so SigNoz can filter on them without parsing
/// the `fields` JSON. `None` (key absent or the wrong JSON type) omits the
/// attribute: `tracing` records nothing for a `None` field, which is the
/// right shape for "only when known" and never a sentinel.
///
/// The DLL's own `account_id` / `player_id` claims are not lifted: identity
/// is the token's, and no hook sends them.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct LiftedFields {
    /// A flat ClientMethod index (see the module docs).
    pub method_index: Option<i64>,
    /// A Mercury message id.
    pub msg_id: Option<i64>,
    /// The entity type (clientIndex), from the DLL's `type_id`.
    pub class_id: Option<i64>,
    pub entity_id: Option<u32>,
    pub target_id: Option<u32>,
    pub source_id: Option<u32>,
    pub pet_id: Option<u32>,
    pub ability_id: Option<i64>,
    pub item_type_id: Option<i64>,
    /// A native address, as the DLL writes it (`"0x01576f90"`).
    pub address: Option<String>,
    /// The map being loaded (`client.streaming.*`).
    pub level_name: Option<String>,
    /// The DLL's own build (`client.dll.attached`).
    pub dll_version: Option<String>,
    /// Whether the SGW.exe build fingerprint matched
    /// (`client.hooks.fingerprint`).
    pub fingerprint_usable: Option<bool>,
    /// The target a governor rollup summarizes (`client.telemetry.rollup`).
    pub rollup_target: Option<String>,
    /// How many events that rollup summarizes. Lifted only alongside
    /// `rollup_target`, so a generic `count` field is never mistaken for
    /// one; `sum(rollup_count)` by `rollup_target` recovers the totals.
    pub rollup_count: Option<i64>,
}

impl LiftedFields {
    pub(super) fn from_fields(fields: &Map<String, Value>) -> Self {
        let int = |k: &str| fields.get(k).and_then(Value::as_i64);
        let text = |k: &str| fields.get(k).and_then(Value::as_str).map(str::to_string);
        let entity = |k: &str| fields.get(k).and_then(entity_id);
        Self {
            method_index: int("method_index"),
            msg_id: int("msg_id"),
            class_id: int("type_id"),
            entity_id: entity("entity_id"),
            target_id: entity("target_id"),
            source_id: entity("source_id"),
            pet_id: entity("pet_id"),
            ability_id: int("ability_id"),
            item_type_id: int("item_type_id"),
            address: text("address"),
            level_name: text("level_name"),
            dll_version: text("dll_version"),
            fingerprint_usable: fields.get("usable").and_then(Value::as_bool),
            rollup_target: text("rollup_target"),
            rollup_count: text("rollup_target").and_then(|_| int("count")),
        }
    }
}

/// The names [`LiftedFields`] resolve to. `'b` is the NameBook's borrow.
#[derive(Debug, Default)]
pub(super) struct NativeNames<'b> {
    pub method_name: Option<&'static str>,
    pub msg_name: Option<&'static str>,
    pub class_name: Option<&'static str>,
    pub entity_name: Option<&'static str>,
    pub target_name: Option<&'static str>,
    pub source_name: Option<&'static str>,
    pub pet_name: Option<&'static str>,
    pub ability_name: Option<&'b str>,
    pub item_name: Option<&'b str>,
    pub address_name: Option<&'static str>,
}

impl<'b> NativeNames<'b> {
    /// Resolve `l` for a row of DLL event `target`. `entity_label` names an
    /// entity ID as the row saw it.
    pub(super) fn resolve(
        target: &str,
        l: &LiftedFields,
        book: &'b NameBook,
        entity_label: impl Fn(u32) -> Option<&'static str>,
    ) -> Self {
        let class = l.class_id.and_then(|c| u8::try_from(c).ok());
        let label = |id: Option<u32>| id.and_then(&entity_label);
        Self {
            method_name: l.method_index.and_then(|i| u16::try_from(i).ok()).and_then(
                |i| match class {
                    Some(c) => names::client_method(c, i),
                    None => names::any_entity_client_method(i),
                },
            ),
            msg_name: l.msg_id.and_then(|m| u8::try_from(m).ok()).and_then(|m| {
                if OUTBOUND_TARGETS.contains(&target) {
                    names::server_msg_name(m)
                } else {
                    names::client_msg_name(m)
                }
            }),
            class_name: class.and_then(names::class_name),
            entity_name: label(l.entity_id),
            target_name: label(l.target_id),
            source_name: label(l.source_id),
            pet_name: label(l.pet_id),
            ability_name: l.ability_id.and_then(|a| book.ability(a)),
            item_name: l.item_type_id.and_then(|i| book.item(i)),
            address_name: l.address.as_deref().and_then(symbol_name),
        }
    }
}

/// What a replayed row is named from: the NameBook, and the entity labels
/// as the row saw them.
pub(super) struct ReplayNames<'a> {
    pub book: &'a NameBook,
    pub entity_label: &'a dyn Fn(u32) -> Option<&'static str>,
}

/// Replay one injected-DLL event through `tracing`, with no entity names
/// (no cell to ask) and the process's NameBook.
#[cfg(test)]
pub(super) fn replay_client_native(claims: &TokenClaims, e: ClientNativeEvent) {
    replay_client_native_named(claims, e, &cimmeria_names::book(), |_| None);
}

/// Replay one injected-DLL event through `tracing`.
///
/// - **Target is static** (`client.native`), which is what routes it to
///   `cimmeria-client`. The DLL's own event name (`client.lua.pcall`)
///   rides in `client_target` and is the log body, so the SigNoz list view
///   shows it.
/// - **`level` is honoured** if it is one of `trace`/`debug`/`info`/
///   `warn`/`error`; anything else is replayed at `info` so a typo in the
///   DLL never drops an event. The raw string is kept in `client_level`.
/// - **Identity** is the token's (`session_id`, `install_id`,
///   `cimmeria.session_kind`, `lab`), never the DLL's own claims.
/// - **IDs are named** (module docs); `entity_label` names an entity ID as
///   this row saw it.
/// - **Two shapes, by target.** `tracing` caps an event at 32 fields, and
///   the ID pairs plus the DLL's status keys exceed it. A status target
///   ([`is_status_target`]: boot, hooks, streaming, governor rollups, the
///   CEGUI log) is replayed with the status keys (`level_name`,
///   `dll_version`, `fingerprint_usable`, `rollup_target`, `rollup_count`);
///   every other target with the ID pairs. Both carry the identity, the
///   address pair, `names_source` and the whole bag in `fields`.
/// - **Names are client-claimed** (`names_source = "client_claimed"`): the
///   IDs, the space and the time come from the uploaded row, so a name is
///   what the row claims, never a server observation.
pub(super) fn replay_client_native_named(
    claims: &TokenClaims,
    e: ClientNativeEvent,
    book: &NameBook,
    entity_label: impl Fn(u32) -> Option<&'static str>,
) {
    let l = LiftedFields::from_fields(&e.fields);
    let n = NativeNames::resolve(&e.target, &l, book, entity_label);
    let kind = claims.session_kind();
    let lab = claims.is_lab();
    let name = e.target.as_str();
    let fields_json = Value::Object(e.fields);

    // `tracing` fixes an event's level at the call site, so one call per
    // level; each macro keeps its field list in one place.
    macro_rules! game_event {
        ($level:expr) => {
            tracing::event!(
                target: "client.native",
                $level,
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                client_target = name,
                client_level = %e.level,
                names_source = NAMES_SOURCE,
                class_id = l.class_id,
                class_name = n.class_name,
                method_index = l.method_index,
                method_name = n.method_name,
                msg_id = l.msg_id,
                msg_name = n.msg_name,
                entity_id = l.entity_id,
                entity_name = n.entity_name,
                target_id = l.target_id,
                target_name = n.target_name,
                source_id = l.source_id,
                source_name = n.source_name,
                pet_id = l.pet_id,
                pet_name = n.pet_name,
                ability_id = l.ability_id,
                ability_name = n.ability_name,
                item_type_id = l.item_type_id,
                item_name = n.item_name,
                address = l.address.as_deref(),
                address_name = n.address_name,
                fields = %fields_json,
                "{name}"
            )
        };
    }
    macro_rules! status_event {
        ($level:expr) => {
            tracing::event!(
                target: "client.native",
                $level,
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                cimmeria.session_kind = kind,
                lab,
                ts_ms = e.ts_ms,
                seq = e.seq,
                client_target = name,
                client_level = %e.level,
                names_source = NAMES_SOURCE,
                address = l.address.as_deref(),
                address_name = n.address_name,
                level_name = l.level_name.as_deref(),
                dll_version = l.dll_version.as_deref(),
                fingerprint_usable = l.fingerprint_usable,
                rollup_target = l.rollup_target.as_deref(),
                rollup_count = l.rollup_count,
                fields = %fields_json,
                "{name}"
            )
        };
    }

    match (!is_status_target(name), e.level.as_str()) {
        (true, "trace") => game_event!(tracing::Level::TRACE),
        (true, "debug") => game_event!(tracing::Level::DEBUG),
        (true, "warn") => game_event!(tracing::Level::WARN),
        (true, "error") => game_event!(tracing::Level::ERROR),
        // `info` and any unrecognised value
        (true, _) => game_event!(tracing::Level::INFO),
        (false, "trace") => status_event!(tracing::Level::TRACE),
        (false, "debug") => status_event!(tracing::Level::DEBUG),
        (false, "warn") => status_event!(tracing::Level::WARN),
        (false, "error") => status_event!(tracing::Level::ERROR),
        (false, _) => status_event!(tracing::Level::INFO),
    }
}
