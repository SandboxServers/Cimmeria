//! The request envelope: one pass over the body that builds untyped JSON,
//! checks the three top-level keys and notices every object key written
//! twice.
//!
//! `serde_json` reads `{"a":1,"a":2}` as `{"a":2}` and reports nothing: the
//! later value wins. Another reader of the same bytes (a proxy, a log
//! shipper, someone with `jq`) may keep the first. A body with a repeated
//! key therefore says one thing to this route and another to whatever sits
//! in front of it, and this route takes one exact payload, so a repeated
//! key is refused wherever it is: in the envelope the request is a 400,
//! inside an element that element is `rejected`.
//!
//! Keys are compared after JSON unescaping, so `"a"` and `"a"` are one
//! key.

use std::cell::Cell;
use std::fmt;

use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use super::dto::{typed_object, ClientDropped, SummaryError, MAX_SUMMARIES, SCHEMA_VERSION};

/// A request that passed the envelope checks. The elements are still
/// untyped JSON: each is validated alone, so one bad element is rejected
/// without failing the others.
#[derive(Debug)]
pub(super) struct Envelope {
    pub client_dropped: ClientDropped,
    /// One entry per element of `summaries`, in order. `None` is an element
    /// that wrote a key twice somewhere inside it: it is `rejected` without
    /// being typed.
    pub elements: Vec<Option<Value>>,
}

/// Builds the [`Value`] `serde_json` itself would build, and sets the flag
/// when an object anywhere inside writes a key twice. The repeat is noted,
/// not raised: an error would end the parse of the whole body, and a repeat
/// inside one element must cost only that element.
struct Unique<'a>(&'a Cell<bool>);

impl<'de> DeserializeSeed<'de> for Unique<'_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Unique<'_> {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Number(Number::from(v)))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Number(Number::from(v)))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        Ok(Number::from_f64(v).map_or(Value::Null, Value::Number))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(Unique(self.0))? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(Unique(self.0))?;
            if object.insert(key, value).is_some() {
                self.0.set(true);
            }
        }
        Ok(Value::Object(object))
    }
}

/// What the `summaries` key held.
enum Summaries {
    /// An array. Elements past the first `MAX_SUMMARIES + 1` are read for
    /// their syntax and dropped: one over the limit is enough to refuse the
    /// request by its count, and a body of 30,000 tiny elements must not
    /// become 30,000 values first.
    Array(Vec<Option<Value>>),
    /// Anything that is not an array.
    Other,
}

/// Reads the value of `summaries`: each element of an array with a
/// repeated-key flag of its own, anything else as [`Summaries::Other`].
struct SummariesSeed;

impl<'de> DeserializeSeed<'de> for SummariesSeed {
    type Value = Summaries;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Summaries, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for SummariesSeed {
    type Value = Summaries;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Summaries, A::Error> {
        let mut elements = Vec::new();
        while elements.len() <= MAX_SUMMARIES {
            let repeated = Cell::new(false);
            let Some(value) = seq.next_element_seed(Unique(&repeated))? else {
                return Ok(Summaries::Array(elements));
            };
            elements.push((!repeated.get()).then_some(value));
        }
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Summaries::Array(elements))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Summaries, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(Summaries::Other)
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Summaries, E> {
        Ok(Summaries::Other)
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Summaries, E> {
        Ok(Summaries::Other)
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Summaries, E> {
        Ok(Summaries::Other)
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Summaries, E> {
        Ok(Summaries::Other)
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<Summaries, E> {
        Ok(Summaries::Other)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Summaries, E> {
        Ok(Summaries::Other)
    }
}

/// The top-level object as read, before any rule is applied to it.
#[derive(Default)]
struct TopLevel {
    schema_version: Option<Value>,
    client_dropped: Option<Value>,
    summaries: Option<Summaries>,
    unknown_key: bool,
}

/// Reads the top-level object. A body whose top level is anything else is
/// a parse error, like a body that is not JSON. The flag is set by a
/// repeated top-level key and by a repeated key anywhere under
/// `schema_version` or `client_dropped`; the elements of `summaries` keep
/// flags of their own.
struct TopLevelVisitor<'a>(&'a Cell<bool>);

impl<'de> Visitor<'de> for TopLevelVisitor<'_> {
    type Value = TopLevel;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<TopLevel, A::Error> {
        let mut top = TopLevel::default();
        while let Some(key) = map.next_key::<String>()? {
            let written_before = match key.as_str() {
                "schema_version" => top
                    .schema_version
                    .replace(map.next_value_seed(Unique(self.0))?)
                    .is_some(),
                "client_dropped" => top
                    .client_dropped
                    .replace(map.next_value_seed(Unique(self.0))?)
                    .is_some(),
                "summaries" => top
                    .summaries
                    .replace(map.next_value_seed(SummariesSeed)?)
                    .is_some(),
                // The request is refused for this key whatever it holds.
                _ => {
                    map.next_value::<IgnoredAny>()?;
                    top.unknown_key = true;
                    false
                }
            };
            if written_before {
                self.0.set(true);
            }
        }
        Ok(top)
    }
}

/// Check the request envelope: a JSON object with exactly `schema_version`
/// (the integer 1), `client_dropped` and 1 to 32 `summaries`, each key
/// written once. The element count is checked before any element is typed.
pub(super) fn parse_envelope(body: &[u8]) -> Result<Envelope, SummaryError> {
    let repeated = Cell::new(false);
    let mut json = serde_json::Deserializer::from_slice(body);
    let top = json
        .deserialize_any(TopLevelVisitor(&repeated))
        // Nothing but whitespace may follow the object.
        .and_then(|top| json.end().map(|()| top));
    let Ok(top) = top else {
        return Err(SummaryError::BadRequest("Body is not a JSON object"));
    };
    if repeated.get() {
        return Err(SummaryError::BadRequest("Repeated key"));
    }
    let (Some(schema_version), Some(client_dropped), Some(summaries)) =
        (top.schema_version, top.client_dropped, top.summaries)
    else {
        return Err(SummaryError::BadRequest("Missing a required key"));
    };
    if top.unknown_key {
        return Err(SummaryError::BadRequest("Unknown top-level key"));
    }
    if schema_version.as_u64() != Some(SCHEMA_VERSION) {
        return Err(SummaryError::BadRequest("Unsupported schema_version"));
    }
    let Some(client_dropped) = typed_object::<ClientDropped>(client_dropped) else {
        return Err(SummaryError::BadRequest("Invalid client_dropped"));
    };
    let elements = match summaries {
        Summaries::Array(elements) if (1..=MAX_SUMMARIES).contains(&elements.len()) => elements,
        _ => {
            return Err(SummaryError::BadRequest(
                "summaries must be an array of 1 to 32 elements",
            ))
        }
    };
    Ok(Envelope {
        client_dropped,
        elements,
    })
}
