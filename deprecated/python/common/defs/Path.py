import Atrea
from common.defs.Resource import Resource


class Path(Resource):
	def __init__(self, row):
		self.id = row['path_id']
		self.nodes = []
		self.addNode(row)

	def addNode(self, row):
		if len(self.nodes) <= row['index']:
			self.nodes += [None] * (row['index'] - len(self.nodes) + 1)
		self.nodes[row['index']] = {
				'x': row['x'],
			    'y': row['y'],
			    'z': row['z']
			}

	@staticmethod
	def loadAll(defs, defMgr):
		paths = Atrea.dbQuery('select * from resources.paths')
		for path in paths:
			if path['path_id'] not in defs:
				defs[path['path_id']] = Path(path)
			defs[path['path_id']].addNode(path)
