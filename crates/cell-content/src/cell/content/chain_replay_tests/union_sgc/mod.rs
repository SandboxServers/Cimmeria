//! Union SGC_W1 continuation (missions 1563-1571, the SGC halves of 1651,
//! 1654 and 1655, crossings C1 and C4). Ledger:
//! `docs/analysis/union-sgc-continuation/`; packets:
//! `docs/analysis/union-zone-planning/sgc-packets/`.
//!
//! Every module drives the seeded chains through the real dispatchers on an
//! instanced SGC_W1 built by [`rig::Rig`].

mod rig;

mod aux_1571;
mod clear_way_1567;
mod depart_1570;
mod disjoint;
mod escort_1564;
mod failsafe_1564;
mod gate_room_1568;
mod generator_1651;
mod letter_1655_human;
mod letter_1655_jaffa;
mod ordnance_1569;
mod return_state;
mod tactics_1565_1566;
mod turnin_1654;
mod virus_1563;
mod world_seeds;
