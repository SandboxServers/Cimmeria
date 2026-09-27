//! The plan a crafting transaction applies, and the checks on its shape
//! that need no database.

use super::failure::CraftTxError;

/// An item instance the request named, with the design the plan expects
/// it to be. The transaction checks both under the row lock: an instance
/// of another design is refused, so a request cannot name one carried
/// item and have a different design consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedItem {
    pub item_id: i32,
    pub design_id: i32,
}

impl NamedItem {
    pub fn new(item_id: i32, design_id: i32) -> Self {
        Self { item_id, design_id }
    }
}

/// A blueprint and its discipline the player must still know when the
/// transaction runs. Checked under a share lock on the player row, after
/// consumption and placement, so a respec cannot slip in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequiredKnowledge {
    pub blueprint_id: i32,
    pub discipline_id: i32,
}

/// What one induction consumes and produces. The verb builds it from the
/// catalog and the request; this module applies it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftTransaction {
    /// Item instances the request named. Each must still belong to the
    /// player, be of its `design_id`, and sit in the main or crafting bag.
    /// Each must also be consumed by the plan: its design is in
    /// `consume`, or the instance itself is in `consume_named`.
    pub named_items: Vec<NamedItem>,
    /// `(item_id, quantity)` to take from exactly that named instance, for
    /// the verbs whose input is one particular item (reverse engineering).
    /// Every instance here must be in `named_items`. Runs before
    /// `consume`, so consumption by design never drains it first.
    pub consume_named: Vec<(i32, i32)>,
    /// `(design_id, quantity)` to consume across the main and crafting
    /// bags. A non-positive quantity refuses the whole transaction.
    pub consume: Vec<(i32, i32)>,
    /// `(design_id, quantity)` to grant. Non-positive quantities are
    /// skipped.
    pub grant: Vec<(i32, i32)>,
    /// `(discipline_id, delta)` for disciplines the player knows, clamped
    /// to `[0, 100]`. Unknown disciplines are skipped.
    pub expertise: Vec<(i32, i32)>,
    /// `(blueprint_id, discipline_id)` to add to the player's known list.
    /// A blueprint is taught only while its discipline is known, checked
    /// under the player row lock; already known ones are skipped.
    pub learn_blueprints: Vec<(i32, i32)>,
    /// Refuse the whole transaction unless the player still knows this
    /// blueprint and discipline.
    pub required_knowledge: Option<RequiredKnowledge>,
}

impl CraftTransaction {
    /// Refuse a plan whose named instances and consumption do not line up.
    /// Either is a bug in the verb that built it, never a player error, so
    /// it rolls back as `persist_failed` rather than a rule refusal.
    pub(super) fn check_shape(&self) -> Result<(), CraftTxError> {
        let invalid = |reason| {
            Err(CraftTxError::Invalid {
                phase: "plan",
                reason,
            })
        };
        for &(item_id, quantity) in &self.consume_named {
            if quantity <= 0 {
                return invalid("invalid_quantity");
            }
            if !self.named_items.iter().any(|n| n.item_id == item_id) {
                return invalid("unnamed_instance");
            }
        }
        for named in &self.named_items {
            let by_design = self.consume.iter().any(|&(d, _)| d == named.design_id);
            let exact = self
                .consume_named
                .iter()
                .any(|&(item_id, _)| item_id == named.item_id);
            if !by_design && !exact {
                return invalid("named_not_consumed");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(plan: &CraftTransaction) -> Option<&'static str> {
        match plan.check_shape() {
            Ok(()) => None,
            Err(CraftTxError::Invalid { reason, .. }) => Some(reason),
            Err(e) => panic!("unexpected {e:?}"),
        }
    }

    #[test]
    fn a_named_instance_must_be_consumed_by_design_or_exactly() {
        let by_design = CraftTransaction {
            named_items: vec![NamedItem::new(10, 8891)],
            consume: vec![(8891, 1)],
            ..CraftTransaction::default()
        };
        assert_eq!(reason(&by_design), None);

        let exact = CraftTransaction {
            named_items: vec![NamedItem::new(10, 8891)],
            consume_named: vec![(10, 1)],
            ..CraftTransaction::default()
        };
        assert_eq!(reason(&exact), None);

        let unrelated = CraftTransaction {
            named_items: vec![NamedItem::new(10, 8891)],
            consume: vec![(5189, 1)],
            ..CraftTransaction::default()
        };
        assert_eq!(reason(&unrelated), Some("named_not_consumed"));
    }

    #[test]
    fn an_exact_consumption_must_name_its_instance_and_a_positive_quantity() {
        let unnamed = CraftTransaction {
            consume_named: vec![(10, 1)],
            ..CraftTransaction::default()
        };
        assert_eq!(reason(&unnamed), Some("unnamed_instance"));

        let zero = CraftTransaction {
            named_items: vec![NamedItem::new(10, 8891)],
            consume_named: vec![(10, 0)],
            ..CraftTransaction::default()
        };
        assert_eq!(reason(&zero), Some("invalid_quantity"));
    }
}
