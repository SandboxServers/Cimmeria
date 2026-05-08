# Skeletons for definitions in BaseApp/CellApp
class Vector3:
	"""
	Generic 3D vector class used by the engine
	"""
	x = 0.0
	y = 0.0
	z = 0.0

	def __init__(self, x=None, y=None, z=None):
		pass

	def length(self) -> float:
		"""
		Returns the length of the vector
		"""
		return 0.0

	def normalizes(self):
		"""
		Normalizes the vector
		"""
		pass


class BaseEntity(object):
	entityId = 0
	databaseId = 0
	cell = object()
	client = object()

	def destroy(self):
		"""
		Destroys the entity.
		If the entity had a cell part, it will also be deleted.
		WARNING: Accessing any property or method of this entity after calling destroy() might cause a crash!
		"""
		pass

	def disconnect(self, killConnection : bool):
		"""
		Disconnects the entity from its controller.
		@param killConnection: False to reset entities, True to close client connection
		"""
		pass



class Minigame(object):
	key = ""
	gameName = ""
	entityId = 0
	difficulty = 0
	techCompetency = 0
	seed = 0
	abilitiesMask = 0
	intelligence = 0
	playerLevel = 0


	def send(self, message):
		"""
		Sends an XT extension message to the client
		@param message: Message to send
		"""
		pass


	def abort(self, reason):
		"""
		Aborts the currently running minigame
		"""
		pass


	def victory(self):
		"""
		Wins the currently running minigame
		"""
		pass



class CellEntity(object):
	entityId = 0
	databaseId = 0
	base = object()
	client = object()
	witnesses = object()
	spaceId = 0
	position = Vector3()
	rotation = Vector3()
	velocity = Vector3()
	maxSpeed = 0.0
	# Are entity messages cacheable on the BaseApp?
	cacheable = False
	# Do witnesses need to receive dynamic updates?
	hasDynamicProperties = True

	def leaveSpace(self):
		"""
		Removes the entity from the space it is on.
		@return True if successful, False if entity is not on a space.
		"""
		return True

	def enterSpace(self, spaceId : int):
		"""
		Moves the entity to the specified space
		@param spaceId ID of new space
		@return True if successful, False if entity is already on a space or if spaceId is invalid.
		"""
		return True

	def loadCell(self):
		"""
		Setup controller and player status.
		Player entities only.
		"""
		pass

	def destroy(self):
		"""
		Destroys the entity (remove it from the space, persist it and free its entityId)
		"""
		pass

	def enableDebugController(self):
		"""
		Enables the debug movement controller on the entity
		@param player: SGWPlayer
		@param target: Targeted entity
		"""
		pass

	def addWaypoint(self, pos : Vector3, callback):
		"""
		Adds a point to the list of waypoints on the entity
		@param pos New position
		@param callback Callback function called when the entity reached its destination
		"""
		pass

	def cancelMovement(self):
		"""
		Clears all active movement controllers on the entity
		"""
		pass

	def moveToSpace(self, spaceId : int, position : Vector3, rotation : Vector3):
		"""
		Moves the entity to the specified space and position.
		@param spaceId ID of new space
		@param position New position of entity
		@param rotation New rotation vector of entity
		"""
		pass

	def moveToWorld(self, spaceName : str, position : Vector3, rotation : Vector3):
		"""
		Moves the entity to the specified space and position.
		@param spaceName World name of new space (a new spaceId is allocated for instanced spaces)
		@param position New position of entity
		@param rotation New rotation vector of entity
		"""
		pass

	def hide(self):
		"""
		Hides the entity (notifies players that it left their AoI)
		"""
		pass

	def dynamicUpdate(self, entity):
		"""
		Requests the server to perform an update on the dynamic properties of the specified entity.
		This calls onDynamicUpdate() on the caller entity where the witness
		@param entity Entity to perform the update on
		"""
		pass

	def createCacheStamp(self, propertySetId, callback, invalidate):
		"""
		Creates a new revision of a cached property set on the BaseApp and notifies witnesses.
		Calls the specified callback function and redirects all mailboxes for the duration of
		the call to the cache message set.
		NOTE: This call is a no-op if the entity is not cacheable.
		WARNING: Do not call RPC methods on the mailboxes of other entities for the duration of the call.
		@param propertySetId Property set to update
		@param callback Cache stamp creator callback
		@param invalidate Invalidate all current cache stamps?
		"""
		pass


def dbPerform(queryString : str):
	"""
	Executes a synchronous database query (returns no results)
	Availability: BaseApp and CellApp.

	@param queryString: SQL query to execute
	"""
	pass

def dbQuery(queryString : str):
	"""
	Executes a synchronous database query
	Availability: BaseApp and CellApp.

	@param queryString: SQL query to execute
	@return: List of result rows
	@rtype: list
	"""
	pass

