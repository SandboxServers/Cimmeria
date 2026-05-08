from importlib import import_module
import Atrea
import Atrea.enums
import random
import weakref
from cell.Crafter import Crafter
from cell.Global import PlayersByName
from cell.SGWMob import SGWMob
from cell.Bag import Bag
from cell.Item import Item
from cell.MissionManager import MissionManager
from cell.ConsoleCommands import Command
from cell.Trade import TradeTransaction
from common import Constants, Logger
from common.Config import Config
from common.Logger import warn, trace, info
from common.defs.Def import DefMgr
from cell.SGWBeing import SGWBeing, mustBeAlive, mustBeIdle
from cell.Inventory import Inventory

def reloadScript(playerName, module, scriptType):
	player = PlayersByName.get(playerName)
	if player and player():
		return player().reloadScript(module, scriptType)
	else:
		return False


class SGWPlayer(SGWBeing):
	posX, posY, posZ = 0.0, 0.0, 0.0
	heading = 0
	playerName = ''
	extraName = ''
	exp = 0
	clientConnected = False
	reloadOnActivate = 0
	primaryColorId = 0xFF
	secondaryColorId = 0xFF
	skinTint = 0xFF

	# Has the game client finished loading?
	clientLoaded = False

	disciplines = []

	# World ID to save when the entity is persisted after leaving a space
	savedWorldId = 0
	# World name to save when the entity is persisted after leaving a space
	savedWorldName = ''

	# Destination gate we're dialing
	dialedAddress = None
	dialingStargate = None
	gateDialTimer = None
	gatePassable = False
	# Is the player moving to another space?
	isInTransit = False

	# Entity we're currently looting
	lootingEntity = None
	# Vendor we're currently interacting with
	vendorEntity = None
	# Ability trainer we're currently interacting with
	trainerEntity = None
	# Minigame we're currently playing
	minigame = None
	# Has the minigame session started yet?
	minigameStarted = False

	# Script instance of this space
	levelScript = None
	# Is this the first time the client sets a waiting state flag?
	firstLoad = True

	# Ring transport interaction status
	ringSourceId = None
	destinationRingId = False

	# ID of next auto-assigned inventory event handler
	nextInventoryHandlerId = 1

	# List of respawners that we have learned so far
	knownRespawnerIds = []

	# List of mobs we're threatening
	threatenedMobs = []

	# Automatically reload weapon when clip is empty
	autoReload = False
	# Automatically reload weapon when its bandolier slot is activated
	autoReloadOnActivate = False
	# Active bandolier slot ID
	# !!! Do NOT use this value as this is only updated when the player is persisted.
	# !!! Use bag.getActiveItem() instead
	bandolierSlotId = 0

	# Amount of ability training points available
	trainingPoints = 0

	# Last entity the player has interacted with
	lastInteractionTarget = 0

	# List of interactions available to the player
	# templateId -> dialogSetMap maps
	availableInteractions = None

	# Id of last active mission reward choice dialog
	lastRewardsMissionId = None
	# (dialogId, missionId) tuple of last offered mission
	offeredMission = None

	# Alternative callback to use when a dynamic update is requested
	dynamicUpdateCallback = None

	# Were we loaded from a backup created by another CellApp?
	loadedFromBackup = False

	# Was mission script loading deferred until we enter a space?
	scriptLoadPending = False


	def __init__(self):
		super().__init__()
		self.cacheable = Atrea.config.cache_player_messages == "true"
		self.inventory = Inventory(self, 0)
		self.missions = MissionManager(self)
		self.crafting = Crafter(self)
		self.disciplines = []
		self.abilityTimers = {}
		self.vendorBuyback = {}
		self.inventoryHandlers = {}
		self.threatenedMobs = []
		self.availableInteractions = {}
		self.displayedDialogs = {}
		self.knownRespawnerIds = []
		self.tradeTransaction = None


	def destroyed(self):
		"""
		Called before CellApp destroys and persists the entity
		"""
		pass


	def load(self):
		"""
		Called when the CellApp wants to load the entity from the database
		"""
		player = Atrea.dbQuery("select * from sgw_player where player_id = " + str(self.databaseId))[0]
		self.accountId = player['account_id']
		self.playerId = player['player_id']
		self.level = player['level']
		self.alignment = player['alignment']
		self.archetype = player['archetype']
		self.gender = player['gender']
		self.playerName = player['player_name']
		self.extraName = player['extra_name']
		self.title = player['title']
		self.posX = player['pos_x']
		self.posY = player['pos_y']
		self.posZ = player['pos_z']
		self.heading = player['heading']
		self.inventory.naquadah = player['naquadah']
		self.exp = player['exp']
		self.bodySet = player['bodyset']
		self.knownStargates = [int(sg) for sg in player['known_stargates'][1:-1].split(',') if sg]
		self.knownRespawnerIds = [int(resp) for resp in player['known_respawners'][1:-1].split(',') if resp]
		self.hiddenStargates = []
		self.components = [comp for comp in player['components'][1:-1].split(',') if comp]
		self.abilities.add([int(ability) for ability in player['abilities'][1:-1].split(',') if ability])
		self.firstLogin = player['first_login']
		self.accessLevel = player['access_level']
		if Config.DEVELOPER_MODE:
			self.accessLevel = 65535
		self.skinColorId = player['skin_color_id']
		self.bandolierSlotId = player['bandolier_slot']
		if player['interaction_maps'] is not None:
			dialogs = player['interaction_maps'][1:-1].split(',')
			# Kludgy way to restore set maps from the table
			# Arrays of composite types are stored in the form {(a,b,c),(a,b,c),...}
			# We need to strip off the array delimiter chars {}, and the record delimiters chars ()
			if len(dialogs) > 1:
				for i in range(0, len(dialogs), 3):
					tplId = int(dialogs[i][2:])
					setMap = DefMgr.get('interaction_set_map', int(dialogs[i + 1]))
					if setMap is not None:
						if tplId not in self.availableInteractions:
							self.availableInteractions[tplId] = {setMap.id: (setMap, int(dialogs[i + 2][:-2]) or None)}
						else:
							self.availableInteractions[tplId][setMap.id] = (setMap, int(dialogs[i + 2][:-2]) or None)
					else:
						warn(Atrea.enums.LC_Interact, '<%s> has invalid interaction set map %d. Removed.' %
													  (self.getName(), int(dialogs[i + 1])))
		# Bandolier active slot needs to be updated here as the inventory load populates the
		# players component list with items in the active container slots
		bandolier = self.inventory.getBag(Atrea.enums.INV_Bandolier)
		bandolier.setActiveSlot(self.bandolierSlotId)
		self.trainingPoints = player['training_points']
		self.crafting.appliedSciencePoints = player['applied_science_points']
		self.inventory.loadItems()
		self.missions.load()
		self.scriptLoadPending = True
		return True


	def persist(self):
		"""
		Called when the CellApp wants to save this entity
		"""
		# Don't update position if client is not yet connected
		# as the internal entity position will be incorrect
		if self.clientConnected and not self.isInTransit:
			# Update persisted bandolier slot
			self.bandolierSlotId = self.inventory.getBag(Atrea.enums.INV_Bandolier).activeSlotId

			# Update DB entity position so it gets persisted
			loc = self.position
			self.posX = loc.x
			self.posY = loc.y
			self.posZ = loc.z
			rot = self.rotation

			if self.space is None:
				worldName = self.savedWorldName
				worldId = self.savedWorldId
			else:
				worldName = self.space.worldName
				worldId = self.space.worldId

			# Save dialog sets in array of composites form -- {(templateId,setId,missionId),...}
			sets = ['"(' + ','.join((str(templateId), str(setObj.id), str(missionId or 0))) + ')"' for (templateId, ints) in self.availableInteractions.items() for setObj, missionId in ints.values()]
			dialogs = '{' + ','.join(sets) + '}'

			Atrea.dbQuery(
				"""update sgw_player set pos_x = %f, pos_y = %f, pos_z = %f,
				world_location = '%s', world_id = %d, first_login = %d,
				heading = %d, bandolier_slot = %d, interaction_maps = '%s',
				training_points = %d, applied_science_points = %d, level = %d,
				exp = %d, naquadah = %d, known_stargates = '%s', abilities = '%s',
				known_respawners = '%s'
				where player_id = %d"""
				 % (loc.x, loc.y, loc.z, repr(worldName)[1:-1], worldId, self.firstLogin, rot.y, self.bandolierSlotId,
					dialogs, self.trainingPoints, self.crafting.appliedSciencePoints, self.level, self.exp,
					self.inventory.naquadah, '{' + ','.join([str(sg) for sg in self.knownStargates]) + '}',
					'{' + ','.join([str(ability) for ability in self.abilities.abilities]) + '}',
					'{' + ','.join([str(resp) for resp in self.knownRespawnerIds]) + '}',
					self.databaseId))
			self.inventory.saveItems()
			self.missions.save()


	def connected(self):
		loc = Atrea.Vector3(self.posX, self.posY, self.posZ)
		dir = Atrea.Vector3(0, self.heading, 0)
		self.position = loc
		self.rotation = dir
		# We need to swap Y and Z for onClientMapLoad as the BW coordinate system differs 
		# from the one used on the server
		dir.y = 0
		dir.z = self.heading
		self.clientConnected = True
		self.client.onClientMapLoad(self.space.worldName, self.space.world.clientMap, self.space.worldId, loc, dir)
		PlayersByName[self.playerName] = weakref.ref(self)


	def disconnected(self):
		del PlayersByName[self.playerName]


	def backup(self):
		"""
		Called when the state of the entity is being saved
		(backup to BaseApp, moving to a different space, ...)
		"""
		# Update persisted bandolier slot
		self.bandolierSlotId = self.inventory.getBag(Atrea.enums.INV_Bandolier).activeSlotId

		data = super().backup()
		data['player'] = (self.accountId, self.playerId, self.level, self.alignment, 
			self.archetype, self.gender, self.playerName, self.extraName,
			self.title, self.heading, self.inventory.naquadah, self.exp,
			self.bodySet, self.knownStargates, self.hiddenStargates, self.components,
			self.firstLogin, self.accessLevel, self.skinColorId, self.destinationRingId,
			self.bandolierSlotId, self.trainingPoints)
		data['abilities'] = self.abilities.backup()
		data['inventory'] = self.inventory.backup()
		data['missions'] = self.missions.backup()
		data['craft'] = self.crafting.backup()
		return data


	def restore(self, backup):
		"""
		Called when the state of the entity is being restored from the BaseApp
		"""
		super().restore(backup)
		self.accountId, self.playerId, self.level, self.alignment, self.archetype, \
			self.gender, self.playerName, self.extraName, self.title, \
			self.heading, self.inventory.naquadah, self.exp, self.bodySet, \
			self.knownStargates, self.hiddenStargates, self.components, self.firstLogin, \
			self.accessLevel, self.skinColorId, self.destinationRingId, \
			self.bandolierSlotId, self.trainingPoints = \
			backup['player']
		
		# We need to get an up-to-date position for the player from the restored entity
		# (as the restored position points to the players spawn point on the old space)
		loc = self.position
		self.posX, self.posY, self.posZ = loc.x, loc.y, loc.z
		
		self.abilities.restore(backup['abilities'])
		self.inventory.restore(backup['inventory'])
		self.missions.restore(backup['missions'])
		self.crafting.restore(backup['craft'])
		self.scriptLoadPending = True
		self.loadedFromBackup = True


	def enteredSpace(self, spaceId):
		"""
		Called when the entity fully entered the specified space
		@param spaceId: ID of space the entity entered
		"""
		super().enteredSpace(spaceId)
		if self.scriptLoadPending:
			self.missions.loadScripts()
			self.scriptLoadPending = False


	def leftSpace(self, spaceId):
		"""
		Called when the entity left the specified space
		(self.spaceId and local mailboxes are still available for the duration of the call)
		@param spaceId: ID of space the entity left
		"""
		self.savedWorldId = self.space.worldId
		self.savedWorldName = self.space.worldName
		self.destroyLevelScript()
		super().leftSpace(spaceId)


	def initLevelScript(self, persistentVars):
		"""
		Initializes the script instance of the level (if exists)
		"""
		world = self.space.world
		if world.hasScript and self.levelScript is None:
			mod = import_module('cell.spaces.' + world.world)
			cls = world.world.split('.')[-1]
			self.levelScript = getattr(mod, cls)(self, persistentVars)


	def destroyLevelScript(self):
		"""
		Destroys the level script instance
		"""
		if self.levelScript is not None:
			self.levelScript.teardown()
			self.levelScript = None


	def feedback(self, msg):
		"""
		Sends feedback about a player action to the client
		@param msg: Feedback message
		"""
		if not Config.DEVELOPER_MODE:
			trace(Atrea.enums.LC_Chat, '(%s) <-- %s' % (self.getName(), msg))
		if self.client is not None:
			self.client.onPlayerCommunication('', 0, Atrea.enums.CHAN_feedback, msg)


	def onDebug(self, msg):
		"""
		Debug event callback from the logger subsystem
		@param msg: Debug message
		"""
		if self.client is not None:
			self.client.onPlayerCommunication('', 0, Atrea.enums.CHAN_feedback, msg)

			
	def debug(self, msg):
		trace(Atrea.enums.LC_Uncategorized, msg)
		if self.client is not None and not Config.DEVELOPER_MODE:
			self.client.onPlayerCommunication('', 0, Atrea.enums.CHAN_feedback, msg)

	def reloadScript(self, module, scriptType):
		if scriptType == 'mission':
			for mission in self.missions.missions.values():
				if module == mission.mission.scriptName:
					self.debug('Reloaded mission script: %s' % module)
					mission.reloadScript()
					return True

			return False
		elif scriptType == 'level':
			if module == self.space.worldName and self.space.world.hasScript:
				self.debug('Reloaded level script: %s' % module)
				self.destroyLevelScript()
				self.initLevelScript([])
				return True
			else:
				warn(Atrea.enums.LC_Uncategorized, 'Cannot reload inactive level script: %s' % module)
				return False
		else:
			warn(Atrea.enums.LC_Uncategorized, 'Unknown script type to reload: %s' % scriptType)
			return False


	def onVisualsUpdated(self):
		"""
		Called by the inventory manager when the visual components of the player
		were updated because of an inventory change
		"""
		self.client.BeingAppearance(self.bodySet, self.components)
		self.witnesses.BeingAppearance(self.bodySet, self.components)


	def setupPlayer(self):
		"""
		First-time player initialization
		"""
		# TODO: Get base stats from table
		archetype = DefMgr.get('archetype', self.archetype)
		self.getStat(Atrea.enums.coordination).update(0, archetype.coordination, archetype.coordination)
		self.getStat(Atrea.enums.engagement).update(0, archetype.engagement, archetype.engagement)
		self.getStat(Atrea.enums.fortitude).update(0, archetype.fortitude, archetype.fortitude)
		self.getStat(Atrea.enums.morale).update(0, archetype.morale, archetype.morale)
		self.getStat(Atrea.enums.perception).update(0, archetype.perception, archetype.perception)
		self.getStat(Atrea.enums.intelligence).update(0, archetype.intelligence, archetype.intelligence)
		self.getStat(Atrea.enums.health).update(0, archetype.health, archetype.health)
		self.getStat(Atrea.enums.focus).update(0, archetype.focus, archetype.focus)
		self.getStat(Atrea.enums.kineticRes).update(0, 40, 2000)
		self.getStat(Atrea.enums.mentalRes).update(0, 20, 2000)
		self.getStat(Atrea.enums.healthRes).update(0, 30, 2000)
		# TODO: This should be updated depending on the active weapon
		self.getStat(Atrea.enums.deploymentBarAmmo).update(0, 0, 1)

		self.faction = 3
		self.kismetEventSetId = 1025
		self.beingName = self.playerName
		self.skinTint = Constants.SKIN_TINTS[self.skinColorId]

		bandolier = self.inventory.getBag(Atrea.enums.INV_Bandolier)
		fists = bandolier.getItem(0)
		if fists is not None:
			self.inventory.removeItem(fists.id, fists.quantity, True)
			warn(Atrea.enums.LC_Item, '<%s> Removed bogus item from first bandolier slot for player' % self.getName())

		fists = Item.create(2699)
		fists.id = -1
		fists.quantity = 1
		fists.characterId = self.playerId
		assert(bandolier.putItem(fists, 0))

		for slotId in range(0, bandolier.slots):
			item = bandolier.getItem(slotId)
			if item:
				self.getStat(Atrea.enums.ammoSlot1 + slotId).update(0, item.ammo, item.type.clipSize)
			else:
				self.getStat(Atrea.enums.ammoSlot1 + slotId).update(0, 0, 0)


	def mapLoaded(self):
		"""
		Called when the client loaded a new map and the entity system was reset.
		"""
		if not self.loadedFromBackup:
			self.setupPlayer()

		Atrea.createCellPlayer(self.entityId)

		world = self.space.world
		self.client.setupWorldParameters(
			world.id, 0, world.minToRealMin, world.minPerDay, 100000,
			world.gravity, world.runSpeed, world.sidewaysRunSpeed, world.backwardsRunSpeed,
			world.walkSpeed, world.sidewaysWalkSpeed, world.backwardsWalkSpeed,
		    world.crouchRunSpeed, world.sidewaysCrouchRunSpeed, world.backwardsCrouchRunSpeed,
		    world.crouchWalkSpeed, world.sidewaysCrouchWalkSpeed, world.backwardsCrouchWalkSpeed,
		    world.swimSpeed, world.sidewaysSwimSpeed, world.backwardsSwimSpeed, world.jumpSpeed)
		worldStargateIds = [sg.id for sg in world.stargates]
		self.client.setupStargateInfo(worldStargateIds, self.knownStargates, self.hiddenStargates)
		self.client.clearClientHintedGenericRegions()
		self.space.playerEntered(self)
		self.client.onTimeofDay(0.129059, 1.0, 1)

		self.client.onLevelUpdate(self.level)
		self.client.onBeingNameUpdate(self.beingName)
		self.client.onStateFieldUpdate(self.stateField)
		self.client.onKismetEventSetUpdate(self.kismetEventSetId)
		self.sendStats(self.client, False, False)
		self.client.onArchetypeUpdate(self.archetype)
		self.client.onAlignmentUpdate(self.alignment)
		self.client.onFactionUpdate(self.faction)

		archetype = DefMgr.get('archetype', self.archetype)
		self.client.onAbilityTreeInfo([
			[a.ability.id for a in archetype.abilityTrees[0]],
			[a.ability.id for a in archetype.abilityTrees[1]],
			[a.ability.id for a in archetype.abilityTrees[2]]
		])
		self.client.onKnownAbilitiesUpdate([id for id in self.abilities.abilities])
		self.client.onResetMapInfo()

		self.client.BeingAppearance(self.bodySet, self.components)
		self.client.onEntityTint(self.primaryColorId, self.secondaryColorId, self.skinTint)

		self.client.onExtraNameUpdate(self.extraName)
		self.client.onExpUpdate(self.exp)
		self.client.onMaxExpUpdate(Constants.LEVEL_EXP[self.level])

		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_AppliedSciencePoints, self.crafting.appliedSciencePoints)
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_TrainingPoints, self.trainingPoints)
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_AccessLevel, self.accessLevel)
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_Gender, self.gender)
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_PvPFlag, 0)
		self.inventory.flushUpdates()

		self.sendDirtyStats()
		bandolier = self.inventory.getBag(Atrea.enums.INV_Bandolier)
		activeItem = bandolier.getActiveItem()
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_AmmoTypeId, activeItem.ammoType if activeItem else 0)

		self.client.onUpdateKnownCrafts(self.crafting.blueprints)
		# TODO: Update this based on cell/environment?
		self.client.onUpdateCraftingOptions({
			'crafting': {'items': [], 'entities': []},
			'research': {'items': [], 'entities': []},
			'reverseEngineering': {'items': [], 'entities': []},
			'alloying': {'items': [], 'entities': []}
		})

		self.missions.resend()

		if self.firstLogin:
			self.client.onPlayMovie('Cine-SGWLogo.SGWLogo', 1)
			self.firstLogin = 0

		self.client.onPlayerDataLoaded()
		self.client.onTargetUpdate(self.targetId)
		self.client.onPlayerCommunication(self.playerName, 0, 9, 'Welcome to Stargate Worlds. Your player id is: ' + str(self.entityId) + '.')

		self.loadCell()
		self.client.onPlayerCommunication(self.playerName, 0, 9, 'Entity data sent.')
		self.base.onPlayerReady()
		self.clientLoaded = True

		# TODO: Load/save StoredVars!
		self.initLevelScript([])
		self.firstLoad = False

		if Config.DEVELOPER_MODE:
			for categoryId in Logger.logLevels:
				Logger.logLevels[categoryId] = Constants.LOG_TRACE
			Logger.LoggerCallback = lambda level,evt,msg: self.onDebug(msg)


	def onClientReady(self):
		if self.firstLoad:
			self.mapLoaded()
		else:
			# Needs to be sent to reset waiting state flags on the client
			# (fixes the "teleport after item use" bug)
			self.client.BeingAppearance(self.bodySet, self.components)
			self.client.onEntityTint(self.primaryColorId, self.secondaryColorId, self.skinTint)

		if self.destinationRingId:
			transporter = self.space.transporters.get(self.destinationRingId)
			transporter.playerLoaded(self)

		# Call post load callbacks and clear them
		self.fire("player.loaded")


	def createOnClient(self, mailbox):
		super().createOnClient(mailbox)
		# TODO: Temporarily removed as this updates the displayed name of each player on the client!
		# mailbox.onExtraNameUpdate(self.extraName)
		bandolier = self.inventory.getBag(Atrea.enums.INV_Bandolier)
		activeItem = bandolier.getItem(bandolier.activeSlotId)
		mailbox.onEntityProperty(Atrea.enums.GENERICPROPERTY_AmmoTypeId, activeItem.ammoType if activeItem else 0)


	def enteredAoI(self, entity):
		"""
		Called when an entity entered out AoI
		@param entity: Entering entity
		"""
		trace(Atrea.enums.LC_Uncategorized, "<%s> entered AoI of <%s>" % (entity.getName(), self.playerName))
		self.dynamicUpdate(entity)


	def teleportTo(self, position, yaw = None, worldName = None):
		"""
		Teleports the player to the specified location.
		This is similar to moveTo(), but sets the waiting state flag on the client so
		teleporting to chunks that are not yet loaded will show a loading screen.
		@param position: New position of player
		@param yaw: Rotation of player
		@param worldName: Destination world name
		"""
		if worldName is None or worldName == self.space.worldName:
			rot = Atrea.Vector3(0, 0, yaw or 0)
			self.client.onClientMapLoad(self.space.worldName, self.space.world.clientMap, self.space.worldId, position, rot)

		self.moveTo(position.x, position.y, position.z, yaw, worldName)


	def addStargateAddress(self, addressId, isHidden = False):
		"""
		Adds a stargate address to the players list of known stargates
		@param addressId: ID of address to add
		@param isHidden: Add the address to the hidden address list?
		"""
		# We need to remove the address from the known list if it is added to the hidden list and vice versa.
		if isHidden:
			if addressId in self.knownStargates:
				self.knownStargates.remove(addressId)
			if addressId not in self.hiddenStargates:
				self.hiddenStargates.append(addressId)
		else:
			if addressId in self.hiddenStargates:
				self.hiddenStargates.remove(addressId)
			if addressId not in self.knownStargates:
				self.knownStargates.append(addressId)
		self.client.updateStargateAddress(addressId, 1, 1 if isHidden else 0)


	def removeStargateAddress(self, addressId):
		"""
		Removes a stargate address from the players list of known stargate addresses
		@param addressId: ID of address to remove
		"""
		if addressId in self.hiddenStargates:
			self.hiddenStargates.remove(addressId)
		elif addressId in self.knownStargates:
			self.knownStargates.remove(addressId)
		self.client.updateStargateAddress(addressId, 0, 0)


	def addRespawner(self, respawnerId):
		"""
		Adds a respawner to the players list of known respawners
		@param respawnerId: ID of respawner to add
		"""
		if respawnerId not in self.knownRespawnerIds:
			self.knownRespawnerIds.append(respawnerId)


	def removeRespawner(self, respawnerId):
		"""
		Removes a respawner from the players list of known respawners
		@param respawnerId: ID of respawner to remove
		"""
		if respawnerId in self.knownRespawnerIds:
			self.knownRespawnerIds.remove(respawnerId)


	def requestDynamicUpdate(self, entity, callback = None):
		"""
		Requests the CellApp to perform a dynamic property update on the selected entity
		@param entity: Entering entity
		@param callback: Callback function to call
		"""
		self.dynamicUpdateCallback = callback
		self.dynamicUpdate(entity)


	def onDynamicUpdate(self, entity):
		"""
		Resends all dynamic entity properties for the specified entity.
		NOTE: For the duration of this call the witnesses mailbox of the entity
		only sends messages to us (the current entity).
		@param entity: Entity to update
		"""
		if self.dynamicUpdateCallback is not None:
			cb = self.dynamicUpdateCallback
			self.dynamicUpdateCallback = None
			cb(entity)
		elif entity.template is not None:
			trace(Atrea.enums.LC_Uncategorized, "Updating dynamic properties on <%s>" % entity.getName())
			intFlags = entity.interactionType
			dlgs = self.availableInteractions.get(entity.template.id)
			if dlgs is not None:
				for dialog, missionId in dlgs.values():
					intFlags |= dialog.flags
			for setMapId, dialogSetMap in entity.getInteractions(self).items():
				intFlags |= dialogSetMap.flags
			entity.witnesses.InteractionType(intFlags)


	def updateDynamicProperties(self, templateId):
		"""
		Resends all dynamic entity properties for the specified entity ID.
		@param templateId: Entity template ID to update
		"""
		entities = self.space.findEntitiesByDbid(templateId)
		for entity in entities:
			self.dynamicUpdate(entity)


	def displayDialog(self, target, dialogId, missionId = None, triggerEvents = True, interactionSetMapId = None):
		"""
		Displays a dialog to the client
		@param target: Entity the player was interacting with
		@param dialogId: Dialog to display
		@param missionId: Mission associated with the dialog
		@param triggerEvents: Trigger entity dialog events?
		@param interactionSetMapId: ID of the interaction set map which triggered this dialog
		"""
		if dialogId not in self.displayedDialogs:
			self.displayedDialogs[dialogId] = True

		if triggerEvents:
			# Allow scripts to handle the dialog event before we run the default dialog handler
			self.first('dialog.open::' + str(dialogId), player = self, target = target, dialogSetMapId = interactionSetMapId)

		self.client.onDialogDisplay(target.entityId if target else 0, dialogId, 0, 1, missionId or 0)


	def addDialog(self, templateId, interactionSetMap, missionId):
		"""
		Adds a new interaction option (via dialogs) with the specified entity type.
		@param templateId: Entity template ID that the player can interact with
		@param interactionSetMap: New dialog option
		@param missionId: Mission that added this option
		"""
		interactions = self.availableInteractions.get(templateId)
		if interactions and interactionSetMap.id in interactions:
			warn(Atrea.enums.LC_Interact, "Interaction set map %d for template %d already added to player <%s>" %
				(interactionSetMap.id, templateId, self.playerName))
			return False

		if interactions is None:
			self.availableInteractions[templateId] = {interactionSetMap.id: (interactionSetMap, missionId)}
		else:
			self.availableInteractions[templateId][interactionSetMap.id] = (interactionSetMap, missionId)
		self.updateDynamicProperties(templateId)
		return True


	def removeDialog(self, templateId, interactionSetMap):
		"""
		Removes a dialog interaction option with the specified entity type.
		@param templateId: Entity template ID that the player can interact with
		@param interactionSetMap: Dialog option to remove
		"""
		interactions = self.availableInteractions.get(templateId)
		if interactions and interactionSetMap.id in interactions:
			del interactions[interactionSetMap.id]
			self.updateDynamicProperties(templateId)
			return True

		warn(Atrea.enums.LC_Interact, "Cannot remove interaction set map %d for template %d from player <%s>" %
			(interactionSetMap.id, templateId, self.playerName))
		return False


	def onMissionRemoved(self, missionId):
		"""
		Called by the mission manager when the specified mission was removed from the players mission list
		(either completed, canceled or failed).
		@param missionId: Removed mission ID
		"""
		updatedTemplateIds = []
		# Remove all dialog set maps that are directly tied to this mission
		for templateId, dialogSetMaps in self.availableInteractions.items():
			invdKeys = []
			for mapId, dialog in dialogSetMaps.items():
				if dialog[1] == missionId:
					invdKeys.append(mapId)
			if len(invdKeys):
				updatedTemplateIds.append(templateId)
				for key in invdKeys:
					del dialogSetMaps[key]

		# Make sure that the dialog set updates are shown on entities
		for templateId in updatedTemplateIds:
			self.updateDynamicProperties(templateId)


	def giveExperience(self, exp):
		"""
		Gives experience points to the player
		@param exp: Amount of exp to give
		"""
		self.exp += exp
		self.client.onExpUpdate(self.exp)
		while self.exp > Constants.LEVEL_EXP[self.level] and self.level <= Constants.MAX_LEVEL:
			self.level += 1
			self.client.giveXPForLevel(self.level)
			self.witnesses.giveXPForLevel(self.level)
			self.client.onMaxExpUpdate(Constants.LEVEL_EXP[self.level])
			self.setLevel(self.level)
			self.feedback('Congratulations! You are now level %d.' % self.level)


	def giveTrainingPoints(self, points):
		"""
		Gives ability training points to the player
		@param points: Amount of points to give
		"""
		self.trainingPoints += points
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_TrainingPoints, self.trainingPoints)


	def consumeTrainingPoints(self, points):
		"""
		Use up ability training points of the player
		@param points: Amount of points to remove
		"""
		assert points <= self.trainingPoints
		self.trainingPoints -= points
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_TrainingPoints, self.trainingPoints)


	def onAppliedSciencePointsChanged(self, points):
		"""
		Called when the amount of applied science points the player has changed.
		@param points: New amount
		"""
		self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_AppliedSciencePoints, points)


	def onDisciplineUpdated(self, disciplineId, expertise):
		"""
		Called when the expertise level of a discipline has changed.
		@param disciplineId: Discipline ID
		@param expertise: New expertise level
		"""
		self.client.onUpdateDiscipline(disciplineId, expertise)


	def onBlueprintsUpdated(self, blueprintIds):
		"""
		Called when the list of crafts the player knows has changed.
		@param blueprintIds: Blueprint IDs
		"""
		self.client.onUpdateKnownCrafts(blueprintIds)


	def onRacialParadigmUpdated(self, racialParadigmId, level):
		"""
		Called when a racial paradigm level was updated
		@param racialParadigmId: Racial paradigm
		@param level: New level
		"""
		self.client.onUpdateRacialParadigmLevel(racialParadigmId, level)


	def onCraftingStarted(self, blueprintId, duration):
		"""
		Called when a racial paradigm level was updated
		@param blueprintId: Blueprint that's being crafted
		@param duration: Duration of the action
		"""
		self.client.onTimerUpdate(blueprintId, Atrea.enums.CraftInductionTimer,
								  self.entityId, 0, duration, Atrea.getGameTime() + duration)


	def addAbility(self, abilityId):
		"""
		Adds the ability (or abilities) to the players list of known abilities
		@param abilityId: ID of ability/abilities to add
		"""
		self.abilities.add(abilityId)
		self.client.onKnownAbilitiesUpdate([id for id in self.abilities.abilities])


	def removeAbility(self, abilityId):
		"""
		Removes the ability (or abilities) from the players list of known abilities
		@param abilityId: ID of ability/abilities to remove
		"""
		self.abilities.remove(abilityId)
		self.client.onKnownAbilitiesUpdate([id for id in self.abilities.abilities])


	def onError(self, msg):
		"""
		Notifies the client that a requested action failed
		@param msg: Error message
		"""
		self.client.onPlayerCommunication('', 0, Atrea.enums.CHAN_server, msg)


	def onItemEquipped(self, bag : Bag, slotId, item : Item):
		"""
		An item was moved to an equipment slot in the players inventory
		@param bag: Equipment bag
		@param slotId: Changed slot ID
		@param item: Equipped item
		"""
		if Atrea.enums.INV_Bandolier <= bag.bagId <= Atrea.enums.INV_Artifact2 and slotId > 0 and item.type.visualComponent is None:
			warn(Atrea.enums.LC_Item, 'Item %d in ragdoll bag %d has no visual component!' % (item.typeId, bag.bagId))

		if bag.bagId == Atrea.enums.INV_Bandolier:
			self.getStat(Atrea.enums.ammoSlot1 + item.slotId).update(0, item.ammo, item.type.clipSize)
			# Make sure that we only send stat updates when the player has already loaded
			if self.witnesses is not None:
				self.sendDirtyStats()
				self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_AmmoTypeId, item.ammoType)
				self.witnesses.onEntityProperty(Atrea.enums.GENERICPROPERTY_AmmoTypeId, item.ammoType)
				equipSeq = self.getItemSequence(Atrea.enums.Item_Equip)
				if equipSeq is not None:
					self.playSequence(equipSeq.seqId, self.entityId)
		self.first('item.equip::' + str(item.typeId), player = self, bagId = bag.bagId, slotId = slotId, quantity = item.quantity)


	def onItemUnequipped(self, bag : Bag, slotId, item : Item):
		"""
		An item was removed from an equipment slot in the players inventory
		@param bag: Equipment bag
		@param slotId: Changed slot ID
		@param item: Unequipped item
		"""
		if bag.bagId == Atrea.enums.INV_Bandolier:
			item.ammo = self.getStat(Atrea.enums.ammoSlot1 + item.slotId).cur
			unequipSeq = self.getItemSequence(Atrea.enums.Item_Unequip)
			if unequipSeq is not None:
				self.playSequence(unequipSeq.seqId, self.entityId)
		self.first('item.unequip::' + str(item.typeId), player = self, bagId = bag.bagId, slotId = slotId, quantity = item.quantity)


	def onTeleportIn(self, regionId):
		"""
		Event notification handler for teleportation (teleport::in event)
		@param regionId: Destination region ID
		@return: True, if the event was handled, False otherwise
		"""
		return self.first('teleport::in', player = self, regionId = regionId)


	def onTeleportOut(self, regionId, destinationId):
		"""
		Event notification handler for teleportation (teleport::in event)
		@param regionId: Source region ID
		@param destinationId: Destination region ID
		@return: True, if the event was handled, False otherwise
		"""
		return self.first('teleport::out', player = self, regionId = regionId, destinationRegionId = destinationId)


	def onAddedToThreatList(self, entityId):
		"""
		Called by a mob when combatant is added to their threat list
		@param entityId: Mob that has added this combatant to their threat list
		"""
		if entityId not in self.threatenedMobs:
			if len(self.threatenedMobs) == 0:
				self.setStateFlag(Atrea.enums.BSF_InCombat)
			self.threatenedMobs.append(entityId)
			self.client.onThreatenedMobsUpdate(entityId, 1)


	def onRemovedFromThreatList(self, entityId):
		"""
		Called by a mob when combatant is removed from their threat list
		@param entityId: Mob that has removed this combatant from their threat list
		"""
		if entityId in self.threatenedMobs:
			self.threatenedMobs.remove(entityId)
			if len(self.threatenedMobs) == 0:
				self.unsetStateFlag(Atrea.enums.BSF_InCombat)
			self.client.onThreatenedMobsUpdate(entityId, 0)


	def setLooting(self, entity):
		if entity is None:
			self.lootingEntity = None
		else:
			self.lootingEntity = weakref.ref(entity)


	def setVendor(self, entity):
		"""
		Updates the vendor the player is currently interacting with
		@param entity: Vendor entity
		"""
		if entity is None:
			self.vendorEntity = None
		else:
			self.vendorEntity = weakref.ref(entity)


	def setTrainer(self, entity):
		"""
		Updates the ability trainer the player is currently interacting with
		@param entity: Trainer entity
		"""
		if entity is None:
			self.trainerEntity = None
		else:
			self.trainerEntity = weakref.ref(entity)


	def getAggressionLevel(self, target):
		"""
		Returns the players aggression level against the target
		@param target:
		@return: Aggression level (see EMobAggressionLevel enum)
		"""
		if isinstance(target, SGWMob) and target.aggressionOverride is not None:
			aggression = target.aggressionOverride
		elif isinstance(target, SGWBeing):
			aggression = Atrea.enums.FACTION_REACTION_TABLE[self.faction][target.faction]
		else:
			aggression = Atrea.enums.AGGRESSION_NEUTRAL
		return aggression


	def onAbilityFinished(self):
		"""
		The entity has finished using an ability.
		"""
		super().onAbilityFinished()
		# getClipSize() is needed to make sure that we don't infinitely reload weapons
		# that have no ammo (staves, zat, fists, ...)
		if self.autoReload and self.getAmmoCount() == 0 and self.getClipSize() > 0:
			self.requestReload(Atrea.enums.RELOAD_ActiveWeapon)


	def getAmmoStat(self):
		"""
		Returns the statId of the ammo slot for the currently equipped weapon.
		@return: Stat ID (se EStats enumeration)
		"""
		return Atrea.enums.ammoSlot1 + self.inventory.getBag(Atrea.enums.INV_Bandolier).activeSlotId


	def getClipSize(self):
		"""
		Returns the clip size the currently equipped weapon.
		@return Clip size of weapon
		"""
		item = self.inventory.getBag(Atrea.enums.INV_Bandolier).getActiveItem()
		return item.type.clipSize if item else 0


	def getAmmoCount(self):
		"""
		Returns the amount of ammo in the currently equipped weapon.
		@return # of ammo
		"""
		return self.getStat(self.getAmmoStat()).cur


	def hasItemMoniker(self, monikerIds):
		"""
		Checks if the currently equipped weapon has at least one of the specified monikers
		@param monikerIds: List of moniker ID-s to check
		@return bool
		"""
		ids = self.inventory.getBag(Atrea.enums.INV_Bandolier).getActiveItem().type.monikerIds
		for monikerId in ids:
			if monikerId in monikerIds:
				return True

		return False


	def getWeaponRange(self, ranged):
		"""
		Returns the minimum and maximum range of the currently equipped weapon.
		@param ranged: Return ranged or melee ranges?
		@return (min, max) ranges tuple
		"""
		weapon = self.inventory.getBag(Atrea.enums.INV_Bandolier).getActiveItem().type
		if ranged:
			return weapon.minRangedRange, weapon.maxRangedRange
		else:
			return weapon.minMeleeRange, weapon.maxMeleeRange


	def consumeAmmo(self, amount):
		"""
		Consumes the specified amount of ammo from the currently equipped weapon.
		@param amount: Amount to consume
		"""
		self.getStat(self.getAmmoStat()).change(-amount)
		self.sendDirtyStats()


	def stoppedAutoCycling(self):
		"""
		Called by the ability manager when it stopped auto cycling an ability
		"""
		self.unsetStateFlag(Atrea.enums.BSF_AutoCycling)


	def makeMissionRewards(self, mission):
		"""
		Creates a list of reward item choices for the client.
		@param mission: Return rewards for this mission
		"""
		rewards = {
			'XP': mission.rewardXp,
			'Naquadah': mission.rewardNaq,
			'ItemGroups': []
		}
		for group in mission.rewards.values():
			items = []
			for i in range(0, len(group.items)):
				items.append({'ItemId': group.items[i].id, 'Index': i})
			rewards['ItemGroups'].append({
				'GroupId': group.id,
				'NumChoices': group.choices,
				'Items': items
			})
		return rewards


	def displayMissionRewards(self, mission):
		"""
		Called by the mission manager when it wants to display a reward selection screen on the client.
		@param mission: Display rewards for this mission
		"""
		self.lastRewardsMissionId = mission.missionId

		# The client doesn't display the reward selection screen if there are no rewards,
		# automatically continue as if no rewards were chosen
		if len(mission.rewards) == 0:
			self.chosenRewards({'GroupChoices': []}, mission.missionId)
			return

		rewards = self.makeMissionRewards(mission)
		self.client.onMissionRewardsDisplay(rewards, mission.missionId)


	def offerMission(self, mission, dialogId):
		"""
		Called by the mission manager when it wants to offer a mission to the client.
		@param mission: Offer for this mission
		@param dialogId: DialogId assigned to this interaction
		@warning: !!!!! DO NOT USE !!!!! This is not implemented in the client :(
		"""
		self.offeredMission = (dialogId, mission.missionId)

		rewards = self.makeMissionRewards(mission)
		self.client.onMissionOfferDisplay(dialogId, rewards, mission.missionId)


	###################################################################################
	#                            SGWPlayer RPC methods
	###################################################################################

	@mustBeAlive
	def interact(self, entityId):
		"""
		Handles player interaction with an entity.
		Handlers are checked in the following order:
		 1) Combat abilities if target is aggressive,
		 2) Subscribed handlers by spawn tag (entity.interact.tag)
		 3) Subscribed handlers by template name (entity.interact.template)
		 4) Entity interaction set map handler
		@param entityId: Entity to interact with
		"""
		target = Atrea.findEntity(entityId)
		if target is None:
			self.onError('Unable to find the entity you want to interact with (%d)' % entityId)
			return

		self.lastInteractionTarget = entityId
		aggression = self.getAggressionLevel(target)
		trace(Atrea.enums.LC_Interact, "<%s> Aggression of target: %d" % (self.getName(), aggression))
		if aggression < Atrea.enums.AGGRESSION_NEUTRAL and not target.isDead():
			self.debug("Using item combat ability on entity %d" % entityId)
			item = self.inventory.getBag(Atrea.enums.INV_Bandolier).getActiveItem()
			melee = item.type.events.get(Atrea.enums.EVENT_ItemMelee)
			ranged = item.type.events.get(Atrea.enums.EVENT_ItemRanged)
			ability = ranged if ranged else melee
			if ability:
				# TODO #2: Use the same auto cycling logic for other abilities
				# (launchAbility(autoCycling=...) should be an override flag -- allowAutoCycling, default False?)
				self.setTargetID(target.entityId)
				self.setStateFlag(Atrea.enums.BSF_AutoCycling)
				autoCycle = self.hasStateFlag(Atrea.enums.BSF_AutoCycling) and not (ability.flags & Atrea.enums.DoNotActivate_AutoCycle)
				status = self.abilities.launchAbility(ability, targetId = self.targetId, autoCycle = autoCycle, isEntityAbility = False)
			else:
				status = Atrea.enums.CONDITION_FEEDBACK_EntityDoesNotHaveAbility

			if status is not True:
				self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, 0, status)
			return

		if target.distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.feedback("You are too far away to interact with that")
			return

		trace(Atrea.enums.LC_Interact, "<%s> Tag of target: %s" % (self.getName(), target.tag))
		if target.tag is not None and self.first('entity.interact.tag::' + target.tag, player = self,
			target = target, aggression = aggression):
			self.debug('Executed tag interaction handler: %s' % target.tag)
			return

		if target.template is not None:
			if self.first('entity.interact.template::' + target.template.templateName,
				player = self, target = target, aggression = aggression):
				self.debug('Executed template interaction handler: %s' % target.template.templateName)
				return

		trace(Atrea.enums.LC_Interact, '<%s> Running default interaction handler for %d' % (self.getName(), entityId))
		target.onInteract(self)


	def callForAid(self, respawnerId):
		# TODO: Check if the player is allowed to call for aid (ie. admin unstuck pending / is dead / ...)
		if respawnerId not in self.knownRespawnerIds:
			warn(Atrea.enums.LC_Interact, 'Cannot respawn at respawner %d that is not known to the player' % respawnerId)
			return

		respawner = DefMgr.get('respawner', respawnerId)
		if respawner is None or respawner.world.id != self.space.world.id:
			warn(Atrea.enums.LC_Interact, 'Illegal respawner %d for world %d' % (respawnerId, self.space.world.id))
			return

		self.client.onEndAidWait()
		self.moveTo(respawner.x, respawner.y, respawner.z)
		# TODO: Reset health, NPC relations, combat state, other stuff ...


	@mustBeAlive
	def useAbility(self, abilityId, targetId):
		"""
		Called when the player launches an ability
		@param abilityId: ID of ability to launch
		@param targetId: Targeted entity (0 = no target)
		"""
		status = self.abilities.useAbility(abilityId, targetId)
		if not status:
			self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, abilityId, status)


	@mustBeAlive
	def useAbilityOnGroundTarget(self, abilityId, locationX, locationY, locationZ):
		"""
		Called when the player launches an ability on a ground target
		@param abilityId: ID of ability to launch
		@param locationX: X coordinate of target location
		@param locationY: Y coordinate of target location
		@param locationZ: Z coordinate of target location
		"""
		location = Atrea.Vector3(locationX, locationY, locationZ)
		status = self.abilities.useAbility(abilityId, 0, location)
		if not status:
			self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, abilityId, status)


	def respawn(self):
		trace(Atrea.enums.LC_Interact, 'SGWPlayer::Cell::respawn')


	def getActiveRespawners(self):
		"""
		Returns the list of respawners thet the player can use
		(respawners that are in the current world and are known by the player)
		"""
		respawners = []
		for respawnerId in self.knownRespawnerIds:
			respawner = DefMgr.get('respawner', respawnerId)
			if respawner is not None and respawner.world.id == self.space.world.id:
				respawners.append(respawner)

		return respawners


	@mustBeAlive
	def unstuck(self):
		"""
		Unstuck the player character
		(currently teleports the player to a respawner)
		"""
		respawners = self.getActiveRespawners()
		respawnerList = []
		for resp in respawners:
			respawnerList.append({'respawnerID': resp.id, 'respawnerName': resp.name})
		
		self.client.onBeginAidWait(100, respawnerList)


	@mustBeAlive
	def resetMyAbilities(self):
		"""
		Called when the client wants to respec
		(untrain / reset all its abilities)
		"""
		if self.trainerEntity is None or self.trainerEntity() is None:
			self.onError("Not interacting with a trainer entity")
			return

		if self.trainerEntity().entity().distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.onError("You are too far away to interact with that")
			return

		self.trainerEntity().onRespec(self)


	def who(self, name, archetype, alignment, playerType):
		trace(Atrea.enums.LC_Interact, 'SGWPlayer::Cell::who')


	@mustBeAlive
	def dialogButtonChoice(self, dialogId, buttonId):
		"""
		Called by the client when it has clicked a button in an open dialog.
		@param dialogId: Dialog the player interacted with
		@param buttonId: Button clicked
		"""
		if dialogId not in self.displayedDialogs:
			warn(Atrea.enums.LC_Interact, '<%s> Dialog choice for inactive dialog %d!' % (self.getName(), dialogId))
			return

		del self.displayedDialogs[dialogId]

		trace(Atrea.enums.LC_Interact, '<%s> dialogButtonChoice:  dialog %d, button %d' %
			 (self.getName(), dialogId, buttonId))
		if self.offeredMission is not None and self.offeredMission[0] == dialogId:
			self.missions.accept(self.offeredMission[1])
			self.offeredMission = None
			return

		dialog = DefMgr.get('dialog', dialogId)
		if dialog.missionId is not None:
			self.missions.accept(dialog.missionId)

		self.fire('dialog.choice::' + str(dialogId), player = self, buttonId = buttonId)


	@mustBeAlive
	def initialResponse(self, interactionSetMapId):
		# TODO: Check distance
		target = self.space.findEntity(self.lastInteractionTarget)
		if not target:
			warn(Atrea.enums.LC_Interact, 'Initial response for interaction set map %d with no target selected!' % interactionSetMapId)
			return

		interactionSetMaps = target.getInteractions(self)
		if interactionSetMapId not in interactionSetMaps:
			trace(Atrea.enums.LC_Interact, 'No interaction set maps available for target <%s>!' % target.getName())
			return

		interactionSetMap, missionId = interactionSetMaps[interactionSetMapId]
		interactionSetMap.interact(self, target)


	@mustBeAlive
	def trainAbility(self, abilityId):
		"""
		Called when the client wants to train an ability at the trainer
		entity it is currently interacting with
		@param abilityId: Ability to train
		"""
		if self.trainerEntity is None or self.trainerEntity() is None:
			self.onError("Not interacting with a trainer entity")
			return

		if self.trainerEntity().entity().distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.onError("You are too far away to interact with that")
			return

		self.trainerEntity().onTrainAbility(self, abilityId)


	@mustBeAlive
	def purchaseItems(self, itemIndices, quantities):
		"""
		Client RPC
		Called when the client wants to buy items from the vendor it is interacting with
		@param itemIndices: List of vendor item indices
		@param quantities: List of item amounts to buy
		"""
		if self.vendorEntity is None or self.vendorEntity() is None:
			self.onError("Not interacting with a vendor entity")
			return

		if self.vendorEntity().entity().distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.onError("You are too far away to interact with that")
			return

		items = [(itemIndices[i], quantities[i]) for i in range(0, len(itemIndices))]
		self.vendorEntity().onPurchaseItems(self, items)


	@mustBeAlive
	def sellItems(self, itemIds, quantities):
		"""
		Client RPC
		Called when the client wants to sell items to the vendor it is interacting with
		@param itemIds: List of client item IDs
		@param quantities: List of item amounts to buy
		"""
		if self.vendorEntity is None or self.vendorEntity() is None:
			self.onError("Not interacting with a vendor entity")
			return

		if self.vendorEntity().entity().distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.onError("You are too far away to interact with that")
			return

		items = [(itemIds[i], quantities[i]) for i in range(0, len(itemIds))]
		self.vendorEntity().onSellItems(self, items)


	@mustBeAlive
	def repairItems(self, itemIds):
		"""
		Client RPC
		Called when the client wants to repair items at the vendor it is interacting with
		@param itemIds: List of client item IDs to repair
		"""
		if self.vendorEntity is None or self.vendorEntity() is None:
			self.onError("Not interacting with a vendor entity")
			return

		if self.vendorEntity().entity().distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.onError("You are too far away to interact with that")
			return

		self.vendorEntity().onRepairItems(self, itemIds)


	@mustBeAlive
	def rechargeItems(self, itemIds):
		"""
		Client RPC
		Called when the client wants to recharge items at the vendor it is interacting with
		@param itemIds: List of client item IDs to recharge
		"""
		if self.vendorEntity is None or self.vendorEntity() is None:
			self.onError("Not interacting with a vendor entity")
			return

		if self.vendorEntity().entity().distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.onError("You are too far away to interact with that")
			return

		self.vendorEntity().onRechargeItems(self, itemIds)


	def setAutoCycle(self, enabled):
		"""
		Changes whether weapon fire is auto-cycling (automatically fires again after the cooldown expires)
		or only fires once per ability use.
		@param enabled: 1 = auto-cycling, 0 = one-shot
		@return:
		"""
		if enabled:
			self.setStateFlag(Atrea.enums.BSF_AutoCycling)
		else:
			self.unsetStateFlag(Atrea.enums.BSF_AutoCycling)
			self.abilities.autoCycle = False


	@mustBeAlive
	def lootItem(self, index):
		"""
		RPC called by the client when looting an item from an entity
		@param index: Server-assigned item index in the loot list
		"""
		if self.lootingEntity and self.lootingEntity():
			if self.lootingEntity().distanceTo(self.position) < Constants.MAX_INTERACT_DISTANCE:
				self.debug("Looting item index %d from %d" % (index, self.lootingEntity().entityId))
				self.lootingEntity().onLootItem(self, index)
			else:
				self.onError('You are too far away to loot that')
		else:
			self.onError("Failed to loot: No such entity")


	def triggerClientHintedGenericRegion(self, id, bEntering, position):
		"""
		Client RPC
		Called when the player entered or left a client hinted generic region.

		@param id: Generic region ID
		@param bEntering: Entering (1) or leaving (0)
		@param position: Position of player when entering region
		"""
		self.space.regions.triggerRegion(id, bEntering, position, self)


	@mustBeAlive
	def requestReload(self, reloadType):
		"""
		Reloads the currently equipped weapon.
		@param reloadType: See EReloadType enumeration
		"""
		if reloadType == Atrea.enums.RELOAD_ActiveWeapon:
			bandolier = self.inventory.getBag(Atrea.enums.INV_Bandolier)
			item = bandolier.getActiveItem()
			if item is not None and item.type.clipSize > 0:
				status = self.launchReloadAbility()
				if status is not True:
					self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, 0, status)
		elif reloadType == Atrea.enums.RELOAD_DeploymentBar:
			# TODO: Reload deployment bar
			pass


	@mustBeAlive
	def chosenRewards(self, rewardChoices, missionId):
		"""
		Called by the client when the player has selected the mission rewards from the initial rewards screen.
		@param rewardChoices: Choices for each item group
		@param missionId: Mission ID
		"""
		if missionId != self.lastRewardsMissionId:
			warn(Atrea.enums.LC_Mission, '<%s> Chosen rewards: mission %d not in interaction list!' % (self.getName(), missionId))
			return

		status = self.missions.getMissionStatus(missionId)
		if status != Constants.MISSION_Active:
			warn(Atrea.enums.LC_Mission, '<%s> Chosen rewards: mission %d is not active!' % (self.getName(), missionId))
			return

		mission = DefMgr.get('mission', missionId)
		rewards = []

		# Make sure that the player sent valid reward choices and build a list of reward items
		for group in rewardChoices['GroupChoices']:
			rewardGroup = mission.rewards.get(group['GroupId'])
			if rewardGroup is None:
				warn(Atrea.enums.LC_Mission, '<%s> Chosen rewards: mission %d has no reward group %d!' %
					 (self.getName(), missionId, group['GroupId']))
				return

			if len(group['Choices']) != rewardGroup.choices:
				warn(Atrea.enums.LC_Mission, '<%s> Chosen rewards: Invalid choice count %d for reward group %d!' %
					 (self.getName(), len(group['Choices']), group['GroupId']))
				return

			for index in group['Choices']:
				if index < 0 or index > len(rewardGroup.items):
					warn(Atrea.enums.LC_Mission, '<%s> Chosen rewards: Invalid item index %d for reward group %d!' %
						 (self.getName(), index, group['GroupId']))
					return

				rewards.append(rewardGroup.items[index])

		# Everything seems OK, give rewards and reset interaction data
		self.lastRewardsMissionId = None
		self.giveExperience(mission.rewardXp)
		self.inventory.addCash(mission.rewardNaq)
		trace(Atrea.enums.LC_Mission, '<%s> Awarded %d exp, %d naq for mission %d' % (self.getName(), mission.rewardXp, mission.rewardNaq, missionId))
		for design in rewards:
			self.inventory.pickedUpItem(design.id, 1)
			trace(Atrea.enums.LC_Mission, '<%s> Awarded %s for mission %d' % (self.getName(), design.name, missionId))
		self.inventory.flushUpdates()

		# Call the reward choice callback and complete the mission
		self.fire('mission.rewards_chosen::' + str(missionId), player = self)
		self.missions.complete(missionId)


	@mustBeAlive
	def petInvokeAbility(self, aEntityId, aAbilityId, aTargetId):
		pass


	@mustBeAlive
	def petAbilityToggle(self, aEntityId, aAbilityId, aToggle):
		pass


	@mustBeAlive
	def petChangeStance(self, aEntityId, aStance):
		pass


	@mustBeAlive
	def setRingTransporterDestination(self, regionId, destinationId):
		"""
		Client RPC
		Called when the client selected a destination region from the list of
		possible destination regions.
		@param regionId: Current region ID
		@param destinationId: Destination region ID
		"""
		if self.destinationRingId:
			warn(Atrea.enums.LC_Uncategorized, 'setRingTransporterDestination(): Already transporting!')
			return

		if self.ringSourceId is None:
			warn(Atrea.enums.LC_Uncategorized, 'setRingTransporterDestination(): No active ring interaction')
			return

		if self.ringSourceId != regionId:
			warn(Atrea.enums.LC_Uncategorized, 'Source region ID differs; known: %d, got: %d' % (self.ringSourceId, regionId))
			return

		region = self.space.transporters.get(regionId)
		region.selectDestination(self, destinationId)


	def onWorldInstanceReset(self):
		pass


	def updateSystemOptions(self, nameValuePairs):
		for option in nameValuePairs:
			if option['name'] == 'autoReload':
				self.autoReload = (option['value'] == '1')
			elif option['name'] == 'reloadOnActivate':
				self.autoReloadOnActivate = (option['value'] == '1')
			else:
				warn(Atrea.enums.LC_Uncategorized, 'Unknown option in updateSystemOptions(): %s' % option['name'])


	@mustBeAlive
	def onOrganizationCreation(self, aOrganizationName):
		pass


	@mustBeAlive
	def spendAppliedSciencePoints(self, disciplineId):
		trace(Atrea.enums.LC_Interact, "<%s> Learn discipline %d" % (self.getName(), disciplineId))
		self.crafting.spendAppliedSciencePoints(disciplineId)


	@mustBeIdle
	def craft(self, blueprintId, items, quantity):
		trace(Atrea.enums.LC_Interact, "<%s> Craft blueprint %d" % (self.getName(), blueprintId))
		self.crafting.craft(blueprintId, items, quantity)


	@mustBeIdle
	def research(self, itemId, kickerIds):
		trace(Atrea.enums.LC_Interact, "<%s> Research: %d" % (self.getName(), itemId))
		self.crafting.research(itemId, kickerIds)


	@mustBeIdle
	def reverseEngineer(self, itemId):
		trace(Atrea.enums.LC_Interact, "<%s> Reverse engineer: %d" % (self.getName(), itemId))
		self.crafting.reverseEngineer(itemId)


	@mustBeIdle
	def alloying(self, blueprintId, currentTierItemId, lowerTierItems):
		trace(Atrea.enums.LC_Interact, "<%s> Alloy: %d" % blueprintId)
		self.crafting.alloy(blueprintId, currentTierItemId, lowerTierItems)


	@mustBeAlive
	def respecCrafting(self):
		trace(Atrea.enums.LC_Interact, "respecCrafting")


	def onClientChallengeResponse(self, aClientChallenge, aClientVersion, aChallengeType, aChallengeObject, aChallengeID1, aChallengeID2, aChallengeValue):
		self.feedback("Challenge: %d" % aClientChallenge)
		self.feedback("Version: %s" % aClientVersion)
		self.feedback("Type: %d" % aChallengeType)
		self.feedback("Object: %s" % aChallengeObject)
		self.feedback("ID1: %d" % aChallengeID1)
		self.feedback("ID2: %d" % aChallengeID2)
		self.feedback("Value: %s" % aChallengeValue)


	@mustBeAlive
	def sendDuelResponse(self, aResponse):
		pass


	@mustBeAlive
	def duelForfeit(self):
		pass


	def isTrading(self):
		"""
		Checks if we're currently trading with a player
		"""
		return self.tradeTransaction is not None


	def beginTrading(self, entityId):
		"""
		Begins a new trade session with the specified trade partner
		@param entityId: Entity ID of trade partner
		"""
		if self.isTrading():
			self.onError("Only one trade session can be open at a time")
			return False

		if entityId == self.entityId:
			self.onError("Cannot trade with yourself")
			return False

		partner = self.space.findEntity(entityId)
		if partner is None or not isinstance(partner, SGWPlayer):
			self.onError("Cannot trade with the selected entity: Not a player")
			return False

		if partner.spaceId != self.spaceId or partner.distanceTo(self.position) > Constants.MAX_INTERACT_DISTANCE:
			self.onError("You are too far away to interact with that")
			return

		if partner.isTrading():
			self.onError("The selected player is currently trading with someone else")
			return False

		transaction = TradeTransaction(self, partner)
		self.onBeginTrading(transaction)
		partner.onBeginTrading(transaction)
		return True


	def cancelTrading(self, entityId):
		"""
		Cancels an active trade session with the specified trade partner
		@param entityId: Entity ID of trade partner
		"""
		if self.tradeTransaction is None or not self.tradeTransaction.hasPlayer(entityId):
			warn(Atrea.enums.LC_Interact, "<%s> Cannot cancel nonexistent trade session with %d" %
										  (self.getName(), entityId))
			return False

		self.tradeTransaction.cancel()
		return True


	def onBeginTrading(self, transaction):
		"""
		Notifies the player that a trade session was started
		@param transaction: Trade transaction
		"""
		assert self.tradeTransaction is None
		self.tradeTransaction = transaction


	def onTradeCompleted(self, resultCode):
		"""
		Notifies the player that the trade session was closed with the partner
		(either successfully or unsuccessfully)
		@param resultCode: one of the ETradeResults constants
		"""
		partner = self.tradeTransaction.getPartnerProposal(self.entityId).player
		self.tradeTransaction = None
		self.client.onTradeResults(partner.entityId, resultCode)
		self.inventory.flushUpdates()


	def onTradeProposalUpdated(self):
		"""
		Notifies the player that the local or remote trade proposal was updated
		"""
		proposal = self.tradeTransaction.getProposal(self.entityId)
		partner = self.tradeTransaction.getPartnerProposal(self.entityId)
		self.client.onTradeState(partner.player.entityId, proposal.toLocalProposal(), partner.toRemoteProposal())


	@mustBeAlive
	def tradeRequest(self, entityId, localProposal):
		"""
		Client RPC
		Called when the client wants to trade with a player entity
		@param entityId: Entity ID of trade partner (SGWPlayer)
		@param localProposal: Initial trade proposal
		"""
		if self.beginTrading(entityId):
			if not self.tradeTransaction.updateProposal(entityId, localProposal):
				self.cancelTrading(entityId)
		else:
			self.client.onTradeResults(entityId, Atrea.enums.Cancelled)


	@mustBeAlive
	def tradeRequestCancel(self, entityId):
		"""
		Client RPC
		Called when the client wants to trade with a player entity
		@param entityId: Entity ID of trade partner (SGWPlayer)
		"""
		self.cancelTrading(entityId)


	@mustBeAlive
	def tradeUpdateProposal(self, entityId, localProposal):
		"""
		Client RPC
		Called by the client to update what this entity currently has in their trade window
		@param entityId: Entity ID of trade partner (SGWPlayer)
		@param localProposal: Updated trade proposal
		"""
		# WORKAROUND: We need to start a trade session here as the 0.8384 QA client
		# doesn't send a tradeRequest RPC at all
		if not self.isTrading():
			if not self.beginTrading(entityId):
				self.client.onTradeResults(entityId, Atrea.enums.Cancelled)
				return

		if self.tradeTransaction is None or not self.tradeTransaction.hasPlayer(entityId):
			warn(Atrea.enums.LC_Item, "<%s> Cannot update trade proposal: no trade session with player %d" %
									  (self.getName(), entityId))
			return False

		if not self.tradeTransaction.updateProposal(self.entityId, localProposal):
			self.tradeTransaction.cancel()


	@mustBeAlive
	def tradeLockState(self, localVersionId, remoteVersionId, lockState):
		"""
		Client RPC
		Called by the client to update the state of their trade locks
		@param localVersionId: Last proposal version sent by us
		@param remoteVersionId: Last proposal we've seen from the trade partner
		@param lockState: Trade lock state (one of the ETRADELOCKSTATE_ constants)
		"""
		if self.tradeTransaction is None:
			warn(Atrea.enums.LC_Interact, "<%s> Sent trade lock state when not trading" % self.getName())
			return

		self.tradeTransaction.updateLockState(self.entityId, localVersionId, remoteVersionId, lockState)


	def cancelMovie(self, MovieName):
		pass


	###################################################################################
	#                            Communicator RPC methods
	###################################################################################

	def processPlayerCommunication(self, speaker, speakerFlags, target, channelId, message):
		"""
		Base RPC
		Called when a cell-only player message must be distributed to other players
		@param speaker: Name of player speaking
		@param speakerFlags: Flags describing the speaker (see ESpeakerFlags enum)
		@param target: Target player name
		@param channelId: Channel (see EChannel enum)
		@param message: Text to speak
		"""
		if channelId in [Atrea.enums.CHAN_say, Atrea.enums.CHAN_emote, Atrea.enums.CHAN_yell]:
			if len(message) > 0 and message[0] == '.':
				Command.exec(self, message[1:])
				return

			self.witnesses.onPlayerCommunication(speaker, speakerFlags, channelId, message)
			# The client only echoes back messages in CHAN_say for some reason
			# For all other channels we need to notify the client as well
			if channelId != Atrea.enums.CHAN_say:
				self.client.onPlayerCommunication(speaker, speakerFlags, channelId, message)
		else:
			self.onError("Speaking on channel %d is not supported yet!" % channelId)

	
	###################################################################################
	#                        OrganizationMember RPC methods
	###################################################################################

	def organizationInviteResponse(self, aRequestID, aResponse):
		pass


	def organizationLeave(self, aOrganizationId):
		pass


	def BroadcastMinimapPing(self, aOrganzationId, aLocation):
		pass


	def strikeTeamResponse(self, aOrganizationId, aResponse):
		pass


	def pvpOrganizationLeaveResponse(self, aOrganizationId, aResponse):
		pass


	def organizationMOTD(self, aOrganizationId, aMOTD):
		pass


	def organizationNote(self, aOrganizationId, aNote):
		pass


	def organizationOfficerNote(self, aOrganizationId, aName, aNote):
		pass


	def organizationSetRankPermissions(self, aOrganizationId, aRank, aPermission):
		pass


	def organizationSetRankName(self, aOrganizationId, aRank, aName):
		pass


	def squadSetLootMode(self, aLootMode):
		pass


	def organizationTransferCash(self, aOrganizationId, aCash):
		pass




	def requestStartMinigame(self, minigame):
		"""
		Requests the player to start a new minigame instance
		This is not meant to be called by external classes; use Minigame.play() instead!
		@param minigame: Minigame request instance
		@return: True if the minigame request was sent, False otherwise
		"""
		if self.minigame is not None:
			warn(Atrea.enums.LC_Minigame, "A minigame session is already in progress")
			return False

		self.minigame = minigame
		self.minigameStarted = False
		info(Atrea.enums.LC_Minigame, "Starting minigame <%s> for player <%s>" % (minigame.name, self.playerName))
		self.client.onStartMinigameDialog(minigame.name, "DifficultyLevel", minigame.techCompetency,
				minigame.gameName, minigame.archetypes, 1, 0, 0)
		# TODO: Is this needed here?
		self.client.addOrUpdateMinigameHelper(self.playerId, "Cpu", " ", self.level, self.archetype, 0)
		self.setStateFlag(Atrea.enums.BSF_PlayingMinigame)
		self.addMovementLock()
		return True


	def handleMinigameResults(self, ticket, resultId, timeRemaining, numAttempts, instrumentsUsed, consumablesUsed):
		if self.minigame and self.minigameStarted:
			self.unsetStateFlag(Atrea.enums.BSF_PlayingMinigame)
			self.removeMovementLock()
			info(Atrea.enums.LC_Minigame, "Minigame result for player <%s>: %d" % (self.playerName, resultId))
			self.minigame.onMinigameResult(resultId)
			self.minigameStarted = False
			self.minigame = None
		else:
			warn(Atrea.enums.LC_Minigame, "Minigame results received when no minigame is running!")


	###################################################################################
	#                           MinigamePlayer RPC methods
	###################################################################################

	def debugStartMinigame(self, aGameId):
		pass


	def debugSpectateMinigame(self):
		pass


	def debugJoinMinigame(self):
		pass


	def debugMinigameInstance(self, instanceId, seed, techComp):
		pass


	@mustBeAlive
	def startMinigame(self):
		if self.minigame is not None:
			if not self.minigameStarted:
				trace(Atrea.enums.LC_Minigame, "Sending minigame start request to BaseApp ...")
				if self.minigame.seed is None:
					seed = random.randint(0, 0x7fffffff)
				else:
					seed = self.minigame.seed
				# TODO: What is abilitiesMask?
				# For Livewire, it seems to be the mask of ability cheats that are available
				# TODO #2: We should use minigameLevelUpdate() for passing self.level
				self.base.startMinigame(self.minigame.gameId, ''.encode(), self.minigame.difficulty,
				    self.minigame.techCompetency, seed, 0, self.getStat(Atrea.enums.intelligence).cur, self.level)
				self.minigameStarted = True
		else:
			warn(Atrea.enums.LC_Minigame, "Player <%s> tried to start minigame when no session is active" % self.playerName)
			self.onError("There is no minigame to start")
	

	def endCurrentMinigame(self):
		"""
		RPC called by the client when it closed the minigame window
		(also called after an area transition)
		"""
		if self.minigame is not None and self.minigameStarted:
			trace(Atrea.enums.LC_Minigame, "Canceling minigame session ...")
			# We need to wait for a BaseApp minigame result message, because depending on the state
			# of the minigame it can return RESULT_Canceled or RESULT_NotStarted which are handled differently
			self.base.endMinigameForPlayer(''.encode())


	@mustBeAlive
	def requestSpectateList(self):
		pass


	@mustBeAlive
	def spectateMinigame(self, playerId):
		pass


	@mustBeAlive
	def registerToMinigameHelp(self, note, inRangeOnly):
		pass


	@mustBeAlive
	def updateRegisterToMinigameHelp(self, note, inRangeOnly, wantsRequests):
		pass


	def minigameStartCancel(self):
		if self.minigame is not None and not self.minigameStarted:
			trace(Atrea.enums.LC_Minigame, "Canceling minigame start request ...")
			self.unsetStateFlag(Atrea.enums.BSF_PlayingMinigame)
			self.removeMovementLock()
			self.minigame.onMinigameResult(Constants.MINIGAME_RESULT_NotStarted)
			self.minigame = None
		else:
			self.onError("There is no minigame to cancel")


	@mustBeAlive
	def minigameCallAccept(self, callingPlayerId):
		pass


	@mustBeAlive
	def minigameCallDecline(self, callingPlayerId):
		pass


	@mustBeAlive
	def minigameCallAbort(self):
		pass


	@mustBeAlive
	def minigameContactRequest(self, contactId):
		pass



	###################################################################################
	#                           GateTravel RPC methods
	###################################################################################

	def onDialGate(self, targetAddressId, sourceAddressId):
		trace(Atrea.enums.LC_Stargate, "onDialGate(%d, %d)" % (targetAddressId, sourceAddressId))
		if targetAddressId == -1:
			info(Atrea.enums.LC_Stargate, "[%s] Canceling stargate dial" % self.playerName)
			self.cancelDialing()
			return

		gate = DefMgr.get('stargate', targetAddressId)
		if gate is None:
			self.cancelDialing()
			warn(Atrea.enums.LC_Stargate, "[%s] Tried to dial invalid stargate address %d" % (self.playerName, targetAddressId))
			self.onError("Failed to dial: invalid stargate address")
			return

		if targetAddressId not in self.knownStargates and targetAddressId not in self.hiddenStargates:
			self.cancelDialing()
			warn(Atrea.enums.LC_Stargate, "[%s] Tried to dial stargate address %d which is not known" % (self.playerName, targetAddressId))
			self.onError("Failed to dial: not a known stargate address")
			return

		world = self.space.world
		if len(world.stargates) == 0:
			self.cancelDialing()
			warn(Atrea.enums.LC_Stargate, "[%s] Tried to dial from a world with no stargates" % self.playerName)
			self.onError("Failed to dial: world has no stargates!")
			return

		info(Atrea.enums.LC_Stargate, "[%s] Dialing address %d" % (self.playerName, targetAddressId))
		self.dialingStargate = world.stargates[0]
		self.beginDialing(gate)


	def beginDialing(self, stargate):
		"""
		Begins an internal dialing sequence that opens the gate after a fixed amount of time passes
		@param stargate: Destination stargate
		"""
		if self.dialedAddress is not None:
			self.cancelDialing()

		self.dialedAddress = stargate
		self.gatePassable = False
		now = Atrea.getGameTime()
		self.gateDialTimer = Atrea.addTimer(now + 4.0, lambda: self.gateDialTimerExpired())


	def cancelDialing(self):
		"""
		Cancels the internal dialing sequence
		"""
		if self.dialedAddress is not None:
			if self.gateDialTimer is not None:
				Atrea.cancelTimer(self.gateDialTimer)
				self.gateDialTimer = None
			else:
				self.fire('stargate::destroy', stargateId = self.dialedAddress.id)
			self.dialedAddress = None
			self.gatePassable = False


	def gateDialTimerExpired(self):
		"""
		Called when the internal dial timer expires (gate is passable after this is triggered)
		"""
		self.gateDialTimer = None
		self.gatePassable = True
		self.client.onSequence(self.dialingStargate.eventSet.getSequence(Atrea.enums.Stargate_MakeGate).seqId,
			self.entityId, self.entityId, 1, Atrea.getGameTime(), [], Atrea.enums.KISMET_VIEW_EventInvoker, 0)
		self.fire('stargate::make', stargateId = self.dialedAddress.id)


	def stargatePassed(self):
		if self.dialedAddress is not None and self.gatePassable:
			# TODO: What else should we send here? Gate splash kismet?
			# self.client.onStargatePassage(self.dialedAddress.id)
			self.isInTransit = True
			addr = self.dialedAddress
			info(Atrea.enums.LC_Stargate, "[%s] Passed the stargate to <%s>" % (self.playerName, addr.world.world))
			self.client.onSequence(self.dialingStargate.eventSet.getSequence(Atrea.enums.Stargate_CrossGate).seqId,
				self.entityId, self.entityId, 1, Atrea.getGameTime(), [], Atrea.enums.KISMET_VIEW_EventInvoker, 0)
			self.fire('stargate::passage', stargateId = self.dialedAddress.id)
			self.dialedAddress = None
			self.gatePassable = False
			self.moveTo(addr.xPos, addr.yPos, addr.zPos, addr.yaw, addr.world.clientMap)


	###################################################################################
	#                        SGWInventoryManager RPC methods
	###################################################################################

	@mustBeAlive
	def removeItem(self, itemId, quantity):
		if not self.inventory.removeItem(itemId, quantity):
			# Feedback?
			pass
		self.inventory.flushUpdates()


	def listItems(self):
		self.inventory.setAllDirty()
		self.inventory.flushUpdates()


	@mustBeAlive
	def moveItem(self, itemId, targetBag, targetSlot, quantity):
		trace(Atrea.enums.LC_Item, '[%s] Requesting item %d move to bagId %d, slot %d' %
								   (self.playerName, itemId, targetBag, targetSlot))
		# When equipping items by right-clicking them, the client sends a negative number
		# as the item quantity ... weird
		if quantity < 0:
			quantity = -quantity
		if not self.inventory.moveItem(itemId, targetBag, targetSlot - 1, quantity):
			# Feedback?
			pass
		self.inventory.flushUpdates()


	@mustBeAlive
	def useItem(self, itemId, targetId):
		trace(Atrea.enums.LC_Item, '[%s] Requesting item %d use on %d' % (self.playerName, itemId, targetId))
		if self.inventory.useItem(itemId, Atrea.findEntity(targetId) if targetId else None):
			useSeq = self.getItemSequence(Atrea.enums.Item_Use)
			if useSeq is not None:
				self.playSequence(useSeq.seqId, self.entityId)
			# Is this really needed here?
			self.inventory.flushUpdates()


	@mustBeAlive
	def repairItemRequest(self, itemId, repairRatio):
		pass


	def onActiveSlotChanged(self, bagId, slotId):
		"""
		Called when the active inventory slot of a bag changed.
		@param bagId: ID of bag whose active slot changed
		@param slotId: New active slot ID
		"""
		if bagId == Atrea.enums.INV_Bandolier:
			self.abilities.onBandolierSlotChange()


	@mustBeAlive
	def requestActiveSlotChange(self, bagId, slotId):
		trace(Atrea.enums.LC_Item, '[%s] Requesting active slot change in bag %d to %d' % (self.playerName, bagId, slotId))
		if self.inventory.updateActiveSlot(bagId, slotId - 1):
			self.inventory.flushUpdates()
		else:
			self.onError("Failed to change active slot")


	@mustBeAlive
	def requestAmmoChange(self, itemId, ammoType):
		pass


	###################################################################################
	#                           MailManager RPC methods
	###################################################################################

	def requestMailHeaders(self, bArchive):
		trace(Atrea.enums.LC_Interact, 'Requesting mail headers, archive: %s' % str(bArchive))
		messageHeaders = []
		
		mails = Atrea.dbQuery("select * from sgw_gate_mail where character_id = " + str(self.databaseId))
		for mail in mails:
			messageHeaders.append({
			'id': mail['mail_id'], 
			'fromText': mail['sender_name'], 
			'fromId': mail['sender_id'], 
			'subjectText': mail['subject'], 
			'subjectId': 0, 
			'cash': mail['cash'], 
			'sentTime': mail['sent_time'], 
			'readTime': mail['read_time'], 
			'flags': mail['flags']})
		
		self.client.onMailHeaderInfo(0, bArchive, messageHeaders, {})


	def sendMailMessage(self, recipientFlags, recipients, subject, body, cash, bCOD, itemId, itemQuantity):
		pass


	def archiveMailMessage(self, mailId):
		print('Archive mail by id: ', str(mailId))


	def deleteMailMessage(self, mailId):
		print('Delete mail by id: ', str(mailId))


	def returnMailMessage(self, mailId):
		print('Return mail by id: ', str(mailId))
		

	def requestMailBody(self, mailId):
		print('Requesting mail body by id: ', str(mailId))
		mail = Atrea.dbQuery("select message, sender_name FROM sgw_gate_mail where mail_id = " + str(mailId))[0]
		self.client.onMailRead(mailId, mail['message'], 0, self.playerName)
		

	def takeCashFromMailMessage(self, mailId):
		print('Taking cash from mail by id: ', str(mailId))
		

	def takeItemFromMailMessage(self, mailId, containerId, slotId):
		print('Taking item from mail by id: ', str(mailId), ', container: ', str(containerId), ', slot: ', str(slotId))
		

	def payCODForMailMessage(self, mailId):
		print('Paying COD for mail by id: ', str(mailId))
	

	###################################################################################
	#                            Missionary RPC methods
	###################################################################################

	def abandonMission(self, missionId):
		"""
		Client RPC
		The client wants to abandon the specified mission
		@param missionId: Mission ID to abandon
		"""
		self.missions.abandon(missionId)

	def shareMission(self, missionId):
		pass

	def shareMissionResponse(self, choice):
		pass
		


	###################################################################################
	#                       ContactListManager RPC methods
	###################################################################################

	def contactListCreate(self, aName, aFlags):
		pass
		

	def contactListDelete(self, aListId):
		pass
		

	def contactListRename(self, aListId, aName):
		pass
		

	def contactListFlagsUpdate(self, aListId, aFlags):
		pass
		

	def contactListAddMembers(self, aListId, aPlayerNames):
		pass
		

	def contactListRemoveMembers(self, aListId, aPlayerNames):
		pass
		
