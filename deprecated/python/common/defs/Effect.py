import Atrea
import Atrea.enums
from xml.sax.saxutils import quoteattr
from common.Logger import warn
from common.defs.Resource import Resource

class Effect(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.ability = defMgr.require('ability', row['ability_id'])
		if self.ability:
			self.ability.addEffect(self)
		self.id = row['effect_id']
		self.name = row['name']
		self.description = row['effect_desc']
		self.targetCollectionId = row['target_collection_id']
		self.channeled = (row['is_channeled'] == 1)
		self.sequence = row['effect_sequence']
		self.pulseCount = row['pulse_count']
		self.pulseDuration = row['pulse_duration']
		self.delay = row['delay']
		self.targetCollectionMethod = Atrea.enums.__dict__[row['target_collection_method']]
		# Uses ETargetCollectionParams
		# TCM_Single: No params
		# TCM_AERadius: Param1 = AE_RADIUS_* range constant name
		# TCM_AECone: Param1 = AE_CONE_* range constant name (or 'Weapon' if uses weapon cone)
		# TCM_AECone: Param2 = AE_ANGLES_* range constant name
		# TCM_Group: ???
		self.param1 = row['tcm_param1']
		self.param2 = row['tcm_param2']
		self.useAbilityVelocity = (row['use_ability_velocity'] == 1)
		self.flags = row['flags']
		self.icon = row['icon_location']
		if self.icon is None:
			self.icon = 'set:CoreWidgets image:IconMissing'
		if row['event_set_id'] is not None:
			self.eventSet = defMgr.require('event_set', row['event_set_id'])
		else:
			self.eventSet = None
		if self.eventSet:
			if self.eventSet.getSequence(Atrea.enums.Ability_Begin) or \
				self.eventSet.getSequence(Atrea.enums.Ability_End) or \
				self.eventSet.getSequence(Atrea.enums.Ability_Failed):
				warn(Atrea.enums.LC_Ability, 'Event set (%d) of effect "%s" (%d) has ability events!' % (self.eventSet.id, self.name, self.id))
		self.scriptName = row['script_name']
		self.nvps = {}

	def getSequence(self, eventId):
		"""
		Returns the kismet event sequence for a given event ID
		@param eventId: Event ID (see ESequenceEventType enumeration)
		@return: KismetEventSequenceObject (or None if not found)
		"""
		if self.eventSet is not None:
			return self.eventSet.getSequence(eventId)
		else:
			return None

	def toXml(self):
		""" Creates a cooked XML resource from the effect definition """
		abilityId = 0
		if self.ability is not None:
			abilityId = self.ability.id
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_EFFECT AbilityId="' + str(abilityId) + \
		      '" EffectId="' + str(self.id) + '" name=' + quoteattr(self.name) + \
		      '" EffectDesc=' + quoteattr(self.description) + ' targetCollectionId="' + str(self.targetCollectionId) + \
		      '" isChanneled="' + str(self.channeled).lower() + '" EffectSequence="' + str(self.sequence) + \
		      '" PulseCount="' + str(self.pulseCount) + '" PulseDuration="' + str(self.pulseDuration) +  \
		      '" Delay="' + str(self.delay) + '" Target_Collection_Method="' + str(self.targetCollectionMethod) + \
		      '" TCM_Param1="' + (self.param1 or '') + '" TCM_Param2="' + (self.param2 or '') + \
		      '" Use_Ability_Velocity="' + str(self.useAbilityVelocity).lower() + '" Flags="' + str(self.flags) + \
		      '" IconLocation="' + str(self.icon) + '" />'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		effects = Atrea.dbQuery('select * from resources.effects')
		for effect in effects:
			defs[effect['effect_id']] = Effect(effect, defMgr)

		nvps = Atrea.dbQuery('select * from resources.effect_nvps')
		for nvp in nvps:
			defs[nvp['effect_id']].nvps[nvp['name']] = nvp['value']
