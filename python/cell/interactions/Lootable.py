import Atrea.enums
import random
import weakref
from cell.interactions.Interaction import Interaction
from common import Constants
from common.Logger import trace, warn
from common.defs.Def import DefMgr

class LootableItem(object):
	def __init__(self, designId, quantity, index):
		if designId is not None:
			self.design = DefMgr.require('item', designId)
		else:
			self.design = None
		self.quantity = quantity
		self.index = index
		self.eligiblePlayerList = []

	def addEligiblePlayer(self, playerId):
		"""
		Adds a player to the eligible player list.
		If the list contains more than 0 elements, only players on the eligible player list may loot the item.
		@param playerId: DBID of the player. This is *NOT* entityId!
		"""
		self.eligiblePlayerList.append(playerId)

	def isPlayerEligible(self, player):
		"""
		Checks if the player can loot this item
		@param player: SGWPlayer object
		@return: True, if the player can loot the item, False otherwise
		"""
		# There is no playerId filtering --> always allow looting
		if len(self.eligiblePlayerList) == 0:
			return True

		# Player is on the eligible list --> allow looting
		if player.playerId in self.eligiblePlayerList:
			return True

		return False


class Lootable(Interaction):
	def __init__(self, entity):
		super().__init__(entity)
		self.loot = []
		self.nextLootIndex = 1
		self.looters = {}


	def interactionSetMapId(self):
		"""
		Returns the interaction set map that should be used when adding this interaction to an entity.
		@return: Interaction set map ID
		"""
		if len(self.loot) == 0:
			return None

		for item in self.loot:
			if item.design is not None and item.design.isMissionItem():
				# TODO: We can't set INTERACTION_MissionLoot here currently due to a 64-bit signedness bug
				return Constants.INTERACTION_Loot

		return Constants.INTERACTION_Loot


	def generateLoot(self):
		"""
		Generates random loot for the entity
		"""
		# TODO: Add missionId/stepId/objectiveId to loot tables
		if self.entity().template is not None and self.entity().template.lootTable is not None:
			self.randomizeLoot(self.entity().template.lootTable.loot)


	def randomizeLoot(self, table):
		"""
		Fill the drop list of this entity based on probabilities and quantities specified in the loot table.
		Loot generation algorithm:
			For each item it generates uniformly distributed random numbers R, R2 in the range 0..1
			Let P be the probability of the item being dropped (0..1)
			Let M be the minimal and N the maximal amount of items dropped
			If R <= P: [M + (M - N) * R2] items are dropped
			If R > P, no items are dropped
		@param table: Loot table (list of EntityTemplateLoot objects)
		"""
		for item in table:
			rand = random.random()
			if rand <= item.probability:
				quantity = item.minQuantity + random.randint(0, item.maxQuantity - item.minQuantity)
				if item.design is not None:
					name = item.design.name
				else:
					name = "naquadah"
				trace(Atrea.enums.LC_Uncategorized, "Randomized %d %s for entity %d (prob %f, min %d, max %d)" % (quantity, name,
				    self.entity().entityId, item.probability, item.minQuantity, item.maxQuantity))
				if quantity > 0:
					if item.design is not None:
						self.addLoot(item.design.id, quantity)
					else:
						self.addLoot(None, quantity)


	def onInteract(self, player, interactionSetMapId):
		"""
		Called when the player wants to interact with this entity
		@param player: SGWPlayer entity
		@param interactionSetMapId: ID of interaction map the player requested
		@return Was the interaction handled?
		"""
		trace(Atrea.enums.LC_Interact, "<%s> interacted with Lootable" % player.getName())

		if len(self.loot) > 0:
			self.looters[player.entityId] = weakref.ref(player)
			player.setLooting(self.entity())
			# TODO: Automatically end interaction if player left interaction range (or moved?)
			self.sendLootList(player, 1)


	def sendLootList(self, player, initialInteraction):
		"""
		Send the lootable item list to the player
		@param player: SGWPlayer
		@param initialInteraction: Is this an initial loot list? (1 = initial, 0 = subsequent)
		@return:
		"""
		# Skip items that the player is not allowed to loot as the client doesn't have an "unlootable" flag
		# for display items
		lootList = []
		for item in self.loot:
			if item.isPlayerEligible(player):
				if item.design is not None:
					lootList.append({
						'itemID': item.design.id,
					    'quantity': item.quantity,
					    'index': item.index,
					    'typeID': Atrea.enums.LOOT_Item
					})
				else:
					lootList.append({
						'itemID': 0,
					    'quantity': item.quantity,
					    'index': item.index,
					    'typeID': Atrea.enums.LOOT_Cash
					})
		# TODO: Update per-player interaction flags here!
		player.client.onLootDisplay(self.entity().entityId, lootList, initialInteraction)


	def onLootItem(self, player, index):
		"""
		Indicates that the player wants to loot an item from this lootable object
		@param player: SGWPlayer object
		@param index: Index of item to loot (LootableItem.index)
		"""
		item = None
		# Find the item in the loot list
		for i in self.loot:
			if i.index == index:
				item = i
				break

		# Illegal item index or item was already looted
		if item is None:
			warn(Atrea.enums.LC_Interact, "Cannot loot item: Invalid index (%d) for entity %d!" % (index, self.entity().entityId))
			player.onError("Loot: Item does not exist")
			return

		# Check if the player is allowed to loot the item
		# (it can send the index of an item it is not eligible for even if we don't send the index in the loot display list)
		if not item.isPlayerEligible(player):
			warn(Atrea.enums.LC_Interact, "Cannot loot item: Player (%d) not eligible for item (%d)!" % (player.entityId, index))
			player.onError("Not eligible to loot item")
			return

		pickedUp = False
		if item.design is not None:
			# Loot table entry has a design --> picking up an item
			if player.inventory.pickedUpItem(item.design.id, item.quantity):
				trace(Atrea.enums.LC_Item, "Picked up item index %d, designId %d, count %d" % (item.index, item.design.id, item.quantity))
				pickedUp = True
			else:
				warn(Atrea.enums.LC_Item, "Failed to add designId %d, count %d to player inventory" % (item.design.id, item.quantity))
				player.onError("Failed to loot item")
		else:
			# No designId --> picking up cash
			player.inventory.addCash(item.quantity)
			trace(Atrea.enums.LC_Cash, "Picked up cash index %d, count %d" % (item.index, item.quantity))
			pickedUp = True

		if pickedUp:
			player.inventory.flushUpdates()
			self.loot.remove(item)
			# Notify everyone who has the loot window open that the list of lootable items has changed
			for player in self.looters.values():
				# We have to recalculate the sent loot list for every player as eligibility status
				# may be different for each one
				self.sendLootList(player(), 0)

			if len(self.loot) == 0:
				# FIXME: This is workaround for not having proper dynamic interaction type updates
				ent = self.entity()
				ent.setInteractionType(ent.interactionType & ~Atrea.enums.INT_NormalLoot)
				ent.lootHandler = None
				ent.staticInteractions[Constants.INTERACTION_Loot] = None


	def addLoot(self, designId, quantity):
		"""
		Adds a new item to the list of lootable items
		@param designId: ItemDef type ID
		@param quantity: Amount of lootable items
		@return: Lootable item entry
		"""
		item = LootableItem(designId, quantity, self.nextLootIndex)
		self.nextLootIndex += 1
		self.loot.append(item)
		return item

