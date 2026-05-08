import Atrea
import Atrea.enums
from common.Logger import warn
from common.defs.Resource import Resource


class RingTransporterRegion(Resource):
	def __init__(self, row, defMgr):
		self.id = row['region_id']
		self.world = defMgr.require('world_info', row['world_id'])
		self.x = row['x']
		self.y = row['y']
		self.z = row['z']
		self.displayNameId = row['display_name_id']
		self.tag = row['tag']
		self.radius = row['radius']
		self.height = row['height']
		self.eventSet = defMgr.require('event_set', row['event_set_id']) if row['event_set_id'] else None
		self.destinationIds = [int(e) for e in row['destination_region_ids'][1:-1].split(',') if e]
		self.pointSetId = row['point_set_id']


	def postLoad(self, defMgr):
		ids = self.destinationIds
		self.destinationIds = []
		for id in ids:
			if id == self.id:
				warn(Atrea.enums.LC_Uncategorized, 'Region %d has its own ID in the destination list. Destination dropped.' % id)
			elif defMgr.require('ring_transporter_region', id) is None:
				warn(Atrea.enums.LC_Uncategorized, 'Region %d has invalid ID %d in the destination list. Destination dropped.' % (self.id, id))
			else:
				self.destinationIds.append(id)


	@staticmethod
	def loadAll(defs, defMgr):
		regions = Atrea.dbQuery('select * from resources.ring_transport_regions')
		for region in regions:
			defs[region['region_id']] = RingTransporterRegion(region, defMgr)
