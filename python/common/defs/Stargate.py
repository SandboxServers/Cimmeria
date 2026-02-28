import Atrea
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class Stargate(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['stargate_id']
		self.name = row['name']
		self.address1 = row['address1']
		self.address2 = row['address2']
		self.address3 = row['address3']
		self.address4 = row['address4']
		self.address5 = row['address5']
		self.address6 = row['address6']
		self.addressOrigin = row['address_origin']
		self.world = defMgr.require('world_info', row['world_id'])
		self.world.stargates.append(self)
		self.xPos = row['x_pos']
		self.yPos = row['y_pos']
		self.zPos = row['z_pos']
		self.pitch = row['pitch']
		self.yaw = row['yaw']
		self.roll = row['roll']
		self.prefabSequence = row['prefab_sequence']
		self.eventSet = defMgr.require('event_set', row['event_set_id']) if row['event_set_id'] else None


	def toXml(self):
		""" Creates a cooked XML resource from the stargate definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_STARGATE id="' + str(self.id) + \
		      '" name=' + quoteattr(self.name) + ' address1="' + str(self.address1) + \
		      '" address2="' + str(self.address2) + '" address3="' + str(self.address3) + \
		      '" address4="' + str(self.address4) + '" address5="' + str(self.address5) + \
		      '" address6="' + str(self.address6) + '" addressOrigin="' + str(self.addressOrigin) +  \
		      '" worldId="' + str(self.world.id) + '" xPos="' + str(self.xPos) + \
		      '" yPos="' + str(self.yPos) + '" zPos="' + str(self.zPos) + \
		      '" pitch="' + str(self.pitch) + '" yaw="' + str(self.yaw) + \
		      '" roll="' + str(self.roll) + '" prefabSequence=' + quoteattr(self.prefabSequence) + ' />'
		return xml


	@staticmethod
	def loadAll(defs, defMgr):
		stargates = Atrea.dbQuery('select * from resources.stargates')
		for stargate in stargates:
			defs[stargate['stargate_id']] = Stargate(stargate, defMgr)
