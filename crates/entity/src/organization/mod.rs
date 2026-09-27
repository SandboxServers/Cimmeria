//! Organization models: Squads, Teams and Commands.
//!
//! The contract every organizations packet builds against (campaign ledger
//! `docs/analysis/organizations/work-packets.md` § "Contract fixed by this
//! ledger"). The client ships the whole UI; the 2009 server never
//! implemented any of it, so every value here is either pinned to
//! `entities/defs/enumerations.xml` or is project policy named by a D-ORG
//! decision in `docs/analysis/organizations/README.md`.
//!
//! - [`types`]: `OrgType`, `OrgRank`, `OrgLeaveReason`, `SquadLootType`.
//! - [`permissions`]: `OrgPermission` and the D-ORG08 / D-ORG21 default rank table.
//! - [`limits`]: the id-space constants (D-ORG05, D-ORG06) and the size and
//!   text caps (D-ORG10).
//! - [`org_text`]: the one implementation of the D-ORG10 text rules.

pub mod limits;
pub mod org_text;
pub mod permissions;
pub mod types;

pub use limits::{
    route_invite_request, route_org_id, InviteRoute, OrgRoute, BASE_INVITE_REQUEST_FLAG,
    MAX_MOTD_UNITS, MAX_NAME_UNITS, MAX_NOTE_UNITS, MAX_OFFICER_NOTE_UNITS, MAX_ORG_ID,
    MAX_RANK_NAME_UNITS, MAX_SQUAD_SIZE, MIN_NAME_UNITS, SQUAD_ORG_ID_MAX, SQUAD_ORG_ID_MIN,
};
pub use org_text::{TextField, TextReject};
pub use permissions::{default_rank_permissions, OrgPermission, PermEditReject};
pub use types::{CashDir, OrgLeaveReason, OrgRank, OrgType, SquadLootType, UnknownValue};

#[cfg(test)]
mod tests;
