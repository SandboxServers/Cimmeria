import Atrea
import Atrea.enums
import re
from base.Chat import ChannelManager
from common import Constants
from common.Config import Config
from common.Logger import trace, error, warn, info
from common.defs.Def import DefMgr

class Account(Atrea.BaseEntity):
	# Try to fill bags in this order when adding items from visual choices
	BagFillOrder = [
		# The first ones must be equipment slots, so default items will get equipped
		# by the newly created char and will show up on the charlist screen
		Atrea.enums.INV_Head,
		Atrea.enums.INV_Face,
		Atrea.enums.INV_Neck,
		Atrea.enums.INV_Chest,
		Atrea.enums.INV_Hands,
		Atrea.enums.INV_Waist,
		Atrea.enums.INV_Back,
		Atrea.enums.INV_Legs,
		Atrea.enums.INV_Feet,
		Atrea.enums.INV_Artifact1,
		Atrea.enums.INV_Artifact2,
		Atrea.enums.INV_Bandolier,
		# Additional items will go to the main inventories
		Atrea.enums.INV_Mission,
		Atrea.enums.INV_Main,
		Atrea.enums.INV_Crafting
	]

	# Database ID of the current account
	# These properties are set by BaseApp native code during initialization!
	accountId = 0
	# Name of the current account
	accountName = ''
	# Permissions of the current account
	accessLevel = 0
	# Has the player requested to enter the game?
	requestedPlay = False

	def __init__(self):
		super().__init__()
		self.characters = []
		pass

	def load(self):
		return True
		
	def destroyed(self):
		pass

	def persist(self):
		pass
		
	def attachedToController(self):
		assert self.accountId == self.databaseId
		trace(Atrea.enums.LC_CharManagement, 'Requesting character list for account %d ...' % self.accountId)
		self.characters = Atrea.dbQuery("SELECT * FROM sgw_player WHERE account_id = " + str(self.accountId) +
				" ORDER BY player_id ASC")
		for char in self.characters:
			char['components'] = char['components'][1:-1].split(',')
			char['items_loaded'] = False
		self.sendCharacterList()
		
	def detachedFromController(self):
		pass

	def cookedDataError(self, categoryId, elementId):
		"""
		Called when the server fails to send a cooked resource to the player.
		@param categoryId: Category of cooked resource
		@param elementId: ID of cooked resource
		"""
		self.client.onCookedDataError(categoryId, elementId)

	def sendCharacterList(self):
		# Convert the list of characters from database format to packet format
		charlist_characters = []
		for char in self.characters:
			if Config.DISALLOW_DUPLICATE_CHARACTERS and ChannelManager.isPlayerOnline(char['player_name']):
				playable = 0
			else:
				playable = 1
			charlist_characters.append({
				'playerId' : char['player_id'],
				'name' : char['player_name'],
				'extraName' : char['extra_name'],
				'alignment': char['alignment'],
				'level': char['level'],
				'gender': char['gender'],
				'worldLocation': char['world_location'],
				'archetype': char['archetype'],
				'title': char['title'],
				'playerType': 1, # TODO: What's this?
				'playable': playable
			})
		
		# Queue character list message
		self.client.onCharacterList(charlist_characters)
		
		
	# Lookup a character from the cached character list
	# Returns the character record if successful, None if character doesn't exist
	def findCharacter(self, char_id):
		for char in self.characters:
			if char['player_id'] == char_id:
				return char
				
		# Return none if no character was found
		return None


	def isCharacterNameAllowed(self, charDef, name, extraName):
		"""
		Checks if the specified character name is allowed at character creation
		"""
		if len(name) < 3:
			warn(Atrea.enums.LC_CharManagement, "Character name '%s' is too short" % name)
			return False

		# Check if name is already in use
		char = Atrea.dbQuery("select count(1) as cnt from sgw_player where player_name = '%s'" % re.escape(name))
		if char[0]['cnt']:
			warn(Atrea.enums.LC_CharManagement, "Character name '%s' already in use" % name)
			return False

		return True



	def createCharacter(self, name, extraName, charDefId, visualChoices, skinTintColorId):
		charDef = DefMgr.get('character_creation', charDefId)
		if charDef is None:
			warn(Atrea.enums.LC_CharManagement, "Invalid charDefId %d when creating character" % charDefId)
			self.client.onCharacterCreateFailed(Atrea.enums.ERROR_CharacterCreationInvalidCharacterType)
			return

		# Validate client visual group choices
		choices = charDef.getAllChoices(visualChoices)
		if type(choices) == int:
			self.client.onCharacterCreateFailed(choices)
			return

		# Check if the specified character name is valid
		if not self.isCharacterNameAllowed(charDef, name, extraName):
			self.client.onCharacterCreateFailed(Atrea.enums.ERROR_InvalidCharacterName)
			return

		if skinTintColorId < 0 or skinTintColorId >= len(Constants.SKIN_TINTS):
			warn(Atrea.enums.LC_CharManagement, "Invalid skin tint %d when creating character" % skinTintColorId)
			self.client.onCharacterCreateFailed(Atrea.enums.ERROR_CharacterCreationInvalidSkinColor)
			return

		components = []
		for choice in choices.values():
			# Don't populate the component list with item components yet,
			# as those should not be saved to the players component list
			if choice['component'] is not None and not choice['item_id']:
				components.append(choice['component'])

		worldId = next((w.id for w in DefMgr.getAll('world_info').values() if w.world == charDef.startingWorld))

		# Add all starting abilities to the newly created character
		ability_ids = [str(a.id) for a in charDef.abilities]

		# TODO: Remove extra name from asgard chars ?
		player = Atrea.dbQuery("""
			insert into sgw_player (account_id, alignment, archetype, gender, player_name,
		        extra_name, world_location, bodyset, pos_x, pos_y, pos_z, world_id, components,
		        skin_color_id, abilities, access_level)
		    values (%d, %d, %d, %d, '%s', '%s', '%s', '%s', %f, %f, %f, %d, '%s', '%d', '%s', %d)
		    returning player_id""" %
            (self.accountId, charDef.alignmentId, charDef.archetypeId, charDef.gender,
             repr(name)[1:-1], repr(extraName)[1:-1], charDef.startingWorld, repr(charDef.bodySet)[1:-1],
			 charDef.startingX, charDef.startingY, charDef.startingZ, worldId,
			 repr('{' + ','.join(components) + '}')[1:-1], skinTintColorId,
			 repr('{' + ','.join(ability_ids) + '}')[1:-1], self.accessLevel))
		playerId = player[0]['player_id']

		slotIndices = {}
		for choice in choices.values():
			if choice['item_id']:
				design = DefMgr.get('item', choice['item_id'])
				if design:
					bagIds = [container.id for container in design.containers]
					index = min([self.BagFillOrder.index(bagId) for bagId in bagIds if bagId in self.BagFillOrder])
					bagId = self.BagFillOrder[index]
					if bagId not in slotIndices:
						slotIndices[bagId] = 1 if bagId == Atrea.enums.INV_Bandolier else 0
					slotId = slotIndices[bagId]
					slotIndices[bagId] += 1
					if slotIndices[bagId] <= Constants.BAG_SIZES[bagId]:
						Atrea.dbQuery("""
							insert into sgw_inventory
							(container_id, slot_id, type_id, character_id, durability, bound)
							values (%d, %d, %d, %d, %d, %s)""" %
						  (bagId, slotId, design.id, playerId, choice['item_durability'], str(choice['item_bound'])))

						# Player already saved, we can add visual components of all items now
						if choice['component'] is not None:
							components.append(choice['component'])
					else:
						warn(Atrea.enums.LC_CharManagement, "Tried to add too many items to to player bag %d" % bagId)
				else:
					error(Atrea.enums.LC_CharManagement, "Failed to add designId to player: %d" % choice['item_id'])

		info(Atrea.enums.LC_CharManagement, "Created character '%s'" % name)

		# Add the player to the created character list
		self.characters.append({
			'player_id' : playerId,
			'player_name' : name,
			'extra_name' : extraName,
			'bodyset' : charDef.bodySet,
			'alignment': charDef.alignmentId,
			'level': 1,
			'gender': charDef.gender,
			'world_location': charDef.startingWorld,
			'archetype': charDef.archetypeId,
			'title': 0,
			'pos_x': charDef.startingX,
			'pos_y': charDef.startingY,
			'pos_z': charDef.startingZ,
			'components': components,
			'items_loaded': False,
			'bandolier_slot': 0,
			'access_level': self.accessLevel,
			'skin_color_id': skinTintColorId})
		self.sendCharacterList()


	def requestCharacterVisuals(self, playerId):
		"""
		Client RPC
		Called when the client wants to retrieve appearance information about the character
		@param playerId: The character being queried
		"""
		char = self.findCharacter(playerId)
		
		if char is not None:
			if not char['items_loaded']:
				trace(Atrea.enums.LC_CharManagement, 'Fetching item visuals for characterID %d' % playerId)
				# We should load/store the active slot and display visuals from that slot
				items = Atrea.dbQuery("select type_id from sgw_inventory "
				                      "where character_id = %d and "
				                      "((container_id in (3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14) and slot_id = 0) or"
									  "(container_id = 3 and slot_id = %d))" %
									  (playerId, char['bandolier_slot']))
				for item in items:
					itemDef = DefMgr.get('item', item['type_id'])
					if itemDef is not None:
						if itemDef.visualComponent is not None:
							char['components'].append(itemDef.visualComponent)
					else:
						error(Atrea.enums.LC_CharManagement, "No ItemDef found for item %d" % item['type_id'])
				char['items_loaded'] = True

			# Send visual components to client
			skinColor = Constants.SKIN_TINTS[char['skin_color_id']]
			self.client.onCharacterVisuals(playerId, char['bodyset'], char['components'], 0xFF, 0xFF, skinColor)
		else:
			# The character ID doesn't exist or belongs to another account
			warn(Atrea.enums.LC_CharManagement, 'Attempted to retrieve visual components for unknown characterID %d' % playerId)


	def playCharacter(self, playerId):
		"""
		Client RPC
		Called when the player presses the "Play" button
		@param playerId: Character to enter with
		"""
		if self.requestedPlay:
			warn(Atrea.enums.LC_LogInOut, 'playCharacter(): Player is already entering a game')
			return
	
		# Retrieve character entry
		char = self.findCharacter(playerId)
		if char is None:
			# The character ID doesn't exist or belongs to another account
			warn(Atrea.enums.LC_LogInOut, 'Attempted to play unknown characterID %d' % playerId)
			self.client.onCharacterLoadFailed()
			return

		if Config.DISALLOW_DUPLICATE_CHARACTERS and ChannelManager.isPlayerOnline(char['player_name']):
			# Check if the character is already in use on the server
			warn(Atrea.enums.LC_LogInOut, 'Attempted to play with already logged in characterID %d' % playerId)
			self.client.onCharacterLoadFailed()
			return

		# Create a new base player entity
		if char['access_level'] > 0:
			className = "SGWGmPlayer"
		else:
			className = "SGWPlayer"
		player = Atrea.createBaseEntityFromDb(className, playerId)
		if player is None:
			# Entity creation failed for some reason (most likely DB error)
			error(Atrea.enums.LC_LogInOut, 'Failed to create character entity for characterID %d' % playerId)
			self.client.onCharacterLoadFailed()
			return

		# Try to spawn the player in its selected cell
		info(Atrea.enums.LC_LogInOut, 'Logging in with character <%s>' % char['player_name'])
		player.account = self
		self.requestedPlay = True
		Atrea.createCellEntity(player.entityId, 0xffffffff, playerId, char['pos_x'], char['pos_y'], char['pos_z'], 0.0, 0.0, 0.0, char["world_location"])


	def playCharacterFailed(self, player):
		"""
		Called by the player when it failed to load for some reason
		"""
		self.client.onCharacterLoadFailed()
		# HACK: We need to give some feedback to the player, as onCharacterLoadFailed() doesn't display anything
		self.client.onCharacterCreateFailed(Atrea.enums.ERROR_CellLoginFailed)
		player.destroy()
		self.requestedPlay = False


	def versionInfoRequest(self, categoryId, version):
		if categoryId not in DefMgr.ResourceCategories:
			warn(Atrea.enums.LC_Resources, 'Client requested invalid resource category %d' % categoryId)
			return

		categoryMaps = {
			12: 'world_info',
			13: 'stargate',
			14: 'container',
			15: 'blueprint',
			16: 'applied_science',
			17: 'discipline',
			18: 'racial_paradigm',
			20: 'interaction'
		}

		# trace('Version info requested; category: ', categoryId, ', ver: ', version)
		category = DefMgr.ResourceCategories[categoryId]
		serverVersion = DefMgr.getVersion(category)
		invalidateAll, added, invalidated = DefMgr.getChangesForVersion(category, version)
		resourcesToSend = []

		if categoryId in categoryMaps:
			if invalidateAll:
				resourcesToSend = DefMgr.getAll(categoryMaps[categoryId])
			elif invalidated and len(invalidated):
				resourcesToSend = invalidated

		self.client.onVersionInfo(categoryId, serverVersion, len(resourcesToSend), 1 if invalidateAll else 0, invalidated or [])
		for resourceId in resourcesToSend:
			Atrea.sendResource(self, categoryId, resourceId)
			
	def onClientVersion(self, versionSeed, clientVersion, language):
		# Client sent us its version number
		# We can't do anything here ... yet
		trace(Atrea.enums.LC_Resources, 'Client versionSeed: %d, clientVersion: %s, language: %s' % (versionSeed, clientVersion, language))
		pass
		
	def elementDataRequest(self, categoryId, key):
		# trace('Client resource requested; category: ', categoryId, ', key: ', key)
		Atrea.sendResource(self, categoryId, key)
		
	def deleteCharacter(self, playerId):
		if self.findCharacter(playerId):
			info(Atrea.enums.LC_CharManagement, "Deleting player %d" % playerId)
			Atrea.dbQuery('delete from sgw_player where player_id = %d' % playerId)
			for idx in range(0, len(self.characters)):
				if self.characters[idx]['player_id'] == playerId:
					del self.characters[idx]
					break
			self.sendCharacterList()
		else:
			warn(Atrea.enums.LC_CharManagement, "Attempted to delete nonexistent/not owned player %d" % playerId)

	def logOff(self):
		pass
	