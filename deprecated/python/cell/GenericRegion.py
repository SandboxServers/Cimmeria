import Atrea
import Atrea.enums
from common.Config import Config
from common.Logger import warn, trace


class GenericRegion:
	def __init__(self, dbid, tag, radius, height, flags, points, callback):
		self.id = None
		self.dbid = dbid
		self.tag = tag
		self.radius = radius
		self.height = height
		self.flags = flags
		self.points = points
		self.callback = callback


	def loaded(self):
		"""
		Indicates that all points were added to this region.
		"""
		self.workaround()
		self.updateAABB()
		if len(self.points) != 4:
			warn(Atrea.enums.LC_Uncategorized, 'Generic region %d has %d points; region will not be triggered!' % (self.dbid, len(self.points)))


	def updateAABB(self):
		"""
		Recalculates the Axis-Aligned Bounding Box (AABB) around the generic region.
		"""
		self.minPos = Atrea.Vector3(
			min([p.x for p in self.points]),
			min([p.y for p in self.points]),
			min([p.z for p in self.points])
		)
		self.maxPos = Atrea.Vector3(
			max([p.x for p in self.points]),
			max([p.y for p in self.points]),
			max([p.z for p in self.points])
		)


	def isPointInRegion(self, point):
		"""
		Checks if the specified point is inside the region.
		@param point: Point to check
		@return: True if the point is inside, False otherwise
		"""
		if len(self.points) == 4:
			thresh = Config.GENERIC_REGION_CHECK_THRESHOLD
			if self.minPos.x - thresh <= point.x <= self.maxPos.x + thresh and \
				self.minPos.y - thresh <= point.y <= self.maxPos.y + thresh and \
				self.minPos.z - thresh <= point.z <= self.maxPos.z + thresh:
				# TODO: Do precise check, AABB isn't enough!
				return True
			else:
				return False
		else:
			warn(Atrea.enums.LC_Uncategorized, "isPointInRegion called for %d points!" % len(self.points))
			return False


	def workaround(self):
		"""
		Workaround for buggy radius attribute
		"""
		if len(self.points) == 1 and Config.GENERIC_REGION_RADIUS_WORKAROUND == True:
			p = self.points[0]
			self.points = [
				Atrea.Vector3(p.x - self.radius, p.y, p.z - self.radius),
				Atrea.Vector3(p.x - self.radius, p.y, p.z + self.radius),
				Atrea.Vector3(p.x + self.radius, p.y, p.z + self.radius),
				Atrea.Vector3(p.x + self.radius, p.y + self.height, p.z - self.radius)
			]
			self.radius = 0
			self.height = 0


class GenericRegionManager:
	regions = None
	pointSets = None
	lastRegionId = 0

	def __init__(self):
		self.regions = {}
		self.pointSets = {}


	def getRegion(self, regionId):
		"""
		Returns the specified generic region
		@param regionId: Region ID
		@return: Region
		"""
		return self.regions.get(regionId)


	def getPointSet(self, pointSetId):
		"""
		Returns the specified generic region
		@param pointSetId: Point Set ID (database ID) of region
		@return: Region
		"""
		return self.pointSets.get(pointSetId)


	def load(self, worldId):
		rows = Atrea.dbQuery("select * from resources.point_sets where world_id = '" + str(worldId) + "' and type = 'AreaSet'")
		regions = {}
		for region in rows:
			regions[region['set_id']] = GenericRegion(region['set_id'], region['name'], region['radius'] or 0,
				region['height'] or 0, region['flags'], [], None)

		rows = Atrea.dbQuery("""
			select *
			from resources.point_sets
			inner join resources.point_set_points using (set_id)
			where world_id = %d and type = 'AreaSet'
			""" % worldId)
		for point in rows:
			regions[point['set_id']].points.append(Atrea.Vector3(point['x'], point['y'], point['z']))

		self.pointSets = regions
		for region in regions.values():
			self.registerRegion(region)


	def registerRegion(self, region):
		"""
		Registers a new client hinted generic region with this region manager
		@param region: Region to register
		@return: Allocated region ID
		"""
		self.lastRegionId += 1
		region.id = self.lastRegionId
		region.loaded()
		self.regions[region.id] = region
		return region.id


	def queryRegions(self, player):
		"""
		Sends all client hinted generic regions to the player via RPC
		@param player: Player
		"""
		for rgn in self.regions.values():
			if rgn.flags & Atrea.enums.REGION_FLAG_ClientHinted:
				player.client.addClientHintedGenericRegion(rgn.id, rgn.height, rgn.radius, rgn.flags, rgn.points)


	def triggerRegion(self, regionId, entering, position, entity):
		"""
		Triggers an enter/exit event on the specified generic region.
		@param regionId: Region to trigger
		@param entering: Is the entity entering or exiting?
		@param position: Position of the entity at the time of the event
		@param entity: Entity
		"""
		if regionId not in self.regions:
			warn(Atrea.enums.LC_Uncategorized, "Entity <%s> attempted to trigger unknown generic region <%d>" % (entity.getName(), regionId))
			return

		# TODO: Check !entering and isPointOutsideRegion() too
		region = self.regions[regionId]
		if entering and not region.isPointInRegion(entity.position):
			warn(Atrea.enums.LC_Uncategorized, "Entity <%s> attempted to trigger generic region <%d> from too far away" % (entity.getName(), regionId))
			return

		trace(Atrea.enums.LC_Uncategorized, '<%s> %s client hinted region %s' %
			 (entity.getName(), 'Entering' if entering else 'Exiting', region.tag if region.tag else str(id)))

		if region.flags & Atrea.enums.REGION_FLAG_Stargate:
			entity.debug("<%s> Entering stargate region %d" % (entity.getName(), regionId))
			entity.stargatePassed()
		elif region.callback is None:
			if region.tag is not None:
				entity.fire('client_hinted_region::' + region.tag, regionId = region.id, entity = entity, entering = entering)
			else:
				entity.fire('client_hinted_region::' + str(region.id), regionId = region.id, entity = entity, entering = entering)
		else:
			region.callback(entering, position, entity)
			
