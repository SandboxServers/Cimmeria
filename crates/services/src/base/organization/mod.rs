//! BaseApp-side organization (Squad/Team/Command) subsystem.
//!
//! Phase 2: Squad (ephemeral) fanout.
//! Phase 3: Team/Command persistence, OrgAuthority, and roster fanout.
//!
//! Module layout:
//!
//! - `authority` — OrgAuthority singleton: in-memory org map, online tracking,
//!   permission-gated mutation, serialises to/from DB.
//! - `fanout` — `broadcast_to_org`: fire-and-forget fan-out to online members.
//! - `handlers` — Phase 2 squad handlers (base-side).
//! - `persistence` — SQL helpers: load_all_orgs, create_org, add/remove member, etc.
//! - `wire` — S→C and C→S wire serialisers/parsers.

pub mod authority;
pub mod fanout;
pub mod handlers;
pub mod persistence;
pub mod wire;

// Re-export the most-used authority type so dispatch callers can write
// `organization::OrgAuthority` instead of `organization::authority::OrgAuthority`.
#[allow(unused_imports)]
pub use authority::OrgAuthority;
