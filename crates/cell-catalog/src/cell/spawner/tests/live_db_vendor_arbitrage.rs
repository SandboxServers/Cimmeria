//! Live-DB guard: no vendor sells an item for less than any vendor buys it
//! back (DA-02 review F1).
//!
//! A buy row priced below a sell list's price for the same design mints
//! naquadah: buy it, sell it back, repeat, and only the `i32` overflow check
//! stops the loop; mail, trade and the org treasury then move the money to
//! any account. The Debug Area's munitions vendor first sold the pistol (55)
//! and the SMG (21) for 1 naquadah while sell list 2 pays 300 and 1000.
//!
//! Seed-wide on purpose: every list a template uses as its
//! `buy_item_list`, against every list any template uses as its
//! `sell_item_list`, so the next cheap debug list trips it too. A buy row
//! that also costs items (`item_list_prices`) is skipped; its real price is
//! not naquadah alone. A buy row hands over `quantity` units for its price,
//! each sellable at the sell row's per-unit price.
//!
//! Revert proof: price item_lists_debug_area_plaza.sql's rows 13017/13018
//! back at 1 and this fails naming designs 55 and 21.
mod live_db {
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_no_buy_row_is_cheaper_than_a_sell_back() {
        let pool = require_db_or_skip!();
        let arbitrage: Vec<(i32, i32, i32, i32, i32, i32)> = sqlx::query_as(
            "SELECT DISTINCT b.item_list_id, b.design_id, b.naquadah, b.quantity, \
                    s.item_list_id, s.naquadah \
               FROM resources.item_list_items b \
               JOIN resources.item_list_items s ON s.design_id = b.design_id \
              WHERE b.item_list_id IN (SELECT buy_item_list FROM resources.entity_templates \
                                        WHERE buy_item_list IS NOT NULL) \
                AND s.item_list_id IN (SELECT sell_item_list FROM resources.entity_templates \
                                        WHERE sell_item_list IS NOT NULL) \
                AND NOT EXISTS (SELECT 1 FROM resources.item_list_prices p \
                                 WHERE p.item_id = b.item_id) \
                AND b.naquadah::bigint < s.naquadah::bigint * b.quantity \
              ORDER BY 2, 1, 5",
        )
        .fetch_all(&pool)
        .await
        .expect("arbitrage query");
        assert!(
            arbitrage.is_empty(),
            "buy rows cheaper than a sell-back, as (buy list, design, buy price, \
             quantity, sell list, sell price per unit): {arbitrage:?}"
        );

        // Control: the query sees the seed's real buy and sell lists, so an
        // empty answer is not an empty join.
        let pairs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resources.item_list_items b \
               JOIN resources.item_list_items s ON s.design_id = b.design_id \
              WHERE b.item_list_id IN (SELECT buy_item_list FROM resources.entity_templates) \
                AND s.item_list_id IN (SELECT sell_item_list FROM resources.entity_templates)",
        )
        .fetch_one(&pool)
        .await
        .expect("control query");
        assert!(
            pairs > 0,
            "the seed must have an item both bought and sold (the Debug Area pistol and SMG)"
        );
    }
}
