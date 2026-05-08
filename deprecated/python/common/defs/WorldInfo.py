import Atrea
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class WorldInfo(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['world_id']
		self.world = row['world']
		self.clientMap = row['client_map']
		self.minToRealMin = row['min_to_real_min']
		self.minPerDay = row['min_per_day']
		self.flags = row['flags']
		self.cellId = row['cell_id']
		self.gravity = row['gravity']
		self.runSpeed = row['run_speed']
		self.sidewaysRunSpeed = row['sideways_run_speed']
		self.backwardsRunSpeed = row['backwards_run_speed']
		self.walkSpeed = row['walk_speed']
		self.sidewaysWalkSpeed = row['sideways_walk_speed']
		self.backwardsWalkSpeed = row['backwards_walk_speed']
		self.crouchRunSpeed = row['crouch_run_speed']
		self.sidewaysCrouchRunSpeed = row['sideways_crouch_run_speed']
		self.backwardsCrouchRunSpeed = row['backwards_crouch_run_speed']
		self.crouchWalkSpeed = row['crouch_walk_speed']
		self.sidewaysCrouchWalkSpeed = row['sideways_crouch_walk_speed']
		self.backwardsCrouchWalkSpeed = row['backwards_crouch_walk_speed']
		self.swimSpeed = row['swim_speed']
		self.sidewaysSwimSpeed = row['sideways_swim_speed']
		self.backwardsSwimSpeed = row['backwards_swim_speed']
		self.jumpSpeed = row['jump_speed']
		self.hasScript = (row['has_script'] == 1)
		self.stargates = []

	def toXml(self):
		""" Creates a cooked XML resource from the stargate definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_WORLD_INFO WorldID="' + str(self.id) + \
		      '" World=' + quoteattr(self.world) + ' MinToRealMin="' + str(self.minToRealMin) + \
		      '" MinPerDay="' + str(self.minPerDay) + '" ClientMap=' + quoteattr(self.clientMap) + ' Flags="' + str(self.flags) + '" />'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		worlds = Atrea.dbQuery('select * from resources.worlds')
		for world in worlds:
			defs[world['world_id']] = WorldInfo(world, defMgr)
