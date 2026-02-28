import Atrea
from common.defs.Resource import Resource


class InteractionSet(Resource):
	def __init__(self, row, defMgr):
		self.id = row['dialog_set_id']
		self.name = row['name']
		self.interactions = {}

	@staticmethod
	def loadAll(defs, defMgr):
		sets = Atrea.dbQuery('select * from resources.dialog_sets')
		for interactionSet in sets:
			defs[interactionSet['dialog_set_id']] = InteractionSet(interactionSet, defMgr)
