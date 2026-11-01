# HT-15c worknote

Session 1 of at most 6. Base 54f1ad3df, branch ht/HT-15c-trim.

## Done

- `server-authority-enforcer/`: 19 flagged files, lint now shows none.
- `game-archaeology-specialist/`: 25 flagged files (including the ip_like flag), lint now shows none. `ue3-reflection-trace-anchors.md` got (a) only.
- 44 files in total (4 over the 40 cap, because both folders were finished in one pass).

## Counts so far

| Method | Files |
|---|---|
| (a) INVALIDATED WHEN + checked | 19 + 3 |
| (b) rewrite as method | 5 |
| (c) delete | 7 |
| (d) unverified, with INVALIDATED WHEN | 10 |

Deleted paths:
- `.claude/agent-memory/server-authority-enforcer/exploit_bandolier_ammo_toctou.md`
- `.claude/agent-memory/server-authority-enforcer/project_chat_contact_audit_2026-05-31.md`
- `.claude/agent-memory/server-authority-enforcer/project_gm_commands_audit_2026-05-31.md`
- `.claude/agent-memory/server-authority-enforcer/project_mission_dialog_audit_2026-05-31.md`
- `.claude/agent-memory/server-authority-enforcer/project_org_squad_duel_unimplemented.md`
- `.claude/agent-memory/server-authority-enforcer/project_world_space_gate_audit_2026-05-31.md`
- `.claude/agent-memory/game-archaeology-specialist/bm-create-auction-next-session-plan.md`

Notes:
- The (d) lint note: `memory_lint.py` still flags `state_no_invalidated` on a file with only `checked: unverified`, so every (d) file also carries an `INVALIDATED WHEN:` line.
- An IP-like address and a test account name were removed from `mercury-section-2-live-capture-findings.md` (replaced by a placeholder).
- Index lines were added for `client-display-knobs.md` and `client-mouse-lock-pd38.md`; the deleted file's index line was removed.

## Lint summary (whole repo)

- Before: `memory_lint files=777 no_type=68 state_no_invalidated=127 index_over=1 index_missing=1 unindexed=203 orphan_folders=18 ip_like=4`
- Now: `memory_lint files=770 no_type=68 state_no_invalidated=86 index_over=1 index_missing=1 unindexed=201 orphan_folders=18 ip_like=3`

## Left

- Remaining flagged files in my folders (counts): movement-teleport-advisor 6, mission-systems-advisor 8, items-systems-advisor 8, minigame-systems-advisor 6, network-security-auth 5, npc-ai-spawn-advisor 5, aoi-witness-broadcast 4, bigworld-engine-advisor 4, combat-systems-advisor 4, database-persistence 2. social-systems-engineer, testing-validation-engineer and documentation-writer have no flag.
- Next file: first `movement-teleport-advisor/` line in the lint output.
- `lab-driver: left` (lab-autopilot AX-61 is open).
