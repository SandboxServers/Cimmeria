//! The round arithmetic of [`super::draw`] and [`super::return_rounds`], with
//! no database: which stacks give up how many rounds, which take how many
//! back, and what does not fit (D-AM05: rounds are never deleted).

/// How many rounds to take from each stack, in stack order, to draw up to
/// `requested`. Never takes more than a stack holds, never more than
/// `requested` in total; a short reserve yields a short draw.
pub(super) fn plan_draw(stacks: &[i32], requested: i32) -> Vec<i32> {
    let mut left = requested.max(0);
    stacks
        .iter()
        .map(|&size| {
            let take = size.max(0).min(left);
            left -= take;
            take
        })
        .collect()
}

/// Where `n` returned rounds go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReturnPlan {
    /// Rounds added to each existing stack, in stack order.
    pub merges: Vec<i32>,
    /// The size of each new stack to open, one free slot each.
    pub new_stacks: Vec<i32>,
    /// Rounds that fit nowhere; the caller keeps them in the clip.
    pub remainder: i32,
}

/// Fill existing stacks first (up to `cap` each), then open up to
/// `free_slots` new stacks of at most `cap`, and report what is left.
pub(super) fn plan_return(stacks: &[i32], cap: i32, free_slots: usize, n: i32) -> ReturnPlan {
    let cap = cap.max(0);
    let mut left = n.max(0);
    let merges = stacks
        .iter()
        .map(|&size| {
            let add = (cap - size).max(0).min(left);
            left -= add;
            add
        })
        .collect();
    let mut new_stacks = Vec::new();
    while left > 0 && cap > 0 && new_stacks.len() < free_slots {
        let size = cap.min(left);
        new_stacks.push(size);
        left -= size;
    }
    ReturnPlan {
        merges,
        new_stacks,
        remainder: left,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_exact_fit_empties_the_stack() {
        assert_eq!(plan_draw(&[12], 12), vec![12]);
    }

    #[test]
    fn draw_from_a_bigger_stack_takes_only_what_was_asked() {
        // 18/30 clip, 100-round stack: 12 load, 88 stay (#1026 AC 3a).
        assert_eq!(plan_draw(&[100], 12), vec![12]);
    }

    #[test]
    fn draw_short_stack_loads_what_is_there() {
        // 18/30 clip, 5-round stack: 5 load (#1026 AC 3a).
        assert_eq!(plan_draw(&[5], 12), vec![5]);
    }

    #[test]
    fn draw_spans_stacks_in_order() {
        assert_eq!(plan_draw(&[4, 10, 7], 12), vec![4, 8, 0]);
    }

    #[test]
    fn draw_of_nothing_or_from_nothing_takes_nothing() {
        assert_eq!(plan_draw(&[10], 0), vec![0]);
        assert_eq!(plan_draw(&[10], -3), vec![0]);
        assert_eq!(plan_draw(&[], 12), Vec::<i32>::new());
        assert_eq!(plan_draw(&[0, -1], 12), vec![0, 0]);
    }

    #[test]
    fn return_merges_into_existing_stacks_first() {
        let p = plan_return(&[490, 100], 500, 5, 30);
        assert_eq!(p.merges, vec![10, 20]);
        assert!(p.new_stacks.is_empty());
        assert_eq!(p.remainder, 0);
    }

    #[test]
    fn return_opens_a_new_stack_when_existing_ones_are_full() {
        let p = plan_return(&[500], 500, 1, 12);
        assert_eq!(p.merges, vec![0]);
        assert_eq!(p.new_stacks, vec![12]);
        assert_eq!(p.remainder, 0);
    }

    #[test]
    fn return_over_capacity_keeps_the_remainder() {
        // Bags full: nothing fits, all 12 stay in the clip.
        assert_eq!(
            plan_return(&[500], 500, 0, 12),
            ReturnPlan {
                merges: vec![0],
                new_stacks: vec![],
                remainder: 12
            }
        );
        // Partial fit: 3 merge, one new stack of 4 (cap 4), 5 left over.
        assert_eq!(
            plan_return(&[1], 4, 1, 12),
            ReturnPlan {
                merges: vec![3],
                new_stacks: vec![4],
                remainder: 5
            }
        );
    }

    #[test]
    fn return_of_nothing_changes_nothing() {
        let p = plan_return(&[10], 500, 3, 0);
        assert_eq!(p.merges, vec![0]);
        assert!(p.new_stacks.is_empty());
        assert_eq!(p.remainder, 0);
    }

    /// Every round is accounted for, whatever the inputs.
    #[test]
    fn return_conserves_rounds() {
        for stacks in [vec![], vec![0], vec![499, 3], vec![500, 500]] {
            for free in 0..3 {
                for n in [0, 1, 12, 30, 1200] {
                    let p = plan_return(&stacks, 500, free, n);
                    let placed: i32 =
                        p.merges.iter().sum::<i32>() + p.new_stacks.iter().sum::<i32>();
                    assert_eq!(placed + p.remainder, n, "{stacks:?} free={free} n={n}");
                    assert!(p.new_stacks.len() <= free);
                }
            }
        }
    }
}
