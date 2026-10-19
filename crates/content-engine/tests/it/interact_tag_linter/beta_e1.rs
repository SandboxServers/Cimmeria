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
    (
        "beta_e1_m473_chains.sql",
        23071,
        "BetaE1_Felger: per-player bind dsm 2440 (chain 23070)",
    ),
    (
        "beta_e1_m473_chains.sql",
        23074,
        "BetaE1_Crate_Arc42: template 2325 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m473_chains.sql",
        23075,
        "BetaE1_Crate_Arc66: template 2325 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m473_chains.sql",
        23076,
        "BetaE1_Crate_Arc42: template 2325 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m473_chains.sql",
        23077,
        "BetaE1_Crate_Arc42: template 2325 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m473_chains.sql",
        23078,
        "BetaE1_Crate_Arc66: template 2325 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m473_chains.sql",
        23079,
        "BetaE1_Crate_Arc66: template 2325 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m473_chains.sql",
        23081,
        "BetaE1_Felger: per-player bind dsm 2439 (chains 23077/23079/23080)",
    ),
    (
        "beta_e1_m473_chains.sql",
        23083,
        "BetaE1_Carter: per-player bind dsm 2438 (chains 23081/23082)",
    ),
    (
        "beta_e1_m478_chains.sql",
        23101,
        "BetaE1_Carter: per-player bind dsm 2460 (chain 23100)",
    ),
    (
        "beta_e1_m478_chains.sql",
        23104,
        "BetaE1_AdminWorkstation: template 2323 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m478_chains.sql",
        23105,
        "BetaE1_AdminWorkstation: template 2323 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m478_chains.sql",
        23108,
        "BetaE1_Carter: per-player bind dsm 3884 (chains 23106/23107)",
    ),
    (
        "beta_e1_m1425_chains.sql",
        23131,
        "BetaE1_Carter: per-player bind dsm 5457 (chain 23130)",
    ),
    (
        "beta_e1_m1425_chains.sql",
        23133,
        "BetaE1_HubComputer: template 2322 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1425_chains.sql",
        23135,
        "BetaE1_ScienceComputer: template 2324 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1426_chains.sql",
        23163,
        "BetaE1_Carter: per-player bind dsm 5460 (chains 23160-23162)",
    ),
    (
        "beta_e1_m1426_chains.sql",
        23164,
        "BetaE1_Carter: per-player bind dsm 5460 (chains 23160-23162)",
    ),
    (
        "beta_e1_m1426_chains.sql",
        23167,
        "BetaE1_FourPiecesCase: template 2321 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1426_chains.sql",
        23168,
        "BetaE1_HubComputer: template 2322 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1426_chains.sql",
        23171,
        "BetaE1_Carter: per-player bind dsm 5462 (chains 23169/23170)",
    ),
    (
        "beta_e1_m1427_chains.sql",
        23191,
        "BetaE1_Carter: per-player bind dsm 5464 (chains 23190/23192)",
    ),
    (
        "beta_e1_m1427_chains.sql",
        23193,
        "BetaE1_RuinNote: template 2335 static INT_MissionWorldObject",
    ),
    (
        "beta_e1_m1427_chains.sql",
        23195,
        "BetaE1_Carter: per-player bind dsm 5466 (chains 23193/23194)",
    ),
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
];

/// Whether `chain_id` in `filename` is a Beta chain that has an
/// `interact_tag` trigger without `set_interaction_type` by design.
pub(super) fn allowed(filename: &str, chain_id: i32) -> bool {
    BETA_E1
        .iter()
        .any(|(f, c, _)| *f == filename && *c == chain_id)
}
