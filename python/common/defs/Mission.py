import Atrea
from xml.sax.saxutils import escape, quoteattr
from common.defs.Resource import Resource


class MissionStep(object):
	__slots__ = ('mission', 'id', 'awardXp', 'difficulty', 'enabled', 'displayLogText', 'index', 'objectives')

	def __init__(self, mission, row):
		self.mission = mission
		self.id = row['step_id']
		self.awardXp = row['award_xp'] == 1
		self.difficulty = row['difficulty']
		self.enabled = row['step_enabled'] == 1
		self.displayLogText = row['step_display_log_text']
		self.index = row['index']
		self.objectives = {}


class MissionObjective(object):
	__slots__ = ('step', 'id', 'awardXp', 'difficulty', 'enabled', 'hidden', 'optional', 'displayLogText', 'tasks')

	def __init__(self, step, row):
		self.step = step
		self.id = row['objective_id']
		self.awardXp = row['award_xp'] == 1
		self.difficulty = row['difficulty']
		self.enabled = row['is_enabled'] == 1
		self.hidden = row['is_hidden'] == 1
		self.optional = row['is_optional'] == 1
		self.displayLogText = row['display_log_text']
		self.tasks = {}


class MissionTask(object):
	__slots__ = ('objective', 'id', 'awardXp', 'difficulty', 'enabled', 'type')

	def __init__(self, objective, row):
		self.objective = objective
		self.id = row['task_id']
		self.awardXp = row['award_xp'] == 1
		self.difficulty = row['difficulty']
		self.enabled = row['is_enabled'] == 1
		self.type = row['task_type']


class MissionRewardGroup(object):
	__slots__ = ('mission', 'id', 'choices', 'items')

	def __init__(self, mission, row):
		self.mission = mission
		self.id = row['group_id']
		self.choices = row['choices']
		self.items = []


