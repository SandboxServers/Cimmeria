//! Beta Site E1 (world 23) `interact_tag` allowlist (Union Beta E1 campaign,
//! docs/analysis/union-zone-planning/beta-work-packets.md). Beta chains bind
//! dialogs per player (`add_dialog_set`, slot = template) and Beta objects carry
//! INT_MissionWorldObject from their template (UB-02a), so no Beta chain calls
//! `set_interaction_type`. Each chain packet appends its own rows.

/// (seed file, chain id, where the interaction bit comes from)
const BETA_E1: &[(&str, i32, &str)] = &[
    (
        "beta_e1_m1423_chains.sql",
        23022,
        "BetaE1_Carter: per-player bind dsm 5433 (chain 23020)",
    ),
    (
        "beta_e1_m381_chains.sql",
        23041,
        "BetaE1_Carter: per-player bind dsm 2305 (chain 23040)",
    ),
    (
        "beta_e1_m381_chains.sql",
        23044,
        "BetaE1_Felger: per-player bind dsm 2304/2302 (chains 23042/23044)",
    ),
    (
        "beta_e1_m381_chains.sql",
        23045,
        "BetaE1_XenoCage_1: template 2320 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m381_chains.sql",
        23046,
        "BetaE1_XenoCage_2: template 2320 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m381_chains.sql",
        23047,
        "BetaE1_XenoCage_3: template 2320 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m381_chains.sql",
        23048,
        "BetaE1_XenoCage_4: template 2320 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m381_chains.sql",
        23049,
        "BetaE1_XenoCage_5: template 2320 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m381_chains.sql",
        23051,
        "BetaE1_Felger: per-player bind dsm 2304/2302 (chains 23042/23044)",
    ),
    (
        "beta_e1_m381_chains.sql",
        23052,
        "BetaE1_Felger: per-player bind dsm 2304/2302 (chains 23042/23044)",
    ),
];

/// Whether `chain_id` in `filename` is a Beta chain that has an
/// `interact_tag` trigger without `set_interaction_type` by design.
pub(super) fn allowed(filename: &str, chain_id: i32) -> bool {
    BETA_E1
        .iter()
        .any(|(f, c, _)| *f == filename && *c == chain_id)
}
