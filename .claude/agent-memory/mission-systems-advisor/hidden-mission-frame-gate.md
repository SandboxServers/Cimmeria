---
name: hidden-mission-frame-gate
description: Hidden missions (682-686, 689) send NO client mission frames since #715; every new send site in cell::missions must call suppress_hidden_mission_frames; client abandonMission refuses hidden missions
metadata:
  type: project
---

Since #715 (2026-09-28, owner decision "match the 2009 reference now"), a mission whose def is `is_hidden` never sends `onMissionUpdate` / `onStepUpdate` / `onObjectiveUpdate`. State, `MissionUpdate` persist and content events are unchanged.

- One predicate: `cell::missions::suppress_hidden_mission_frames(is_hidden, entity_id, player_id, mission_id, site)` in `crates/cell-content/src/cell/missions/mod.rs`; logs DEBUG `mission client frames suppressed` with `reason=hidden_mission`.
- Wired at: accept, abandon, advance_step, complete_objective (both the objective frame and the completion frames), complete_mission_direct, GM `.missionfail` (cell-console).
- Resend (login/respawn) was already filtered by `MissionManager::active_missions()`.
- Client `abandonMission` (cell method 52) refuses hidden missions (python `MissionInstance.abandon` :293); chain/GM abandons still remove them.
- Objective-level `hidden` is a different flag (rides visible missions' frames) and is NOT gated.

**Why:** reference `MissionManager.py` guards ~10 sites with `if not mission.mission.isHidden`. Revert a frame type only if in-game UAT (Cellblock T25/T15) shows the client needed it.

**How to apply:** any new code that sends a mission frame must go through the predicate; a chain-replay test that counts frames for 682-689 should expect zero. Guards: `missions/hidden_frames_tests.rs`, `chain_replay_tests/mission_689_hidden_frames.rs` (live DB).
