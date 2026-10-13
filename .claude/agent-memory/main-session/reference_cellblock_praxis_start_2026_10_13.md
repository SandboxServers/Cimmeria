# Cellblock fresh-character start, 2026-10-13

Live lab creation and first play of a fresh SGU Commando landed in SGC_W1
(world 58). A fresh Praxis Commando landed in Castle_CellBlock (world 12) at
the stasis-room start. This matches the seeded world-entry profiles in
`crates/base-session/src/base/world_entry/gm_only_worlds.rs`. The Praxis
Commando took the non-Jaffa tutorial branch and received the Frost and Guard
corpse interactions and pistol mission.

Use `alignment = "praxis", archetype = "Commando"` for the automated fresh
Cellblock walkthrough. The earlier SGU setup in the UAT spec and campaign
notes was stale. This observation does not establish the later combat and
Livewire steps; the first guard encounter remains blocked by the pistol
behavior tracked in issue #1667.
