//! The hold anchor: where a server-held entity stood when its hold began.

use super::MovementValidator;

impl MovementValidator {
    /// The anchor for `entity_id` while `held`, recording `current` the first
    /// time. When not held, drops any anchor and returns `None`.
    pub fn hold_anchor(
        &mut self,
        entity_id: u32,
        held: bool,
        current: [f32; 3],
    ) -> Option<[f32; 3]> {
        if held {
            Some(*self.hold_anchors.entry(entity_id).or_insert(current))
        } else {
            self.hold_anchors.remove(&entity_id);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::MovementValidator;

    #[test]
    fn first_held_call_records_current_and_later_calls_keep_it() {
        let mut v = MovementValidator::new();
        assert_eq!(
            v.hold_anchor(1, true, [1.0, 2.0, 3.0]),
            Some([1.0, 2.0, 3.0])
        );
        assert_eq!(
            v.hold_anchor(1, true, [9.0, 9.0, 9.0]),
            Some([1.0, 2.0, 3.0]),
            "the anchor must not follow later positions"
        );
    }

    #[test]
    fn not_held_call_drops_the_anchor() {
        let mut v = MovementValidator::new();
        v.hold_anchor(1, true, [1.0, 0.0, 1.0]);
        assert_eq!(v.hold_anchor(1, false, [5.0, 0.0, 5.0]), None);
        assert_eq!(
            v.hold_anchor(1, true, [7.0, 0.0, 7.0]),
            Some([7.0, 0.0, 7.0]),
            "a new hold must re-anchor where the entity now stands"
        );
    }

    #[test]
    fn forget_drops_the_anchor() {
        let mut v = MovementValidator::new();
        v.hold_anchor(1, true, [1.0, 0.0, 1.0]);
        v.forget(1);
        assert_eq!(
            v.hold_anchor(1, true, [4.0, 0.0, 4.0]),
            Some([4.0, 0.0, 4.0])
        );
    }

    #[test]
    fn authorized_teleport_drops_the_anchor() {
        let mut v = MovementValidator::new();
        v.hold_anchor(1, true, [1.0, 0.0, 1.0]);
        v.note_authorized_teleport(1, Instant::now());
        assert_eq!(
            v.hold_anchor(1, true, [6.0, 0.0, 6.0]),
            Some([6.0, 0.0, 6.0])
        );
    }
}
