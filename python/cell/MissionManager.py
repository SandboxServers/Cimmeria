import imp
from importlib import import_module
import weakref
import Atrea
import Atrea.enums
from common import Constants
from common.Logger import warn, trace, error, exc
from common.defs.Def import DefMgr


class MissionInstance(object):
	script = None

	def __init__(self, manager, row = None, missionId = None):
		super().__init__()
		self.manager = weakref.ref(manager)
		if row:
			# Load mission information from the database
			self.load(row['mission_id'], row['status'], row['current_step_id'], row['completed_step_ids'][1:-1].split(','),
					  row['completed_objective_ids'][1:-1].split(','), row['active_objective_ids'][1:-1].split(','),
					  row['failed_objective_ids'][1:-1].split(','), row['repeats'])
		elif missionId:
			# Assigns a new mission instance
			self.missionId = missionId
			self.mission = DefMgr.require('mission', self.missionId)
			self.status = Constants.MISSION_Not_Active
			self.repeats = 0
			self.dirty = True
			self.new = True
			assert(self.accept())


	def load(self, missionId, status, currentStepId, completedSteps, completedObjectives, activeObjectives, failedObjectives, repeats):
		self.dirty = False
		self.new = False
		self.missionId = missionId
		self.mission = DefMgr.require('mission', self.missionId)
		self.status = status
		self.repeats = repeats
		if currentStepId is None:
			self.currentStepId, self.currentStep = None, None
		else:
			self.updateCurrentStep(currentStepId, False)
		stepIds = [int(stepId) for stepId in completedSteps if stepId]
		self.completedSteps = [self.mission.steps[stepId] for stepId in stepIds]
		objectiveIds = [int(objectiveId) for objectiveId in completedObjectives if objectiveId]
		self.completedObjectives = [self.mission.objectives[objectiveId] for objectiveId in objectiveIds]
		objectiveIds = [int(objectiveId) for objectiveId in activeObjectives if objectiveId]
		self.activeObjectives = [self.mission.objectives[objectiveId] for objectiveId in objectiveIds]
		objectiveIds = [int(objectiveId) for objectiveId in failedObjectives if objectiveId]
		self.failedObjectives = [self.mission.objectives[objectiveId] for objectiveId in objectiveIds]


	def loadScripts(self):
		if self.status == Constants.MISSION_Active:
			self.initScript()


	def save(self, playerId):
		"""
		Persists the progress of the mission to the database
		"""
		if self.dirty:
			completedSteps = '{' + ','.join([str(s.id) for s in self.completedSteps]) + '}'
			completedObjectives = '{' + ','.join([str(s.id) for s in self.completedObjectives]) + '}'
			activeObjectives = '{' + ','.join([str(s.id) for s in self.activeObjectives]) + '}'
			failedObjectives = '{' + ','.join([str(s.id) for s in self.failedObjectives]) + '}'
			if self.currentStepId is None:
				stepId = 'null'
			else:
				stepId = str(self.currentStepId)
			if self.new:
				Atrea.dbQuery('''
					insert into sgw_mission
						(player_id, mission_id, status, current_step_id, completed_step_ids,
						 completed_objective_ids, active_objective_ids, failed_objective_ids, repeats)
					values (%d, %d, %d, %s, '%s', '%s', '%s', '%s', %d)
					''' % (
				playerId, self.missionId, self.status, stepId,
				completedSteps, completedObjectives, activeObjectives, failedObjectives, self.repeats))
			else:
				Atrea.dbQuery('''
					update	sgw_mission
					set		status = %d, current_step_id = %s, completed_step_ids = '%s',
							completed_objective_ids = '%s', active_objective_ids = '%s', failed_objective_ids = '%s',
							repeats = %d
					where	player_id = %d and mission_id = %d
					''' % (
				self.status, stepId, completedSteps, completedObjectives,
				activeObjectives, failedObjectives, self.repeats, playerId, self.missionId))
		self.new = False
		self.dirty = False


	def backup(self):
		"""
		Makes a persistent copy of the mission that can be restored later.
		@return: Persistent state of mission
		"""
		return {
			'missionId': self.missionId,
			'status': self.status,
			'currentStep': self.currentStepId,
			'completedSteps': [str(s.id) for s in self.completedSteps],
			'completedObjs': [str(s.id) for s in self.completedObjectives],
			'activeObjs': [str(s.id) for s in self.activeObjectives],
			'failedObjs': [str(s.id) for s in self.failedObjectives],
			'repeats': self.repeats
		}


	def restore(self, backup):
		"""
		Restore a previously backed up copy of this mission.
		@param backup: Backed up mission data
		"""
		self.load(backup['missionId'], backup['status'], backup['currentStep'], backup['completedSteps'],
				  backup['completedObjs'], backup['activeObjs'], backup['failedObjs'], backup['repeats'])


	def canOffer(self):
		"""
		Checks if the mission can be offered to the player.
		@return: True if can be offered, False otherwise
		"""
		if self.status == Constants.MISSION_Active:
			warn(Atrea.enums.LC_Mission, "Cannot offer already active mission %d" % self.missionId)
			return False

		if self.status == Constants.MISSION_Failed and not self.mission.canRepeatOnFail:
			warn(Atrea.enums.LC_Mission, "Cannot offer unrepeatable failed mission %d" % self.missionId)
			return False

		if self.status == Constants.MISSION_Completed and self.repeats > self.mission.numRepeats:
			warn(Atrea.enums.LC_Mission, "Cannot offer unrepeatable mission %d" % self.missionId)
			return False

		return True


	def accept(self):
		"""
		Assigns the mission to the player.
		@return: True if successful, False otherwise
		"""
		if not self.canOffer():
			return False

		self.status = Constants.MISSION_Active
		self.updateCurrentStep(self.mission.stepsByIndex[0].id)
		self.completedSteps = []
		self.completedObjectives = []
		self.activeObjectives = []
		self.failedObjectives = []
		self.addStepObjectives(self.currentStepId)
		self.dirty = True
		self.initScript()
		player = self.manager().player()
		player.fire("mission.accepted::" + str(self.missionId))
		player.fire("mission_step.started::" + str(self.currentStepId))
		for objective in self.activeObjectives:
			player.fire("mission_objective.started::" + str(objective.id))
		trace(Atrea.enums.LC_Mission, "<%s> accepted mission %d" % (player.getName(), self.missionId))
		return True


	def getStepStatus(self, stepId):
		"""
		Returns the current state of the step.
		@param stepId: Step to check
		@return: One of MISSION_xxx status codes.
		"""
		if self.status == Constants.MISSION_Completed:
			# Mission is completed -> all steps are completed
			return Constants.MISSION_Completed
		elif self.status == Constants.MISSION_Failed:
			# Mission failed -> all steps failed
			return Constants.MISSION_Failed

		assert(stepId in self.mission.steps)
		# Get the step index for the current step & the checked step
		# If the checked step index is higher, we'll assume that the previous step is already completed
		stepIndex = self.mission.steps[stepId].index
		curIndex = self.currentStep.index
		if curIndex < stepIndex:
			return Constants.MISSION_Not_Active
		elif curIndex == stepIndex:
			return Constants.MISSION_Active
		else:
			return Constants.MISSION_Completed


	def getObjectiveStatus(self, objectiveId):
		"""
		Returns the current state of the objective.
		@param objectiveId: Objective to check
		@return: One of MISSION_xxx status codes.
		"""
		if self.status == Constants.MISSION_Completed:
			# Mission is completed -> all objectives are completed
			return Constants.MISSION_Completed
		elif self.status == Constants.MISSION_Failed:
			# Mission failed -> all objectives failed
			return Constants.MISSION_Failed

		assert(objectiveId in self.mission.objectives)
		objective = self.mission.objectives[objectiveId]
		if objective in self.activeObjectives:
			return Constants.MISSION_Active
		elif objective in self.failedObjectives:
			return Constants.MISSION_Failed
		elif objective in self.completedObjectives:
			return Constants.MISSION_Completed
		else:
			return Constants.MISSION_Not_Active


	def canDisplayRewards(self):
		"""
		Checks if the rewards list for this mission can be sent to the client.
		@return: True if allowed, False otherwise
		"""
		if self.status != Constants.MISSION_Active:
			warn(Atrea.enums.LC_Mission, "Cannot display rewards for inactive mission %d" % self.missionId)
			return False
		else:
			return True


	def complete(self):
		"""
		Completes the mission.
		@return: True if successful, False otherwise
		"""
		if self.status != Constants.MISSION_Active:
			warn(Atrea.enums.LC_Mission, "Cannot complete inactive mission %d" % self.missionId)
			return False

		player = self.manager().player()
		if self.currentStepId is not None:
			player.fire("mission_step.completed::" + str(self.currentStepId))
		for objective in self.activeObjectives:
			player.fire("mission_objective.completed::" + str(objective.id))
		player.fire("mission.completed::" + str(self.missionId))

		self.status = Constants.MISSION_Completed
		self.completedSteps.append(self.currentStep)
		self.currentStepId = None
		self.currentStep = None
		self.completedObjectives.extend(self.activeObjectives)
		self.activeObjectives = []
		self.repeats += 1
		self.dirty = True
		self.destroyScript()
		trace(Atrea.enums.LC_Mission, "<%s> completed mission %d" % (player.getName(), self.missionId))
		return True


	def fail(self):
		"""
		Fails the mission.
		@return: True if successful, False otherwise
		"""
		if self.status != Constants.MISSION_Active:
			warn(Atrea.enums.LC_Mission, "Cannot fail inactive mission %d" % self.missionId)
			return False

		assert self.mission.canFail or self.mission.canAbandon

		player = self.manager().player()
		player.fire("mission_step.failed::" + str(self.currentStepId))
		for objective in self.activeObjectives:
			player.fire("mission_objective.failed::" + str(objective.id))
		player.fire("mission.failed::" + str(self.missionId))

		self.status = Constants.MISSION_Failed
		self.currentStepId = None
		self.currentStep = None
		self.failedObjectives.extend(self.activeObjectives)
		self.activeObjectives = []
		self.repeats += 1
		self.dirty = True
		self.destroyScript()
		trace(Atrea.enums.LC_Mission, "<%s> failed mission %d" % (player.getName(), self.missionId))
		return True


	def abandon(self):
		"""
		Abandons the mission.
		@return: True if successful, False otherwise
		"""
		if self.mission.isHidden or not self.mission.canAbandon:
			warn(Atrea.enums.LC_Mission, "Cannot abandon hidden/non-abandonable mission %d" % self.missionId)
			return False

		return self.fail()


	def advance(self, stepId, force = False):
		"""
		Advances the mission to the specified step
		@param stepId: Step to advance to
		@param force: Bypass step ordering checks
		@return: True if successful, False otherwise
		"""
		if self.status != Constants.MISSION_Active:
			warn(Atrea.enums.LC_Mission, "Cannot advance mission %d: In status %d" % (self.missionId, self.status))
			return False, None, None

		if stepId not in self.mission.steps:
			warn(Atrea.enums.LC_Mission, "Cannot advance mission %d: Invalid step ID %d" % (self.missionId, stepId))
			return False, None, None

		currentIndex = self.currentStep.index
		nextIndex = self.mission.steps[stepId].index
		if currentIndex >= nextIndex and not force:
			warn(Atrea.enums.LC_Mission, "Cannot advance mission %d from step index %d to %d" % (self.missionId, currentIndex, nextIndex))
			return False, None, None

		player = self.manager().player()
		trace(Atrea.enums.LC_Mission, "<%s> advanced mission %d from step %d to %d" %
									  (player.getName(), self.missionId, self.currentStepId, stepId))
		player.fire("mission_step.completed::" + str(self.currentStepId))

		# Complete all objectives in the previous step that aren't explicitly marked as failed
		completed = self.completeStepObjectives(self.currentStepId)

		# Add all objectives belonging to the next step
		added = self.addStepObjectives(stepId)

		self.completedSteps.append(self.currentStep)
		self.updateCurrentStep(stepId)

		player.fire("mission_step.started::" + str(self.currentStepId))
		return True, completed, added


	def getStepDetails(self, stepId):
		"""
		Returns all details of a mission step
		@param stepId: Step to return
		@return: Step details
		"""
		assert(stepId in self.mission.steps)
		return self.mission.steps[stepId]


	def updateCurrentStep(self, stepId, dirty = True):
		"""
		Changes the active mission step
		@param stepId: New step ID
		@param dirty: Set dirty flag?
		"""
		assert(stepId in self.mission.steps)
		self.currentStepId = stepId
		self.currentStep = self.mission.steps[self.currentStepId]
		self.dirty = self.dirty or dirty


	def addStepObjectives(self, stepId):
		"""
		Adds all objectives to the mission from the specified step
		@param stepId: Step ID
		@return: List of added objective IDs
		"""
		added = []
		activeIds = [objective.id for objective in self.activeObjectives]
		for objective in self.mission.objectives.values():
			objId = objective.id
			if objective.step.id == stepId and objId not in activeIds:
				added.append(objId)
				self.activeObjectives.append(objective)
				self.manager().player().fire("mission_objective.started::" + str(objective.id))
		if len(added):
			self.dirty = True
		return added


	def completeStepObjectives(self, stepId):
		"""
		Complete all objectives of the specified step
		@param stepId: Step ID
		@return: List of completed objective IDs
		"""
		completed = []
		activeIds = [objective.id for objective in self.activeObjectives]
		for objective in self.mission.objectives.values():
			objId = objective.id
			if objective.step.id == stepId and objId in activeIds:
				completed.append(objId)
				self.completedObjectives.append(objective)
				self.activeObjectives.remove(objective)
				self.manager().player().fire("mission_objective.completed::" + str(objective.id))
		if len(completed):
			self.dirty = True
		return completed


	def failStepObjectives(self, stepId):
		"""
		Fail all objectives of the specified step
		@param stepId: Step ID
		@return: List of failed objective IDs
		"""
		failed = []
		activeIds = [objective.id for objective in self.activeObjectives]
		for objective in self.mission.objectives.values():
			objId = objective.id
			if objective.step.id == stepId and objId in activeIds:
				failed.append(objId)
				self.failedObjectives.append(objective)
				self.activeObjectives.remove(objective)
				self.manager().player().fire("mission_objective.failed::" + str(objective.id))
		if len(failed):
			self.dirty = True
		return failed


	def completeObjective(self, objectiveId):
		"""
		Completes the specified mission objective
		@param objectiveId: Objective ID
		@return: True if objective was set as completed, False otherwise
		"""
		objective = next((obj for obj in self.activeObjectives if obj.id == objectiveId), None)
		if objective is not None:
			self.completedObjectives.append(objective)
			self.activeObjectives.remove(objective)
			self.dirty = True
			self.manager().player().fire("mission_objective.completed::" + str(objectiveId))
			return True
		else:
			return False


	def failObjective(self, objectiveId):
		"""
		Fails the specified mission objective
		@param objectiveId: Objective ID
		@return: True if objective was set as failed, False otherwise
		"""
		objective = next((obj for obj in self.activeObjectives if obj.id == objectiveId), None)
		if objective is not None:
			self.failedObjectives.append(objective)
			self.activeObjectives.remove(objective)
			self.dirty = True
			self.manager().player().fire("mission_objective.failed::" + str(objectiveId))
			return True
		else:
			return False


	def reloadScript(self):
		"""
		Reloads the dynamic script class for the mission
		@return: True if successful, False otherwise
		"""
		if self.script is None:
			return False

		self.script.persist()
		vars = self.script.savedVars
		self.script.teardown()

		mod = import_module('cell.missions.' + self.mission.scriptName)
		imp.reload(mod)
		cls = self.mission.scriptName.split('.')[-1]
		self.script = getattr(mod, cls)(self.manager().player(), vars)
		self.script.restore()
		return True


	def initScript(self):
		"""
		Initializes the dynamic script of the mission
		"""
		spaceOk = (self.mission.scriptSpaces is None or self.manager().player().space.worldName in self.mission.scriptSpaces)
		if self.mission.scriptName is not None and self.script is None and spaceOk:
			try:
				mod = import_module('cell.missions.' + self.mission.scriptName)
				cls = self.mission.scriptName.split('.')[-1]
				# TODO: Load/save StoredVars!
				self.script = getattr(mod, cls)(self.manager().player(), [])
			except Exception:
				self.script = None
				error(Atrea.enums.LC_Mission, "Exception while initializing mission script %s" % self.mission.scriptName)
				exc(Atrea.enums.LC_Mission)


	def destroyScript(self):
		"""
		Destroys the dynamic script of the mission
		"""
		if self.script is not None:
			self.script.teardown()
			self.script = None



