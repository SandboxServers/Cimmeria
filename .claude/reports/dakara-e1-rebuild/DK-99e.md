# DK-99e report

- Guide: docs/guides/unified-uat.md. Rows DK-U21/24/25/26/28/30 replaced verbatim from the README on this branch (after A1 fixes); DK-U32..U37 inserted after DK-U31; relative links fixed; status line (A2), prerequisites (A3/A4, DK-U37 text and minigame/SGC sentences), test order split 15a/15b (A5).
- A1 in README: removed "Two-way only once US-30 is merged", DK-U30 O5961 wording, "behind US-30 and US-31" in the M4/M5 intro; also pointed the M3/M4 "1647 GM-completed" sentences at the guide's setup (needed because `.missioncomplete` does not exist).
- R1 SQL verified against db/sgw/Missions/Tables/sgw_mission.sql, db/sgw/Players/Tables/sgw_player.sql, db/resources/Missions/Tables/mission_steps.sql and mission_objectives.sql: all table and column names match, no correction. status 2 = MISSION_COMPLETED (crates/entity/src/missions.rs). Seed has 1647 steps 4910-4917. Not run against a live DB or client.
- Grounding: Vocuum = template 457 (GA-Hologram_Platform00), spawn 8098 at (129.40,-21.53,-66.50) world 61, chains 8364/8365; gate 27 arrival (167.00,-10.80,18.00); Livewire start_minigame difficulty 1 in chains 8223 and 8258.
- Never run in a client: every step in the guide, including Livewire windows, the pad, gate 27 arrival, template 37 dial window, the R1 setup.
- RESUME.md rewritten (25 lines). markdownlint clean for the three files; test_ledgers.py passes; no link-check script found, links checked by hand. regen --check shows unrelated stale gen blocks (tests counts) in files I did not touch; not edited.
- Debt: the DK-99e gate in wave-99.md still mentions US-30/US-31 fallback text; left as is (not in scope file).
