# Dakara DK-05 static population

The owner chose DK-02's isolated military outpost as a speculative Naquadah Repository and Loth'ta camp. The map has no matching name or recovered script. DK-05-A-01 is `(290, -17, 95)` on world 61, navmesh component 279; a client walk must confirm or correct identity and exact position. Keep the spawn and discovery region centre paired when correcting.

DK-05 spawn ids 8004-8007 place Bra'tac and Moh'katan in world 62, Rak'nor at the gate plaza and Loth'ta at the outpost. All are stationary with 30-second respawn timers. Region 2127 is the speculative Repository. Static spawns reappear on world load; they do not create mission dialog binds. Later Dakara mission packets must add step-gated `player_loaded` chains for `Dakara_E1` and `Dakara_E1_StoryRm` to restore transient dialog-set binds after relog. Do not seed actionless chains as fake restores.

Evidence and correction row: `docs/analysis/dakara-e1-rebuild/worknotes/DK-05.md` and `placements/A-world61-story-placements.md`. M2 client UAT pending.