def log(severity, category, message : str, *args):
	"""
	Logs a message to the server console.
	Availability: BaseApp and CellApp.

	@param severity: Severity level (TRACE, DEBUG1, DEBUG2, INFO, WARNING, ERROR, CRITICAL)
	@param category: Name of message category (max 8 chars)
	@param message: Message (format string)
	@param args: Message arguments
	"""
	pass

def sendResource(entity, categoryId : int, elementId : int):
	"""
	BaseApp: Sends a cooked resource to the client
	(Only works on player entities!)
	@param entity: Base player object
	@param categoryId: Type of resource to send
	@param elementId: Key of resource to send
	"""
	pass

def createBaseEntity(className : str):
	"""
	Creates a new base entity
	Availability: BaseApp-only.

	@param className: Type of entity to create (must have a classIndex and python implementation)
	@return: New entity (or None on failure)
	"""
	pass


def createBaseEntityFromDb(className : str, databaseId : int):
	"""
	Creates and loads a new base entity
	Availability: BaseApp-only.

	@param className: Type of entity to create (must have a classIndex and python implementation)
	@param databaseId: Unique ID of database object
	@return: New entity (or None on failure)
	"""
	pass

def createCellEntity(entityId, spaceId, databaseId, x, y, z, yaw, pitch, roll, worldName):
	"""
	Creates a new cell entity for the base entity specified by entityId.
	If spaceId is valid (not -1), the base tries to find a valid CellApp for the space and
	creates the entity on that space; if spaceId is not set, the base tries to find a cell
	for the specified world name (or creates a new instance if the world is instanced).
	May fail if entity is not a valid base entity, spaceId or worldName is invalid, a CellApp fails.
	cellCreated(spaceId) entity method is called if cell entity was created.
	cellCreateFailed() is called if entity creation fails.
	Availability: BaseApp-only.

	@param entityId: Id of base entity
	@param spaceId: Destination space for cell entity (-1 = not set, determined by worldName)
	@param databaseId: ID of database object
	@param x: X coordinate of cell entity
	@param y: Y coordinate of cell entity
	@param z: Z coordinate of cell entity
	@param yaw: Yaw of cell entity
	@param pitch: Pitch of cell entity
	@param roll: Roll of cell entity
	@param worldName: World name identifying a unique world instance or an instanced world
	"""
	pass

def switchEntity(entityId, newEntityId):
	"""
	Swaps the active (controlled) entity on the connection and resets the
	entity system on the client side.
	Availability: BaseApp-only.

	@param entityId: Old entity ID (must be an entity with a controller)
	@param newEntityId: New entity ID (must not have a controller)
	"""
	pass

def registerMinigameSession(entityId, gameName, difficulty, techCompetency, seed, abilitiesMask, intelligence,
	playerLevel, callback):
	"""
	Registers a new minigame session with the session manager.
	The method returns an authentication key that the client can use to authenticate itself
	when connecting to the minigame server.
	Availability: BaseApp-only.

	@param entityId: Entity playing the minigame
	@param gameName: Name of minigame being played
	@param difficulty: Difficulty level (1-5)
	@param techCompetency: Tech competency level
	@param seed: Random seed of game
	@param abilitiesMask: Mask of cheat abilities
	@param intelligence: Players intelligence level
	@param playerLevel: Players level
	@param callback: Minigame completion callback method (argument: resultCode)
	@return: Full minigame URL the server must pass to the game client
	@rtype: str
	"""
	pass

def cancelMinigameSession(entityId):
	"""
	Cancels a new or in progress minigame session.
	The result callback registered with registerMinigameSession() will be called with result code
	MINIGAME_RESULT_Canceled after the session is canceled (this may take a while if the session is
	already in progress).
	Availability: BaseApp-only.

	@param entityId: Entity whose minigame is being canceled
	@return: True if the cancel request is successful, False if the game is already ended
	@rtype: bool
	"""
	pass

def reloadClasses():
	"""
	Development helper utility; re-extends all Python entity classes with native extensions.
	This *must* be called after reloading any of the entity modules.
	Availability: BaseApp and CellApp.
	"""
	pass

def createCellPlayer(entityId):
	"""
	Notifies the controller that a cell player was created for the cell entity.
	This does not work across cell boundaries and only accepts entities on the current cell.
	Availability: CellApp.

	@param entityId: Player entity
	"""
	pass

def destroyCellEntity(entityId):
	"""
	Destroys the specified cell entity. This does not work across cell boundaries and only
	accepts entities on the current cell.
	Availability: CellApp.

	@param entityId: Entity to destroy
	"""
	pass

