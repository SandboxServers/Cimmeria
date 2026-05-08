from math import floor
from random import randint, random
import weakref
import Atrea
import Atrea.enums
from common import Constants
from common.Logger import warn, trace
from common.defs.Def import DefMgr


class Crafter(object):
	# Amount of discipline training points available
	appliedSciencePoints = 0

	# Known disciplines (disciplineId -> expertise map)
	disciplines = None
	# Racial paradigm levels (paradigmId -> level map)
	racialParadigms = None
	# Known blueprint IDs
	blueprints = None

	# Timer for current action
	timer = None

	# Item we're currently crafting
	craftingBlueprintId = None
	# Amount of items we're crafting
	craftingQuantity = 0

	# Researching will give expertise in this discipline
	researchDiscipline = None
	# Amount of expertise gained
	reserarchExpertise = 0

	# Reverse engineering gained items
	revEngItems = None

	# Item we're currently alloying
	alloyingBlueprintId = None

	def __init__(self, entity):
		self.entity = weakref.ref(entity)
		self.disciplines = {}
		self.racialParadigms = {}
		self.blueprints = []
		for paradigm in DefMgr.getAll('racial_paradigm'):
			self.racialParadigms[paradigm] = 1


	def backup(self):
		"""
		Makes a persistent copy of the crafting manager that can be restored later.
		@return: Persistent crafting data
		"""
		return self.appliedSciencePoints, self.disciplines, self.racialParadigms, self.blueprints


	def restore(self, backup):
		"""
		Restore a previously backed up copy of the crafting manager.
		@param backup: Backed up crafting data
		"""
		self.appliedSciencePoints, self.disciplines, self.racialParadigms, self.blueprints = backup


	def updateRacialParadigmLevel(self, racialParadigmId, level):
		"""
		Changes the racial paradigm level of the player
		@param racialParadigmId: Racial paradigm up todate
		@param level: New level
		"""
		self.racialParadigms[racialParadigmId] = level
		self.entity().onRacialParadigmUpdated(racialParadigmId, level)


	def giveBlueprints(self, blueprintIds):
		"""
		Gives blueprints to the player
		@param blueprintIds: List of blueprint IDs
		"""
		self.blueprints = list(set(self.blueprints + blueprintIds))
		self.entity().onBlueprintsUpdated(self.blueprints)


	def giveAppliedSciencePoints(self, points):
		"""
		Gives discipline training points to the player
		@param points: Amount of points to give
		"""
		self.appliedSciencePoints += points
		self.entity().onAppliedSciencePointsChanged(points)


	def consumeAppliedSciencePoints(self, points):
		"""
		Use up discipline training points of the player
		@param points: Amount of points to remove
		"""
		assert points <= self.appliedSciencePoints
		self.appliedSciencePoints -= points
		self.entity().onAppliedSciencePointsChanged(points)


	def gainExpertise(self, disciplineId, expertise):
		"""
		Gives expertise points to the selected discipline
		@param disciplineId: Discipline ID
		@param expertise: Amount of gained expertise points
		"""
		assert expertise >= 0
		assert disciplineId in self.disciplines
		self.disciplines[disciplineId] += expertise
		if self.disciplines[disciplineId] > 100:
			self.disciplines[disciplineId] = 100
		self.entity().onDisciplineUpdated(disciplineId, self.disciplines[disciplineId])


	def learnDiscipline(self, disciplineId, expertise = 1):
		"""
		Learns a new discipline at the lowest expertise level
		@param disciplineId: Discipline ID to learn
		@param expertise: Initial expertise level
		@return:
		"""
		discipline = DefMgr.get('discipline', disciplineId)
		if discipline is None:
			warn(Atrea.enums.LC_Crafting, '<%s> cannot train unknown discipline %d' % (self.entity().getName(), disciplineId))
			return False

		if disciplineId in self.disciplines:
			warn(Atrea.enums.LC_Crafting, '<%s> discipline %d is already known!' % (self.entity().getName(), disciplineId))
			return False

		self.disciplines[disciplineId] = expertise
		self.entity().onDisciplineUpdated(disciplineId, expertise)
		return True


	def forgetDiscipline(self, disciplineId):
		"""
		Forgets a previously learned discipline
		@param disciplineId: Discipline ID to forget
		@return:
		"""
		if disciplineId not in self.disciplines:
			warn(Atrea.enums.LC_Crafting, '<%s> discipline %d is not known!' % (self.entity().getName(), disciplineId))
			return False

		del self.disciplines[disciplineId]
		self.entity().onDisciplineUpdated(disciplineId, 0)
		return True


	def spendAppliedSciencePoints(self, disciplineId):
		"""
		Called when the client wants to learn a new discipline.
		@param disciplineId: Discipline ID to learn
		@return:
		"""
		if self.appliedSciencePoints < 1:
			warn(Atrea.enums.LC_Crafting, '<%s> Doesnt have any applied science points' % self.entity().getName())
			return False

		discipline = DefMgr.get('discipline', disciplineId)
		if discipline is None:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot train unknown discipline %d' % (self.entity().getName(), disciplineId))
			return False

		if self.racialParadigms[discipline.racialParadigm.id] < discipline.racialParadigmLevel:
			warn(Atrea.enums.LC_Crafting, '<%s> Racial paradigm %d level %d required to learn discipline %d' %
				(self.entity().getName(), discipline.racialParadigm.id, discipline.racialParadigmLevel, disciplineId))
			return False

		for prerequisite in discipline.requiredDisciplines:
			if prerequisite.id not in self.disciplines:
				warn(Atrea.enums.LC_Crafting, '<%s> Prerequisite discipline %d needed to learn discipline %d' %
					(self.entity().getName(), prerequisite.id, disciplineId))
				return False

			if self.disciplines[prerequisite.id] < 50:
				warn(Atrea.enums.LC_Crafting, '<%s> Prerequisite discipline %d level %d needed to learn discipline %d' %
					(self.entity().getName(), prerequisite.id, 50, disciplineId))
				return False

		learned = self.learnDiscipline(disciplineId)
		if learned:
			self.consumeAppliedSciencePoints(1)
		return learned


	def craft(self, blueprintId, itemIds, quantity):
		"""
		Crafts new items using the specified blueprint.
		@param blueprintId: Blueprint used when crafting
		@param itemIds: Source component IDs
		@param quantity: Amount of items to craft
		@return:
		"""
		if self.entity().busy:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot craft: entity is busy')
			return False

		if blueprintId not in self.blueprints:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot craft unknown blueprint %d' % (self.entity().getName(), blueprintId))
			return False

		items = [self.entity().inventory.getItem(itemId) for itemId in itemIds]
		if not all(items):
			warn(Atrea.enums.LC_Crafting, '<%s> At least one crafting item ID is invalid' % self.entity().getName())
			return False

		for item in items:
			if item.bagId != Atrea.enums.INV_Main and item.bagId != Atrea.enums.INV_Crafting:
				warn(Atrea.enums.LC_Crafting, '<%s> Crafting: item %d is not in main/crafting bag' % (self.entity().getName(), item.id))
				return False

		# Find out which component set the player was trying to use
		blueprint = DefMgr.require('blueprint', blueprintId)
		if blueprint.alloy:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot craft alloy blueprint %d' % (self.entity().getName(), blueprintId))
			return False

		componentSet = None
		for components in blueprint.componentSets.values():
			if all(comp['item'].id in (i.typeId for i in items) for comp in components):
				componentSet = components
				break

		if componentSet is None:
			warn(Atrea.enums.LC_Crafting, '<%s> Invalid items for blueprint %d' % (self.entity().getName(), blueprintId))
			return False

		# Check if the item quantity is sufficient
		for component in componentSet:
			count = sum((item.quantity for item in items if item.typeId == component['item'].id), 0)
			if count < component['quantity'] * quantity:
				warn(Atrea.enums.LC_Crafting, '<%s> not enough items of type %d to craft blueprint %d (got %d needed %d)' %
					(self.entity().getName(), component['item'].id, blueprintId, count, component['quantity'] * quantity))
				return False

		# Consume all items and begin crafting
		# self.entity().beginBusy()
		for component in componentSet:
			assert self.entity().inventory.removeItemByDesign(component['item'].id, component['quantity'] * quantity)
		self.entity().inventory.flushUpdates()

		# Set up a crafting timer
		trace(Atrea.enums.LC_Crafting, '<%s> Crafting blueprint %d (x %d)' % (self.entity().getName(), blueprintId, quantity))
		self.craftingBlueprintId = blueprintId
		self.craftingQuantity = quantity
		duration = 3.0
		completeTime = Atrea.getGameTime() + duration
		self.timer = Atrea.addTimer(completeTime, self.craftingCompleted)
		self.entity().onCraftingStarted(blueprintId, duration)


	def craftingCompleted(self):
		"""
		Called by the timer manager when the crafting induction timer expired
		"""
		assert self.craftingBlueprintId is not None
		blueprint = DefMgr.require('blueprint', self.craftingBlueprintId)
		quantity = blueprint.quantity * self.craftingQuantity
		trace(Atrea.enums.LC_Crafting, '<%s> Crafted item %d (x %d)' % (self.entity().getName(), blueprint.product.id, quantity))
		self.entity().inventory.pickedUpItem(blueprint.product.id, quantity)
		self.entity().inventory.flushUpdates()

		# Gain expertise in the discipline if we aren't on 100% already
		self.gainExpertise(blueprint.discipline.id, 1)

		self.timer = None
		self.craftingBlueprintId = None
		self.craftingQuantity = None
		# self.entity().endBusy()


	def research(self, itemId, kickerIds):
		"""
		Begin researching the specified item.
		When the research is completed, the player has a chance of gaining 5 points of expertise to
		one of the disciplines attached to the researched item.
		Chance: 100 - expertise + 5 * kickerCount
		@param itemId: Item to research
		@param kickerIds: Optional kickers for researching
		@return:
		"""
		if self.entity().busy:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot research: entity is busy')
			return False

		item = self.entity().inventory.getItem(itemId)
		if item is None:
			warn(Atrea.enums.LC_Crafting, '<%s> Reserarch: Invalid item id %d' % (self.entity().getName(), itemId))
			return False

		if not item.type.researchable:
			warn(Atrea.enums.LC_Crafting, '<%s> Research: design %d is not researchable' % (self.entity().getName(), item.type.id))
			return False

		kickers = [self.entity().inventory.getItem(itemId) for itemId in kickerIds]
		if not all(kickers):
			warn(Atrea.enums.LC_Crafting, '<%s> At least one kicker item ID is invalid' % self.entity().getName())
			return False

		for kicker in kickers:
			if not kicker.type.kicker:
				warn(Atrea.enums.LC_Crafting, '<%s> Research: design %d is not a kicker' % (self.entity().getName(), kicker.typeId))
				return False

		# Consume the items being researched
		# self.entity().beginBusy()
		assert self.entity().inventory.removeItem(itemId, 1)
		for kicker in kickers:
			assert self.entity().inventory.removeItem(kicker.id, 1)
		self.entity().inventory.flushUpdates()

		# Determine the chance of gaining expertise
		trace(Atrea.enums.LC_Crafting, '<%s> Researching item %d' % (self.entity().getName(), itemId))
		# Find all discipline that are researchable
		disciplines = [discipline.id for discipline in item.type.disciplines if discipline.id in self.disciplines and
						0 < self.disciplines[discipline.id] < item.type.techCompetency]
		# Select a random discipline that we'll give expertise to
		if len(disciplines) > 0:
			discipline = disciplines[randint(0, len(disciplines))]
			chance = 100 - self.disciplines[discipline] + len(kickers) * 5
			trace(Atrea.enums.LC_Crafting, '<%s> Chance for discipline %d: %d%%' % (self.entity().getName(), discipline, chance))
			if random() * 100 < chance:
				self.researchDiscipline = DefMgr.get('discipline', discipline)
				self.researchExpertise = 5
			else:
				self.researchDiscipline = None
		else:
			self.researchDiscipline = None

		if self.researchDiscipline is not None:
			trace(Atrea.enums.LC_Crafting, '<%s> Will gain %d expertise in <%s> from research' %
			     (self.entity().getName(), self.researchExpertise, self.researchDiscipline.name))
		else:
			trace(Atrea.enums.LC_Crafting, '<%s> Will not gain any expertise from research' % self.entity().getName())

		# Set up a research timer
		duration = 3.0
		completeTime = Atrea.getGameTime() + duration
		self.timer = Atrea.addTimer(completeTime, self.researchCompleted)
		self.entity().onCraftingStarted(0, duration)


	def researchCompleted(self):
		"""
		Called by the timer manager when the research induction timer expired
		"""
		if self.researchDiscipline is not None:
			self.gainExpertise(self.researchDiscipline.id, self.researchExpertise)
			self.entity().feedback('%s expertise increased to %d' % (self.researchDiscipline.name, self.disciplines[self.researchDiscipline.id]))
		else:
			self.entity().feedback('No experience gained from researching')

		self.timer = None
		self.researchDiscipline = None
		self.researchExpertise = 0
		# self.entity().endBusy()


	def reverseEngineer(self, itemId):
		"""
		Begin reverse engineering the specified item.
		@param itemId: Item to research
		"""
		if self.entity().busy:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot reverse engineer: entity is busy')
			return False

		item = self.entity().inventory.getItem(itemId)
		if item is None:
			warn(Atrea.enums.LC_Crafting, '<%s> RevEng: Invalid item id %d' % (self.entity().getName(), itemId))
			return False

		if not item.type.reverseEngineerable:
			warn(Atrea.enums.LC_Crafting, '<%s> RevEng: design %d is not reverse engineerable' % (self.entity().getName(), item.type.id))
			return False

		# Find all blueprints for this item; if more than one is found, select one randomly
		blueprints = [bp for bp in DefMgr.getAll('blueprint').values() if bp.product.id == item.type.id]
		if len(blueprints) < 1:
			warn(Atrea.enums.LC_Crafting, '<%s> RevEng: design %d has no blueprints' % (self.entity().getName(), item.type.id))
			return False
		blueprint = blueprints[randint(0, len(blueprints))]
		components = blueprint.componentSets[blueprint.componentSets.keys()[randint(0, len(blueprint.componentSets))]]

		# Consume the items being reverse engineered
		# self.entity().beginBusy()
		assert self.entity().inventory.removeItem(itemId, 1)
		self.entity().inventory.flushUpdates()

		expertise = self.disciplines.get(blueprint.discipline.id, 0)
		if blueprint.product.techCompetency > expertise:
			bias = blueprint.product.techCompetency / expertise
		else:
			bias = 1.0 + 0.4 * ((blueprint.product.techCompetency - expertise) / expertise)

		trace(Atrea.enums.LC_Crafting, '<%s> RevEng: bias %f for design %d' % (self.entity().getName(), bias, blueprint.product.id))
		self.revEngItems = {}
		for component in components:
			rand = random() * bias
			if rand > 1.0:
				rand = 1.0
			quantity = floor(rand * component['quantity'])
			if quantity > 0:
				self.revEngItems[component['item'].id] = quantity

		duration = 3.0
		completeTime = Atrea.getGameTime() + duration
		self.timer = Atrea.addTimer(completeTime, self.reverseEngineeringCompleted)
		self.entity().onCraftingStarted(0, duration)


	def reverseEngineeringCompleted(self):
		"""
		Called by the timer manager when the reverse engineering induction timer expired
		"""
		for designId, quantity in self.revEngItems.items():
			trace(Atrea.enums.LC_Crafting, '<%s> RevEng item %d (x %d)' % (self.entity().getName(), designId, quantity))
			self.entity().inventory.pickedUpItem(designId, quantity)
		self.entity().inventory.flushUpdates()

		self.timer = None
		self.revEngItems = None
		# self.entity().endBusy()


	def alloy(self, blueprintId, currentTierItemId, lowerTierItems):
		"""
		Begin alloying using the specified blueprint.
		@param blueprintId: Blueprint ID to alloy
		@param currentTierItemId: Component in the current tier
		@param lowerTierItems: Component(s) in the previous tier
		@return:
		"""
		if self.entity().busy:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot alloy: entity is busy')
			return False

		if blueprintId not in self.blueprints:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot alloy unknown blueprint %d' % (self.entity().getName(), blueprintId))
			return False

		component = self.entity().inventory.getItem(currentTierItemId)
		elementaryComponents = [self.entity().inventory.getItem(itemId) for itemId in lowerTierItems]
		if component is None or not all(elementaryComponents):
			warn(Atrea.enums.LC_Crafting, '<%s> At least one alloying item ID is invalid' % self.entity().getName())
			return False

		if component.bagId != Atrea.enums.INV_Main and component.bagId != Atrea.enums.INV_Crafting:
			warn(Atrea.enums.LC_Crafting, '<%s> Alloying: component %d is not in main/crafting bag' % (self.entity().getName(), component.id))
			return False

		for item in elementaryComponents:
			if item.bagId != Atrea.enums.INV_Main and item.bagId != Atrea.enums.INV_Crafting:
				warn(Atrea.enums.LC_Crafting, '<%s> Alloying: elementary component %d is not in main/crafting bag' % (self.entity().getName(), item.id))
				return False

		# Find out which component set the player was trying to use
		blueprint = DefMgr.require('blueprint', blueprintId)
		if not blueprint.alloy:
			warn(Atrea.enums.LC_Crafting, '<%s> Cannot alloy crafting blueprint %d' % (self.entity().getName(), blueprintId))
			return False

		requirement = blueprint.componentSets[blueprint.componentSets.keys()[0]][0]
		if requirement['item'].id != component.typeId is None:
			warn(Atrea.enums.LC_Crafting, '<%s> Invalid component for blueprint %d' % (self.entity().getName(), blueprintId))
			return False

		if component.quantity < requirement['quantity'] is None:
			warn(Atrea.enums.LC_Crafting, '<%s> Insufficient components for blueprint %d' % (self.entity().getName(), blueprintId))
			return False

		# Check if we have the required amount of elementary components
		if blueprint.requiresElementaryComponents:
			# Check the first item ID to determine the quality of components
			if len(elementaryComponents) == 0:
				warn(Atrea.enums.LC_Crafting, '<%s> Blueprint %d needs elementary components' % (self.entity().getName(), blueprintId))
				return False

			quality = elementaryComponents[0].type.quality
			count = Constants.ALLOYING_ELEMENTARY_COUNTS[quality]
			if len(elementaryComponents) != count:
				warn(Atrea.enums.LC_Crafting, '<%s> Blueprint %d needs %d elementary components of quality %d' %
											  (self.entity().getName(), blueprintId, count, quality))
				return False

			# Check that items are in the correct tier
			# TODO: Should all components be of the same type?
			for item in elementaryComponents:
				if item.type.tier != component.type.tier - 1:
					warn(Atrea.enums.LC_Crafting, '<%s> Elementary components %d is of tier %d, not %d!' %
												  (self.entity().getName(), item.id, item.type.tier, component.type.tier))
					return False

		# Everything seems OK.
		# Consume all items and begin alloying
		# self.entity().beginBusy()
		assert self.entity().inventory.removeItemByDesign(component.id, requirement['quantity'])
		for comp in elementaryComponents:
			assert self.entity().inventory.removeItemByDesign(comp.id, 1)
		self.entity().inventory.flushUpdates()

		# Set up a crafting timer
		trace(Atrea.enums.LC_Crafting, '<%s> Alloying blueprint %d' % (self.entity().getName(), blueprintId))
		self.alloyingBlueprintId = blueprintId
		duration = 3.0
		completeTime = Atrea.getGameTime() + duration
		self.timer = Atrea.addTimer(completeTime, self.alloyingCompleted)
		self.entity().onCraftingStarted(blueprintId, duration)


	def alloyingCompleted(self):
		"""
		Called by the timer manager when the alloying induction timer expired
		"""
		assert self.alloyingBlueprintId is not None
		blueprint = DefMgr.require('blueprint', self.alloyingBlueprintId)
		quantity = blueprint.quantity
		trace(Atrea.enums.LC_Crafting, '<%s> Alloyed item %d (x %d)' % (self.entity().getName(), blueprint.product.id, quantity))
		self.entity().inventory.pickedUpItem(blueprint.product.id, quantity)
		self.entity().inventory.flushUpdates()

		# Gain expertise in the discipline if we aren't on 100% already
		self.gainExpertise(blueprint.discipline.id, 1)

		self.timer = None
		self.alloyingBlueprintId = None
		# self.entity().endBusy()
