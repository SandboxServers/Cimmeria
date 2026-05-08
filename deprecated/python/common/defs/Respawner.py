import Atrea
from common.defs.Resource import Resource

class Respawner(Resource):
	def __init__(self, row, defMgr):
		self.id = row['respawner_id']
		self.world = defMgr.require('world_info', row['world_id'])
		self.name = row['name']
		self.x = row['pos_x']
		self.y = row['pos_y']
		self.z = row['pos_z']


	@staticmethod
	def loadAll(defs, defMgr):
		respawners = Atrea.dbQuery('select * from resources.respawners')
		for respawner in respawners:
			defs[respawner['respawner_id']] = Respawner(respawner, defMgr)
