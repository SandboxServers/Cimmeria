import Atrea
import Atrea.enums
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class Item(Resource):
	"""
	@ivar id: Unique identifier of item template
	@ivar appliedScience: Applied science branch item belongs to
	@ivar description: Ingame description of item
	@ivar iconLocation: Item icon in inventory; "set:<icon set name> image:<image name in set>"
	@ivar elementary: Unknown
	@ivar kicker: Unknown
	@ivar researchable: Can its blueprint be obtained using research?
	@ivar reverseEngineerable: Can its blueprint be obtained using reverse engineering?
	@ivar name: Name of item type
	@ivar quality: See EItemQuality enumeration
	@ivar techCompetency: Unknown
	@ivar tier: Item tier (unknown)
	@ivar containers: Containers that can hold the item (EInventoryContainerId enum values)
	@ivar deletable: Can the item be deleted?
	@ivar sellable: Can the item be sold to vendors?
	@ivar maxStackSize: Maximum number of items in an inventory stack
	@ivar monikerIds: List of moniker CRCs
	@ivar minRangedRange: Min weapon range for ranged attacks
	@ivar maxRangedRange: Max weapon range for ranged attacks
	@ivar minMeleeRange: Min weapon range for melee attacks
	@ivar maxMeleeRange: Max weapon range for melee attacks
	@ivar disciplines: List of disciplines required to equip item (?)
	@ivar unique: Is the item unique? (Cannot be traded)
	@ivar visualComponent: Visual component attached to player when item is equipped
	@ivar className: Name of Python class instantiated when creating an item instance (None: no special class)
	@ivar eventSets: Events triggered when performing certain actions
	@ivar useAbility: Ability triggered when using the item
	"""
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['item_id']
		if row['applied_science_id']:
			self.appliedScience = defMgr.require('applied_science', row['applied_science_id'])
		else:
			self.appliedScience = None
		self.description = row['description']
		self.iconLocation = row['icon_location']
		if self.iconLocation is None:
			self.iconLocation = 'set:CoreWidgets image:IconMissing'
		self.flags = row['flags']
		self.elementary = (self.flags & Atrea.enums.ITEM_FLAG_ElementaryComponent != 0)
		self.kicker = (self.flags & Atrea.enums.ITEM_FLAG_Kicker != 0)
		self.researchable = (self.flags & Atrea.enums.ITEM_FLAG_Craft_Research != 0)
		self.reverseEngineerable = (self.flags & Atrea.enums.ITEM_FLAG_Craft_RevEng != 0)
		self.name = row['name']
		self.quality = Atrea.enums.__dict__[row['quality_id']]
		self.techCompetency = row['tech_comp']
		self.tier = row['tier']
		self.containers = \
			[defMgr.require('container', int(e)) for e in row['container_sets'][1:-1].split(',') if e]
		self.deletable = (self.flags & Atrea.enums.ITEM_FLAG_CanBeDeleted != 0)
		self.sellable = (self.flags & Atrea.enums.ITEM_FLAG_CanBeSold != 0)
		self.maxStackSize = row['max_stack_size']
		self.monikerIds = [int(e) for e in row['moniker_ids'][1:-1].split(',') if e]
		self.maxRangedRange = row['max_ranged_range']
		self.minRangedRange = row['min_ranged_range']
		self.maxMeleeRange = row['max_melee_range']
		self.minMeleeRange = row['min_melee_range']
		self.disciplines = \
			[defMgr.require('discipline', int(e)) for e in row['discipline_ids'][1:-1].split(',') if e]
		self.unique = (self.flags & Atrea.enums.ITEM_FLAG_Unique != 0)
		self.visualComponent = row['visual_component']
		self.defaultAmmoTypeId = Atrea.enums.__dict__[row['default_ammo_type']] if row['default_ammo_type'] else None
		self.ammoTypeIds = [Atrea.enums.__dict__[t] for t in row['ammo_types'][1:-1].split(',') if t]
		self.clipSize = row['clip_size']
		self.charges = row['charges']
		# The EItemEvent enumeration contains a list of valid events
		self.events = {}


	def isMissionItem(self):
		"""
		Checks if the item is part of a mission (put into mission bag)
		"""
		return Atrea.enums.INV_Mission in self.containers


	def defaultBag(self):
		"""
		Returns the default bag the item is added to when its picked up
		"""
		if Atrea.enums.INV_Main in [cont.id for cont in self.containers]:
			return Atrea.enums.INV_Main
		elif Atrea.enums.INV_Mission in [cont.id for cont in self.containers]:
			return Atrea.enums.INV_Mission
		elif Atrea.enums.INV_Crafting in [cont.id for cont in self.containers]:
			return Atrea.enums.INV_Crafting
		else:
			return None


	def toXml(self):
		""" Creates a cooked XML resource from the item definition """
		appliedScienceId = 0
		if self.appliedScience:
			appliedScienceId = self.appliedScience.id
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_ITEM AppliedScienceID="' + str(appliedScienceId) + \
		      '" Description=' + quoteattr(self.description) + ' ID="' + str(self.id) + \
		      '" IconLocation="' + self.iconLocation + '" IsElementaryComponent="' + str(self.elementary).lower() + \
		      '" IsKicker="' + str(self.kicker).lower() + '" IsResearchable="' + str(self.researchable).lower() + \
		      '" IsReverseEngineerable="' + str(self.reverseEngineerable).lower() + \
		      '" Name=' + quoteattr(self.name) + ' QualityID="' + str(self.quality) + \
		      '" TechComp="' + str(self.techCompetency) + '" Tier="' + str(self.tier) + '" ItemFlags="' + str(self.flags) + '">'
		xml += '<InventorySet IsDeletable="' + str(self.deletable).lower() + '" IsSellable="' + str(self.sellable).lower() + \
		       '" MaxStackSize="' + str(self.maxStackSize) + '" />'
		xml += '<RequirementsSet IsUnique="' + str(self.unique).lower() + '" />'
		for eventId, ability in self.events.items():
			xml += '<ItemEventSet AbilityID="' + str(ability.id) + '" EventID="' + str(eventId) + '" />'
		if self.maxMeleeRange:
			xml += '<MeleeRanges MaxRange="' + str(self.maxMeleeRange) + '" MinRange="' + str(self.minMeleeRange) + '" />'
		if self.maxRangedRange:
			xml += '<RangeRanges MaxRange="' + str(self.maxRangedRange) + '" MinRange="' + str(self.minRangedRange) + '" />'
		for container in self.containers:
			xml += '<ContainerSet>' + str(container.id) + '</ContainerSet>'
		for monikerId in self.monikerIds:
			xml += '<Moniker MonikerID="' + str(monikerId) + '" />'
		for discipline in self.disciplines:
			xml += '<DisciplineList DisciplineID="' + str(discipline.id) + '" />'
		xml += '</COOKED_ITEM>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		items = Atrea.dbQuery('select * from resources.items')
		for i in items:
			defs[i['item_id']] = Item(i, defMgr)
		eventSets = Atrea.dbQuery('select * from resources.items_event_sets')
		for evt in eventSets:
			ability = defMgr.require('ability', evt['ability_id'])
			if ability is not None:
				defs[evt['item_id']].events[evt['event_id']] = ability
