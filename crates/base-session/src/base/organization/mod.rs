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
//! Modelled on `base::contact_list`. The handlers and the org fanout arrive
//! with the later packets (ORG-05 to ORG-08).

pub mod api;
pub mod audit;
pub mod character_delete;
pub mod persistence;
