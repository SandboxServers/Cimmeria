###################################################################################
#                         Mission management commands
###################################################################################
from common import Constants


def acceptMission(player, target, missionId):
	"""
	Assigns the mission to the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to assign
	"""
	if target.missions.accept(missionId):
		player.feedback("Mission %d accepted for player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to accept mission %d" % missionId)


def abandonMission(player, target, missionId):
	"""
	Abandons the mission on the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to abandon
	"""
	if target.missions.accept(missionId):
		player.feedback("Mission %d abandoned for player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to abandon mission %d" % missionId)


def advanceMission(player, target, missionId, stepId):
	"""
	Advances the mission to the specified step for the selected player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to advance
	@type  stepId: int
	@param stepId: Step to advance to
	"""
	if target.missions.advance(missionId, stepId):
		player.feedback("Mission %d advanced for player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to advance mission %d" % missionId)


def clearMission(player, target, missionId):
	"""
	Removes all mission data from the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to clear
	"""
	if target.missions.clear(missionId):
		player.feedback("Mission %d cleared on player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to clear mission %d" % missionId)


def clearActiveMissions(player, target):
	"""
	Removes all active missions from the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	"""
	for missionId in list(target.missions.missions.keys())[:]:
		target.missions.clear(missionId)
	player.feedback("Cleared all missions for player %s" % target.getName())


def clearMissionHistory(player, target):
	"""
	Removes all historical mission data from the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	"""
	for missionId in list(target.missions.missions.keys())[:]:
		if target.missions.missions[missionId]['status'] != Constants.MISSION_Active:
			target.missions.clear(missionId)
	player.feedback("Cleared mission history for player %s" % target.getName())


def completeMission(player, target, missionId):
	"""
	Completes the mission for the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to complete
	"""
	if target.missions.complete(missionId):
		player.feedback("Mission %d completed for player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to complete mission %d" % missionId)


def displayMissionRewards(player, target, missionId):
	"""
	Display the mission rewards dialog for the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to display dialog for
	"""
	if target.missions.displayRewards(missionId):
		player.feedback("Mission %d rewards displayed to player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to display rewards for mission %d" % missionId)


def displayMissionList(player, target):
	"""
	Display the list of active missions on the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	"""
	player.feedback("Missions of player %s:" % target.getName())
	for mission in target.missions.missions.values():
		if mission.status == Constants.MISSION_Active:
			player.feedback(" - %d: %s" % (mission.missionId, mission.mission.historyText))


def displayFullMissionList(player, target):
	"""
	Display the list of *ALL* missions on the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	"""
	player.feedback("All missions of player %s:" % target.getName())
	for mission in target.missions.missions.values():
		player.feedback(" - %d: %s (%d)" % (mission.missionId, mission.mission.historyText,
									        mission.status))


def displayMissionDetails(player, target, missionId):
	"""
	Display mission progression details for the specified mission
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to display details for
	"""
	if missionId not in target.missions.missions:
		player.onError("Player %s has no mission %d" % (target.getName(), missionId))
		return

	mission = target.missions.missions[missionId]
	player.feedback("Player %s, mission %d:" % (target.getName(), mission.mission.missionId))
	player.feedback(" - Name: %s" % mission.mission.historyText)
	player.feedback(" - Status: %d" % mission.status)
	player.feedback(" - Step: %s" % str(mission.currentStepId))
	player.feedback(" - Script: %s (%s)" % (str(mission.mission.scriptName), 'OK' if mission.script else 'Not loaded'))
	player.feedback(" - Completed Steps: %s" % ', '.join([str(step.id) for step in mission.completedSteps]))
	player.feedback(" - Objectives: %s" % ', '.join([str(obj.id) for obj in mission.activeObjectives]))
	player.feedback(" - Completed Objectives: %s" % ', '.join([str(obj.id) for obj in mission.completedObjectives]))
	player.feedback(" - Failed Objectives: %s" % ', '.join([str(obj.id) for obj in mission.failedObjectives]))


def failMission(player, target, missionId):
	"""
	Fails the mission on the targeted player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to fail
	"""
	if target.missions.accept(missionId):
		player.feedback("Mission %d failed by player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to fail mission %d" % missionId)


def reloadMission(player, target, missionId):
	"""
	Reloads the mission script for the selected player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to fail
	"""
	if target.missions.reload(missionId):
		player.feedback("Mission %d reloaded by player %s" % (missionId, target.getName()))
	else:
		player.onError("Failed to reload mission %d" % missionId)


def resetMission(player, target, missionId, stepId):
	"""
	Forcibly advances the mission to the specified step for the selected player
	@param player: SGWPlayer
	@param target: Targeted player
	@type  missionId: int
	@param missionId: Mission to fail
	@type  stepId: int
	@param stepId: Step ID to advance to
	"""
	if target.missions.advance(missionId, stepId, True):
		player.feedback("Mission %s reset to step %d on player %s" % (missionId, stepId, target.getName()))
	else:
		player.onError("Failed to advance mission %s" % missionId)