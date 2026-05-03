//! Mission state tracking for player entities.
//!
//! Each player has a `MissionManager` that tracks active, completed, and failed
//! missions. Missions have a hierarchy: Mission → Steps → Objectives → Tasks.
//!
//! Reference: `python/cell/MissionManager.py`, `db/sgw/Missions/Tables/sgw_mission.sql`

use std::collections::HashMap;

// ── Mission Status Codes ─────────────────────────────────────────────────────
// From `python/common/Constants.py`

pub const MISSION_NOT_ACTIVE: i8 = 0;
pub const MISSION_ACTIVE: i8 = 1;
pub const MISSION_COMPLETED: i8 = 2;
pub const MISSION_FAILED: i8 = 3;

// ── Objective / Step Status ──────────────────────────────────────────────────

pub const STATUS_ACTIVE: i8 = 0;
pub const STATUS_COMPLETED: i8 = 1;

// ── Data Structures ──────────────────────────────────────────────────────────

/// A tracked mission objective.
#[derive(Debug, Clone)]
pub struct MissionObjective {
    pub objective_id: i32,
    pub status: i8,
    pub hidden: bool,
    pub optional: bool,
}

/// A tracked mission step.
#[derive(Debug, Clone)]
pub struct MissionStep {
    pub step_id: i32,
    pub status: i8,
}

/// A single active mission instance tracked per player.
#[derive(Debug, Clone)]
pub struct MissionInstance {
    pub mission_id: i32,
    pub status: i8,
    pub current_step_id: Option<i32>,
    pub active_objectives: Vec<MissionObjective>,
    pub completed_objectives: Vec<i32>,
    pub completed_steps: Vec<i32>,
    /// Whether this mission is hidden from the mission log.
    pub is_hidden: bool,
    /// Cumulative count of completions+failures. Repeatable missions gate
    /// re-acceptance on this vs. `mission.numRepeats` (python parity:
    /// `MissionManager.py:134` checks `repeats > mission.numRepeats`).
    /// Bumped by [`Self::complete`] and [`Self::fail`] to match python's
    /// `MissionManager.py:252,281`.
    pub repeats: i32,
}

impl MissionInstance {
    /// Create a new active mission.
    pub fn new(mission_id: i32, step_id: i32, objectives: Vec<MissionObjective>) -> Self {
        Self {
            mission_id,
            status: MISSION_ACTIVE,
            current_step_id: Some(step_id),
            active_objectives: objectives,
            completed_objectives: Vec::new(),
            completed_steps: Vec::new(),
            is_hidden: false,
            repeats: 0,
        }
    }

    /// Mark this mission as completed.
    pub fn complete(&mut self) {
        self.status = MISSION_COMPLETED;
        if let Some(step_id) = self.current_step_id.take() {
            self.completed_steps.push(step_id);
        }
        for obj in &mut self.active_objectives {
            obj.status = STATUS_COMPLETED;
        }
        // Python parity: `MissionManager.py:252` increments on completion.
        // The repeat counter is what gates re-acceptance for repeatable
        // missions; missing this bump strands the player at the cap.
        self.repeats += 1;
    }

    /// Mark this mission as failed.
    pub fn fail(&mut self) {
        self.status = MISSION_FAILED;
        self.current_step_id = None;
        // Python parity: `MissionManager.py:281` — fail also counts as a
        // repeat tick. Important for missions where failure is part of the
        // expected lifecycle (failed-then-retry chains).
        self.repeats += 1;
    }

    /// Complete a specific objective by ID.
    pub fn complete_objective(&mut self, objective_id: i32) -> bool {
        if let Some(obj) = self
            .active_objectives
            .iter_mut()
            .find(|o| o.objective_id == objective_id)
        {
            obj.status = STATUS_COMPLETED;
            self.completed_objectives.push(objective_id);
            true
        } else {
            false
        }
    }
}

// ── MissionManager ───────────────────────────────────────────────────────────

/// Per-player mission manager.
///
/// Tracks all active missions. Mirrors `python/cell/MissionManager.py`.
#[derive(Debug)]
pub struct MissionManager {
    /// Active missions keyed by mission_id.
    missions: HashMap<i32, MissionInstance>,
}

