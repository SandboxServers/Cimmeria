//! UO-30 Omega Consumables: template 63828 (vendor bit 32768, moniker 27264,
//! list 63816) and its eight spawns 63865-63872 (D-UO8 B, D-UO16 B, D-UO29,
//! D-UO35).
//!
//! Revert proof: set 63828's `faction` to 10 and
//! `omega_consumables_template_sells_list_63816` fails; price a tier-1 row at
//! 75 and `omega_consumables_list_follows_d_uo29` fails; set spawn 63871's
//! `is_stationary` to false and `omega_consumables_stand_in_every_area` fails.
mod live_db {
    use super::super::live_db_union_omega_vendor_common::*;
    use crate::test_support::require_db_or_skip;

    /// `INT_VENDOR_CONSUMABLES`.
    const BIT: i64 = 32768;
    const TEMPLATE: i32 = 63828;
    const MONIKER: i32 = 27264;
    const LIST: i32 = 63816;
    const SPAWNS: [(i32, i32, &str); 8] = [
        (63865, TEMPLATE, "UnionOmega_Consumables_Common1"),
        (63866, TEMPLATE, "UnionOmega_Consumables_Common2"),
        (63867, TEMPLATE, "UnionOmega_Consumables_Common3"),
        (63868, TEMPLATE, "UnionOmega_Consumables_Sgc1"),
        (63869, TEMPLATE, "UnionOmega_Consumables_Sgc2"),
        (63870, TEMPLATE, "UnionOmega_Consumables_Sgc3"),
        (63871, TEMPLATE, "UnionOmega_Consumables_Jaffa1"),
        (63872, TEMPLATE, "UnionOmega_Consumables_Jaffa2"),
    ];

    #[tokio::test]
    async fn omega_consumables_template_sells_list_63816() {
        let pool = require_db_or_skip!();
        assert_vendor_template(&pool, TEMPLATE, BIT, MONIKER, LIST).await;
    }

    #[tokio::test]
    async fn omega_consumables_list_follows_d_uo29() {
        let pool = require_db_or_skip!();
        let rows = load_list(&pool, LIST).await;
        assert_list(LIST, &rows, Price::Consumable, None);
    }

    #[tokio::test]
    async fn omega_consumables_stand_in_every_area() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }
}
