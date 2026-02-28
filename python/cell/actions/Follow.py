import weakref
import Atrea
import Atrea.enums
from common.Logger import trace, warn


class Follow(object):
	# Min distance between follower and target when planning the path
	followRange = 5.0
	# Don't start following the target until its in this range, even if it is moving
	nearRange = 10.0
	# Do path updates less frequently if the target is further than this
	farRange = 50.0
	# Re-plan the current path after this amount of seconds
	updateRateSecs = 1.0
	# Re-plan the current path after this amount of seconds if entity is outside the far range
	farUpdateRateSecs = 2.0
	# Path update wait time when planning failed
	errorUpdateRateSecs = 5.0
	# Last update timer ID
	timerId = None


	def __init__(self, entity, target):
		"""
		Starts following the target entity
		"""
		super().__init__()
		self.entity = weakref.ref(entity)
		self.target = weakref.ref(target)


	def start(self):
		"""
		Starts the action
		"""
		assert(self.timerId is None)
		self.__update()


	def cancel(self):
		"""
		Cancels the action
		"""
		if self.timerId is not None:
			Atrea.cancelTimer(self.timerId)
		if self.entity() is not None:
			self.entity().cancelMovement()


	def __update(self):
		"""
		Follow logic update method
		Executes every few seconds to refresh target entity state and update paths
		"""
		# The follower and target entity are weak references
		# Check if either of them is dead
		entity = self.entity()
		target = self.target()
		if entity is None:
			trace(Atrea.enums.LC_Location, "Follow: Owner entity was released, exiting")
			return

		if target is None:
			trace(Atrea.enums.LC_Location, "Follow: Lost target entity")
			entity.fire("follow.targetLost")
			return

		# Calculate the distance between the follower and the target entity
		pos = entity.position
		targetPos = target.position
		dv = Atrea.Vector3(pos.x - targetPos.x, pos.y - targetPos.y, pos.z - targetPos.z)
		dist = dv.length()
		updateRate = self.updateRateSecs if dist < self.farRange else self.farUpdateRateSecs
		# If we're outside the near range, start following the entity
		if dist > self.nearRange:
			trace(Atrea.enums.LC_Location, "Follow: Distance is %f, rate %f, following target" % (dist, updateRate))
			path = entity.findDetailedPathTo(targetPos, minDistance = self.followRange)
			if path is None:
				warn(Atrea.enums.LC_Location, "Follow: Could not generate a path, retrying later")
				entity.fire("follow.routingFailed")
				updateRate = self.errorUpdateRateSecs
			else:
				# Cancel what the entity is currently doing and start moving along the new path
				# TODO: This calculates the path to the entity twice!
				# TODO: addWaypoint() needs a "don't route" option and min/maxDistance (?)
				entity.cancelMovement()
				entity.addWaypoint(path[-1], lambda: None)
				entity.fire("follow.routeUpdated")
		else:
			trace(Atrea.enums.LC_Location, "Follow: Distance is %f, ticking ..." % dist)
			entity.fire("follow.idle")
		self.timerId = Atrea.addTimer(Atrea.getGameTime() + updateRate, lambda: self.__update())
