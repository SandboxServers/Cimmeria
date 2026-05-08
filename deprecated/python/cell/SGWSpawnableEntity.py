import math
import weakref
import Atrea
import Atrea.enums
from cell.SGWEntity import SGWEntity
from cell.SpaceManager import SpaceManager
from common import Constants
from common.defs.Def import DefMgr
from common.defs.Sequence import Sequence
from common.defs.EventSet import EventSet
from common.Logger import trace, warn


class SGWSpawnableEntity(SGWEntity):
	interactionType = 0
	entityFlags = 0
	staticMeshName = ''
	bodySet = ''
	kismetEventSetId = 0
	beingNameId = 0
	
	primaryColorId = 0
	secondaryColorId = 0
	skinTint = 0

	# Is the entity visible?
	visible = True

	# Space the entity is currently on
	# None when it is "floating" (transitioning between spaces)
	space = None

	# Entity template we were created from
	# None for dynamically created entities and players
	template = None

	# Unique tag assigned to spawnlist entries
	tag = None
	# ID of entity in spawnlist table
	spawnId = None

	# Handlers for tatic sinteraction maps
	# Map of (interactionSetMapId -> Interaction objects)
	staticInteractions = {}

	# Loot manager object
	lootHandler = None

	def __init__(self):
		super().__init__()
		self.cacheable = Atrea.config.cache_entity_messages == "true"
		self.interactionList = {}
		self.staticInteractions = {}


	def destroyed(self):
		"""
		Called before CellApp destroys and persists the entity
		"""
		pass


	def load(self):
		"""
		Called when the CellApp wants to load the entity from the database
		"""
		return True


	def persist(self):
		"""
		Called when the CellApp wants to save this entity
		"""
		pass


	def backup(self):
		"""
		Called when the state of the entity is being saved
		(backup to BaseApp, moving to a different space, ...)
		@return Persistent data of the entity
		"""
		return {
			'spawnable': (self.interactionType, self.entityFlags, self.bodySet,
			self.kismetEventSetId, self.beingNameId, self.databaseId, self.primaryColorId,
			self.secondaryColorId, self.skinTint)
		}


	def restore(self, backup):
		self.interactionType, self.entityFlags, self.bodySet, self.kismetEventSetId, \
			self.beingNameId, self.databaseId, self.primaryColorId, self.secondaryColorId, \
			self.skinTint = backup['spawnable']
			
			
	def enteredSpace(self, spaceId):
		"""
		Called when the entity fully entered the specified space
		@param spaceId: ID of space the entity entered
		"""
		self.space = SpaceManager.get(spaceId)


	def leftSpace(self, spaceId):
		"""
		Called when the entity left the specified space
		(self.spaceId and local mailboxes are still available for the duration of the call)
		@param spaceId: ID of space the entity left
		"""
		self.space = None


	def cacheStampsReset(self):
		"""
		Called when the cached messages on the BaseApp were reset.
		"""
		self.createCacheStamp(0, lambda: self.createOnClient(self.witnesses), True)


	def createAppearanceOnClient(self, mailbox):
		if self.staticMeshName and self.bodySet:
			mailbox.onStaticMeshNameUpdate(self.staticMeshName, self.bodySet)


	def createOnClient(self, mailbox):
		if self.template is not None and self.template.speakerId is not None:
			mailbox.onEntityProperty(Atrea.enums.GENERICPROPERTY_DatabaseId, self.template.speakerId)
		if self.kismetEventSetId:
			# Don't send event set ID if it is not set, as the client will erroneously
			# try to wait for resource ID #0, which doesn't exist
			mailbox.onKismetEventSetUpdate(self.kismetEventSetId)
		self.createAppearanceOnClient(mailbox)
		mailbox.InteractionType(self.interactionType)
		if self.beingNameId != 0:
			mailbox.onBeingNameIDUpdate(self.beingNameId)
		mailbox.onEntityFlags(self.entityFlags)
		if self.visible:
			mailbox.onVisible(1)
		else:
			self.hide()


	def getName(self) -> str:
		"""
		Returns the name of the entity
		@return: Entity name
		"""
		if self.beingNameId != 0:
			text = DefMgr.get('text', self.beingNameId)
			if text is not None:
				return text.text
			else:
				return "<Name ID %d>" % self.beingNameId
		else:
			return "<Unnamed>"


	def setVisible(self, visible):
		"""
		Toggle visibility of this entity
		@param visible: Is the entity visible or not?
		"""
		if visible != self.visible:
			if visible:
				if self.client is not None:
					self.client.onVisible(1)
					self.playSequence(Constants.SEQUENCE_VisibilitySafety, self.entityId, playOnWitnesses = False)
				self.witnesses.onVisible(1)
			else:
				self.hide()
		self.visible = visible


	def setStaticMeshName(self, staticMesh):
		"""
		Changes the static mesh name of this entity
		@param staticMesh: Static mesh name
		"""
		self.staticMeshName = staticMesh
		if self.staticMeshName:
			if self.client is not None:
				self.client.onStaticMeshNameUpdate(self.staticMeshName, self.bodySet)
			self.witnesses.onStaticMeshNameUpdate(self.staticMeshName, self.bodySet)


	def setBodySet(self, bodySet):
		"""
		Changes the body set name of this entity
		@param bodySet: Body set name
		"""
		self.bodySet = bodySet
		if self.staticMeshName:
			if self.client is not None:
				self.client.onStaticMeshNameUpdate(self.staticMeshName, self.bodySet)
			self.witnesses.onStaticMeshNameUpdate(self.staticMeshName, self.bodySet)


	def setNameId(self, nameId):
		"""
		Changes the name ID of this entity
		"""
		self.beingNameId = nameId
		if nameId is not None:
			if self.client is not None:
				self.client.onBeingNameIDUpdate(self.beingNameId)
			self.witnesses.onBeingNameIDUpdate(self.beingNameId)


	def setEventSet(self, eventSetId):
		"""
		Changes the kismet event set of this entity
		"""
		self.kismetEventSetId = eventSetId
		if eventSetId is not None:
			if self.client is not None:
				self.client.onKismetEventSetUpdate(self.kismetEventSetId)
			self.witnesses.onKismetEventSetUpdate(self.kismetEventSetId)


	def setInteractionType(self, interactionType):
		"""
		Changes the interaction type of this entity
		@param interactionType: Interaction flags (see EInteractionNotificationType enum)
		"""
		self.interactionType = interactionType
		if self.client is not None:
			self.client.InteractionType(self.interactionType)
		self.witnesses.InteractionType(self.interactionType)


	def playSequence(self, seqId, targetId = None, primaryTarget = 1, time = None, nvps = None, viewType = None,
			instanceId = None, playOnSelf = True, playOnWitnesses = True):
		"""
		Plays a kismet sequence on the client and all witnesses

		@param seqId: Kismet sequence ID to play
		@param targetId: Target entity ID
		@param primaryTarget: Is this a primary target? 1 = True, 0 = False
		@param time: BigWorld time to play sequence at
		@param nvps: Name value pairs injected into sequence
		@param viewType: Kismet view type (see EKismetViewType enum)
		@param instanceId: Effect/Ability/... instance ID
		"""
		type = viewType or Atrea.enums.KISMET_VIEW_EventInvoker
		trace(Atrea.enums.LC_Visuals, 'Sequence %d (source %s, target %d, type %d)' % (seqId, self.getName(), targetId or 0, type))
		if self.client is not None and playOnSelf:
			self.client.onSequence(seqId, self.entityId, targetId or 0, primaryTarget, time or 0,
				nvps or [], type, instanceId or 0)
		if playOnWitnesses:
			self.witnesses.onSequence(seqId, self.entityId, targetId or 0, primaryTarget, time or 0,
				nvps or [], type, instanceId or 0)


	def lookAt(self, point):
		"""
		Updates the rotation of the entity so it "looks at" the specified point.
		@param point: Reference point
		"""
		pos = self.position
		x, z =  point.x - pos.x, point.z - pos.z
		len = math.sqrt(x*x + z*z)
		x, z = x / len, z / len
		rot = Atrea.Vector3()
		if x > 0:
			rot.y = math.pi - math.acos(-z)
		else:
			rot.y = math.pi + math.acos(-z)
		self.rotation = rot


	def moveTo(self, x, y, z, yaw = None, worldName = None):
		"""
		Moves the entity to the specified location on the local space or a remote space.
		(This will instantly move the entity and not "walk" the entity to the target location)
		@param x: X coordinate of new position
		@param y: Y coordinate of new position
		@param z: Z coordinate of new position
		@param yaw: Yaw (Z rotation)
		@param worldName: Name of destination world (None: don't move to a different space)
		"""
		pos = Atrea.Vector3(x, y, z)
		rot = Atrea.Vector3()
		if yaw is not None:
			rot.z = yaw
		if worldName is None or worldName == self.space.worldName:
			self.position = pos
			if yaw is not None:
				self.rotation = rot
		else:
			self.moveToWorld(worldName, pos, rot)


	def distanceTo(self, position):
		"""
		Calculates distance to the specified position.
		@param position: Target position
		@return: Distance
		"""
		s = self.position
		dv = Atrea.Vector3(s.x - position.x, s.y - position.y, s.z - position.z)
		return dv.length()


	def facing(self, entity):
		"""
		Returns the facing angle relative to the specified entity
		@param entity: Entity to check against
		@return: angle (rad)
		"""
		pos = self.position
		epos = entity.position
		edir = Atrea.Vector3(epos.x - pos.x, 0, epos.z - pos.z)
		if edir.length() == 0:
			return 0
		zdir = Atrea.Vector3(0, 0, 1)
		edot = (edir.x * zdir.x + edir.y * zdir.y + edir.z * zdir.z)
		eang = math.acos(edot / (edir.length() * zdir.length()))
		dangle = self.rotation.y - math.copysign(eang, edir.x)
		if dangle < 0:
			dangle += 2 * math.pi
		return dangle


	def facingType(self, entity):
		"""
		Returns the facing type relative to the specified entity
		@param entity: Entity to check against
		@return: Facing type (see ECASPosition enumeration)
		"""
		pdist = math.sqrt((entity.position.x - self.position.x)**2 + (entity.position.z - self.position.z)**2)
		ydist = entity.position.y - self.position.y
		if pdist < Constants.CAS_CLOSE_DISTANCE:
			# Check if we're above/below the entity by checking if the height difference
			# is larger than the comparison height
			if ydist > Constants.CAS_ENTITY_HEIGHT:
				return Atrea.enums.CAS_POSITION_ABOVE
			if ydist < -Constants.CAS_ENTITY_HEIGHT:
				return Atrea.enums.CAS_POSITION_BELOW
		elif abs(ydist / pdist) > Constants.CAS_ABOVE_TAN:
			if ydist > 0:
				return Atrea.enums.CAS_POSITION_ABOVE
			else:
				return Atrea.enums.CAS_POSITION_BELOW


		facing = self.facing(entity)
		if facing < Constants.CAS_FRONT_FACING or facing > 2 * math.pi - Constants.CAS_FRONT_FACING:
			return Atrea.enums.CAS_POSITION_FRONT
		elif math.pi - Constants.CAS_REAR_FACING < facing < math.pi + Constants.CAS_REAR_FACING:
			return Atrea.enums.CAS_POSITION_REAR
		else:
			return Atrea.enums.CAS_POSITION_FLANK


	def getEventSet(self) -> EventSet:
		"""
		Returns the kismet event set of this entity
		@return: Kismet event set
		"""
		if self.kismetEventSetId:
			return DefMgr.get('event_set', self.kismetEventSetId)
		else:
			return None


	def getSequence(self, eventId : int) -> Sequence:
		"""
		Returns the kismet event sequence for the specified event.
		@return: Kismet event sequence or None, if no sequence was found
		"""
		eventSet = self.getEventSet()
		if eventSet:
			return eventSet.getSequence(eventId)
		else:
			return None


	def getInteractions(self, player):
		"""
		Returns the list of available interaction set maps for the player
		@param player: Player that is interacting with us
		@return: Interaction set maps
		"""
		maps = {}

		if self.template is not None:
			if self.template.interactionSet is not None:
				builtins = self.template.interactionSet.interactions.items()
				maps = {setMapId: setMap for setMapId, setMap in builtins if setMap.visible(player)}

			# Check if the player has any additional interactions for us
			# TODO: Move this to SGWPlayer!
			playerMaps = player.availableInteractions.get(self.template.id)
			if playerMaps is not None:
				for setMap, missionId in playerMaps.values():
					if setMap.visible(player):
						maps[setMap.id] = setMap

		return maps


	def onInitialResponse(self, player, interactionSetMapId):
		"""
		Called when the player selected an option from the list of available topics.
		@param player: SGWPlayer entity
		@param interactionSetMapId:
		"""
		interactions = self.getInteractions(player)
		if interactionSetMapId not in interactions:
			warn(Atrea.enums.LC_Interact, '<%s> has no interaction set map %d!' % (self.getName(), interactionSetMapId))
			return

		interactions[interactionSetMapId].interact(player, self)


	def onInteract(self, player):
		"""
		Called when the player wants to interact with this entity
		@param player: SGWPlayer entity
		"""
		interactions = self.getInteractions(player)
		if len(interactions) == 0:
			player.debug("This person has nothing to say to you.")
			return

		# If there is only one topic available, don't show the initial interaction window
		if len(interactions) == 1:
			setMapId, setMap = next (iter (interactions.items()))
			setMap.interact(player, self)
		else:
			# Build a list of available topics
			topics = []
			for interactionSetMapId in interactions:
				topics.append({
					'dialogSetMapID': interactionSetMapId,
					'level': 0,
					'missionId': 0
				})
			player.client.onInitialInteraction(topics)


	def onLootItem(self, player, index):
		"""
		Called when the player wants to loot an item from this entity
		@param player: SGWPlayer entity
		@param index: Index of the item the player wants to loot
		"""
		if self.lootHandler is not None:
			self.lootHandler.onLootItem(player, index)
		else:
			player.debug("onLootItem: Entity %d doesn't have any loot handlers!" % self.entityId)


