import Atrea
import Atrea.enums
from common.Logger import warn
from common.defs.Resource import Resource


class TrainerAbilityList(Resource):
	def __init__(self, row, defMgr):
		self.id = row['list_id']
		self.abilities = {}


	@staticmethod
	def loadAll(defs, defMgr):
		abilityLists = Atrea.dbQuery('select * from resources.trainer_ability_lists')
		for abilityList in abilityLists:
			defs[abilityList['list_id']] = TrainerAbilityList(abilityList, defMgr)

		abilities = Atrea.dbQuery('select * from resources.trainer_abilities')
		for abilityInfo in abilities:
			abList = defs[abilityInfo['list_id']]
			archetype = Atrea.enums.__dict__[abilityInfo['archetype']]
			ar = defMgr.require('archetype', archetype)
			ability = defMgr.require('ability', abilityInfo['ability_id'])

			if ability.id not in ar.abilities:
				warn(Atrea.enums.LC_Resources, 'Cannot train ability %d: not in ability tree of archetype %s' %
					(ability.id, ar.name))
				continue

			if archetype not in abList.abilities:
				abList.abilities[archetype] = [ability]
			else:
				abList.abilities[archetype].append(ability)

