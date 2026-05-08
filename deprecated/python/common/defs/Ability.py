import Atrea
import Atrea.enums
from operator import attrgetter
from xml.sax.saxutils import quoteattr
from common.Logger import warn
from common.defs.Resource import Resource


class Ability(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['ability_id']
		self.name = row['name']
		self.description = row['description']
		# See enumeration EAbilityTypes
		self.typeId = Atrea.enums.__dict__[row['type_id']]
		self.cooldown = row['cooldown']
		# Bitmask; see enumeration EAbilityFlags
		self.flags = row['flags']
		self.icon = row['icon']
		if self.icon is None:
			self.icon = 'set:CoreWidgets image:IconMissing'
		self.ranged = (row['is_ranged'] == 1)
		self.minRange = row['min_range']
		self.maxRange = row['max_range']
		self.passive = (row['passive_yn'] == 1)
		# Uses ETargetCollectionParams
		# TCM_Single: No params
		# TCM_AERadius: Param1 = AE_RADIUS_* range constant name
		# TCM_AECone: Param1 = AE_CONE_* range constant name (or 'Weapon' if uses weapon cone)
		# TCM_AECone: Param2 = AE_ANGLES_* range constant name
		# TCM_Group: ???
		self.param1 = row['param1']
		self.param2 = row['param2']
		# See enumeration ETargetType
		self.targetTypeId = row['target_type_id']
		# See enumeration ETargetCollectionMethod
		self.targetCollectionMethod = Atrea.enums.__dict__[row['target_collection_method']]
		self.tauntAdjustment = row['taunt_adjustment']
		# See enumeration EThreatLevel
		self.threatLevelId = Atrea.enums.__dict__[row['threat_level_id']]
		self.trainingCost = row['training_cost']
		self.velocity = row['velocity']
		self.warmup = row['warmup']
		self.effectIds = [int(e) for e in row['effect_ids'][1:-1].split(',') if e]
		self.effects = []
		self.monikerIds = [int(e) for e in row['moniker_ids'][1:-1].split(',') if e]
		if row['event_set_id'] is not None:
			self.eventSet = defMgr.require('event_set', row['event_set_id'])
		else:
			self.eventSet = None
		if self.eventSet:
			if self.eventSet.getSequence(Atrea.enums.Effect_Init) or \
				self.eventSet.getSequence(Atrea.enums.Effect_Hit_Normal) or \
				self.eventSet.getSequence(Atrea.enums.Effect_Pulse_Begin):
				warn(Atrea.enums.LC_Ability, 'Event set (%d) of ability "%s" (%d) has effect events!' % (self.eventSet.id, self.name, self.id))
		self.requiredAmmo = row['required_ammo']
		if row['positions'] is not None:
			self.positions = sum([1 << Atrea.enums.__dict__[p] for p in row['positions'][1:-1].split(',') if p], 0)
		else:
			# Bitmask containing FRONT, FLANK, REAR, ABOVE, BELOW
			self.positions = 31
		self.prerequisiteMonikerGroups = []
		self.itemMonikers = [int(moniker) for moniker in row['item_monikers'][1:-1].split(',') if moniker]

		if self.flags & Atrea.enums.UseWeaponRange and len(self.itemMonikers) == 0:
			warn(Atrea.enums.LC_Resources, 'Ability <%s> uses weapon cooldown but requires no weapon!' % self.name)


	def addEffect(self, effect):
		self.effects.append(effect)


	def addMonikerGroup(self, group):
		"""
		Adds a set of monikers to the list of prerequisite monikers for launching the ability.
		The ability can have multiple moniker groups, each group containing a list of moniker IDs.
		When launching the ability, the player must have at least moniker from each group.
		@param group: Group to add
		"""
		self.prerequisiteMonikerGroups.append(
			[int(e) for e in group['moniker_ids'][1:-1].split(',') if e]
		)

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

	def requiresWeapons(self):
		"""
		Checks if a weapon is required for using this ability.
		@return: True if a weapon is required, False otherwise
		"""
		return len(self.itemMonikers) > 0

	def canUseWithMonikers(self, monikerIds):
		"""
		Checks if the specified item (weapon) monikers satisfy ability requirements
		@param monikerIds: Item moniker IDs
		@return: True if ability can be used; False otherwise
		"""
		if not len(self.itemMonikers):
			return True

		for monikerId in self.itemMonikers:
			if monikerId in monikerIds:
				return True

		return False

	def postLoad(self, defMgr):
		# The effects need to be sorted so they are applied in the correct order
		# when launching the ability
		self.effects = sorted(self.effects, key = attrgetter('sequence'))

	def toXml(self):
		""" Creates a cooked XML resource from the ability definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_ABILITY AbilityId="' + str(self.id) + \
		      '" AbilityName=' + quoteattr(self.name) + ' AbilityDesc=' + quoteattr(self.description) + \
		      ' AbilityTypeId="' + str(self.typeId) + '" TargetTypeId="' + str(self.targetTypeId) + \
		      '" WarmupSeconds="' + str(self.warmup) + '" CooldownSeconds="' + str(self.cooldown) + \
		      '" PassiveYN="' + str(self.passive).lower() + '" IsRanged="' + str(self.ranged).lower() +  \
		      '" TrainingCost="' + str(self.trainingCost) + '" Threat_Level_id="' + str(self.threatLevelId) + \
		      '" Taunt_Adjustment="' + str(self.tauntAdjustment) + '" Target_Collection_Method="' + \
		      str(self.targetCollectionMethod) + '" TCM_Param1="' + (self.param1 or '') + \
		      '" TCM_Param2="' + (self.param2 or '') + '" Velocity="' + str(int(self.velocity)) + \
		      '" Flags="' + str(self.flags) + '" IconLocation="' + self.icon + \
		      '" MinRange="' + str(self.minRange) + '" MaxRange="' + str(self.maxRange) + '" xmlns:CookedData1="SGW" ' \
		      'xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" ' \
			  'xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" ' \
			  'xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'
		for effect in self.effectIds:
			xml += '<EffectIds>' + str(effect) + '</EffectIds>'
		for monikerId in self.monikerIds:
			xml += '<Moniker MonikerID="' + str(monikerId) + '" />'
		xml += '</COOKED_ABILITY>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		abilities = Atrea.dbQuery('select * from resources.abilities')
		for ability in abilities:
			defs[ability['ability_id']] = Ability(ability, defMgr)

		groups = Atrea.dbQuery('select * from resources.ability_moniker_groups')
		for group in groups:
			defs[group['ability_id']].addMonikerGroup(group)