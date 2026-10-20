//! Array helper for SFS values.
//!
//! SmartFoxServer 1.x sends arrays as objects keyed by their index, so a
//! minigame list travels as `<obj t='a'>` with `"0"`, `"1"`, ... children.

use std::collections::HashMap;

use super::protocol::SfsValue;

impl SfsValue {
    /// Build an SFS array from `items`, keyed `"0"`, `"1"`, ... in order.
    pub fn list<I: IntoIterator<Item = SfsValue>>(items: I) -> SfsValue {
        let map: HashMap<String, SfsValue> = items
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect();
        SfsValue::Object(map)
    }
}

#[cfg(test)]
#[path = "protocol_list_tests.rs"]
mod tests;
