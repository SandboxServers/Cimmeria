from common.defs import Def
import Atrea.enums

class Item(object):
	# Bags that contain equipped items
	equippedBags = [
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
		Atrea.enums.INV_Artifact2
	]

	ammoTypeNames = {
		Atrea.enums.AMMO_NONE : 'AMMO_NONE',
		Atrea.enums.Bullet_Default : 'Bullet_Default',
		Atrea.enums.Bullet_Armor_Piercing : 'Bullet_Armor_Piercing',
		Atrea.enums.Bullet_Hollow_Point : 'Bullet_Hollow_Point',
		Atrea.enums.Bullet_Incendiary : 'Bullet_Incendiary',
		Atrea.enums.Bullet_EMP : 'Bullet_EMP',
		Atrea.enums.Bullet_Explosive : 'Bullet_Explosive',
		Atrea.enums.Dagger_Default : 'Dagger_Default',
		Atrea.enums.Dagger_Metallic : 'Dagger_Metallic',
		Atrea.enums.Dagger_Poison : 'Dagger_Poison',
		Atrea.enums.Dagger_Electrical : 'Dagger_Electrical',
		Atrea.enums.Dagger_Disease : 'Dagger_Disease',
		Atrea.enums.Dagger_Plasma : 'Dagger_Plasma',
		Atrea.enums.Dart_Default : 'Dart_Default',
		Atrea.enums.Dart_Poison : 'Dart_Poison',
		Atrea.enums.Dart_Disease : 'Dart_Disease',
		Atrea.enums.Dart_Tranquilizer : 'Dart_Tranquilizer',
		Atrea.enums.Dart_EMP : 'Dart_EMP',
		Atrea.enums.Dart_Radioactive : 'Dart_Radioactive',
		Atrea.enums.Dart_Stim : 'Dart_Stim',
		Atrea.enums.Dart_Coagulant : 'Dart_Coagulant',
		Atrea.enums.Dart_Nanites : 'Dart_Nanites',
		Atrea.enums.Dart_Antidote : 'Dart_Antidote',
		Atrea.enums.Dart_Adrenaline : 'Dart_Adrenaline'
	}

	# Database status flags
	# Unchanged: No change was done on the item, don't save
	DB_Unchanged = 0
	# Changed: The item was changed (quantity, slot, etc.)
	DB_Changed = 1
	# Added: The item was newly added; DBID will be updated after the item is saved to DB
	DB_Added = 2
	# Deleted: The item was deleted from the inventory, but not yet committed to DB
	DB_Deleted = 3

	# Local ID of the item (may differ from item_id in the database if the item is locally created)
	id = 0
	# Database ID of the item (-1 if the item doesn't have an ID yet)
	databaseId = -1
	characterId = 0
	typeId = 0
	quantity = 0
	slotId = 0
	bagId = 0
	bound = False
	durability = 0
	ammoTypes = []
	ammoType = 0
	ammo = 0
	charges = 0
	type = None
	dirty = False
	dbStatus = DB_Unchanged
	buybackPrice = 0

	def __init__(self):
		super().__init__()

	@staticmethod
	def create(typeId):
		type = Def.DefMgr.get('item', typeId)
		assert(type is not None)
		item = Item()
		item.typeId = type.id
		item.type = type
		item.ammoType = type.defaultAmmoTypeId or Atrea.enums.AMMO_NONE
		# TODO: What ammo types does the item have when created? Add initialAmmoTypes?
		item.ammoTypes = type.ammoTypeIds
		return item
		
	def backup(self):
		return (self.typeId, self.id, self.databaseId, self.characterId, self.quantity, self.slotId,
			self.bagId, self.bound, self.durability, self.ammoTypes, self.ammoType, self.charges,
			self.dbStatus)
	
	@staticmethod
	def restore(backup):
		item = Item.create(backup[0])
		item.typeId, item.id, item.databaseId, item.characterId, item.quantity, item.slotId, \
			item.bagId, item.bound, item.durability, item.ammoTypes, item.ammoType, item.charges, \
			item.dbStatus = backup
		return item

	def load(self, row):
		self.id = row['item_id']
		self.characterId = row['character_id']
		self.databaseId = row['item_id']
		self.typeId = row['type_id']
		self.quantity = row['stack_size']
		self.slotId = row['slot_id']
		self.bagId = row['container_id']
		self.bound = (row['bound'] == 1)
		self.durability = row['durability']
		self.ammoTypes = [Atrea.enums.__dict__[t] for t in row['ammo_types'][1:-1].split(',') if t]
		self.ammoType = Atrea.enums.__dict__[row['ammo_type']]
		self.ammo = row['ammo']
		self.charges = row['charges']
		self.dbStatus = self.DB_Unchanged

	def save(self):
		"""
		Commit all changes performed on this item to the database and reset item database state to DB_Unchanged
		"""
		ammoType = repr(self.ammoTypeNames[self.ammoType])
		ammoTypes = repr('{' + ','.join([self.ammoTypeNames[t] for t in self.ammoTypes]) + '}')
		if self.dbStatus == self.DB_Changed:
			assert(self.databaseId != -1)
			Atrea.dbQuery(('update sgw_inventory set stack_size=%d, charges=%d, container_id=%d, slot_id=%d, ' +
				'durability=%d, ammo_type=%s, type_id=%d, flags=%d, character_id=%d, bound=%s, ammo_types=%s, ammo=%d '
				'where item_id=%d') %
				(self.quantity, self.charges, self.bagId, self.slotId, self.durability, ammoType,
				self.typeId, 0, self.characterId, repr(self.bound), ammoTypes, self.ammo, self.databaseId))
		elif self.dbStatus == self.DB_Added:
			row = Atrea.dbQuery(('insert into sgw_inventory (stack_size, charges, container_id, slot_id, durability, ' +
			    'ammo_type, type_id, flags, character_id, bound, ammo_types, ammo) values (%d, %d, %d, %d, %d, %s, %d, ' +
			    '%d, %d, %s, %s, %d) returning item_id') %
				(self.quantity, self.charges, self.bagId, self.slotId, self.durability, ammoType,
				self.typeId, 0, self.characterId, repr(self.bound), ammoTypes, self.ammo))
			self.databaseId = row[0]['item_id']
		elif self.dbStatus == self.DB_Deleted:
			Atrea.dbQuery('delete from sgw_inventory where item_id=%d' % self.databaseId)
		self.dbStatus = self.DB_Unchanged

	def setCreated(self):
		"""
		Marks the item as dirty and locally created
		"""
		assert(self.dbStatus == self.DB_Unchanged or self.dbStatus == self.DB_Added)
		self.dirty = True
		self.dbStatus = self.DB_Added

	def setDeleted(self):
		"""
		Marks the item as dirty and locally deleted
		"""
		self.dirty = True
		if self.dbStatus == self.DB_Unchanged or self.dbStatus == self.DB_Changed:
			self.dbStatus = self.DB_Deleted
		elif self.dbStatus == self.DB_Added:
			# If the item is newly added, but wasn't persisted yet, then we don't
			# have to do any db operations at all
			self.dbStatus = self.DB_Unchanged

	def setDirty(self):
		"""
		Marks the item as dirty both locally and in the database
		"""
		self.dirty = True
		assert(self.dbStatus != self.DB_Deleted)
		if self.dbStatus == self.DB_Unchanged:
			self.dbStatus = self.DB_Changed

	def hasTemporaryId(self):
		"""
		Checks if the item has a locally assigned temporary Id
		@return: True if the item has a temporary databaseId, False otherwise
		"""
		return self.dbStatus == self.DB_Added

	def canDelete(self):
		"""
		Checks if the item can be deleted
		@return: True if the item can be deleted; False otherwise
		"""
		return not self.bound and self.type.deletable

	def canSell(self):
		"""
		Checks if the item can be sold
		@return: True if the item can be sold; False otherwise
		"""
		return not self.bound and self.type.sellable

	def canMoveTo(self, player, bag):
		"""
		Checks if the item can be moved to the specified bag
		@param player: Player that will receive the item
		@type player: SGWPlayer
		@param bag: Bag that will receive the item
		@type bag: Bag
		@return: True if the item can be moved; false otherwise
		"""
		if self.bound:
			return False

		if bag.bagId in self.equippedBags and not self.canEquip(player):
			return False

		return bag.bagId in [cont.id for cont in self.type.containers]

	def canEquip(self, player):
		"""
		Checks if the player can equip the item
		@param player: Player to check against
		@type player: SGWPlayer
		@return: True if the item can be equipped; false otherwise
		"""
		# FIXME: Temporarily disabled until we can find out whats wrong with disciplines
		#for discipline in self.type.disciplines:
		#	if discipline not in player.disciplines:
		#		return False
		return True

	def toInvItem(self):
		"""
		Returns a dict of this item that can be sent via client RPC
		"""
		return {
			'id': self.id,
			'dbid': self.typeId,
			'stackSize': self.quantity,
			'slotID': self.slotId + 1,
			'containerID': self.bagId,
			'isBound': self.bound,
			'durability': self.durability,
			'ammoTypes': self.ammoTypes,
			'curAmmoType': self.ammoType,
			'charges': self.charges
		}

