//! Teams and Commands — BaseApp side.
//!
//! The base is the authority for persistent organizations, with the
//! database as the source of truth (D-ORG04, campaign ledger
//! `docs/analysis/organizations/`). Squads are cell state and never reach
//! this module (D-ORG03).
//!
//! - [`api`]: the lock and access reads every mutation starts with, and the
//!   vault predicate (ORG-API, which the Bank / Vault campaign builds on).
//! - [`persistence`]: the reads and the ORG-LOCK writes over
//!   `sgw_organizations`, `sgw_organization_ranks` and
//!   `sgw_organization_members`.
//! - [`character_delete`]: the character delete, which locks the
//!   character's organizations before the member rows cascade.
//! - [`audit`]: exports the member-delete trigger's `sgw_organization_events`
//!   rows to the `org` log target.
//!
//! - [`handlers`]: login restore, presence, leave and disband (ORG-06), and
//!   the fanout the later packets build on.
//!
//! Modelled on `base::contact_list`. Invite, kick, rank and text handlers
//! arrive with ORG-07 and ORG-08.

pub mod api;
pub mod audit;
pub mod character_delete;
pub mod handlers;
pub mod persistence;