impl MissionManager {
    pub fn new() -> Self {
        Self {
            missions: HashMap::new(),
        }
    }

    /// Add a new active mission.
    pub fn add_mission(&mut self, mission: MissionInstance) {
        self.missions.insert(mission.mission_id, mission);
    }

    /// Remove a mission (abandon).
    pub fn remove_mission(&mut self, mission_id: i32) -> Option<MissionInstance> {
        self.missions.remove(&mission_id)
    }

    /// Get a mission by ID.
    pub fn get_mission(&self, mission_id: i32) -> Option<&MissionInstance> {
        self.missions.get(&mission_id)
    }

    /// Get a mutable mission by ID.
    pub fn get_mission_mut(&mut self, mission_id: i32) -> Option<&mut MissionInstance> {
        self.missions.get_mut(&mission_id)
    }

    /// Get all active (non-hidden) missions for resend.
    pub fn active_missions(&self) -> Vec<&MissionInstance> {
        self.missions
            .values()
            .filter(|m| m.status == MISSION_ACTIVE && !m.is_hidden)
            .collect()
    }

    /// Get all tracked missions regardless of status or hidden flag.
    pub fn all_missions(&self) -> impl Iterator<Item = &MissionInstance> {
        self.missions.values()
    }

    /// Number of tracked missions.
    pub fn count(&self) -> usize {
        self.missions.len()
    }

    /// Serialize the resend messages for all active missions.
    ///
    /// Returns a list of (method_index, args) pairs to send to the client.
    /// Called during mapLoaded to restore mission state on the client.
    ///
    /// Reference: `python/cell/MissionManager.py:559-574`
    pub fn serialize_resend(&self) -> Vec<(u16, Vec<u8>)> {
        let mut messages = Vec::new();

        for mission in self.active_missions() {
            // onMissionUpdate(missionId, status=0, giverName=0)
            let mut args = Vec::with_capacity(9);
            args.extend_from_slice(&mission.mission_id.to_le_bytes());
            args.push(STATUS_ACTIVE as u8);
            args.extend_from_slice(&0i32.to_le_bytes()); // missionGiverName
            messages.push((80u16, args)); // flat index 80

            // onStepUpdate(stepId, status=0) for current step
            if let Some(step_id) = mission.current_step_id {
                let mut args = Vec::with_capacity(5);
                args.extend_from_slice(&step_id.to_le_bytes());
                args.push(STATUS_ACTIVE as u8);
                messages.push((81u16, args)); // flat index 81
            }

            // onObjectiveUpdate per active objective
            for obj in &mission.active_objectives {
                let mut args = Vec::with_capacity(7);
                args.extend_from_slice(&obj.objective_id.to_le_bytes());
                args.push(obj.status as u8);
                args.push(if obj.hidden { 1 } else { 0 });
                args.push(if obj.optional { 1 } else { 0 });
                messages.push((82u16, args)); // flat index 82
            }
        }

        messages
    }
}

