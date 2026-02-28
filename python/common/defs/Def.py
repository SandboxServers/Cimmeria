from binascii import unhexlify
import zipfile
from zipfile import ZIP_DEFLATED
import Atrea
import Atrea.enums
from common.Logger import warn, trace, info
from common.defs.Resource import Resource
import common.defs.Ability
import common.defs.AbilitySet
import common.defs.AppliedScience
import common.defs.Archetype
import common.defs.Blueprint
import common.defs.CharacterCreation
import common.defs.Container
import common.defs.Dialog
import common.defs.Discipline
import common.defs.Effect
import common.defs.EntityTemplate
import common.defs.ErrorText
import common.defs.Interaction
import common.defs.InteractionSet
import common.defs.InteractionSetMap
import common.defs.Item
import common.defs.ItemList
import common.defs.LootTable
import common.defs.Sequence
import common.defs.EventSet
import common.defs.Mission
import common.defs.Path
import common.defs.PetCommand
import common.defs.RacialParadigm
import common.defs.Respawner
import common.defs.RingTransporterRegion
import common.defs.SpecialWords
import common.defs.Stargate
import common.defs.Text
import common.defs.TrainerAbilityList
import common.defs.WorldInfo

class DefMgr(object):
	# Map of client category IDs to server names
	ResourceCategories = {
		1: "sequence",
		2: "ability",
		3: "mission",
		4: "item",
		5: "dialog",
		6: "event_set",
		7: "character_creation",
		8: "interaction_set_map",
		9: "effect",
		10: "text",
		11: "error_text",
		12: "world_info",
		13: "stargate",
		14: "container",
		15: "blueprint",
		16: "applied_science",
		17: "discipline",
		18: "racial_paradigm",
		19: "special_words",
		20: "interaction",
		# Pet commands are not implemented in the client
		21: "pet_command",
		22: "behavior_event"
	}

	__classes = {
		'ability': common.defs.Ability.Ability,
		'ability_set': common.defs.AbilitySet.AbilitySet,
		'applied_science': common.defs.AppliedScience.AppliedScience,
		'archetype': common.defs.Archetype.Archetype,
		'blueprint': common.defs.Blueprint.Blueprint,
		'character_creation': common.defs.CharacterCreation.CharacterCreation,
		'container': common.defs.Container.Container,
		'discipline': common.defs.Discipline.Discipline,
		'dialog': common.defs.Dialog.Dialog,
		'effect': common.defs.Effect.Effect,
		'event_set': common.defs.EventSet.EventSet,
		'error_text': common.defs.ErrorText.ErrorText,
		'interaction': common.defs.Interaction.Interaction,
		'interaction_set': common.defs.InteractionSet.InteractionSet,
		'interaction_set_map': common.defs.InteractionSetMap.InteractionSetMap,
		'item': common.defs.Item.Item,
		'item_list': common.defs.ItemList.ItemList,
		'loot_table': common.defs.LootTable.LootTable,
		'mission': common.defs.Mission.Mission,
	    'path': common.defs.Path.Path,
	    'pet_command': common.defs.PetCommand.PetCommand,
		'racial_paradigm': common.defs.RacialParadigm.RacialParadigm,
		'respawner': common.defs.Respawner.Respawner,
		'ring_transporter_region': common.defs.RingTransporterRegion.RingTransporterRegion,
		'sequence': common.defs.Sequence.Sequence,
		'special_words': common.defs.SpecialWords.SpecialWords,
		'stargate': common.defs.Stargate.Stargate,
	    'template': common.defs.EntityTemplate.EntityTemplate,
		'text': common.defs.Text.Text,
		'trainer_ability_list': common.defs.TrainerAbilityList.TrainerAbilityList,
		'world_info': common.defs.WorldInfo.WorldInfo
	}

	__paks = {
		'ability': 'CookedDataAbilities',
		'applied_science': 'CookedSciences',
		'blueprint': 'CookedBlueprints',
		'character_creation': 'CookedCharCreation',
		'container': 'CookedDataContainers',
		'discipline': 'CookedDisciplines',
		'dialog': 'CookedDataDialogs',
		'effect': 'CookedDataEffects',
		'event_set': 'CookedDataKismetSetEvent',
		'error_text': 'ErrorStrings',
		'interaction': 'CookedInteractions',
		'interaction_set_map': 'CookedInteractionSet',
		'item': 'CookedDataItems',
		'mission': 'CookedDataMissions',
		'racial_paradigm': 'CookedParadigm',
		'sequence': 'CookedDataKismetSeqEvent',
		'special_words': 'SpecialWords',
		'stargate': 'CookedDataStargates',
		'text': 'TextStrings',
		'world_info': 'CookedWorldInfo'
	}

	# This is the order in which resource categories are loaded
	__loadOrder = [
		# Load resources with no dependencies first
		'applied_science',
	    'container',
	    'dialog',
	    'error_text',
	    'interaction',
	    'racial_paradigm',
	    'text',
	    'world_info',
	    'path',
	    'special_words',
	    'pet_command',
	    'interaction_set',
	    # Respawner depends on WorldInfo
	    'respawner',
	    # InteractionSetMap depends on Dialog, InteractionSet
	    'interaction_set_map',
	    # Discipline depends on AppliedScience, RacialParadigm
	    'discipline',
	    # EventSet depends on Sequence (not yet implemented)
	    'sequence',
	    'event_set',
	    # Ability depends on EventSet
	    'ability',
	    # AbilitySet depends on Ability
	    'ability_set',
	    # Archetype depends on Ability
	    'archetype',
	    # TrainerAbilityList depends on Archetype, Ability
	    'trainer_ability_list',
	    # Item depends on AppliedScience, Discipline, Ability
	    'item',
		# ItemList depends on Item
		'item_list',
		# LootTable depends on Item
		'loot_table',
	    # CharacterCreation depends on Item
	    'character_creation',
	    # Mission depends on Item
	    'mission',
	    # Blueprint depends on Item, Discipline
	    'blueprint',
	    # Effect depends on Ability and EventSet
	    'effect',
	    # Stargate depends on WorldInfo and EventSet
	    'stargate',
		# Ring region depends on WorldInfo and EventSet
		'ring_transporter_region',
	    # Template looting depends on LootTable, vending on EventSet, mob abilities on AbilitySet
	    # (other dependencies are expected too, so keep this one at the bottom of the list)
	    'template'
	]

	# When a circular dependency exists between resources (A references B that references A again),
	# one of the dependent types must be postloaded (references must be checked after every resource was loaded).
	__postLoad = [
		'ability',
		'ring_transporter_region',
	    'discipline'
	]
	
	__defs = {}
	__versions = {}
	clientOnly = False

	@staticmethod
	def getAllDefs():
		"""
		Returns all resources in all resource groups
		"""
		return DefMgr.__defs

	@staticmethod
	def getAll(type):
		"""
		Returns all resources in the specified resouce group
		@param type: Type of resource
		@return: (resourceId, resource) dict
		"""
		if type not in DefMgr.__defs:
			raise Exception("Unknown resource definition type: %s" % type)
		return DefMgr.__defs[type]

	@staticmethod
	def get(type, id):
		"""
		Retrieves a resource with the specified identifier
		@param type: Type of resource
		@param id: Unique identifier of resource
		@return: Resource object, or None if resource was not found
		"""
		if type not in DefMgr.__defs:
			raise Exception("Unknown resource definition type: %s" % type)
		return DefMgr.__defs[type].get(id)

	@staticmethod
	def require(type, id):
		"""
		Retrieves a resource with the specified identifier.
		Throws an exception if the resource doesn't exist
		@param type: Type of resource
		@param id: Unique identifier of resource
		@return: Resource object
		"""
		if type not in DefMgr.__defs:
			raise Exception("Unknown resource definition type: %s" % type)
		resource = DefMgr.__defs[type].get(id)
		if not resource:
			# raise Exception("Resource %d of type '%s' does not exist" % (id, type))
			warn(Atrea.enums.LC_Resources, "Referenced resource %s of type '%s' does not exist" % (str(id), type))
		return resource


	@staticmethod
	def load(clientOnly = False):
		"""
		Loads resources from DB for all registered defs
		@param clientOnly: Only load cookable client resources
		"""
		DefMgr.clientOnly = clientOnly
		for category in DefMgr.__loadOrder:
			if not clientOnly or DefMgr.__classes[category].versioning:
				DefMgr.loadCategory(category)

		for cls in DefMgr.__postLoad:
			if not clientOnly or DefMgr.__classes[cls].versioning:
				for resource in DefMgr.__defs[cls].values():
					resource.postLoad(DefMgr)


	@staticmethod
	def hasCategory(category):
		"""
		Checks if the specified category name exists
		@param category: Category name
		"""
		return category in DefMgr.__classes


	@staticmethod
	def loadCategory(category):
		"""
		Loads all resources from the database for the specified category
		@param category: Category name
		"""
		mgr = dict()
		info(Atrea.enums.LC_Resources, 'Loading %s resources ...' % category)
		DefMgr.__classes[category].loadAll(mgr, DefMgr)
		DefMgr.__defs[category] = mgr
		if DefMgr.__classes[category].versioning:
			DefMgr.__versions[category] = DefMgr.loadHistory('RESOURCE_' + DefMgr.__classes[category].__name__)


	@staticmethod
	def getVersion(type):
		"""
		Returns the version number of the specified resource category.
		@param type: Resource category to retrieve
		@return: Version number
		"""
		if type not in DefMgr.__versions:
			raise Exception("Resource type is not versioned: %s" % type)
		return DefMgr.__versions[type][0]['version']


	@staticmethod
	def getChangesForVersion(type, version):
		"""
		Returns the list of keys that were added and invalidated after the specified version.
		@param type: Resource category to retrieve
		@param version: Version number to check
		@return: (invalidateAll, addedKeys, invalidatedKeys)
		"""
		serverVersion = DefMgr.getVersion(type)
		versions = DefMgr.__versions[type]
		if version > serverVersion:
			warn(Atrea.enums.LC_Resources, 'Version %d too new for category %s (local %d), purging all resources' % (version, type, serverVersion))
			return True, None, None

		if version < serverVersion - len(versions):
			trace(Atrea.enums.LC_Resources, 'Version %d too old for category %s (local %d), purging all resources' % (version, type, serverVersion))
			return True, None, None

		added = []
		invalidated = []
		for ver in reversed(range(0, serverVersion - version)):
			if versions[ver]['invalidate_all']:
				trace(Atrea.enums.LC_Resources, 'Version %d invalidates all resources for category %s' % (ver, serverVersion))
				return True, None, None
			added.extend(versions[ver]['added'])
			invalidated.extend(versions[ver]['invalidated'])
		trace(Atrea.enums.LC_Resources, '%d added, %d invalidated from version %d to %d for category %s' % (len(added), len(invalidated), version, serverVersion, type))
		return False, added, invalidated


	@staticmethod
	def loadHistory(type):
		hist = Atrea.dbQuery(
			'''
			select *
			from   resources.resource_versions
			where  type = '%s'
			order by version desc
			limit %d
			''' % (type, 300)
		)
		history, added, invalidated = [], [], []
		lastver = None
		for item in hist:
			if lastver is not None and item['version'] != lastver - 1:
				warn(Atrea.enums.LC_Resources, 'Missing version info for %s (version %d), assuming invalidate_all = true' % (type, lastver - 1))
				item['invalidate_all'] = True

			if item['invalidate_all']:
				# trace(Atrea.enums.LC_Resources, 'Version %d invalidates all resources' % item['version'])
				added = []
				invalidated = []
			else:
				added.extend([int(k) for k in item['new_keys'][1:-1].split(',') if k])
				invalidated.extend([int(k) for k in item['invalidated_keys'][1:-1].split(',') if k])
				# trace(Atrea.enums.LC_Resources, 'Version %d adds %d, invalidates %d' % (item['version'], len(added), len(invalidated)))

			history.append({
				'added': added[:],
				'invalidated': invalidated[:],
				'invalidate_all': item['invalidate_all'] == 1,
				'version': item['version']
			})

			lastver = item['version']
			if item['invalidate_all'] == 't':
				break

		return history

	@staticmethod
	def makeResource(type, id):
		# Handle special cases (character creation, special words) where
		# resources don't have an ID
		if type == 'character_creation':
			return DefMgr.makeCharCreationResource()
	
		resource = DefMgr.get(type, id)
		if not resource:
			warn(Atrea.enums.LC_Resources, "Couldn't find resource %d of type %s" % (id, type))
			return None

		trace(Atrea.enums.LC_Resources, 'Sending cooked resource %d (%s)' % (id, type))
		return resource.toXml().encode('utf-8')

	@staticmethod
	def makeCharCreationResource():
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_CHAR_CREATION>'
		for d in DefMgr.__defs['character_creation'].values():
			xml += d.toXml()
		xml += "</COOKED_CHAR_CREATION>"
		return xml.encode()

	@staticmethod
	def makePakMetaData(version):
		return unhexlify('%08x' % version)[::-1]

	@staticmethod
	def makePaks():
		for category, elements in DefMgr.__defs.items():
			if category in DefMgr.__paks:
				trace(Atrea.enums.LC_Resources, 'Creating cache %s.pak ...' % DefMgr.__paks[category])
				pak = zipfile.ZipFile('../data/cache/' + DefMgr.__paks[category] + '.pak', 'w', ZIP_DEFLATED)
				pak.writestr('MetaData', DefMgr.makePakMetaData(DefMgr.getVersion(category)))
				for key, element in elements.items():
					pak.writestr('_' + str(key), element.toXml().encode())
				pak.close()



	