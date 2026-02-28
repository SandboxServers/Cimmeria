import Atrea
import Atrea.enums
from cell.SGWBeing import SGWBeing
from cell.interactions.Lootable import Lootable
from common import Constants
from common.Logger import warn, trace
from common.defs.Def import DefMgr


class SGWMob(SGWBeing):
	# Ability check result constants
	ABILITY_Usable = 1
	ABILITY_CoolingDown = 2
	ABILITY_Filtered = 3
	ABILITY_NeedsAmmo = 4

	# Type of ammo this entity is using during combat
	ammoTypeId = Atrea.enums.AMMO_NONE

	aggressionOverride = None
	# Current state of mob AI
	aiState = Atrea.enums.AI_STATE_Spawning
	# AI update/tick timer ID
	aiTimer = None
	# List of entites that are threatening us (entityId -> threat level)
	threat = {}
	# Abilities we can choose from
	abilitySet = None
	# Enable AI debug output
	enableAiDebug = False
	
	def __init__(self):
		super().__init__()
		self.threat = {}
		
	def createOnClient(self, mailbox):
		super().createOnClient(mailbox)
		if self.aggressionOverride is not None:
			mailbox.onAggressionOverrideUpdate(self.aggressionOverride)
			mailbox.onEntityProperty(Atrea.enums.GENERICPROPERTY_MobAggression, self.aggressionOverride)
		mailbox.onEntityProperty(Atrea.enums.GENERICPROPERTY_AmmoTypeId, self.ammoTypeId)


	def enteredSpace(self, spaceId):
		"""
		Called when the entity fully entered the specified space
		@param spaceId: ID of space the entity entered
		"""
		super().enteredSpace(spaceId)
		self.doAiAction()


	def setAggression(self, aggressionLevel):
		"""
		Changes the aggression level of this entity
		"""
		self.aggressionOverride = aggressionLevel
		if self.client is not None:
			self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_MobAggression, self.aggressionOverride)
		self.witnesses.onEntityProperty(Atrea.enums.GENERICPROPERTY_MobAggression, self.aggressionOverride)


	def aiDebug(self, msg):
		"""
		Displays an AI debugging message.
		@param msg: Message to display
		"""
		if self.enableAiDebug:
			trace(Atrea.enums.LC_Mob, msg)


	def threatGenerated(self, entityId, threat):
		"""
		An entity has generated threat towards this mob.
		@param entityId: Threatening entity
		@param threat: Amount of threat generated
		"""
		self.aiDebug("AI: Entity %d generated %f threat" % (entityId, threat))
		if entityId in self.threat:
			self.threat[entityId] += threat
		else:
			self.threat[entityId] = threat
			instigator = self.space.findEntity(entityId)
			instigator.onAddedToThreatList(self.entityId)
		if self.aiState == Atrea.enums.AI_STATE_Idle and not self.isDead():
			self.aiDebug("AI: Entering combat state")
			self.aiBeginCombat()


	def onAbilityFinished(self):
		"""
		The entity has finished using an ability.
		"""
		self.aiDebug("AI: Current ability finished")
		self.doAiAction()


	def onAbilityInterrupted(self):
		"""
		The entity was interrupting during an ability use.
		"""
		self.aiDebug("AI: Current ability interrupted")
		self.doAiAction()


	def onAttacked(self, attackerId, healthChange, focusChange, damageType):
		"""
		Notifies the entity that it is being attacked by someone else.
		@param attackerId: Attacker entity
		@param healthChange: Amount of health lost/gained
		@param focusChange: Amount of focus lost/gained
		@param damageType: Damage type (see EDamageType enumeration)
		"""
		# TODO: What should we do with healing effects? (positive health/focus change)
		self.threatGenerated(attackerId, -healthChange * 2 - focusChange)


	def onDead(self):
		"""
		Called when the entity dies
		"""
		super().onDead()
		if self.template is not None and self.template.lootTable is not None:
			loot = Lootable(self)
			loot.generateLoot()
			lootMapId = loot.interactionSetMapId()
			if lootMapId is not None:
				# FIXME: This is workaround for not having proper dynamic interaction type updates
				self.setInteractionType(self.interactionType | Atrea.enums.INT_NormalLoot)
				self.staticInteractions[Constants.INTERACTION_Loot] = loot
				self.lootHandler = loot

		# Reset the threatened entity list (the mob forgets all combat state when it dies)
		for entityId in self.threat:
			entity = self.space.findEntity(entityId)
			if entity is not None:
				entity.onRemovedFromThreatList(self.entityId)
		self.threat = {}


	def getInteractions(self, player):
		"""
		Returns the list of available interaction set maps for the player
		@param player: Player that is interacting with us
		@return: Interaction set maps
		"""
		maps = super().getInteractions(player)

		if self.lootHandler is not None:
			lootMapId = self.lootHandler.interactionSetMapId()
			if lootMapId is not None:
				maps[lootMapId] = DefMgr.get('interaction_set_map', Constants.INTERACTION_Loot)

		return maps


	def aiBeginCombat(self):
		"""
		Enters combat AI mode and initiates combat actions.
		"""
		self.aiState = Atrea.enums.AI_STATE_Fighting
		self.setStateFlag(Atrea.enums.BSF_InCombat)
		self.doAiAction()


	def getTopThreateningEntity(self):
		"""
		Returns the entity that generated the highest level of threat to us so far
		@return Threatening entity
		"""
		max, chosen = 0, None
		removeEntities = []
		for entityId, level in self.threat.items():
			if max < level:
				# TODO: We should store a secondaryId (or extend the entityId with some counter) to make sure that
				# we're still referring to the entity that was originally added to the threat list and not an
				# entity that eventually got the same entityId!
				entity = self.space.findEntity(entityId)
				if entity is None:
					removeEntities.append(entityId)
				elif entity.isDead():
					removeEntities.append(entityId)
					entity.onRemovedFromThreatList(self.entityId)
				else:
					# TODO: Add other factors into target selection (distance, cover, LOS, ...)
					max = level
					chosen = entity

		for entityId in removeEntities:
			del self.threat[entityId]

		if chosen:
			self.aiDebug("AI: Selected entity <%s> as a target" % chosen.getName())
		else:
			self.aiDebug("AI: No target selected")
		return chosen


	def classifyHostileAbility(self, target, ability):
		"""
		Determines whether we can use the selected hostile ability on the target or not.
		@param target: Entity that we're targeting
		@param ability: Ability being checked
		@return:
		"""
		# Ability must be hostile
		if ability.typeId == Atrea.enums.ABILITY_TYPE_Heal or ability.typeId == Atrea.enums.ABILITY_TYPE_Buff or \
			ability.typeId == Atrea.enums.ABILITY_TYPE_Undefined:
			self.aiDebug("AI: Ability <%s> filtered: Not hostile" % ability.name)
			return self.ABILITY_Filtered

		# Only single-target abilities are supported at the moment
		# TODO: Add deploy/AoE abilities
		if ability.targetCollectionMethod != Atrea.enums.TCM_Single:
			self.aiDebug("AI: Ability <%s> filtered: Not single target" % ability.name)
			return self.ABILITY_Filtered

		# Check if the ability is being cooled down
		if self.abilities.isCoolingDown(ability):
			self.aiDebug("AI: Ability <%s> is cooling down" % ability.name)
			return self.ABILITY_CoolingDown

		if ability.requiredAmmo > self.getAmmoCount():
			self.aiDebug("AI: Ability <%s> needs a weapon reload" % ability.name)
			return self.ABILITY_NeedsAmmo

		# TODO: Check distance, LOS

		self.aiDebug("AI: Ability <%s> is OK" % ability.name)
		return self.ABILITY_Usable


	def selectHostileAbility(self, target):
		"""
		Select a hostile ability we're going to use against the target entity.
		!!! 3 ability categories:
		- usable (can use it immediately)
		- currently unusable (cooldown pending, etc.)
		- unusable against entity (resistance? target type?)
		@param target: Entity that we're targeting
		@return AbilityId of the selected ability (or None if no ability was selected)
		"""
		usable, cooldown, ammo = [], [], []

		if self.abilitySet is not None:
			for ability in self.abilitySet.abilities.values():
				type = self.classifyHostileAbility(target, ability)
				if type == self.ABILITY_Usable:
					usable.append(ability)
				elif type == self.ABILITY_CoolingDown:
					cooldown.append(ability)
				elif type == self.ABILITY_NeedsAmmo:
					ammo.append(ability)

		if len(usable) > 0:
			# TODO: Which ability should we use? Use some weighting/rand here.
			self.aiDebug("AI: Selected ability <%s> out of %d available" % (usable[0].name, len(usable)))
			return usable[0]
		elif len(ammo) > 0 and not self.abilities.isCoolingDown(DefMgr.get('ability', Constants.ABILITY_ReloadWeapon)):
			self.aiDebug("AI: No ability available, %d needs ammo; reloading" % len(ammo))
			self.launchReloadAbility()
			return None
		elif len(cooldown) > 0:
			self.aiDebug("AI: No ability available, %d being cooled down" % len(cooldown))
			return None
		else:
			# TODO: Flee?
			self.aiDebug("AI: No ability available!")
			return None


	def doAiFightingAction(self):
		"""
		Initiates an AI combat action.
		"""
		entity = self.getTopThreateningEntity()
		if entity is not None:
			self.lookAt(entity.position)
			ability = self.selectHostileAbility(entity)
			if ability:
				launched = self.abilities.launchAbility(ability, entity.entityId, isEntityAbility = False)
				if launched is not True:
					warn(Atrea.enums.LC_Mob, "AI: Failed to launch ability <%s>: %s (%d)!" %
											 (ability.name, Atrea.enumNames.EConditionHandlerFeedback[launched], launched))
			else:
				self.aiDebug("AI: No ability available; waiting 0.5 sec")
				Atrea.addTimer(Atrea.getGameTime() + 0.5, lambda: self.doAiAction())
		elif len(self.threat) == 0:
			# No entities are threatening us; revert to our previous behavior
			self.aiDebug("AI: Threat list empty; going idle")
			self.aiState = Atrea.enums.AI_STATE_Idle
			self.unsetStateFlag(Atrea.enums.BSF_InCombat)
			self.doAiAction()


	def doAiSpawnAction(self):
		"""
		Perform initial actions a mob performs when spawning
		(currently this only reloads the active weapon)
		"""
		if self.template is not None and self.template.weapon is not None:
			ammo = self.template.weapon.clipSize
			ammoStat = self.getStat(Atrea.enums.ammoSlot1)
			ammoStat.update(0, ammo, ammo)
		self.aiState = Atrea.enums.AI_STATE_Idle


	def doAiAction(self):
		"""
		Initiates an AI action depending on the current AI state.
		"""
		self.aiDebug("AI: Executing AI action in state %d" % self.aiState)
		if self.isDead():
			self.aiDebug("AI: I'm dead!")
			self.aiState = Atrea.enums.AI_STATE_Dead
			return

		if self.aiState == Atrea.enums.AI_STATE_Fighting:
			self.doAiFightingAction()
		elif self.aiState == Atrea.enums.AI_STATE_Spawning:
			self.doAiSpawnAction()
			# TODO: Add other states here!


	def getAmmoStat(self):
		"""
		Returns the statId of the ammo slot for the currently equipped weapon.
		@return: Stat ID (se EStats enumeration)
		"""
		return Atrea.enums.ammoSlot1


	def getClipSize(self):
		"""
		Returns the clip size the currently equipped weapon.
		@return Clip size of weapon
		"""
		if self.template is None or self.template.weapon is None:
			warn(Atrea.enums.LC_Mob, 'getClipSize(): Mob %d doesnt have a weapon!' % self.entityId)
			return 0

		return self.template.weapon.clipSize


	def getAmmoCount(self):
		"""
		Returns the amount of ammo in the currently equipped weapon.
		@return # of ammo
		"""
		if self.template is None or self.template.weapon is None:
			warn(Atrea.enums.LC_Mob, 'getAmmoCount(): Mob %d doesnt have a weapon!' % self.entityId)
			return 0

		return self.getStat(Atrea.enums.ammoSlot1).cur


	def consumeAmmo(self, amount):
		"""
		Consumes the specified amount of ammo from the currently equipped weapon.
		@param amount: Amount to consume
		"""
		self.getStat(Atrea.enums.ammoSlot1).change(-amount)
		self.sendDirtyStats()


	def hasItemMoniker(self, monikerIds):
		"""
		Checks if the currently equipped weapon has at least one of the specified monikers
		@param monikerIds: List of moniker ID-s to check
		@return bool
		"""
		if self.template is None or self.template.weapon is None:
			warn(Atrea.enums.LC_Mob, 'hasItemMoniker(): Mob %d doesnt have a weapon!' % self.entityId)
			return 0

		ids = self.template.weapon.monikerIds
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
		weapon = self.template.weapon
		if ranged:
			return weapon.minRangedRange, weapon.maxRangedRange
		else:
			return weapon.minMeleeRange, weapon.maxMeleeRange


