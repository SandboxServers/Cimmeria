//! Union Omega hub services (`docs/analysis/union-zone-planning/omega-work-packets.md`,
//! D-UO19, D-UO31, D-UO35). Each service template carries its interaction bit as a
//! template default, so no service chain calls `set_interaction_type`.

/// (seed file, chain id, where the interaction bit comes from)
const UNION_OMEGA: &[(&str, i32, &str)] = &[
    (
        "union_omega_services_chains.sql",
        63850,
        "UnionOmega_Auctioneer: template 305 default INT_Auction (4)",
    ),
    (
        "union_omega_services_chains.sql",
        63851,
        "UnionOmega_Medic: template 63855 default INT_NonAStoryMissionAvaliable (134217728)",
    ),
    (
        "union_omega_services_chains.sql",
        63852,
        "UnionOmega_Guide_Biology: template 63829 default INT_Trainer (128)",
    ),
    (
        "union_omega_services_chains.sql",
        63853,
        "UnionOmega_Guide_Electronics: template 63829 default INT_Trainer (128)",
    ),
    (
        "union_omega_services_chains.sql",
        63854,
        "UnionOmega_Guide_Engineering: template 63829 default INT_Trainer (128)",
    ),
    (
        "union_omega_services_chains.sql",
        63855,
        "UnionOmega_Guide_Physics: template 63829 default INT_Trainer (128)",
    ),
    (
        "union_omega_services_chains.sql",
        63856,
        "UnionOmega_Inn: template 63856 default INT_NonAStoryMissionAvaliable (134217728)",
    ),
    (
        "union_omega_services_chains.sql",
        63857,
        "UnionOmega_Battlemaster: template 63857 default INT_NonAStoryMissionAvaliable (134217728)",
    ),
];

/// Whether `chain_id` in `filename` is an Omega service chain with an
/// `interact_tag` trigger and no `set_interaction_type`, by design.
pub(super) fn allowed(filename: &str, chain_id: i32) -> bool {
    UNION_OMEGA
        .iter()
        .any(|(f, c, _)| *f == filename && *c == chain_id)
}
