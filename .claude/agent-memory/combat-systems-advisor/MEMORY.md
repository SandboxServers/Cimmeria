# Combat Systems Advisor — Memory Index

- [pvp-duel-readiness.md](pvp-duel-readiness.md) — PvP/duel collision map: the PvE-only gate (handle.rs:223), the 5 death-tail stages that misfire for a player victim, the BSF_InCombat stuck-bit hazard
- [health-mutation-paths.md](health-mutation-paths.md) — Every HEALTH.cur mutation path + which have a ChainEngine; scripts-run-after-death, npc_ai_submit threat leak, DoT-never-kills
- [npc-ability-sets.md](npc-ability-sets.md) — NPC ability_sets seed traps: one-ability-per-set PK, event_set_id animation gate, max_range poisoning, melee-at-30m, weapon mesh in `components`
- [shipped-data-combat-evidence.md](shipped-data-combat-evidence.md) — what the ORIGINAL resource DB proves about combat formulas: taxonomy shipped, values didn't; 17-row effect_nvps; EF_DontUseQR is the QR gate; cover carries no numbers
- [combat-exit-tail-parity.md](combat-exit-tail-parity.md) — What a live-NPC combat exit (submit/leash) must copy from apply_death_transition vs. what's death-only; tick cadences; aggression non-persistence; QR-miss still generates threat
- [ontimerupdate-wire-and-clock.md](ontimerupdate-wire-and-clock.md) — Method 12's 21-byte layout + byte offsets, absolute-BigWorldTimeComplete evidence (python + RE), the SET_GAME_TIME/tickSync clock trap, and all six emit paths
- [fire-los-and-eye-heights.md](fire-los-and-eye-heights.md) — NA31 player fire LoS (error 39, tolerance rays) + body_sets.eye_height from ref-mesh bounds
- [navmesh-los-reliability.md](navmesh-los-reliability.md) — navmesh LoS vs collision geometry: 45% false Blocked; PRU desk (S11); rejected heuristics; no fire-time LoS until an occluder exists
- [qr-direction-and-cover.md](qr-direction-and-cover.md) — python QR beta branches were inverted (NA32 swapped them); cover QR units from alias.xml; test new QR terms on damage
