//! Live-DB guard for [`load_ability_sets`] (AB-N2, `gmSetMobAbilitySet`):
//! the sets load grouped by id with their seeded abilities, so the GM
//! command has data to swap a mob onto. Fails if the query drops rows
//! (set 353 has five), mis-groups them, or returns nothing.

mod live_db {
    use crate::cell::spawner::load_ability_sets;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_ability_sets_load_grouped_by_set() {
        let pool = require_db_or_skip!();
        let sets = load_ability_sets(&pool).await.expect("load ability sets");
        assert_eq!(sets.get(&1), Some(&vec![579]), "set 1: pistol");
        assert_eq!(sets.get(&350), Some(&vec![221, 1156]), "Straegis pet");
        assert_eq!(
            sets.get(&353),
            Some(&vec![1653, 3326, 3327, 3328, 3329]),
            "every row of a multi-ability set, ascending"
        );
    }
}
