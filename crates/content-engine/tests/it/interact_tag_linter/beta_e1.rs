//! Beta Site E1 (world 23) `interact_tag` allowlist (Union Beta E1 campaign,
//! docs/analysis/union-zone-planning/beta-work-packets.md). Beta chains bind
//! dialogs per player (`add_dialog_set`, slot = template) and Beta objects carry
//! INT_MissionWorldObject from their template (UB-02a), so no Beta chain calls
//! `set_interaction_type`. Each chain packet appends its own rows.

/// (seed file, chain id, where the interaction bit comes from)
const BETA_E1: &[(&str, i32, &str)] = &[
    (
        "beta_e1_m1500_chains.sql",
        23222,
        "BetaE1_Carter: per-player bind dsm 5871 (chain 23220)",
    ),
    (
        "beta_e1_m1502_chains.sql",
        23241,
        "BetaE1_RasNor: per-player bind dsm 5885/5887 (chains 23240/23248)",
    ),
    (
        "beta_e1_m1502_chains.sql",
        23244,
        "BetaE1_JaffaBox: template 2326 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1502_chains.sql",
        23245,
        "BetaE1_JaffaBox: template 2326 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1502_chains.sql",
        23247,
        "BetaE1_JaffaBox: template 2326 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1502_chains.sql",
        23250,
        "BetaE1_RasNor: per-player bind dsm 5885/5887 (chains 23240/23248)",
    ),
];

/// Whether `chain_id` in `filename` is a Beta chain that has an
/// `interact_tag` trigger without `set_interaction_type` by design.
pub(super) fn allowed(filename: &str, chain_id: i32) -> bool {
    BETA_E1
        .iter()
        .any(|(f, c, _)| *f == filename && *c == chain_id)
}
