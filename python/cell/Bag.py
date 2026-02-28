from cell.Item import Item
import Atrea.enums
import weakref
from common.Logger import trace, error
import common.defs.Item
from common.defs.Def import DefMgr

class Bag:
	# Type id of this bag
	bagId = 0
	# Size of bag
	slots = 0
	# Index of active slot in bag
	activeSlotId = 0
	# Active slot has changed since last update
	activeSlotDirty = True
	# Available slots
	items = []
	# Ref to player inventory
	inventory = None

	def __init__(self, bagId : int, slots : int, inventory):
		self.bagId = bagId
		self.items = []
		self.inventory = weakref.ref(inventory)
		self.resize(slots)
		self.clean()


	def backup(self):
		"""
		Makes a persistent copy of this bag that can be restored later.
		@return: Persistent data of this bag
		"""
		items = []
		for item in self.items:
			if item is None or item.id == -1:
				items.append(None)
			else:
				items.append(item.id)
		return self.slots, self.activeSlotId, items


	def restore(self, backup):
		"""
		Restore a previously backed up copy of this bag.
		@param backup: Backed up bag data
		"""
		slots, self.activeSlotId, items = backup
		self.resize(slots)
		for i in range(0, len(items)):
			if items[i] is not None:
				self.items[i] = self.inventory().items[items[i]]


	def isValidSlot(self, slotId : int) -> bool:
		"""
		Checks if the specified slot ID is valid for this bag
		@param slotId: Slot ID to check
		@return: True if valid; False otherwise
		"""
		return 0 <= slotId < self.slots


	def isSlotEmpty(self, slotId : int) -> bool:
		"""
		Checks if the specified slot ID is occupied in the bag
		@param slotId: Slot ID to check
		@return: True if empty; False otherwise
		"""
		return self.items[slotId] is None


	def canResize(self, size : int) -> bool:
		"""
		Checks if the bag can be resized to the specified size.
		@param size: New bag size
		@return: False, if the more than "size" slots are occupied; True otherwise
		"""
		occupied = sum(1 for item in self.items if item)
		if occupied > size:
			return False
		else:
			return True


	def setActiveSlot(self, slotId : int):
		assert(0 <= slotId < self.slots)
		if self.activeSlotId != slotId:
			self.activeSlotDirty = True
			self.activeSlotId = slotId


	def resize(self, size : int):
		"""
		Resizes the bag and moves items from removed slots if necessary.
		@param size: New bag size
		"""
		assert(self.canResize(size))
		# Move items from deleted slots to free slots
		nextFree = 0
		for i in range(size, self.slots):
			if self.items[i] is not None:
				for j in range(nextFree, self.slots):
					if self.items[j] is None:
						self.items[j] = self.items[i]
						nextFree = j
						break

		if self.slots > size:
			# Remove slots that were deleted because of the bag shrinking
			del self.items[self.slots:size]

		# Add new (empty) bag slots
		for i in range(self.slots, size):
			self.items.append(None)

		self.slots = size


	def clean(self):
		"""
		Removes all items from the bag
		"""
		self.items[:] = []
		for i in range(0, self.slots):
			self.items.append(None)


	def getItem(self, slotId : int) -> Item:
		"""
		Returns the item in the specified slot, or None if the slot is empty.

		@param slotId: ID of slot
		@return: Item in slot
		"""
		assert(0 <= slotId < self.slots)
		return self.items[slotId]


	def getActiveItem(self) -> Item:
		"""
		Returns the item in the active inventory slot
		@return: Active item
		"""
		return self.items[self.activeSlotId]


	def findItem(self, designId : int) -> Item:
		"""
		Searches the bag for occurrences of the specified design and returns the
		first matched stack.

		@param designId: Design to search for
		@return: Found stack
		"""
		for stack in self.items:
			if stack is not None:
				if stack.typeId == designId:
					return stack

		return None


	def addDbItem(self, item : Item) -> bool:
		"""
		Loads (and adds) an item from the database
		@param item: Item to add
		@type item: Item
		"""
		if item.slotId >= self.slots:
			error(Atrea.enums.LC_Item, 'Unable to load item %d: invalid slot ID %d for bag ID %d' % (item.id, item.slotId, self.bagId))
			return False

		if self.items[item.slotId] is not None:
			error(Atrea.enums.LC_Item, 'Unable to load item #%d: slot ID %d is already occupied for bag ID %d' % (item.id, item.slotId, self.bagId))
			return False

		self.items[item.slotId] = item
		item.dirty = True
		self.inventory().onSlotUpdate(self, item.slotId, None, item)
		return True


	def addItem(self, designId : int, quantity : int) -> bool:
		"""
		Adds new item(s) to one or more slots in the bag
		@param designId: ID of item definition to add
		@param quantity: Amount of items to add
		@return: True if all items were added; False otherwise
		"""
		assert(quantity >= 0)
		if quantity == 0:
			return True

		design = DefMgr.get('item', designId)
		if design is None:
			error(Atrea.enums.LC_Item, 'Invalid item def id when adding item: %d' % designId)
			return False

		# Check if the bag can hold the specified design
		if not self.canAddItem(design, quantity):
			return False

		# First pass - check if we have slots that contain the same designId, and grow their stacks
		# This is more intuitive than filling the inventory with new stacks from the first slot
		for slot in self.items:
			if slot is not None and slot.typeId == design.id:
				# Try to top up the stack; if we run out of items then exit the loop early
				# as the second pass would add an empty stack to the bag otherwise
				add = design.maxStackSize - slot.quantity
				if add > quantity:
					add = quantity
				if add > 0:
					slot.quantity += add
					slot.setDirty()
					quantity -= add
					if quantity == 0:
						return True

		# Second pass - create new stacks until we run out of items
		for i in range(0, len(self.items)):
			if self.items[i] is None:
				add = design.maxStackSize
				if add > quantity:
					add = quantity
				# TODO: Centralize item allocation
				# Inventory.allocateItem(designId, quantity) ?
				item = Item.create(designId)
				item.quantity += add
				self.inventory().allocateItemId(item)
				item.characterId = self.inventory().player().playerId
				item.bagId = self.bagId
				item.slotId = i
				quantity -= add
				item.setCreated()
				self.items[i] = item
				# Notify the inventory that we added a new item stack
				self.inventory().onSlotUpdate(self, i, None, item)
				if quantity == 0:
					return True

		raise AssertionError('addItem() failed to add all items to bag!')


	def canAddItems(self, items):
		"""
		Checks if the specified items can be added to the bag
		@param items: {designId: quantity} dict
		@return: True if the bag can hold the items; False otherwise
		"""
		freeSlots = sum(1 for item in self.items if not item)
		slots = 0
		for designId, quantity in items.items():
			design = DefMgr.get('item', designId)
			itemSlots, stackQty = self.getSpaceRequirements(design, quantity)
			slots += itemSlots

		return slots <= freeSlots


	def getSpaceRequirements(self, item, quantity):
		"""
		Checks if the specified item can be added to the bag
		@param item: Item definition
		@param quantity: Amount of items to add
		@return: (slots, stackedQuantity)
		"""
		slots, stacked, added = 0, 0, 0
		for slot in self.items:
			if slot is None:
				added += item.maxStackSize
				slots += 1
			elif slot.typeId == item.id:
				added += item.maxStackSize - slot.quantity
				stacked += item.maxStackSize - slot.quantity
			if added >= quantity:
				break

		return slots, stacked


	def canAddItem(self, item : common.defs.Item.Item, quantity : int) -> bool:
		"""
		Checks if the specified item can be added to the bag
		@param item: Item definition
		@param quantity: Amount of items to add
		@return: True if the bag can hold the items; False otherwise
		"""
		added = 0
		for slot in self.items:
			if slot is None:
				added += item.maxStackSize
			elif slot.typeId == item.id:
				added += item.maxStackSize - slot.quantity
			if added >= quantity:
				return True

		trace(Atrea.enums.LC_Item, 'Not enough space in bag to add item (has space for %d items, needed %d)' % (added, quantity))
		return False


	def putItem(self, item : Item, slot : int) -> bool:
		if slot < 0 or slot >= self.slots:
			error(Atrea.enums.LC_Item, 'Unable to put item #%d: invalid slot ID %d for bag ID %d' % (item.id, slot, self.bagId))
			return False

		if self.items[slot] is not None:
			error(Atrea.enums.LC_Item, 'Unable to put item #%d: slot ID %d is occupied in bag ID %d' % (item.id, slot, self.bagId))
			return False

		self.items[slot], item.bagId, item.slotId = item, self.bagId, slot
		item.setDirty()
		self.inventory().onSlotUpdate(self, item.slotId, None, item)
		return True


	def removeItem(self, slot : int, quantity : int) -> Item:
		"""
		Removes the item from the selected slot and returns the removed item.
		If the quantity is less than the amount of items in the slot, the stack is split
		@param slot: Slot to remove from
		@param quantity: Quantity to remove
		@return: Item, or None if removal fails
		"""
		if quantity < 1:
			error(Atrea.enums.LC_Item, 'Bad split quantity %d' % quantity)
			return None

		if slot < 0 or slot >= self.slots:
			error(Atrea.enums.LC_Item, 'Unable to remove item: invalid slot ID %d for bag ID %d' % (slot, self.bagId))
			return None

		if self.items[slot] is None:
			error(Atrea.enums.LC_Item, 'Unable to remove item: slot ID %d is not occupied in bag ID %d' % (slot, self.bagId))
			return None

		item = self.items[slot]
		if quantity > item.quantity:
			error(Atrea.enums.LC_Item, 'Tried to split %d items from a stack of %d' % (quantity, item.quantity))
			return None

		if item.quantity == quantity:
			self.items[slot] = None
			self.inventory().onSlotUpdate(self, slot, item, None)
			item.bagId, item.slotId, item.dirty = None, None, True
			return item
		else:
			# Subtract quantity from old item
			item.quantity -= quantity
			item.setDirty()

			# Allocate the quantity to a new item stack
			newItem = Item.create(item.typeId)
			self.inventory().allocateItemId(newItem)
			newItem.characterId = self.inventory().player().playerId
			newItem.quantity = quantity
			newItem.setCreated()
			return newItem


	def flushUpdates(self, player):
		"""
		Checks if the active slot changed and flushes slot changes if it did.
		@param player: Owner of the bag
		@type player: SGWPlayer
		"""
		if self.activeSlotDirty:
			player.client.onActiveSlotUpdate(self.bagId, self.activeSlotId + 1)
			self.activeSlotDirty = False


	def serializeItems(self, items : list):
		"""
		Serialize items in this bag for sending to a client
		@param items: List to serialize to
		"""
		for item in self.items:
			if item is not None and item.dirty:
				items.append(item.toInvItem())
				item.dirty = False
			
