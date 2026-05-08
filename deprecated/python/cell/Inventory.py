from cell.Bag import Bag
from cell.Item import Item
import weakref
import Atrea.enums
from cell.SGWSpawnableEntity import SGWSpawnableEntity
from common import Constants
from common.Constants import BAG_SIZES
from common.Logger import trace, warn, info
from common.defs.Def import DefMgr

class Inventory(object):
	# Amount of cash we have
	naquadah = 0
	# Available bags
	bags = {}
	# LocalId:object pairs of stored items
	items = {}
	# List of items that should be deleted
	itemsPendingDeletion = []
	# Reference to the player object
	player = None
	# Indicates that the bag list is dirty and should be updated at the client
	bagsDirty = False
	# Indicates that the inventory is dirty and should be updated at the client
	# TODO: Implement partial updates for inventory!
	dirty = False
	# The visual components of the player were changed and need to be updated
	visualsDirty = False
	# List of item indices that were removed and were not yet sent to the player
	unseenRemovedItems = []
	# Next temporary item identifier to use
	nextId = 1
	# Maximal temporary identifier in use by the server
	MAX_TEMP_ID = 9999


	def __init__(self, player, naquadah : int):
		"""
		Creates the players inventory
		@param player: Player that the inventory belongs to
		@type player: SGWPlayer
		@param naquadah: Amount of cash the player has
		"""
		super().__init__()
		self.player = weakref.ref(player)
		self.naquadah = naquadah
		self.cashDirty = True
		# Initialize default bags
		self.bags = {}
		for bagId in range(1, Atrea.enums.INV_CommandBank):
			self.bags[bagId] = Bag(bagId, BAG_SIZES[bagId], self)
		self.items = {}
		self.itemsPendingDeletion = []
		self.bagsDirty = True
		self.unseenRemovedItems = []


	def backup(self):
		"""
		Makes a persistent copy of the inventory that can be restored later.
		@return: Persistent data of the inventory
		"""
		backup = {
			'naq': self.naquadah,
			'naqDirty': self.cashDirty,
			'bags': {bagId : bag.backup() for bagId, bag in self.bags.items()},
			'items': [item.backup() for item in self.items.values()],
			'deletion': [item.backup() for item in self.itemsPendingDeletion]
		}
		return backup


	def restore(self, backup):
		"""
		Restore a previously backed up copy of this inventory.
		@param backup: Backed up inventory data
		"""
		self.naquadah = backup['naq']
		self.cashDirty = backup['naqDirty']
		for item in backup['items']:
			i = Item.restore(item)
			self.items[i.id] = Item.restore(item)
		# We need to restore bags after the items, as bag slots
		# resolve inventory item refs using their ID
		for bagId, bag in backup['bags'].items():
			self.bags[bagId].restore(bag)
		for item in backup['deletion']:
			self.itemsPendingDeletion.append(Item.restore(item))


	def loadItems(self):
		"""
		Remove all items from the inventory and reload all items from database
		@return:
		"""
		self.items = {}
		self.itemsPendingDeletion = []
		self.unseenRemovedItems = []
		# Empty all bags to avoid item duplication
		for bagId in self.bags:
			self.bags[bagId].clean()
		
		# Query all items
		items = Atrea.dbQuery("SELECT * FROM sgw_inventory WHERE character_id = " + str(self.player().databaseId))
		for row in items:
			item = Item.create(row['type_id'])
			item.load(row)
			self.loadItem(item)
		self.dirty = True


	def loadItem(self, item : Item):
		"""
		Adds an item loaded from the database to its container
		@param item: Item to add
		@type item: Item
		"""
		if item.type is None:
			warn(Atrea.enums.LC_Item, 'Unable to load item #%d: invalid type ID' % item.id)
			return

		if item.bagId in self.bags:
			if self.bags[item.bagId].addDbItem(item):
				self.items[item.id] = item
		else:
			warn(Atrea.enums.LC_Item, 'Unable to load item #%d: invalid bag ID %d' % (item.id, item.bagId))


	def saveItems(self):
		"""
		Saves all changed/added/deleted items to the database and resets their DB status flags.
		"""
		# Save and forget all items that should have been deleted
		for item in self.itemsPendingDeletion:
			item.save()
		del self.itemsPendingDeletion[:]

		# Save all items still in the inventory
		for item in self.items.values():
			# Buyback items are only available until the player logs out
			if item.bagId != Atrea.enums.INV_Buyback and item.id != -1:
				item.save()


	def findItem(self, designId : int, bagId : int = None, mainBagsOnly : bool = True) -> Item:
		"""
		Searches the inventory for occurrences of the specified design and returns the
		first matched stack. If mainBagsOnly = False, the main bags are searched first,
		and if no match was found, the equipment bags are searched next.

		@param designId: Design to search for
		@param bagId: Which bag to search (None = all bags)
		@param mainBagsOnly: Search only main/mission/crafting or equipment bags as well?
		@return: Found stack
		"""
		if bagId is not None:
			bags = [bagId]
		elif mainBagsOnly:
			bags = [Atrea.enums.INV_Main, Atrea.enums.INV_Mission, Atrea.enums.INV_Crafting]
		else:
			bags = self.bags.keys()

		for bag in bags:
			stack = self.bags[bag].findItem(designId)
			if stack is not None:
				return stack

		return None


	def pickedUpItem(self, designId : int, quantity : int) -> bool:
		"""
		Adds new item(s) to one or more slots in the inventory.
		@param designId: ID of item definition to add
		@param quantity: Amount of items to add
		@return: True if all items were added; False otherwise
		"""
		# Check if the item can be added to the main/mission/crafting inventory
		itemDef = DefMgr.get('item', designId)
		if itemDef is None:
			warn(Atrea.enums.LC_Item, 'Cannot pick up item design %d: Invalid design ID' % designId)
			return False

		bagId = itemDef.defaultBag()
		if bagId is None:
			# The item doesn't list main/mission/crafting containers as allowed containers, so
			# we can't put this item anywhere
			warn(Atrea.enums.LC_Item, 'Cannot pick up item design %d: Main/mission/crafting container disallowed' % designId)
			return False

		if self.addItem(bagId, designId, quantity):
			# TODO: Add slotId to event
			info(Atrea.enums.LC_Item, 'Player <%s> picked up %d of %d' % (self.player().getName(), quantity, designId))
			self.player().first('item.pickup::' + str(designId), player = self.player(), bagId = bagId, quantity = quantity)
			return True
		else:
			return False


	def addItem(self, bagId : int, designId : int, quantity : int) -> bool:
		"""
		Adds new item(s) to one or more slots in the bag
		@param bagId: Target bag (EInventoryContainerId enum)
		@param designId: ID of item definition to add
		@param quantity: Amount of items to add
		@return: True if all items were added; False otherwise
		"""
		if bagId not in self.bags:
			self.player().debug('Unable to add item design #%d: invalid bag ID %d' %(designId, bagId))
			return False

		# TODO: We need to check if the bag can hold the specified design
		if self.bags[bagId].addItem(designId, quantity):
			self.dirty = True
			return True
		else:
			return False


	def categorizeItemsByBag(self, items):
		"""
		Categorize items based on the bag the items belong to
		@param items: {designId: quantity} dict
		@return: {bagId: {designId: quantity, ...}} dict
		"""
		bags = {}
		for designId, quantity in items.items():
			design = DefMgr.get('item', designId)
			bagId = design.defaultBag()
			if bagId is None:
				warn(Atrea.enums.LC_Item, 'Cannot pick up item design %d: No default bag' % designId)
				return False
			if bagId not in bags:
				bags[bagId] = {}
			bags[bagId][designId] = quantity
		return bags


	def canAddItems(self, bags):
		"""
		Adds the specified items to the players inventory
		@param bags: {bagId: {designId: quantity, ...}} dict
		@return: True if the items were added; False otherwise
		"""
		# Check if each bag has sufficient space
		for bagId, items in bags.items():
			if not self.bags[bagId].canAddItems(items):
				return False

		return True


	def addItems(self, items):
		"""
		Adds the specified items to the players inventory
		@param items: {designId: quantity} dict
		@return: True if the items were added; False otherwise
		"""
		bags = self.categorizeItemsByBag(items)
		if not self.canAddItems(bags):
			return False

		# Add items to the bags
		for bagId, items in bags.items():
			for designId, quantity in items.items():
				assert self.bags[bagId].addItem(designId, quantity)

		self.dirty = True
		return True


	def removeItemByDesign(self, designId : int, quantity : int, force : bool = False) -> bool:
		"""
		Removes items by type from the inventory
		@param designId: Item type (design ID)
		@param quantity: Quantity to remove
		@param force: Is the item deletion forced? (eg. from a mission script)
		@return: True if the item was removed; False otherwise
		"""
		if quantity < 0:
			warn(Atrea.enums.LC_Item, 'removeItemByDesign(): Bad quantity %d' % quantity)
			return False
		elif quantity == 0:
			return True

		# Check if we have enough items to remove
		items = [i for i in self.items.values() if i.typeId == designId]
		maxQuantity = sum((i.quantity for i in items), 0)
		if maxQuantity < quantity:
			trace(Atrea.enums.LC_Item, 'Not enough items to remove of design %d (has %d, need %d)' %
				(designId, maxQuantity, quantity))
			return False

		# Remove items in slots one by one until we either run out of items
		# or removed the required number of items
		remaining = quantity
		for item in items:
			if item.quantity > remaining:
				item.quantity -= remaining
				item.setDirty()
				break
			else:
				remaining -= item.quantity
				assert self.removeItem(item.id, item.quantity, force)

		return True


	def removeItem(self, itemId : int, quantity : int, force : bool = False) -> bool:
		"""
		Removes an item from the inventory
		@param itemId: ID of item to remove
		@param quantity: Quantity to remove
		@param force: Is the item deletion forced? (eg. from a mission script)
		@return: True if the item was removed; False otherwise
		"""
		if quantity < 0:
			warn(Atrea.enums.LC_Item, 'Bad remove quantity %d' % quantity)
			return False
		elif quantity == 0:
			return True

		if itemId not in self.items:
			warn(Atrea.enums.LC_Item, 'Unable to remove nonexistent item #%d' % itemId)
			return False

		item = self.items[itemId]
		if quantity > item.quantity:
			warn(Atrea.enums.LC_Item, 'Tried to remove more items than the stack has (%d)' % quantity)
			return False

		if not force and not item.canDelete():
			warn(Atrea.enums.LC_Item, 'Unable to delete item #%d: deletion not allowed' % itemId)
			return False

		removed = self.bags[item.bagId].removeItem(item.slotId, quantity)
		if removed is not None:
			removed.setDeleted()
			self.itemsPendingDeletion.append(removed)
			del self.items[removed.id]
			self.unseenRemovedItems.append(removed.id)
		return True


	def moveItem(self, itemId : int, targetBagId : int, targetSlot : int, quantity : int) -> bool:
		"""
		Moves an item from one slot to another
		@param itemId: Unique Id of the item
		@param targetBagId: Item will be moved to this bag
		@param targetSlot: Item will be moved to this slot
		@param quantity: Amount of items to move
		@return: True if the item was moved successfully, False otherwise
		"""
		if quantity < 0:
			warn(Atrea.enums.LC_Item, 'Bad move quantity: %d' % quantity)
			return False
		elif quantity == 0:
			return True

		if itemId not in self.items:
			warn(Atrea.enums.LC_Item, 'Unable to move nonexistent item #%d' % itemId)
			return False

		if targetBagId not in self.bags:
			warn(Atrea.enums.LC_Item, 'Unable to move item #%d: invalid bag ID %d' % (itemId, targetBagId))
			return False

		if not self.bags[targetBagId].isValidSlot(targetSlot):
			warn(Atrea.enums.LC_Item, 'Unable to move item #%d: invalid slot ID %d' % (itemId, targetSlot))
			return False

		item = self.items[itemId]
		if item.bagId == targetBagId and item.slotId == targetSlot:
			trace(Atrea.enums.LC_Item, 'Ignored move for item %d (tried to move to the same slot)' % itemId)
			return True

		if not item.canMoveTo(self.player(), self.bags[targetBagId]):
			trace(Atrea.enums.LC_Item, 'Unable to move item #%d: container not allowed or discipline missing' % itemId)
			# Force the client to move the item back to its original location
			item.dirty = True
			self.dirty = True
			return False

		if self.bags[targetBagId].isSlotEmpty(targetSlot):
			# Destination slot is empty, move item
			removed = self.bags[item.bagId].removeItem(item.slotId, quantity)
			self.bags[targetBagId].putItem(removed, targetSlot)
		elif quantity == item.quantity:
			# Get the item occupying the destination slot
			dstItem = self.bags[targetBagId].getItem(targetSlot)
			if dstItem.typeId == item.typeId and dstItem.quantity + item.quantity <= item.type.maxStackSize:
				# The two stacks are of the same type and the free stack space is large enough --> combine them
				dstItem.quantity += item.quantity
				dstItem.setDirty()
				self.bags[item.bagId].removeItem(item.slotId, item.quantity)
			else:
				# Destination slot is occupied, swap items
				# Check if the item being removed can be swapped to the old items slot
				# (if not, abort)
				if not dstItem.canMoveTo(self, self.bags[item.bagId]):
					trace(Atrea.enums.LC_Item, 'Unable to swap with item #%d: container not allowed or discipline missing' % itemId)
					# Force the client to move the item back to its original location
					item.setDirty()
					self.dirty = True
					return False

				srcBag, srcSlot = self.bags[item.bagId], item.slotId
				removed = srcBag.removeItem(srcSlot, quantity)
				removedTarget = self.bags[dstItem.bagId].removeItem(dstItem.slotId, dstItem.quantity)
				self.bags[targetBagId].putItem(removed, targetSlot)
				srcBag.putItem(removedTarget, srcSlot)
		else:
			raise NotImplementedError("Tried to split a stack to an occupied inventory slot - this is not handled yet!")

		self.dirty = True
		return True


	def useItem(self, itemId : int, target : SGWSpawnableEntity) -> bool:
		"""
		Performs an "use" action on the specified item
		(The actual action performed depends on the item class; the default Item does nothing)
		@param itemId: ID of item to use
		@param target: Target entity or None
		"""
		if itemId not in self.items:
			warn(Atrea.enums.LC_Item, 'Unable to use nonexistent item #%d' % itemId)
			return False

		item = self.items[itemId]
		trace(Atrea.enums.LC_Item, "Using item %d <%s> on %d" % (itemId, item.type.name, target.entityId if target else 0))
		self.player().first('item.use::' + str(item.typeId), player = self.player(), item = item, target = target)


	def updateActiveSlot(self, bagId : int, slotId : int) -> bool:
		"""
		Changes the active slot in the bag (and the equipped item, if its an equipped container)
		@param bagId: Bag whose active slot changed
		@param slotId: New active slot ID
		@return: True if slot change is successful, False otherwise
		"""
		if bagId not in self.bags:
			warn(Atrea.enums.LC_Item, 'Unable to set active slot in invalid bag ID %d' % bagId)
			return False

		if not self.bags[bagId].isValidSlot(slotId) or self.bags[bagId].getItem(slotId) is None:
			warn(Atrea.enums.LC_Item, 'Unable to set active slot to invalid slot ID %d' % slotId)
			return False

		# Update visual components if an equipped item was added/removed
		bag = self.bags[bagId]
		oldItem = bag.getItem(bag.activeSlotId)
		if oldItem is not None:
			if oldItem.type.visualComponent in self.player().components:
				trace(Atrea.enums.LC_Visuals, '<%s> Removed visual component %s' % (self.player().getName(), oldItem.type.visualComponent))
				self.player().components.remove(oldItem.type.visualComponent)
				self.visualsDirty = True
			self.player().onItemUnequipped(bag, slotId, oldItem)

		newItem = bag.getItem(slotId)
		if newItem is not None:
			if newItem.type.visualComponent is not None:
				trace(Atrea.enums.LC_Visuals, '<%s> Added visual component %s' % (self.player().getName(), newItem.type.visualComponent))
				self.player().components.append(newItem.type.visualComponent)
				self.visualsDirty = True
			self.player().onItemEquipped(bag, slotId, newItem)

		bag.setActiveSlot(slotId)
		self.player().onActiveSlotChanged(bag.bagId, slotId)
		return True


	def getBag(self, bagId : int) -> Bag:
		return self.bags.get(bagId)


	def getItem(self, itemId : int) -> Item:
		return self.items.get(itemId)


	def onSlotUpdate(self, bag : Bag, slotId : int, oldItem : Item, newItem : Item):
		"""
		Called by a bag when the contents of a slot was updated
		@param bag: Bag that was updated
		@param slotId: Slot that was updated
		@param oldItem: Old item in the slot
		@param newItem: New item in the slot
		"""
		trace(Atrea.enums.LC_Item, 'Slot update: bag %d, slot %d, item %s -> %s' %
								   (bag.bagId, slotId, oldItem.typeId if oldItem else 0, newItem.typeId if newItem else 0))
		if bag.bagId in Item.equippedBags:
			# Add/remove player visual components
			if oldItem is not None and bag.activeSlotId == slotId and oldItem.type.visualComponent in self.player().components:
				trace(Atrea.enums.LC_Visuals, '<%s> Removed visual component %s' % (self.player().getName(), oldItem.type.visualComponent))
				self.player().components.remove(oldItem.type.visualComponent)
				self.visualsDirty = True
				self.player().onItemUnequipped(bag, slotId, oldItem)
			if newItem is not None and bag.activeSlotId == slotId and newItem.type.visualComponent is not None:
				trace(Atrea.enums.LC_Visuals, '<%s> Added visual component %s' % (self.player().getName(), newItem.type.visualComponent))
				self.player().components.append(newItem.type.visualComponent)
				self.visualsDirty = True
				self.player().onItemEquipped(bag, slotId, newItem)

		if bag.bagId == Atrea.enums.INV_Bandolier and bag.activeSlotId == slotId:
			# We need to select a different active slot if the item in the currently active slot was removed
			# This is currently hardcoded to slot 1 / slotId 0 (as it always contains fists)
			# FIXME: This will cause weapon swaps in the active slot to reset the active slot ID
			# This could be avoided by making the inventory completely transactional (which will be needed anyway)
			if newItem is None:
				self.updateActiveSlot(bag.bagId, 0)


	def setAllDirty(self):
		"""
		Marks all items as dirty.
		Used to send a full inventory update to the client.
		"""
		for item in self.items.values():
			item.dirty = True


	def addCash(self, amount : int):
		"""
		Adds naquadah to the players cash pool
		@param amount: Amount of cash to give (>= 0)
		"""
		assert(amount >= 0)
		self.naquadah += amount
		self.cashDirty = True
		trace(Atrea.enums.LC_Cash, '<%s> Gained %d naquadah' % (self.player().getName(), amount))


	def removeCash(self, amount : int):
		"""
		Removes naquadah from the players cash pool
		@param amount: Amount of cash to remove (>= 0)
		"""
		assert(amount >= 0)
		assert(amount <= self.naquadah)
		self.naquadah -= amount
		self.cashDirty = True
		trace(Atrea.enums.LC_Cash, '<%s> Lost %d naquadah' % (self.player().getName(), amount))


	def flushUpdates(self):
		"""
		Send updates to the client about inventory status.
		This should be called after each inventory update batch (moving items to a different slot,
		deleting items, trading, items given by a mission giver, looting, ...)
		"""
		if self.bagsDirty:
			baglist = []
			for bag in self.bags.values():
				baglist.append({'BagId': bag.bagId, 'NumberOfSlots': bag.slots})
		
			self.player().client.onBagInfo(baglist)
			self.bagsDirty = False

		for bag in self.bags.values():
			bag.flushUpdates(self.player())
			
		if self.dirty:
			items = []
			for bagId in self.bags:
				self.bags[bagId].serializeItems(items)
			if len(items):
				self.player().client.onUpdateItem(items)
			self.dirty = False
			
		if self.cashDirty:
			self.player().client.onCashChanged(self.naquadah)
			self.cashDirty = False

		if len(self.unseenRemovedItems):
			self.player().client.onRemoveItem(self.unseenRemovedItems)
			self.unseenRemovedItems = []

		if self.visualsDirty:
			self.player().onVisualsUpdated()


	def allocateItemId(self, item : Item):
		"""
		Allocates a temporary identifier for an item.
		This is needed because items aren't persisted to the DB at the time they are created on the server,
		so the server needs to be able to assign a local identifier to them.
		"""
		while True:
			self.nextId += 1
			if self.nextId > self.MAX_TEMP_ID:
				self.nextId = 1
			if self.nextId not in self.items:
				break

		# self.nextId contains an unused identifier when we get here
		self.items[self.nextId] = item
		item.id = self.nextId


