//! Black Market (`console/black_market.rs`): the GM test tools (BM-07).

use super::{spec, Spec, Target};

// `min` is 0 on `.bm_expire` so a bare `.bm_expire` reaches its own usage
// line and logs `bm.gm_rejected reason=no_auction_id`.
pub(super) const SPECS: &[Spec] = &[
    spec(
        "bm_seed",
        0,
        1,
        Target::None,
        "List Black Market test auctions from the system seller, 8 by default ([count 1-60])",
    ),
    spec(
        "bm_expire",
        0,
        1,
        Target::None,
        "Expire a Black Market auction now and settle it as the sweep would (auctionId)",
    ),
    spec(
        "bm_list",
        0,
        0,
        Target::None,
        "Show the newest active Black Market auctions with their ids",
    ),
];