impl Default for MissionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mission() -> MissionInstance {
        MissionInstance::new(
            100, // mission_id
            200, // step_id
            vec![
                MissionObjective {
                    objective_id: 300,
                    status: STATUS_ACTIVE,
                    hidden: false,
                    optional: false,
                },
                MissionObjective {
                    objective_id: 301,
                    status: STATUS_ACTIVE,
                    hidden: true,
                    optional: true,
                },
            ],
        )
    }

    #[test]
    fn new_mission_is_active() {
        let m = test_mission();
        assert_eq!(m.status, MISSION_ACTIVE);
        assert_eq!(m.current_step_id, Some(200));
        assert_eq!(m.active_objectives.len(), 2);
    }

    #[test]
    fn complete_mission() {
        let mut m = test_mission();
        m.complete();
        assert_eq!(m.status, MISSION_COMPLETED);
        assert_eq!(m.current_step_id, None);
        assert!(m.completed_steps.contains(&200));
        assert!(m
            .active_objectives
            .iter()
            .all(|o| o.status == STATUS_COMPLETED));
        // Python parity: complete() increments repeats. This is the bug
        // class fixed by #118 — without the bump, repeatable missions
        // appear to reset on relog instead of advancing the counter.
        assert_eq!(m.repeats, 1);
    }

    #[test]
    fn fail_mission() {
        let mut m = test_mission();
        m.fail();
        assert_eq!(m.status, MISSION_FAILED);
        assert_eq!(m.current_step_id, None);
        // Python parity: fail() also increments repeats (MissionManager.py:281).
        assert_eq!(m.repeats, 1);
    }

    #[test]
    fn repeats_increments_across_multiple_completions() {
        // Simulates a repeatable mission cycled through accept→complete twice.
        // After the second pass, repeats should be 2 — what gates re-acceptance
        // against the mission's `numRepeats` cap.
        let mut m = test_mission();
        m.complete();
        assert_eq!(m.repeats, 1);

        // Second pass: re-init objectives the same way the manager would on
        // re-acceptance, but reuse the same instance to test the counter
        // accumulates rather than resets.
        m.status = MISSION_ACTIVE;
        m.current_step_id = Some(200);
        m.complete();
        assert_eq!(m.repeats, 2);
    }

    #[test]
    fn new_mission_starts_at_zero_repeats() {
        // Sanity: a fresh mission instance has never been completed.
        let m = test_mission();
        assert_eq!(m.repeats, 0);
    }

    #[test]
    fn complete_objective() {
        let mut m = test_mission();
        assert!(m.complete_objective(300));
        assert!(m.completed_objectives.contains(&300));
        assert_eq!(m.active_objectives[0].status, STATUS_COMPLETED);
    }

    #[test]
    fn complete_unknown_objective() {
        let mut m = test_mission();
        assert!(!m.complete_objective(999));
    }

    #[test]
    fn mission_manager_add_and_get() {
        let mut mgr = MissionManager::new();
        mgr.add_mission(test_mission());
        assert_eq!(mgr.count(), 1);
        assert!(mgr.get_mission(100).is_some());
        assert!(mgr.get_mission(999).is_none());
    }

    #[test]
    fn mission_manager_remove() {
        let mut mgr = MissionManager::new();
        mgr.add_mission(test_mission());
        let removed = mgr.remove_mission(100);
        assert!(removed.is_some());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn active_missions_filters_hidden() {
        let mut mgr = MissionManager::new();
        let mut m1 = test_mission();
        m1.mission_id = 1;
        mgr.add_mission(m1);

        let mut m2 = test_mission();
        m2.mission_id = 2;
        m2.is_hidden = true;
        mgr.add_mission(m2);

        let active = mgr.active_missions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].mission_id, 1);
    }

    #[test]
    fn serialize_resend_format() {
        let mut mgr = MissionManager::new();
        mgr.add_mission(test_mission());

        let messages = mgr.serialize_resend();
        // Should have: 1 onMissionUpdate + 1 onStepUpdate + 2 onObjectiveUpdate = 4
        assert_eq!(messages.len(), 4);

        // onMissionUpdate
        assert_eq!(messages[0].0, 80);
        assert_eq!(messages[0].1.len(), 9);
        let mission_id = i32::from_le_bytes([
            messages[0].1[0],
            messages[0].1[1],
            messages[0].1[2],
            messages[0].1[3],
        ]);
        assert_eq!(mission_id, 100);
        assert_eq!(messages[0].1[4], 0); // STATUS_ACTIVE

        // onStepUpdate
        assert_eq!(messages[1].0, 81);
        assert_eq!(messages[1].1.len(), 5);
        let step_id = i32::from_le_bytes([
            messages[1].1[0],
            messages[1].1[1],
            messages[1].1[2],
            messages[1].1[3],
        ]);
        assert_eq!(step_id, 200);

        // onObjectiveUpdate (first)
        assert_eq!(messages[2].0, 82);
        assert_eq!(messages[2].1.len(), 7);
        let obj_id = i32::from_le_bytes([
            messages[2].1[0],
            messages[2].1[1],
            messages[2].1[2],
            messages[2].1[3],
        ]);
        assert_eq!(obj_id, 300);
        assert_eq!(messages[2].1[5], 0); // not hidden
        assert_eq!(messages[2].1[6], 0); // not optional

        // onObjectiveUpdate (second, hidden + optional)
        assert_eq!(messages[3].0, 82);
        assert_eq!(messages[3].1[5], 1); // hidden
        assert_eq!(messages[3].1[6], 1); // optional
    }

    #[test]
    fn serialize_resend_empty_when_no_missions() {
        let mgr = MissionManager::new();
        assert!(mgr.serialize_resend().is_empty());
    }

    #[test]
    fn serialize_resend_skips_completed() {
        let mut mgr = MissionManager::new();
        let mut m = test_mission();
        m.complete();
        mgr.add_mission(m);

        // Completed missions are not resent
        assert!(mgr.serialize_resend().is_empty());
    }

    // ── Mission restoration from saved data (re-login) ───────────────────

    #[test]
    fn restore_active_mission_from_saved() {
        let mut mgr = MissionManager::new();

        // Simulate what CellService does on InitPlayerState
        let objectives = vec![
            MissionObjective {
                objective_id: 800,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            },
            MissionObjective {
                objective_id: 801,
                status: STATUS_COMPLETED,
                hidden: false,
                optional: false,
            },
        ];
        let mut mission = MissionInstance::new(622, 700, objectives);
        mission.status = MISSION_ACTIVE;
        mission.completed_objectives = vec![801];

        mgr.add_mission(mission);

        assert_eq!(mgr.count(), 1);
        let m = mgr.get_mission(622).unwrap();
        assert_eq!(m.status, MISSION_ACTIVE);
        assert_eq!(m.current_step_id, Some(700));
        assert_eq!(m.active_objectives.len(), 2);
        assert_eq!(m.active_objectives[0].status, STATUS_ACTIVE);
        assert_eq!(m.active_objectives[1].status, STATUS_COMPLETED);
    }

    #[test]
    fn restore_completed_mission_prevents_reacceptance() {
        let mut mgr = MissionManager::new();

        let mut mission = MissionInstance::new(622, 700, vec![]);
        mission.status = MISSION_COMPLETED;
        mission.completed_steps = vec![700];
        mgr.add_mission(mission);

        // Completed mission is in the manager
        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.get_mission(622).unwrap().status, MISSION_COMPLETED);

        // But it should NOT appear in active_missions (for resend)
        assert!(mgr.active_missions().is_empty());

        // serialize_resend should skip it
        assert!(mgr.serialize_resend().is_empty());
    }

    #[test]
    fn restore_multiple_missions_preserves_all() {
        let mut mgr = MissionManager::new();

        // Active mission 622
        let m1 = MissionInstance::new(
            622,
            700,
            vec![MissionObjective {
                objective_id: 800,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        mgr.add_mission(m1);

        // Completed mission 638
        let mut m2 = MissionInstance::new(638, 710, vec![]);
        m2.status = MISSION_COMPLETED;
        mgr.add_mission(m2);

        // Active mission 681
        let m3 = MissionInstance::new(
            681,
            720,
            vec![MissionObjective {
                objective_id: 900,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        mgr.add_mission(m3);

        assert_eq!(mgr.count(), 3);
        assert_eq!(mgr.active_missions().len(), 2); // 622 and 681

        // Resend should only include the 2 active missions
        let resend = mgr.serialize_resend();
        // Each active mission = 1 mission update + 1 step update + 1 objective = 3 messages
        // 622: 3 messages, 681: 3 messages = 6 total
        assert_eq!(resend.len(), 6);
    }

    #[test]
    fn all_missions_includes_every_status() {
        let mut mgr = MissionManager::new();

        let m1 = MissionInstance::new(1, 10, vec![]);
        let mut m2 = MissionInstance::new(2, 20, vec![]);
        m2.complete();
        let mut m3 = MissionInstance::new(3, 30, vec![]);
        m3.fail();

        mgr.add_mission(m1);
        mgr.add_mission(m2);
        mgr.add_mission(m3);

        let all: Vec<i32> = mgr.all_missions().map(|m| m.mission_id).collect();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&1));
        assert!(all.contains(&2));
        assert!(all.contains(&3));
    }
}
