class SpaceManager(object):
	spaces = {}
	worlds = {}

	@staticmethod
	def registerSpace(space):
		SpaceManager.spaces[space.spaceId] = space
		if not space.instanced:
			SpaceManager.worlds[space.worldName] = space

	@staticmethod
	def unregisterSpace(space):
		del SpaceManager.spaces[space.spaceId]
		if not space.instanced:
			del SpaceManager.worlds[space.worldName]

	@staticmethod
	def get(spaceId):
		return SpaceManager.spaces.get(spaceId)

	@staticmethod
	def getByWorldName(worldName):
		return SpaceManager.worlds.get(worldName)