class Mission(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.missionId = row['mission_id']
		self.historyText = row['history_text']
		self.awardXp = (row['award_xp'] == 1)
		self.canAbandon = (row['can_abandon'] == 1)
		self.canFail = (row['can_fail'] == 1)
		self.canRepeatOnFail = (row['can_repeat_on_fail'] == 1)
		self.difficulty = row['difficulty']
		self.isAStory = (row['is_a_story'] == 1)
		self.isEnabled = (row['is_enabled'] == 1)
		self.isHidden = (row['is_hidden'] == 1)
		self.isOverrideMission = (row['is_override_mission'] == 1)
		self.isShareable = (row['is_shareable'] == 1)
		self.level = row['level']
		self.missionDefn = row['mission_defn']
		self.missionLabel = row['mission_label']
		self.numRepeats = row['num_repeats']
		self.showFactionChangeIcon = (row['show_faction_change_icon'] == 1)
		self.showInstanceIcon = (row['show_instance_icon'] == 1)
		self.showPvpIcon = (row['show_pvp_icon'] == 1)
		self.scriptName = row['script_name']
		self.scriptSpaces = [e for e in row['script_spaces'][1:-1].split(',') if e] if row['script_spaces'] else None
		self.rewardNaq = row['reward_naq']
		self.rewardXp = row['reward_xp']
		self.objectives = {}
		self.steps = {}
		self.stepsByIndex = {}
		self.tasks = {}
		self.rewards = {}


	def toXml(self):
		""" Creates a cooked XML resource from the dialog definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?>'
		xml += '<COOKED_MISSION AwardXP="' + str(self.awardXp).lower() + '" CanAbandon="' + str(self.canAbandon).lower() + \
		'" CanFail="' + str(self.canFail).lower() + '" CanRepeatOnFail="' + str(self.canRepeatOnFail).lower() + \
		'" Difficulty="' + str(self.difficulty) + '" IsAStory="' + str(self.isAStory).lower() + '" IsEnabled="' + str(self.isEnabled).lower() + \
		'" IsHidden="' + str(self.isHidden).lower() + '" IsOverrideMission="' + str(self.isOverrideMission).lower() + \
		'" IsShareable="' + str(self.isShareable).lower() + '" Level="' + str(self.level) + \
		'" MissionDefn=' + quoteattr(self.missionDefn) + ' MissionID="' + str(self.missionId) + \
		'" MissionLabel=' + quoteattr(self.missionLabel) + ' NumRepeats="' + str(self.numRepeats) + '" ShowFactionChangeIcon="' + str(self.showFactionChangeIcon).lower() + \
		'" ShowInstanceIcon="' + str(self.showInstanceIcon).lower() + '" ShowPVPIcon="' + str(self.showPvpIcon).lower() + '">'
		xml += '<HistoryText>'+escape(self.historyText)+'</HistoryText>'
		for stepIndex in range(len(self.stepsByIndex)):
			step = self.stepsByIndex[stepIndex]
			xml += '<Steps AwardXP="' + str(step.awardXp).lower() + '" Difficulty="'+str(step.difficulty)+ \
			'" StepEnabled="'+str(step.enabled).lower()+'" StepID="'+str(step.id)+'">'
			for obj in step.objectives.values():
				xml += '<Objectives AwardXP="'+str(obj.awardXp).lower()+'" Difficulty="'+str(obj.difficulty)+ \
				'" IsEnabled="'+str(obj.enabled).lower()+'" IsHidden="'+str(obj.hidden).lower()+ \
				'" IsOptional="'+str(obj.optional).lower()+'" ObjectiveID="'+str(obj.id)+'">'
				xml += '<DisplayLogText>'+escape(obj.displayLogText)+'</DisplayLogText>'
				for task in obj.tasks.values():
					xml += '<Tasks AwardXP="'+str(task.awardXp).lower()+'" Difficulty="'+str(task.difficulty)+ \
					'" IsEnabled="'+str(task.enabled).lower()+'" TaskID="'+str(task.id)+'" TaskType="'+str(task.type)+'" />'
				xml += '</Objectives>'
			xml += '<StepDisplayLogText>'+escape(step.displayLogText)+'</StepDisplayLogText>'
			xml += '</Steps>'
		xml += '</COOKED_MISSION>'
		
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		missions = Atrea.dbQuery('select * from resources.missions')
		for mission in missions:
			defs[mission['mission_id']] = Mission(mission, defMgr)

		stepMissions = {}
		steps = Atrea.dbQuery('select * from resources.mission_steps')
		for s in steps:
			mission = defs[s['mission_id']]
			step = MissionStep(mission, s)
			stepMissions[step.id] = mission
			mission.steps[step.id] = step
			mission.stepsByIndex[step.index] = step

		objectiveMissions = {}
		objectives = Atrea.dbQuery('select * from resources.mission_objectives')
		for obj in objectives:
			stepId = obj['step_id']
			objective = MissionObjective(stepMissions[stepId].steps[stepId], obj)
			objectiveMissions[objective.id] = stepMissions[stepId]
			stepMissions[stepId].objectives[objective.id] = objective
			stepMissions[stepId].steps[stepId].objectives[objective.id] = objective

		tasks = Atrea.dbQuery('select * from resources.mission_tasks')
		for t in tasks:
			objId = t['objective_id']
			task = MissionTask(objectiveMissions[objId].objectives[objId], t)
			objectiveMissions[objId].tasks[task.id] = task
			objectiveMissions[objId].objectives[objId].tasks[task.id] = task

		missionGroups = {}
		groups = Atrea.dbQuery('select * from resources.mission_reward_groups')
		for g in groups:
			mission = defs[g['mission_id']]
			group = MissionRewardGroup(mission, g)
			missionGroups[group.id] = group
			mission.rewards[group.id] = group

		rewards = Atrea.dbQuery('select * from resources.mission_rewards')
		for r in rewards:
			missionGroups[r['group_id']].items.append(defMgr.require('item', r['item_id']))
