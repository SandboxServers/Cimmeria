# Owner decisions (2026-10-18)

- D1: Dev and lab accounts (and their characters) DO exist on the colo, seeded once, for works-on-not-my-machine testing.
- D2: `docker restart` must NOT wipe accounts or characters. Anything else about restart/deploy behavior: let it do its thing (resources reseed).
- D3: A forced DB reseed (`setup.ps1 -ForceDatabase` and equivalents) reseeds RESOURCES ONLY. It no longer wipes accounts/characters. (Overrides the earlier recommendation of a full wipe.)
- D4 (O4): First persistent deploy starts FRESH from seeds; no import of the pre-switch dump required (D-PP12 import may be dropped or kept optional for disaster recovery only).
- D5 (O6): Dangling ids: delete orphan rows with JSON log, relocate players in removed worlds to Castle_CellBlock.
- D6 (O5): Seed accounts missing from the DB are AUTO-INSERTED on every rebuild (adds a packet; must not touch existing accounts).
- D7 (O7-O9): Accept defaults: fail closed never wipe; on-host backups newest 10; dependabot ignores Postgres majors.
- D8 (O10): Main checkout test DB moves to `sgwtest`; reload-db refuses `sgw`. PP-15 unblocked, stays in wave 1.
- Implication to design: a full wipe of player data needs an explicit, separate, deliberately named action (architect to decide).