def createSpace(worldName, creatorId):
	"""
	Creates a new space instance on this CellApp.
	If a creator entity is specified, the space will be tore down when the creator entity leaves.
	Availability: CellApp.

	@param worldName: World to instantiate
	@param creatorId: ID of creator entity (0 = no creator)
	@return: Space ID of new space
	@rtype: int
	"""
	pass

def destroySpace(spaceId):
	"""
	Destroys the specified space. All entities on the space will be deleted.
	Players on the space will be disconnected.
	This does not work across cell boundaries and only accepts spaces on the current cell.
	Availability: CellApp.

	@param spaceId: Space ID of space to destroy
	"""
	pass

def findSpace(worldName):
	"""
	Searches for a space instance using its world name.
	If the world is not instanced and an instance is running on this cell, the function returns
	the space ID. If no space was found, it returns -1.
	This does not work across cell boundaries and only finds spaces on the current cell.
	Availability: CellApp.

	@param worldName: World name to search for
	@return: Space ID of space or -1 if not found
	@rtype: int
	"""
	pass

def findEntity(entityId):
	"""
	Returns the Python object for the entity.
	If the entity is not found, the method returns None.
	This does not work across cell boundaries and only finds entities on the current cell.
	Availability: CellApp.

	@param entityId: ID of entity to look for
	@return: Entity
	"""
	pass


def findEntityOnSpace(entityId, spaceId):
	"""
	Returns the Python object for the entity.
	If the entity is not found, the space is not found, or the entity is not on the
	specified space, the method returns None.
	This does not work across cell boundaries and only finds entities on the current cell.
	Availability: CellApp.

	@param entityId: ID of entity to look for
	@param spaceId: Space the entity should be on
	@return: Entity
	"""
	pass


def findEntitiesByDbid(databaseId, spaceId):
	"""
	Returns a list containing Python objects for every entity matching the databaseID.
	If the space is not found the method returns None.
	This does not work across cell boundaries and only finds entities on the current cell.
	Availability: CellApp.

	@param databaseId: DatabaseID of entities to look for
	@param spaceId: Space the entities should be on
	@return: Entity list
	"""
	pass


def findAllEntities():
	"""
	Returns a list of all entities living on the BaseApp.
	Availability: BaseApp.

	@return: List of all base entities
	@rtype: list
	"""
	pass


def findEntities(spaceId):
	"""
	Returns a list of all entities on the given space.
	This does not work across cell boundaries and only finds entities on the current cell.
	Availability: CellApp.

	@param spaceId: Space to search
	@return: List of entities on the space
	@rtype: list
	"""
	pass

def getGameTime():
	"""
	Returns the game time in seconds.
	Availability: CellApp.

	@return: Game time
	@rtype: float
	"""
	pass

def addTimer(completeTime, callback):
	"""
	Adds a new one-shot game timer that completes at the game tick after completeTime.
	When the timer expires, its callback method is called.
	Availability: CellApp.

	@param completeTime: Game time when this timer completes
	@param callback: Callback function called when the timer completes
	@return: Timer ID of new timer
	@rtype: int
	"""
	pass

def cancelTimer(timerId):
	"""
	Cancels a pending timer identified by timerId.
	Availability: CellApp.
	@warning: Do NOT call this method if you are unsure if the timer exists or not.
	@warning: Timer IDs are reused, so canceling an already completed timer might cancel a different timer instead!

	@param timerId: ID of timer to cancel
	"""
	pass

def findPath(spaceId, startPos, destinationPos):
	"""
	Tries to use the space navigation mesh to find a valid path from the starting
	position to the destination position.
	This does not work across cell boundaries and only finds entities on the current cell.
	Availability: CellApp.

	@param spaceId: Space
	@param startPos: Starting position of entity
	@param destinationPos: Destination of entity
	@return: Movement path nodes (or None if no path found)
	@rtype: list
	"""
	pass

def findDetailedPath(spaceId, startPos, destinationPos, startExtents, destinationExtents,
	minDistance, maxDistance, maxPolys, smoothingStepSize):
	"""
	Tries to use the space navigation mesh to find a valid path from the starting
	position to the destination position.
	This does not work across cell boundaries and only finds entities on the current cell.
	findDetailedPath() is internally the same as findPath() but with more pathfinding parameters.
	Availability: CellApp.

	@param spaceId: Space
	@param startPos: Starting position of entity
	@param destinationPos: Destination of entity
	@param startExtents: Max distance between starting pos and starting poly
	@param destinationExtents: Max distance between destination pos and destination poly
	@param minDistance: Minimum allowed distance from destination
	@param maxDistance: Maximum allowed distance from destination
	@param maxPolys: Maximum number of polygons in initial path
	@param smoothingStepSize: Distance between points in the generated path
	@return: Movement path nodes (or None if no path found)
	@rtype: list
	"""
	pass

