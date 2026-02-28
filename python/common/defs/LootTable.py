import Atrea
from common.defs.Resource import Resource

class Loot(object):
	def __init__(self, row, defMgr):
		self.id = row['loot_id']
		if row['design_id'] is not None:
			self.design = defMgr.require('item', row['design_id'])
		else:
			self.design = None
		self.minQuantity = row['min_quantity']
		self.maxQuantity = row['max_quantity']
		self.probability = row['probability']


class LootTable(Resource):
	def __init__(self, row, defMgr):
		self.id = row['loot_table_id']
		self.description = row['description']
		self.loot = []


	@staticmethod
	def loadAll(defs, defMgr):
		templates = Atrea.dbQuery('select * from resources.loot_tables')
		for template in templates:
			defs[template['loot_table_id']] = LootTable(template, defMgr)

		loot = Atrea.dbQuery('select * from resources.loot')
		for item in loot:
			defs[item['loot_table_id']].loot.append(Loot(item, defMgr))
