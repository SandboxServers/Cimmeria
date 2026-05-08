import Atrea
from common.defs.Resource import Resource


class AbilitySet(Resource):
	def __init__(self, row, defMgr):
		self.id = row['ability_set_id']
		self.abilities = {}


	@staticmethod
	def loadAll(defs, defMgr):
		abilitySets = Atrea.dbQuery('select * from resources.ability_sets')
		for abilitySet in abilitySets:
			defs[abilitySet['ability_set_id']] = AbilitySet(abilitySet, defMgr)

		abilities = Atrea.dbQuery('select * from resources.ability_set_abilities')
		for abilityInfo in abilities:
			ability = defMgr.require('ability', abilityInfo['ability_id'])
			defs[abilityInfo['ability_set_id']].abilities[ability.id] = ability

