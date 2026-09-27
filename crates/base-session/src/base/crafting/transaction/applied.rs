//! What a committed crafting transaction changed, with before and after
//! quantities, and its `completed` event fields.

use crate::base::crafting::session::JobReport;

/// A stack consumption touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumedStack {
    pub item_id: i32,
    pub type_id: i32,
    pub container_id: i32,
    pub before: i32,
    /// 0 when the stack was emptied and its row deleted.
    pub after: i32,
}

/// A product stack the transaction created (`before == 0`) or merged into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedStack {
    pub item_id: i32,
    pub design_id: i32,
    pub container_id: i32,
    pub slot_id: i32,
    pub before: i32,
    pub after: i32,
}

impl GrantedStack {
    pub fn quantity(&self) -> i32 {
        self.after - self.before
    }
}

/// A discipline whose expertise changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpertiseChange {
    pub discipline_id: i32,
    pub before: i32,
    pub after: i32,
}

/// Blueprints the transaction taught.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintsLearned {
    /// The blueprints that were not known before, in id order.
    pub taught: Vec<i32>,
    /// How many blueprints the player knew before.
    pub known_before: usize,
    /// The whole known list after, sorted: what 139 sends.
    pub blueprint_ids: Vec<i32>,
}

impl BlueprintsLearned {
    /// The `blueprints` field of `blueprint_learned`:
    /// `blueprint_id:known_before→known_after` per blueprint taught.
    pub fn field(&self) -> String {
        self.taught
            .iter()
            .map(|id| format!("{id}:false→true"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// What a committed transaction changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftApplied {
    /// Every stack step consumption took, in order.
    pub consumed: Vec<ConsumedStack>,
    /// Product stacks, new or merged.
    pub granted: Vec<GrantedStack>,
    pub expertise: Vec<ExpertiseChange>,
    /// Set only when the plan taught at least one new blueprint.
    pub blueprints: Option<BlueprintsLearned>,
}

impl CraftApplied {
    /// Stacks consumed to nothing: their rows are gone.
    pub fn drained(&self) -> Vec<ConsumedStack> {
        self.consumed
            .iter()
            .filter(|c| c.after == 0)
            .copied()
            .collect()
    }

    /// Instances whose rows still exist and changed: sent in one
    /// `onUpdateItem`.
    pub fn updated_item_ids(&self) -> Vec<i32> {
        let drained: Vec<i32> = self.drained().iter().map(|c| c.item_id).collect();
        let mut ids = Vec::new();
        let touched = self
            .consumed
            .iter()
            .map(|c| c.item_id)
            .chain(self.granted.iter().map(|g| g.item_id));
        for id in touched {
            if !drained.contains(&id) && !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    /// `item_id:type_id:qty_before→qty_after`, comma-separated.
    pub fn consumed_field(&self) -> String {
        self.consumed
            .iter()
            .map(|c| format!("{}:{}:{}→{}", c.item_id, c.type_id, c.before, c.after))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// `type_id:bag:slot:qty_before→qty_after`, comma-separated. A merge
    /// has a non-zero `qty_before`; a new stack starts at 0.
    pub fn granted_field(&self) -> String {
        self.granted
            .iter()
            .map(|g| {
                format!(
                    "{}:{}:{}:{}→{}",
                    g.design_id, g.container_id, g.slot_id, g.before, g.after
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    /// `discipline_id:expertise_before→expertise_after`, comma-separated.
    pub fn expertise_field(&self) -> String {
        self.expertise
            .iter()
            .map(|e| format!("{}:{}→{}", e.discipline_id, e.before, e.after))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The `completed` event's item and expertise lists. A verb adds its
    /// own ids and roll to the report before returning it.
    pub fn report(&self) -> JobReport {
        JobReport {
            consumed: self.consumed_field(),
            granted: self.granted_field(),
            expertise: self.expertise_field(),
            blueprints_learned: self
                .blueprints
                .as_ref()
                .map(BlueprintsLearned::field)
                .unwrap_or_default(),
            ..JobReport::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied() -> CraftApplied {
        CraftApplied {
            consumed: vec![
                ConsumedStack {
                    item_id: 10,
                    type_id: 8891,
                    container_id: 15,
                    before: 2,
                    after: 0,
                },
                ConsumedStack {
                    item_id: 11,
                    type_id: 8891,
                    container_id: 1,
                    before: 2,
                    after: 1,
                },
            ],
            granted: vec![
                GrantedStack {
                    item_id: 12,
                    design_id: 2893,
                    container_id: 1,
                    slot_id: 5,
                    before: 5,
                    after: 8,
                },
                GrantedStack {
                    item_id: 13,
                    design_id: 5188,
                    container_id: 15,
                    slot_id: 0,
                    before: 0,
                    after: 1,
                },
            ],
            expertise: vec![ExpertiseChange {
                discipline_id: 21,
                before: 98,
                after: 100,
            }],
            blueprints: None,
        }
    }

    #[test]
    fn event_fields_carry_before_and_after() {
        let a = applied();
        assert_eq!(a.consumed_field(), "10:8891:2→0,11:8891:2→1");
        assert_eq!(a.granted_field(), "2893:1:5:5→8,5188:15:0:0→1");
        assert_eq!(a.expertise_field(), "21:98→100");
        assert_eq!(a.granted[0].quantity(), 3);
        let report = a.report();
        assert_eq!(report.consumed, a.consumed_field());
        assert_eq!(report.granted, a.granted_field());
        assert_eq!(report.expertise, a.expertise_field());
        assert_eq!(report.blueprints_learned, "");
    }

    #[test]
    fn taught_blueprints_are_listed_before_and_after() {
        let learned = BlueprintsLearned {
            taught: vec![1, 7],
            known_before: 2,
            blueprint_ids: vec![1, 3, 5, 7],
        };
        assert_eq!(learned.field(), "1:false→true,7:false→true");
        let a = CraftApplied {
            blueprints: Some(learned),
            ..CraftApplied::default()
        };
        assert_eq!(a.report().blueprints_learned, "1:false→true,7:false→true");
    }

    #[test]
    fn drained_stacks_are_removed_and_the_rest_updated() {
        let a = applied();
        assert_eq!(
            a.drained().iter().map(|c| c.item_id).collect::<Vec<_>>(),
            vec![10]
        );
        assert_eq!(a.updated_item_ids(), vec![11, 12, 13]);
    }
}