class MissionManager(object):
	def __init__(self, player):
		super().__init__()
		self.player = weakref.ref(player)
		self.missions = {}
		self.clearedMissions = []


	def load(self):
		"""
		Loads all missions from the database
		"""
		missions = Atrea.dbQuery('select * from sgw_mission where player_id = %d' % self.player().playerId)
		for row in missions:
			instance = MissionInstance(self, row = row)
			self.missions[instance.missionId] = instance


	def loadScripts(self):
		"""
		Loads and executes all missions scripts
		"""
		for mission in self.missions.values():
			mission.loadScripts()


	def save(self):
		"""
		Persists the progress of all missions to the database
		"""
		for mission in self.missions.values():
			mission.save(self.player().playerId)

		for missionId in self.clearedMissions:
			Atrea.dbQuery('delete from sgw_mission where player_id = %d and mission_id = %d' %
						  (self.player().playerId, missionId))
		self.clearedMissions = []


	def backup(self):
		"""
		Makes a persistent copy of all missions that can be restored later.
		@return: Persistent state of missions
		"""
		return [m.backup() for m in self.missions.values()]


	def restore(self, backup):
		"""
		Restore a previously backed up copy of this mission.
		@param backup: Backed up mission data
		"""
		for mission in backup:
			m = MissionInstance(self)
			m.restore(mission)
			self.missions[m.missionId] = m


	def resend(self):
		"""
		Resends the mission and objective list for the player.
		Mainly used when the mission data was cleared on the client (login, ...)
		"""
		for missionId, mission in self.missions.items():
			if mission.status == Constants.MISSION_Active and not mission.mission.isHidden:
				# TODO: Use MissionGiverName ?
				self.player().client.onMissionUpdate(missionId, 0, 0)
				self.player().client.onStepUpdate(mission.currentStepId, 0)
				for objective in mission.activeObjectives:
					hidden = 1 if objective.hidden else 0
					optional = 1 if objective.optional else 0
					self.player().client.onObjectiveUpdate(objective.id, 0, hidden, optional)
				# TODO: Send task statuses
		return True


	def getMissionStatus(self, missionId):
		"""
		Returns the current state of the mission.
		@param missionId: Mission to check
		@return: One of MISSION_xxx status codes.
		"""
		if missionId not in self.missions:
			return Constants.MISSION_Not_Active

		return self.missions[missionId].status


	def getStepStatus(self, missionId, stepId):
		"""
		Returns the current state of the step.
		@param missionId: Mission to check
		@param stepId: Step to check
		@return: One of MISSION_xxx status codes.
		"""
		if missionId not in self.missions:
			return Constants.MISSION_Not_Active

		return self.missions[missionId].getStepStatus(stepId)


	def getObjectiveStatus(self, missionId, objectiveId):
		"""
		Returns the current state of the objective.
		@param missionId: Mission to check
		@param objectiveId: Objective to check
		@return: One of MISSION_xxx status codes.
		"""
		if missionId not in self.missions:
			return Constants.MISSION_Not_Active

		return self.missions[missionId].getObjectiveStatus(objectiveId)


	def accept(self, missionId):
		"""
		Accepts a new mission
		@param missionId: Mission to accept
		@return: True if successful, False otherwise
		"""
		missionDef = DefMgr.get('mission', missionId)
		if missionDef is None:
			warn(Atrea.enums.LC_Mission, "Cannot accept nonexistent mission %d" % missionId)
			return False

		if missionId in self.missions:
			mission = self.missions[missionId]
			if not mission.accept():
				return False
		else:
			mission = MissionInstance(self, missionId = missionId)
			self.missions[missionId] = mission
			if missionId in self.clearedMissions:
				self.clearedMissions.remove(missionId)

		if not mission.mission.isHidden:
			# TODO: Use MissionGiverName ?
			self.player().client.onMissionUpdate(missionId, 0, 0)
			self.player().client.onStepUpdate(mission.currentStepId, 0)
			for objective in mission.activeObjectives:
				hidden = 1 if objective.hidden else 0
				optional = 1 if objective.optional else 0
				self.player().client.onObjectiveUpdate(objective.id, 0, hidden, optional)
			# TODO: Send task statuses
		return True


	def offer(self, missionId, dialogId):
		"""
		Offers the mission to the selected player.
		When the client accepts the offer, the mission is assigned automatically.
		@param missionId: Mission to offer
		@param dialogId: Dialog displayed when offering the mission
		@return: True if successful, False otherwise
		@warning: !!!!! DO NOT USE !!!!! This is not implemented in the client :(
		"""
		missionDef = DefMgr.get('mission', missionId)
		if missionDef is None:
			warn(Atrea.enums.LC_Mission, "Cannot offer nonexistent mission %d" % missionId)
			return False

		mission = self.missions.get(missionId)
		if mission is not None and not mission.canOffer():
			return False

		self.player().offerMission(missionDef, dialogId)
		return True


	def displayRewards(self, missionId):
		"""
		Displays the rewards selection dialog on the client.
		When the client selects the rewards, the mission is completed automatically.
		@param missionId: Mission to display
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot display rewards for mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		if not mission.canDisplayRewards():
			return False

		self.player().displayMissionRewards(mission.mission)
		return True


	def complete(self, missionId):
		"""
		Indicates that the specified mission is completed
		@param missionId: Mission to complete
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot complete mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		if not mission.complete():
			return False

		# TODO: Use MissionGiverName ?
		if not mission.mission.isHidden:
			self.player().client.onMissionUpdate(missionId, 1, 0)
		self.player().onMissionRemoved(missionId)
		return True


	def fail(self, missionId):
		"""
		Indicates that the specified mission failed
		@param missionId: Mission to fail
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot fail mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		if not mission.fail():
			return False

		if not mission.mission.isHidden:
			# TODO: Use MissionGiverName ?
			# TODO: How to fail a mission? Status 2?
			self.player().client.onMissionUpdate(missionId, 1, 0)
		self.player().onMissionRemoved(missionId)
		return True


	def completeObjective(self, missionId, objectiveId):
		"""
		Indicates that the specified mission objective is completed
		@param missionId: Mission to complete
		@param objectiveId: Mission objective to complete
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot complete objective for mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		if not mission.completeObjective(objectiveId):
			return False

		if not mission.mission.isHidden:
			hidden = 1 if mission.mission.objectives[objectiveId].hidden else 0
			optional = 1 if mission.mission.objectives[objectiveId].optional else 0
			self.player().client.onObjectiveUpdate(objectiveId, 1, hidden, optional)
		return True


	def failObjective(self, missionId, objectiveId):
		"""
		Indicates that the specified mission objective failed
		@param missionId: Mission to fail
		@param objectiveId: Mission objective to fail
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot fail objective for mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		if not mission.failObjective(objectiveId):
			return False

		if not mission.mission.isHidden:
			# TODO: How to fail an objective? Status 2?
			hidden = 1 if mission.mission.objectives[objectiveId].hidden else 0
			optional = 1 if mission.mission.objectives[objectiveId].optional else 0
			self.player().client.onObjectiveUpdate(objectiveId, 1, hidden, optional)
		return True


	def abandon(self, missionId):
		"""
		Abandons the specified mission
		@param missionId: Mission to abandon
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot abandon mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		if not mission.abandon():
			return False

		# TODO: Should abandoned missions be failed or removed entirely?
		if not mission.mission.isHidden:
			# TODO: Use MissionGiverName ?
			# TODO: How to fail a mission? Status 2?
			self.player().client.onMissionUpdate(missionId, 1, 0)
		self.player().onMissionRemoved(missionId)
		return True


	def clear(self, missionId):
		"""
		Removes all mission data for the specified mission
		@param missionId: Mission to clear
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot clear mission %d: Not in mission list" % missionId)
			return False

		self.clearedMissions.append(missionId)
		mission = self.missions[missionId]
		if mission.status == Constants.MISSION_Active and not mission.mission.isHidden:
			self.player().client.onMissionUpdate(missionId, 1, 0)
		self.player().onMissionRemoved(missionId)
		del self.missions[missionId]
		return True


	def reload(self, missionId):
		"""
		Reloads the mission script for the specified mission
		@param missionId: Mission to reload
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot reload mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		return mission.reloadScript()


	def advance(self, missionId, stepId, force = False):
		"""
		Advances the mission to the specified step
		@param missionId: Mission to advance
		@param stepId: Step to advance to
		@param force: Bypass step ordering checks
		@return: True if successful, False otherwise
		"""
		if missionId not in self.missions:
			warn(Atrea.enums.LC_Mission, "Cannot advance mission %d: Not in mission list" % missionId)
			return False

		mission = self.missions[missionId]
		prevStepId = mission.currentStepId
		status, completed, added = mission.advance(stepId, force)
		if not status:
			return False

		if not mission.mission.isHidden:
			if prevStepId != stepId:
				self.player().client.onStepUpdate(prevStepId, 1)
				self.player().client.onStepUpdate(stepId, 0)
			for objectiveId in completed:
				hidden = 1 if mission.mission.objectives[objectiveId].hidden else 0
				optional = 1 if mission.mission.objectives[objectiveId].optional else 0
				self.player().client.onObjectiveUpdate(objectiveId, 1, hidden, optional)
			for objectiveId in added:
				hidden = 1 if mission.mission.objectives[objectiveId].hidden else 0
				optional = 1 if mission.mission.objectives[objectiveId].optional else 0
				self.player().client.onObjectiveUpdate(objectiveId, 0, hidden, optional)

		return True

