//! Entity ids as names that stay the same from run to run. A raw id is
//! different on every server, so a golden could never agree on one.

use std::collections::HashMap;

/// The names of the entity ids one row meets. Numbering of unknown ids is
/// per row, in call order, so it is only meaningful inside one row.
pub struct EntityLabels {
    player: u32,
    p2: Option<u32>,
    /// Ids the entity snapshots name, with the label each one gets.
    known: HashMap<u32, String>,
    /// Ids met that no snapshot names; the index plus one is the `?<n>`.
    unknown: Vec<u32>,
    /// Set on a [`Self::frozen`] copy: an id not yet numbered is `?` and
    /// stays unnumbered.
    frozen: bool,
}

impl EntityLabels {
    /// `entities`: the entities.json attachment (below), or None.
    pub fn new(player: u32, p2: Option<u32>, entities: Option<&serde_json::Value>) -> Self {
        let mut known = HashMap::new();
        let snapshots = entities.and_then(|e| e.get("snapshots")?.as_array());
        let all = snapshots
            .into_iter()
            .flatten()
            .filter_map(|s| s.get("entities")?.as_array())
            .flatten();
        for entity in all {
            let id = entity.get("entity_id").and_then(|v| v.as_u64());
            let (Some(id), Some(label)) =
                (id.and_then(|id| u32::try_from(id).ok()), label_of(entity))
            else {
                continue;
            };
            // The first snapshot to name an id wins: the bind snapshot is
            // taken before the row has moved anything.
            known.entry(id).or_insert(label);
        }
        Self {
            player,
            p2,
            known,
            unknown: Vec::new(),
            frozen: false,
        }
    }

    /// A copy that names every id this one has a label for, and writes `?`
    /// for one it has not numbered, without numbering it. Used to sort
    /// events by a rendering that does not depend on which came first.
    pub fn frozen(&self) -> Self {
        Self {
            player: self.player,
            p2: self.p2,
            known: self.known.clone(),
            unknown: self.unknown.clone(),
            frozen: true,
        }
    }

    /// 0 -> `none`; player -> `self`; p2 -> `p2`; another player -> `player`;
    /// then `tag:<tag>`, `spawn:<id>`, `tpl:<id>`; else `?<n>` by first call.
    pub fn label(&mut self, id: u32) -> String {
        if id == 0 {
            return "none".to_string();
        }
        if id == self.player {
            return "self".to_string();
        }
        if self.p2 == Some(id) {
            return "p2".to_string();
        }
        if let Some(label) = self.known.get(&id) {
            return label.clone();
        }
        let n = match self.unknown.iter().position(|&u| u == id) {
            Some(i) => i + 1,
            None if self.frozen => return "?".to_string(),
            None => {
                self.unknown.push(id);
                self.unknown.len()
            }
        };
        format!("?{n}")
    }
}

/// The label a snapshot entity gets when it is not self or p2. None when
/// the snapshot gives nothing to name it by, so it numbers as unknown.
fn label_of(entity: &serde_json::Value) -> Option<String> {
    if entity.get("is_player").and_then(|v| v.as_bool()) == Some(true) {
        return Some("player".to_string());
    }
    if let Some(tag) = entity.get("tag").and_then(|v| v.as_str()) {
        return Some(format!("tag:{tag}"));
    }
    if let Some(spawn) = entity.get("spawn_id").and_then(|v| v.as_u64()) {
        return Some(format!("spawn:{spawn}"));
    }
    entity
        .get("template_id")
        .and_then(|v| v.as_u64())
        .map(|tpl| format!("tpl:{tpl}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The seed fixture (`fixtures/fs_p3_entities.json`): self, a tagged
    /// NPC and a tagged NPC with a template, and 100745 left out on purpose.
    #[test]
    fn labels_follow_the_precedence() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/fs_p3_entities.json")).unwrap();
        let mut labels = EntityLabels::new(8, Some(9), Some(&fixture));
        assert_eq!(labels.label(8), "self");
        assert_eq!(labels.label(100751), "tag:ArmYourself_FrostBody");
        assert_eq!(labels.label(100745), "?1");
        assert_eq!(labels.label(0), "none");
        assert_eq!(labels.label(9), "p2");
        assert_eq!(labels.label(100999), "?2");
        assert_eq!(labels.label(100745), "?1");

        // Each entity carries every lower-precedence key too, so swapping
        // any two rules in label_of changes a label.
        let other = json!({ "snapshots": [
            { "at": "bind", "entities": [
                { "entity_id": 5, "is_player": true, "tag": "T", "spawn_id": 1, "template_id": 2 },
                { "entity_id": 6, "spawn_id": 77, "template_id": 14 },
                { "entity_id": 7, "template_id": 14 },
                { "entity_id": 10, "tag": "First" },
            ] },
            { "at": "read", "entities": [
                { "entity_id": 10, "tag": "Second" },
                { "entity_id": 11, "spawn_id": null, "template_id": 3 },
            ] },
        ] });
        let mut labels = EntityLabels::new(8, None, Some(&other));
        assert_eq!(labels.label(5), "player");
        assert_eq!(labels.label(6), "spawn:77");
        assert_eq!(labels.label(7), "tpl:14");
        assert_eq!(labels.label(10), "tag:First", "the first snapshot wins");
        assert_eq!(labels.label(11), "tpl:3", "a null spawn_id falls through");
    }
}
