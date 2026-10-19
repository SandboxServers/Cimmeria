//! Beta Site E1 (world 23) `interact_tag` allowlist (Union Beta E1 campaign,
//! docs/analysis/union-zone-planning/beta-work-packets.md). Beta chains bind
//! dialogs per player (`add_dialog_set`, slot = template) and Beta objects carry
//! INT_MissionWorldObject from their template (UB-02a), so no Beta chain calls
//! `set_interaction_type`. Each chain packet appends its own rows.

/// (seed file, chain id, where the interaction bit comes from)
const BETA_E1: &[(&str, i32, &str)] = &[
    (
        "beta_e1_m481_chains.sql",
        23312,
        "BetaE1_Carter: per-player bind dsm 2465 (chain 23310)",
    ),
    (
        "beta_e1_m481_chains.sql",
        23314,
        "BetaSite_00000007: template 3 static INT_RingNetwork (32)",
    ),
    (
        "beta_e1_m481_chains.sql",
        23315,
        "BetaSite_Beam_000007_Ship: template 3 static INT_RingNetwork (32)",
    ),
    (
        "beta_e1_m481_chains.sql",
        23316,
        "BetaSite_00000007: template 3 static INT_RingNetwork (32)",
    ),
    (
        "beta_e1_m481_chains.sql",
        23317,
        "BetaE1_Mimir: per-player bind dsm 2467 (chain 23312)",
    ),
    (
        "beta_e1_m420_chains.sql",
        23341,
        "BetaE1_Mimir: per-player bind dsm 2344/2343 (chains 23340/23342)",
    ),
    (
        "beta_e1_m420_chains.sql",
        23343,
        "BetaE1_AsgardScanner_1: template 2330 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m420_chains.sql",
        23344,
        "BetaE1_AsgardScanner_2: template 2330 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m420_chains.sql",
        23345,
        "BetaE1_AsgardScanner_3: template 2330 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m420_chains.sql",
        23346,
        "BetaE1_AsgardScanner_4: template 2330 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m420_chains.sql",
        23347,
        "BetaE1_AsgardScanner_5: template 2330 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m420_chains.sql",
        23349,
        "BetaE1_Mimir: per-player bind dsm 2344/2343 (chains 23340/23342)",
    ),
    (
        "beta_e1_m420_chains.sql",
        23350,
        "BetaE1_Mimir: per-player bind dsm 2344/2343 (chains 23340/23342)",
    ),
];

/// Whether `chain_id` in `filename` is a Beta chain that has an
/// `interact_tag` trigger without `set_interaction_type` by design.
pub(super) fn allowed(filename: &str, chain_id: i32) -> bool {
    BETA_E1
        .iter()
        .any(|(f, c, _)| *f == filename && *c == chain_id)
}
