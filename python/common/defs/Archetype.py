import Atrea
import Atrea.enums
from common.defs.Resource import Resource

Archetypes = [
	Atrea.enums.ARCHETYPE_Any,
	Atrea.enums.ARCHETYPE_Soldier,
	Atrea.enums.ARCHETYPE_Commando,
	Atrea.enums.ARCHETYPE_Scientist,
	Atrea.enums.ARCHETYPE_Archeologist,
	Atrea.enums.ARCHETYPE_Asgard,
	Atrea.enums.ARCHETYPE_Goauld,
	Atrea.enums.ARCHETYPE_Sholva,
	Atrea.enums.ARCHETYPE_Jaffa
]

class ArchetypeAbility(object):
	def __init__(self, row, defMgr):
		self.ability = defMgr.require('ability', row['ability_id'])
		self.level = row['level']
		self.prerequisites = [defMgr.require('ability', int(a)) for a in row['prerequisite_abilities'][1:-1].split(',') if a]


class Archetype(Resource):
	def __init__(self, row):
		self.id = Atrea.enums.__dict__[row['archetype']]
		self.name = row['name']
		self.coordination = row['coordination']
		self.engagement = row['engagement']
		self.fortitude = row['fortitude']
		self.morale = row['morale']
		self.perception = row['perception']
		self.intelligence = row['intelligence']
		self.health = row['health']
		self.focus = row['focus']
		self.healthPerLevel = row['healthPerLevel']
		self.focusPerLevel = row['focusPerLevel']
		self.abilityTrees = [[], [], []]
		self.abilities = {}

	@staticmethod
	def loadAll(defs, defMgr):
		archetypes = Atrea.dbQuery('select * from resources.archetypes')
		for archetype in archetypes:
			typeId = Atrea.enums.__dict__[archetype['archetype']]
			defs[typeId] = Archetype(archetype)

		abilities = Atrea.dbQuery('select * from resources.archetype_ability_tree order by ability_index')
		for ability in abilities:
			archetype = Atrea.enums.__dict__[ability['archetype']]
			arAbility = ArchetypeAbility(ability, defMgr)
			defs[archetype].abilityTrees[ability['tree_index']].append(arAbility)
			defs[archetype].abilities[arAbility.ability.id] = arAbility
