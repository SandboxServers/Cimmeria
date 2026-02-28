import Atrea.enums
from cell.interactions.Interaction import Interaction
from common.Logger import trace


class Vendor(Interaction):
	# List of bags that we can search for sellable/repairable/rechargeable items
	FilterBags = [
		Atrea.enums.INV_Main,
		Atrea.enums.INV_Bandolier,
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
		Atrea.enums.INV_Crafting
	]

	def __init__(self, entity):
		# TODO: Add a data validation method that checks all entity templates
		# (eg. SGWBeing.validate(), Attachments.Vendor.validate(), ...)
		super().__init__(entity)


	def onInteract(self, player, interactionSetMapId):
		"""
		Called when the player wants to interact with this entity
		@param player: SGWPlayer entity
		@param interactionSetMapId: ID of interaction map the player requested
		@return Was the interaction handled?
		"""
		if not self.entity().isDead():
			trace(Atrea.enums.LC_Interact, '<%s> interacted with Vendor' % player.getName())
			player.setVendor(self)
			self.sendVendingList(player)


	def filterInventory(self, player, itemList):
		"""
		Returns all items from the players inventory that are on the item filter list

		@param player: SGWPlayer
		@param itemList: Filter item list
		@return: Filtered (item, filter) tuples
		"""
		filtered = []
		for bagId in self.FilterBags:
			bag = player.inventory.getBag(bagId)
			for slotId in range(0, bag.slots):
				slot = bag.getItem(slotId)
				if slot is not None:
					filter = next((item for item in itemList if item.design.id == slot.typeId), None)
					if filter:
						filtered.append((slot, filter))
		return filtered


	def sendVendingList(self, player):
		"""
		Send the vendor's item list to the player
		@param player: SGWPlayer
		"""
		buyList = []
		if self.entity().template.buyItemList:
			buyable = self.entity().template.buyItemList.indexedItems
			for i in range(0, len(buyable)):
				design = buyable[i]
				item = {
					# The client expects that vending list indices are contiguous and start from 0!
					'index': i,
					'itemId': design.design.id,
					'costList': [],
					'usable': 0, # TODO: Is this flag used for anything?
					'quantity': design.quantity  # TODO: Is this value used for anything?
				}

				if design.naquadahCost > 0:
					item['costList'].append({
						'type': Atrea.enums.COST_Naquadah,
						'typeId': 0,
						'quantity': design.naquadahCost
					})

				for costDesign, costQuantity in design.itemCosts:
					item['costList'].append({
						'type': Atrea.enums.COST_Item,
						'typeId': costDesign.id,
						'quantity': costQuantity
					})
				buyList.append(item)

		buybackList = []
		buybackBag = player.inventory.getBag(Atrea.enums.INV_Buyback)
		for slot in range(0, buybackBag.slots):
			item = buybackBag.getItem(slot)
			if item is not None:
				buybackList.append({
					'cost': item.buybackPrice,
					'itemId': item.id
				})

		# Add all player items to the sellable items list if they're in the
		# sell filter of the entity template
		sellList = []
		if self.entity().template.sellItemList:
			sellable = self.filterInventory(player, self.entity().template.sellItemList.indexedItems)
			for item, filter in sellable:
				if item.type.sellable:
					sellList.append({
						'cost': filter.naquadahCost,
						'itemId': item.id
					})

		# Only allow repairing items where durability < 100%
		repairList = []
		if self.entity().template.repairItemList:
			repairable = self.filterInventory(player, self.entity().template.repairItemList.indexedItems)
			for item, filter in repairable:
				if item.durability < 100 and item.durability != -1:
					repairList.append({
						# Make sure that cheap items cannot be repaired for free
						'cost': max(int(filter.naquadahCost * (100 - item.durability) / 100), 1),
						'itemId': item.id
					})

		# Only allow repairing items where charges < max charges
		rechargeList = []
		if self.entity().template.rechargeItemList:
			rechargeable = self.filterInventory(player, self.entity().template.rechargeItemList.indexedItems)
			for item, filter in rechargeable:
				if item.charges < item.type.charges:
					rechargeList.append({
						# Make sure that cheap items cannot be recharged for free
						'cost': max(int(filter.naquadahCost * (item.type.charges - item.charges) /
									item.type.charges), 1),
						'itemId': item.id
					})

		# TODO: What is VendorType?
		player.client.onStoreOpen(self.entity().entityId, 1, buyList, sellList, buybackList, repairList, rechargeList)


	def canPurchaseItem(self, player, item, quantity):
		"""
		Checks if the player meets all prerequisites for buying this item

		@param player: SGWPlayer object
		@param item: Item to buy
		@param quantity: Amount of items to buy
		@return: True if the player can buy the item, False otherwise
		"""
		if player.inventory.naquadah < item.naquadahCost * quantity:
			return False

		for design, costQuantity in item.itemCosts:
			slot = player.inventory.findItem(design.id, mainBagsOnly = True)
			if slot is None or slot.quantity < costQuantity * quantity:
				return False

		return True


	def onPurchaseItems(self, player, items):
		"""
		Indicates that the player wants to buy items from this vendor
		@param player: SGWPlayer object
		@param items: List of (vendorItemIndex, quantity) tuples
		"""
		# TODO: Make this transactional!
		# (avoids multiple inventory / player updates)
		for index, quantity in items:
			self.onPurchaseItem(player, index, quantity)
		player.inventory.flushUpdates()


	def onPurchaseItem(self, player, index, buyQuantity):
		"""
		Indicates that the player wants to buy an item from this vendor
		@param player: SGWPlayer object
		@param index: Index of item to buy (ItemListItem.id)
		@param buyQuantity: Amount of items to buy
		"""
		if index >= 0 and index < len(self.entity().template.buyItemList.indexedItems) is None:
			player.debug("Cannot buy item: Invalid index (%d) for entity %d!" % (index, self.entity().entityId))
			player.onError("Vending: Item does not exist")
			return

		# Find the item in the vending list
		item = self.entity().template.buyItemList.indexedItems[index]

		if buyQuantity == 0:
			return

		if not self.canPurchaseItem(player, item, buyQuantity):
			player.debug("Cannot buy item: Prerequisite check failed for item (%d)!" % index)
			player.onError("Vending: Prerequisites not met for item")
			return

		# Remove total item cost from the players inventory
		# TODO: Wrap this in an InventoryTransaction block
		player.inventory.removeCash(item.naquadahCost * buyQuantity)
		for design, costQuantity in item.itemCosts:
			slot = player.inventory.findItem(design.id, mainBagsOnly = True)
			assert(player.inventory.removeItem(slot.id, costQuantity * buyQuantity))

		if player.inventory.pickedUpItem(item.design.id, buyQuantity):
			player.debug("Bought item index %d, designId %d, count %d" % (index, item.design.id, buyQuantity))
		else:
			player.debug("Failed to add designId %d, count %d to player inventory" % (item.design.id, buyQuantity))
			player.onError("Failed to purchase item")


	def onSellItems(self, player, items):
		"""
		Indicates that the player wants to sell items to this vendor
		@param player: SGWPlayer object
		@param items: List of (itemId, quantity) tuples
		"""
		# TODO: Make this transactional!
		# (avoids multiple inventory / player updates)
		for itemId, quantity in items:
			self.onSellItem(player, itemId, quantity)
		player.inventory.flushUpdates()


	def onSellItem(self, player, itemId, sellQuantity):
		"""
		Indicates that the player wants to buy an item from this vendor
		@param player: SGWPlayer object
		@param itemId: ID of item to sell (Item.id)
		@param sellQuantity: Amount of items to sell
		"""
		item = player.inventory.getItem(itemId)
		if item is None:
			player.debug("Cannot sell item: Invalid item ID (%d)!" % itemId)
			player.onError("Vending: Item does not exist")
			return

		if not item.canSell():
			player.debug("Cannot sell item: Item %d is not sellable!" % itemId)
			player.onError("Vending: Item is not sellable")
			return

		if item.quantity < sellQuantity:
			player.debug("Cannot sell item: Quantity too large for item %d!" % itemId)
			player.onError("Vending: Not enough items in inventory")
			return

		if item.bagId not in self.FilterBags:
			player.debug("Cannot sell item: Item %d is not in a filtered bag!" % itemId)
			player.onError("Vending: Cannot sell items from this container")
			return

		sellList = self.entity().template.sellItemList.indexedItems
		sellInfo = next((i for i in sellList if i.design.id == item.typeId), None)
		if sellInfo is None:
			player.debug("Cannot sell item: Vendor doesn't buy design ID %d!" % item.typeId)
			player.onError("Vending: Vendor doesn't buy this type of item")
			return

		# TODO: Wrap this in an InventoryTransaction block
		assert(player.inventory.removeItem(itemId, sellQuantity))
		player.inventory.addCash(sellInfo.naquadahCost * sellQuantity)

		# Notify the player that this item was removed
		if item.quantity == sellQuantity:
			player.client.onStoreUpdate([{
				'itemId': item.id,
				'sellPrice': 0,
				'repairPrice': 0,
				'rechargePrice': 0
			}])


	def onRepairItems(self, player, items):
		"""
		Indicates that the player wants to repair items at this vendor
		@param player: SGWPlayer object
		@param items: List of item IDs
		"""
		# TODO: Make this transactional!
		# (avoids multiple inventory / player updates)
		for itemId in items:
			self.onRepairItem(player, itemId)
		player.inventory.flushUpdates()


	def onRepairItem(self, player, itemId):
		"""
		Indicates that the player wants to repair an item at this vendor
		@param player: SGWPlayer object
		@param itemId: ID of item to repair (Item.id)
		"""
		item = player.inventory.getItem(itemId)
		if item is None:
			player.debug("Cannot repair item: Invalid item ID (%d)!" % itemId)
			player.onError("Vending: Item does not exist")
			return

		if item.quantity != 1:
			player.debug("Cannot repair item: Item %d is stackable!" % itemId)
			player.onError("Vending: Cannot repair stackable items")
			return

		if item.bagId not in self.FilterBags:
			player.debug("Cannot repair item: Item %d is not in a filtered bag!" % itemId)
			player.onError("Vending: Cannot repair items in this container")
			return

		repairList = self.entity().template.repairItemList.indexedItems
		repairInfo = next((i for i in repairList if i.design.id == item.typeId), None)
		if repairInfo is None:
			player.debug("Cannot repair item: Vendor doesn't repair design ID %d!" % item.typeId)
			player.onError("Vending: Vendor doesn't repair this type of item")
			return

		# TODO: Wrap this in an InventoryTransaction block
		player.inventory.removeCash(max(int(repairInfo.naquadahCost * (100 - item.durability) / 100), 1))
		item.durability = 100
		item.setDirty()

		# Notify the player that this item is not repairable anymore
		player.client.onStoreUpdate([{
			'itemId': item.id,
			'sellPrice': 0, # TODO: Get sell price
			'repairPrice': 0,
			'rechargePrice': 0 # TODO: Get recharge price
		}])


	def onRechargeItems(self, player, items):
		"""
		Indicates that the player wants to recharge items at this vendor
		@param player: SGWPlayer object
		@param items: List of item IDs
		"""
		# TODO: Make this transactional!
		# (avoids multiple inventory / player updates)
		for itemId in items:
			self.onRechargeItem(player, itemId)
		player.inventory.flushUpdates()


	def onRechargeItem(self, player, itemId):
		"""
		Indicates that the player wants to recharge an item at this vendor
		@param player: SGWPlayer object
		@param itemId: ID of item to recharge (Item.id)
		"""
		item = player.inventory.getItem(itemId)
		if item is None:
			player.debug("Cannot recharge item: Invalid item ID (%d)!" % itemId)
			player.onError("Vending: Item does not exist")
			return

		if item.quantity != 1:
			player.debug("Cannot recharge item: Item %d is stackable!" % itemId)
			player.onError("Vending: Cannot recharge stackable items")
			return

		if item.bagId not in self.FilterBags:
			player.debug("Cannot recharge item: Item %d is not in a filtered bag!" % itemId)
			player.onError("Vending: Cannot recharge items in this container")
			return

		rechargeList = self.entity().template.rechargeItemList.indexedItems
		rechargeInfo = next((i for i in rechargeList if i.design.id == item.typeId), None)
		if rechargeInfo is None:
			player.debug("Cannot recharge item: Vendor doesn't recharge design ID %d!" % item.typeId)
			player.onError("Vending: Vendor doesn't recharge this type of item")
			return

		# TODO: Wrap this in an InventoryTransaction block
		cost = max(int(rechargeInfo.naquadahCost * (item.type.charges - item.charges) / item.type.charges), 1)
		player.inventory.removeCash(cost)
		item.charges = item.type.charges
		item.setDirty()

		# Notify the player that this item is not repairable anymore
		player.client.onStoreUpdate([{
			'itemId': item.id,
			'sellPrice': 0, # TODO: Get sell price
			'repairPrice': 0, # TODO: Get repair price
			'rechargePrice': 0
		}])

