import Atrea
import Atrea.enums
from common import Constants
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class InteractionSetMap(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['dialog_set_map_id']
		self.dialogSetId = row['dialog_set_id']
		self.dialog = defMgr.require('dialog', row['dialog_id']) if row['dialog_id'] else None
		self.topicText = row['topic_text']
		self.flags = row['interaction_flags']
		self.minLevel = row['min_level']
		self.factions = [int(f) for f in row['factions'][1:-1].split(',') if f]
		self.alignments = [int(a) for a in row['alignments'][1:-1].split(',') if a]
		self.missionsCompleted = [defMgr.require('mission', int(m)) for m in row['missions_completed'][1:-1].split(',') if m]
		self.missionsNotAccepted = [defMgr.require('mission', int(m)) for m in row['missions_not_accepted'][1:-1].split(',') if m]
		self.handlers = []

	def toXml(self):
		""" Creates a cooked XML resource from the interaction set definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_INTERACTION_SET_MAP DialogSetMapID="' + str(self.id) + \
		      '" DialogID="' + str(self.dialog.id if self.dialog else 0) + '" TopicText=' + quoteattr(self.topicText) + \
		      ' DialogSetID="' + str(self.dialogSetId) + '" InteractionFlags="' + str(self.flags) + '" />'
		return xml

	def visible(self, player):
		"""
		Checks if the interaction set is visible for the specified entity.
		@param player: Entity to check
		@return: True if visible, False otherwise
		"""
		if player.level < self.minLevel:
			return False

		if len(self.factions) > 0 and player.faction not in self.factions:
			return False

		if len(self.alignments) > 0 and player.alignment not in self.alignments:
			return False

		for mission in self.missionsCompleted:
			if player.missions.getMissionStatus(mission.missionId) != Constants.MISSION_Completed:
				return False

		for mission in self.missionsNotAccepted:
			status = player.missions.getMissionStatus(mission.missionId)
			if status != Constants.MISSION_Not_Active and status != Constants.MISSION_Failed:
				return False

		return True

	def conditional(self):
		"""
		Checks if this interaction is tied to any condition.
		@return: True if conditional, False otherwise
		"""
		return self.minLevel > 0 or len(self.factions) or len(self.alignments) or len(self.missionsCompleted) or \
			len(self.missionsNotAccepted)

	def interact(self, instigator, target, triggerEvents = True):
		"""
		Called when the interaction was triggered.
		@param instigator: Player triggering the event
		@param target: Entity the player is interacting with
		@param triggerEvents: Trigger entity dialog events?
		"""
		if self.dialog is not None:
			# TODO: Get missionFlags from dialog set map? Is this value used at all?
			instigator.displayDialog(target, self.dialog.id, 0, triggerEvents, self.id)

		for handler in self.handlers:
			handler(self, instigator, target)

	@staticmethod
	def loadAll(defs, defMgr):
		texts = Atrea.dbQuery('select * from resources.dialog_set_maps')
		for text in texts:
			setMap = InteractionSetMap(text, defMgr)
			defs[text['dialog_set_map_id']] = setMap
			if not defMgr.clientOnly:
				dialogSet = defMgr.get('interaction_set', setMap.dialogSetId)
				dialogSet.interactions[setMap.id] = setMap
