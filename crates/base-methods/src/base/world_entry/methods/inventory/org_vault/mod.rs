//! The Team (19) and Command (20) vaults on the base (bank-vault BV-07;
//! D-BV09, D-BV12, D-BV13, D-BV14, D-BV18).
//!
//! The items live in `sgw_organization_vault_items`, not `sgw_inventory`
//! (whose `character_id` is `NOT NULL`, audit A-23); see that table's header
//! for the shape and why no existing inventory query can reach its rows.
//! Moves in and out go through the ordinary `moveItem` (audit A-05), which
//! routes to `move_::org` when either end is an org vault.
//!
//! - [`access`]: the lock order and authorization every vault action starts
//!   with.
//! - [`open`]: the open path (`BankCellToBase::OrgVaultOpen`).
//!
//! Fan-out to other online members is not built: `broadcast_to_org` arrives
//! with ORG-07. Until then only the actor's client is updated, and another
//! member with the window open sees the change when they next open it (a
//! stale item they drag is refused and removed from their view).

pub(crate) mod access;
mod open;
#[cfg(test)]
mod tests;

pub(crate) use open::org_label;
pub use open::{handle_org_vault_open, org_vault_bag_info, OrgVaultIo, OrgVaultOpenRequest};
