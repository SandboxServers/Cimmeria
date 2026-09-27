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
//! - [`creation`]: founding a Team or Command (ORG-05): the registrar's
//!   eligibility check, the named creation with its D-ORG15 cost, the GM
//!   `.org_create`, and the founder's push.
//!
//! - [`handlers`]: login restore, presence, leave and disband (ORG-06);
//!   invite, invite response, kick, rank change, the GM `.org_join` and
//!   `.org_rank`, and `broadcast_to_org` (ORG-07).
//! - [`invites`]: the pending Team and Command invites, held on the
//!   invitee's session (D-ORG06).
//!
//! Modelled on `base::contact_list`. The text and rank-editor handlers
//! arrive with ORG-08.

pub mod api;
pub mod audit;
pub mod character_delete;
pub mod creation;
pub mod handlers;
pub mod invites;
pub mod persistence;
