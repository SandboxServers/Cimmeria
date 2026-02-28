import Atrea
import Atrea.enums
from cell.AbilityManager import AbilityManager
from cell.SGWSpawnableEntity import SGWSpawnableEntity
from common import Constants
from common.Logger import error
from common.defs.Ability import Ability
from common.defs.Def import DefMgr


def mustBeAlive(func):
	"""
	Requires that the player has finished loading and is alive before executing the action.
	"""
	def mustBeAliveDecorator(self, *args, **kwargs):
		if not self.clientLoaded:
			self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, 0, Atrea.enums.CONDITION_FEEDBACK_InvalidEntity)
		elif self.isDead():
			self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, 0, Atrea.enums.CONDITION_FEEDBACK_NotLiving)
		else:
			return func(self, *args, **kwargs)
	return mustBeAliveDecorator


def mustBeIdle(func):
	"""
	Requires that the player has finished loading and is alive before executing the action.
	"""
	def mustBeIdleDecorator(self, *args, **kwargs):
		if not self.clientLoaded:
			self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, 0, Atrea.enums.CONDITION_FEEDBACK_InvalidEntity)
		elif self.isDead():
			self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, 0, Atrea.enums.CONDITION_FEEDBACK_NotLiving)
		elif self.busy:
			self.client.onErrorCode(Atrea.enums.ERRORCODE_SYSTEM_Ability, 0, Atrea.enums.CONDITION_FEEDBACK_WeaponCooldownNotReady)
		else:
			return func(self, *args, **kwargs)
	return mustBeIdleDecorator

class Stat(object):
	"""
	Stat of a combatant entity
	Each stat has two sets of values:
	 - baseMin/Max/Cur: These are the "base" values for the entity, they only change when the entity levels up, etc.
	 - Min/Max/Cur: These are the dynamic values; eg. max health can change due to effects, etc.
	Min/Max values determine the bounds of the Current value.
	"""
	__slots__ = ('min', 'cur', 'max', 'baseMin', 'baseCur', 'baseMax', 'dirty', 'baseDirty', 'callback')

	def __init__(self, min, cur, max, baseMin, baseCur, baseMax):
		self.min = min
		self.cur = cur
		self.max = max
		self.baseMin = baseMin
		self.baseMax = baseMax
		self.baseCur = baseCur
		self.dirty = False
		self.baseDirty = False
		self.callback = None


	def clone(self):
		stat = Stat(self.min, self.cur, self.max, self.baseMin, self.baseCur, self.baseMax)
		stat.dirty, stat.baseDirty, stat.callback = self.dirty, self.baseDirty, self.callback
		return stat


	def setMin(self, minValue):
		"""
		Updates minimum current value of stat.
		Sets dirty flag for the property.
		@param minValue: New minimum value for stat
		"""
		self.min = minValue
		self.dirty = True
		if self.cur < self.min:
			self.cur = self.min


	def setMax(self, maxValue):
		"""
		Updates minimum current value of stat.
		Sets dirty flag for the property.
		@param maxValue: New minimum value for stat
		"""
		self.max = maxValue
		self.dirty = True
		if self.cur > self.max:
			self.cur = self.max


	def setCurrent(self, newValue):
		"""
		Updates current value of stat.
		Sets dirty flag for the property.
		@param newValue: New value for stat
		@return: New stat value (may differ from the absolute value specified!)
		"""
		oldCur = self.cur
		self.cur = int(newValue)
		if self.cur < self.min:
			self.cur = self.min
		elif self.cur > self.max:
			self.cur = self.max
		self.dirty = True
		if self.callback is not None:
			self.cur = self.callback(oldCur, self.cur)
		return self.cur


	def change(self, relativeChange):
		"""
		Changes current value of stat with a relative amount.
		(adds the value to the current value of the stat).
		This method will clamp the current stat value at Min and Max.
		Sets dirty flag for the property.
		@param relativeChange: Signed change for stat
		@return: Stat change amount (may differ from the relative change amount specified!)
		"""
		oldCur = self.cur
		self.cur += relativeChange
		if self.cur < self.min:
			self.cur = self.min
		elif self.cur > self.max:
			self.cur = self.max
		self.dirty = True
		if self.callback is not None:
			self.cur = self.callback(oldCur, self.cur)
		return self.cur - oldCur


	def changeByPercent(self, multiplier):
		"""
		Change value of the stat by a percentage of the current value.
		@param multiplier: Stat value multiplier (eg. 0.05 = 5%)
		@return: Stat change amount
		"""
		relativeChange = round(self.cur * multiplier)
		return self.change(relativeChange)


	def changeByMaxPercent(self, multiplier):
		"""
		Change value of the stat by a percentage of the Max value.
		@param multiplier: Stat value multiplier (eg. 0.05 = 5%)
		@return: Stat change amount
		"""
		relativeChange = round(self.max * multiplier)
		return self.change(relativeChange)


	def changeByMinMaxPercent(self, multiplier):
		"""
		Change value of the stat by a percentage of the (Max - Min) range.
		@param multiplier: Stat value multiplier (eg. 0.05 = 5%)
		@return: Stat change amount
		"""
		relativeChange = round((self.max - self.min) * multiplier)
		return self.change(relativeChange)


	def update(self, min, current, max):
		"""
		Updates all aspects of the stat (min, current and max value).
		Sets dirty flag for the property.
		@param min: Minimal value of stat
		@param current: Current value of stat
		@param max: Maximal value of stat
		"""
		self.min = min
		self.cur = current
		self.max = max
		self.dirty = True



class SGWBeing(SGWSpawnableEntity):
	"""
	List of all stats that aren't in use / are only used partially:
	Interrupts and resistances: coordination, interruptRes
	Regeneration: morale, healthRegen, focusRegen, energyRegen
	Base/Current stats: fortitude, morale
	Stealth system: perception, stealthRating, stealthMovement, revealRating, disguiseRating, disguiseDetection
	Minigames: intelligence
	Movement/Cover: movementSpeedMod, rangeModifier, coverQRModifier, tracking, stabilization, coverAccuracy,
		coverDefense, crouchingAccuracy, crouchingDefense, rotationSpeedMod
	Timers: speedReload, speedPet
	Inventory: ammoSlot1-5, deploymentBarAmmo
	Etc: energy, recovery, restoration, subtlety

	StateFlags:
		BSF_AutoCycling - use only on client (self)
		BSF_Crouching - add AI support for this
		BSF_InCombat - needs hostile/hostility list for PC/NPCs
		BSF_PlayingMinigame - player only
		BSF_InStealth - needs stealth system
		BSF_MovementLock - set/unset by various combatant states
		BSF_Walking - allow for NPCs?
		BSF_Holster - unset when firing (add RequiredMonikers to abilities?)

	CombatantFlags:
		PLAYER_STATE_Blind - ???
		PLAYER_STATE_Confuse - ???
		PLAYER_STATE_DoT - Internal state flag, does not do anything ?
		PLAYER_STATE_Disease - ???
		PLAYER_STATE_Disorient - ???
		PLAYER_STATE_Fear - Almost the same as stun + server-side forced movement?
		PLAYER_STATE_KnockBack - ???
		PLAYER_STATE_KnockDown - ???
		PLAYER_STATE_Slow - Is the slow effect already applied by the Effect?
		PLAYER_STATE_Snare - ???
		PLAYER_STATE_Stun - BSF_MovementLock + No ability/item use
		PLAYER_STATE_Suppression - ???
		PLAYER_STATE_Alive - Do we need to handle this or will STATE_Dead suffice?
		PLAYER_STATE_Dead - Handled properly
	"""
	archetype = Atrea.enums.ARCHETYPE_Any
	alignment = Atrea.enums.ALIGNMENT_Undefined
	faction = Atrea.enums.FACTION_None
	level = 1
	beingName = ""
	targetId = 0
	stateField = 0
	meleeRange = 0
	combatantState = 0
	cheatFlags = 0
	# Amount of conditions triggering movement lock
	movementLocks = 0
	# Is the entity doing something?
	busy = False
	# Current action being performed by the entity
	action = None

	# Combat stats that can be sent to all clients
	publicStats = [
		Atrea.enums.movementSpeedMod,
	    Atrea.enums.health,
	    Atrea.enums.focus,
	    Atrea.enums.ammoSlot1,
	    Atrea.enums.ammoSlot2,
	    Atrea.enums.ammoSlot3,
	    Atrea.enums.ammoSlot4,
	    Atrea.enums.ammoSlot5,
	    Atrea.enums.rotationSpeedMod,
	    Atrea.enums.energy,
	    Atrea.enums.energyRegen
	]


	def onHealthChanged(self, oldHealth, newHealth):
		"""
		Called when the health of the entity changes
		@param oldHealth: Old health value
		@param newHealth: New health value
		@return Accepted health value
		"""
		if self.isDead():
			# Health cannot be changed when the entity is dead
			return oldHealth

		if newHealth < oldHealth and self.cheatFlags & Constants.CHEAT_Invulnerable:
			# Debug command to disable health damage
			return oldHealth

		if oldHealth > 0 and newHealth == 0:
			self.setCombatantStateFlag(Atrea.enums.PLAYER_STATE_Dead)
		return newHealth


	# Template stats for the entity
	# All new entities will be created with these stats
	statsTemplate = {
		Atrea.enums.coordination: Stat(0, 1, 1, 0, 1, 1),
		Atrea.enums.engagement: Stat(0, 1, 1, 0, 1, 1),
		Atrea.enums.fortitude: Stat(0, 1, 1, 0, 1, 1),
		Atrea.enums.morale: Stat(0, 1, 1, 0, 1, 1),
		Atrea.enums.perception: Stat(0, 1, 1, 0, 1, 1),
		Atrea.enums.intelligence: Stat(0, 1, 1, 0, 1, 1),
		Atrea.enums.movementSpeedMod: Stat(0, 100, 500, 0, 100, 500),
		Atrea.enums.health: Stat(0, 100, 100, 0, 100, 100),
		Atrea.enums.focus: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.healthRegen: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.focusRegen: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.accuracy: Stat(-1000, 0, 1000, -1000, 0, 1000),
		Atrea.enums.defense: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.qrMod: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.physicalAF: Stat(0, 0, 50000, 0, 0, 50000),
		Atrea.enums.energyAF: Stat(0, 0, 50000, 0, 0, 50000),
		Atrea.enums.hazmatAF: Stat(0, 0, 50000, 0, 0, 50000),
		Atrea.enums.psionicAF: Stat(0, 0, 50000, 0, 0, 50000),
		Atrea.enums.kineticRes: Stat(0, 0, 2000, 0, 0, 2000),
		Atrea.enums.mentalRes: Stat(0, 0, 2000, 0, 0, 2000),
		Atrea.enums.healthRes: Stat(0, 0, 2000, 0, 0, 2000),
		Atrea.enums.stealthRating: Stat(0, 0, 100, 0, 0, 100),
		Atrea.enums.rangeModifier: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.coverQRModifier: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.ammoSlot1: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.ammoSlot2: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.ammoSlot3: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.ammoSlot4: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.ammoSlot5: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.deploymentBarAmmo: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.response: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.damage: Stat(-100, 0, 100, -100, 0, 100),
		Atrea.enums.penetration: Stat(-100, 0, 100, -100, 0, 100),
		Atrea.enums.physicalDensity: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.energyDensity: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.hazmatDensity: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.psionicDensity: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.tracking: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.stabilization: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.awareness: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.interruptRes: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.coverAccuracy: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.coverDefense: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.crouchingAccuracy: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.crouchingDefense: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.stealthMovement: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.revealRating: Stat(0, 0, 100, 0, 0, 100),
		Atrea.enums.negation: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.PhysicalDamagePercent: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.EnergyDamagePercent: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.HazmatDamagePercent: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.PsionicDamagePercent: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.UntypedDamagePercent: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.disguiseRating: Stat(0, 0, 500, 0, 0, 500),
		Atrea.enums.disguiseDetection: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.mitigation: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.rotationSpeedMod: Stat(0, 100, 500, 0, 100, 500),
		Atrea.enums.energy: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.energyRegen: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.speedReload: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.speedGrenade: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.speedDeploy: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.speedAttack: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.recovery: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.restoration: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.subtlety: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.speedPet: Stat(0, 0, 0, 0, 0, 0),
		Atrea.enums.absorbPhysical: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbEnergy: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbHazmat: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbPsionic: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbUntyped: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbPhysicalItem: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbEnergyItem: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbHazmatItem: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbPsionicItem: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbUntypedItem: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbPhysicalEnergy: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbEnergyEnergy: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbHazmatEnergy: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbPsionicEnergy: Stat(0, 0, 1000, 0, 0, 1000),
		Atrea.enums.absorbUntypedEnergy: Stat(0, 0, 1000, 0, 0, 1000)
	}


	def __init__(self):
		super().__init__()
		self.components = []
		self.stats = {}
		for statId, stat in self.statsTemplate.items():
			self.stats[statId] = stat.clone()
		self.getStat(Atrea.enums.health).callback = self.onHealthChanged
		self.abilities = AbilityManager(self)
		self.combatantStates = {}


	def backup(self):
		"""
		Called when the state of the entity is being saved
		(backup to BaseApp, moving to a different space, ...)
		@return Persistent data of the entity
		"""
		data = super().backup()
		data['being'] = (self.archetype, self.alignment, self.faction, self.level,
			self.beingName, self.targetId, self.stateField, self.meleeRange)
		data['stats'] = [(id, s.min, s.cur, s.max, s.baseMin, s.baseCur, s.baseMax) for id, s in self.stats.items()]
		return data


	def restore(self, backup):
		super().restore(backup)
		self.archetype, self.alignment, self.faction, self.level, self.beingName, \
			self.targetId, self.stateField, self.meleeRange = backup['being']
		self.stats = {}
		for id, min, cur, max, baseMin, baseCur, baseMax in backup['stats']:
			self.stats[id] = Stat(min, cur, max, baseMin, baseCur, baseMax)


	def leftSpace(self, spaceId):
		"""
		Called when the entity left the specified space
		(self.spaceId and local mailboxes are still available for the duration of the call)
		@param spaceId: ID of space the entity left
		"""
		self.cancelAction()
		super().leftSpace(spaceId)


	def getStat(self, statId):
		"""
		Returns the object holding all values of the stat.
		@param statId: ID of stat (see EStats enumeration)
		"""
		return self.stats[statId]


	def changeTarget(self, targetId):
		"""
		Changes the target of this entity
		@param targetId: Id of target entity
		"""
		self.targetId = targetId
		if self.client is not None:
			self.client.onTargetUpdate(self.targetId)
		self.witnesses.onTargetUpdate(self.targetId)


	def sendStats(self, mailbox, dirty, clearDirty):
		"""
		Sends available entity stats on the mailbox
		@param mailbox: Receiver end of stats
		@param dirty: Only sends dirty stats if True
		@param clearDirty: Removes dirty flag from all stats if True
		@return:
		"""
		# TODO: There should be a global hasDirtyStats flag to avoid checking all stats
		# each time this function is called (which may be a lot)
		if dirty:
			statList = {id: stat for id, stat in self.stats.items() if stat.dirty}
			baseStatList = {id: stat for id, stat in self.stats.items() if stat.baseDirty}
		else:
			statList = baseStatList = self.stats

		stats = []
		bases = []
		if self.client is not None and mailbox == self.client:
			for statId, stat in statList.items():
				stats.append({
					'StatId': statId,
				    'Min': stat.min,
				    'Current': stat.cur,
				    'Max': stat.max
				})
			for statId, stat in baseStatList.items():
				bases.append({
					'StatId': statId,
				    'Min': stat.baseMin,
				    'Current': stat.baseCur,
				    'Max': stat.baseMax
				})
		else:
			for statId in self.publicStats:
				if statId in statList:
					stat = statList[statId]
					stats.append({
						'StatId': statId,
						'Min': stat.min,
						'Current': stat.cur,
						'Max': stat.max
					})
				if statId in baseStatList:
					stat = baseStatList[statId]
					bases.append({
						'StatId': statId,
						'Min': stat.baseMin,
						'Current': stat.baseCur,
						'Max': stat.baseMax
					})

		if len(bases):
			mailbox.onStatBaseUpdate(bases)
		if len(stats):
			mailbox.onStatUpdate(stats)

		if clearDirty:
			for stat in self.stats.values():
				stat.dirty = False


	def sendDirtyStats(self):
		"""
		Resends all changed stat values to client and witnesses
		"""
		if self.client is not None:
			self.sendStats(self.client, True, False)
		self.sendStats(self.witnesses, True, True)


	def createAppearanceOnClient(self, mailbox):
		if self.bodySet and self.components:
			mailbox.BeingAppearance(self.bodySet, self.components)
			mailbox.onEntityTint(self.primaryColorId, self.secondaryColorId, self.skinTint)
		else:
			super().createAppearanceOnClient(mailbox)


	def createOnClient(self, mailbox):		
		super().createOnClient(mailbox)
		mailbox.onLevelUpdate(self.level)
		mailbox.onTargetUpdate(self.targetId)
		if self.beingName != "":
			mailbox.onBeingNameUpdate(self.beingName)
		mailbox.onAlignmentUpdate(self.alignment)
		mailbox.onFactionUpdate(self.faction)
		mailbox.onStateFieldUpdate(self.stateField)
		
		if self.meleeRange != 0:
			mailbox.onMeleeRangeUpdate(self.meleeRange)
		if self.archetype != Atrea.enums.ARCHETYPE_Any:
			mailbox.onArchetypeUpdate(self.archetype)
		
		self.sendStats(mailbox, False, False)


	def getItemSequence(self, eventId):
		"""
		Returns the kismet event sequence for the specified item event.
		@return: Kismet event sequence or None, if no sequence was found
		"""
		eventSet = DefMgr.get('event_set', Constants.ARCHETYPE_ITEM_EVENT_SETS[self.archetype])
		return eventSet.getSequence(eventId) if eventSet else None


	def getName(self) -> str:
		"""
		Returns the name of the entity
		@return: Entity name
		"""
		if self.beingName != "":
			return self.beingName
		else:
			return super().getName()


	def onDead(self):
		"""
		Called when the entity dies
		"""
		self.abilities.onDead()

		deathSeq = self.getSequence(Atrea.enums.Entity_Death)
		if deathSeq:
			self.abilities.playSequence(deathSeq.seqId, self.entityId, 0)
		self.setStateFlag(Atrea.enums.BSF_Dead)
		if self.tag is not None:
			self.space.fire('entity.dead.tag::' + str(self.tag), entity = self)
		if self.template is not None:
			self.space.fire('entity.dead.template::' + str(self.template.templateName), entity = self)


	def onRevived(self):
		"""
		Called when the entity is revived
		"""
		self.unsetStateFlag(Atrea.enums.BSF_Dead)


	def isDead(self):
		"""
		@return: True if the entity is dead, False otherwise
		"""
		return (self.combatantState & Atrea.enums.PLAYER_STATE_Dead) != 0


	def onAbilityCooledDown(self, ability : Ability):
		"""
		Called when the cooldown timer of an ability expired
		@param ability: Ability
		"""
		pass


	def onAbilityFinished(self):
		"""
		The entity has finished using an ability.
		"""
		pass


	def onAbilityInterrupted(self):
		"""
		The entity was interrupting during an ability use.
		"""
		pass


	def onDamageReceived(self, attacker):
		"""
		Called when the entity received damage from an effect.
		"""
		pass


	def onAttacked(self, attackerId, healthChange, focusChange, damageType):
		"""
		Notifies the entity that it is being attacked by someone else.
		@param attackerId: Attacker entity
		@param healthChange: Amount of health lost/gained
		@param focusChange: Amount of focus lost/gained
		@param damageType: Damage type (see EDamageType enumeration)
		"""
		pass


	def addBodyComponent(self, component):
		"""
		Adds a body component to the component list of the entity
		@param component: Component to add
		"""
		self.components.append(component)
		if self.client is not None:
			self.client.BeingAppearance(self.bodySet, self.components)
		self.witnesses.BeingAppearance(self.bodySet, self.components)


	def removeBodyComponent(self, component):
		"""
		Removes a body component from the component list of the entity
		@param component: Component to remove
		"""
		self.components.remove(component)
		if self.client is not None:
			self.client.BeingAppearance(self.bodySet, self.components)
		self.witnesses.BeingAppearance(self.bodySet, self.components)


	def setName(self, name):
		"""
		Changes the name of this entity
		"""
		self.beingName = name
		if name is not None:
			if self.client is not None:
				self.client.onBeingNameUpdate(self.beingName)
			self.witnesses.onBeingNameUpdate(self.beingName)


	def setAlignment(self, alignment):
		"""
		Changes the alignment of this entity
		"""
		self.alignment = alignment
		if alignment is not None:
			if self.client is not None:
				self.client.onAlignmentUpdate(self.alignment)
			self.witnesses.onAlignmentUpdate(self.alignment)


	def setFaction(self, faction):
		"""
		Changes the faction of this entity
		"""
		self.faction = faction
		if faction is not None:
			if self.client is not None:
				self.client.onFactionUpdate(self.faction)
			self.witnesses.onFactionUpdate(self.faction)


	def setArchetype(self, archetype):
		"""
		Changes the archetype of this entity
		"""
		self.archetype = archetype
		if archetype is not None:
			if self.client is not None:
				self.client.onArchetypeUpdate(self.archetype)
			self.witnesses.onArchetypeUpdate(self.archetype)


	def setLevel(self, level):
		"""
		Changes the level of this entity
		"""
		assert(level >= 1)
		self.level = level
		if self.level > Constants.MAX_LEVEL:
			self.level = Constants.MAX_LEVEL

		if level is not None:
			if self.client is not None:
				self.client.onLevelUpdate(self.level)
			self.witnesses.onLevelUpdate(self.level)


	def getCombatantStateFlag(self, flag):
		"""
		Checks if the specified combatant state flag is set (set count > 0).
		@param flag: Flag to check (see ECombatantState enumeration)
		@return: True if flag is set, False otherwise
		"""
		return self.combatantState & flag


	def setCombatantStateFlag(self, flag):
		"""
		Sets a combatant state flag. Set/unset operations are counted, so
		setting a flag 2 times and then unsetting it will not remove the flag, only
		decreases the set counter. Unsetting a second times removes the flag entirely.

		@param flag: Flag to set (see ECombatantState enumeration)
		@return: Current set counter for the flag
		"""
		if flag not in self.combatantStates:
			self.combatantStates[flag] = 1
			if flag == Atrea.enums.PLAYER_STATE_Dead:
				self.onDead()
		else:
			self.combatantStates[flag] += 1
		self.combatantState |= flag
		return self.combatantStates[flag]


	def unsetCombatantStateFlag(self, flag):
		"""
		Unsets a combatant state flag. Set/unset operations are counted, so
		setting a flag 2 times and then unsetting it will not remove the flag, only
		decreases the set counter. Unsetting a second times removes the flag entirely.

		@param flag: Flag to set (see ECombatantState enumeration)
		@return: Current set counter for the flag
		"""
		if flag in self.combatantStates and self.combatantStates[flag] > 0:
			self.combatantStates[flag] -= 1
			if self.combatantStates[flag] == 0:
				self.combatantState &= ~flag
				if flag == Atrea.enums.PLAYER_STATE_Dead:
					self.onRevived()
			return self.combatantStates[flag]
		else:
			error(Atrea.enums.LC_Uncategorized, "Attempted to unset combatant state flag %d too many times!" % flag)
			return 0


	def hasStateFlag(self, flag):
		"""
		Checks if specified state flag is set on the entity
		@param flag: Flag to check (see EStateField enum; only one flag allowed!)
		@return True if flag is set, False otherwise
		"""
		return self.stateField & (1 << flag) != 0


	def setStateFlag(self, flag):
		"""
		Sets a being state flag
		@param flag: Flag to set (see EStateField enum; only one flag allowed!)
		"""
		if not self.stateField & (1 << flag):
			self.stateField |= 1 << flag
			if self.client is not None:
				self.client.onStateFieldUpdate(self.stateField)
			self.witnesses.onStateFieldUpdate(self.stateField)


	def unsetStateFlag(self, flag):
		"""
		Resets a being state flag
		@param flag: Flag to reset (see EStateField enum; only one flag allowed!)
		"""
		if self.stateField & (1 << flag):
			self.stateField &= ~(1 << flag)
			if self.client is not None:
				self.client.onStateFieldUpdate(self.stateField)
			self.witnesses.onStateFieldUpdate(self.stateField)


	def addMovementLock(self):
		"""
		Increases the movement lock counter
		"""
		if self.movementLocks == 0:
			self.setStateFlag(Atrea.enums.BSF_MovementLock)
		self.movementLocks += 1


	def removeMovementLock(self):
		"""
		Decreases the movement lock counter
		@return: True if movement is locked, False otherwise
		"""
		assert self.movementLocks > 0
		self.movementLocks -= 1
		if self.movementLocks == 0:
			self.unsetStateFlag(Atrea.enums.BSF_MovementLock)
		return self.movementLocks == 0


	def findPathTo(self, destination):
		"""
		Returns a navigation path to the destination location
		@param destination: Destination vector
		@return: Path to destinazion (None if no path found)
		"""
		return Atrea.findPath(self.spaceId, self.position, destination)


	def findDetailedPathTo(self, destination, startExtents = Atrea.Vector3(0.5, 0.5, 0.5),
		destinationExtents = Atrea.Vector3(3.0, 3.0, 3.0), minDistance = 0.0, maxDistance = 0.0,
		maxPoints = 64, smoothingStep = 1.25):
		"""
		Returns a navigation path to the destination location
		@param destination: Destination vector
		@param startExtents: Max distance between starting pos and starting poly
		@param destinationExtents: Max distance between destination pos and destination poly
		@param minDistance: Minimum allowed distance from destination
		@param maxDistance: Maximum allowed distance from destination
		@param maxPoints: Maximum number of points in initial path
		@param smoothingStep: Distance between smoothed points in the generated path
		@return: Path to destination (None if no path found)
		"""
		return Atrea.findDetailedPath(self.spaceId, self.position, destination,
			startExtents, destinationExtents, minDistance, maxDistance, maxPoints,
			smoothingStep)


	def cancelAction(self):
		"""
		Cancels the current action.
		"""
		if self.action is not None:
			self.action.cancel()
			self.action = None


	def setAction(self, action):
		"""
		Updates the action currently being performed by the entity.
		If another action was in progress, it is canceled.
		@param action: Action instance
		"""
		self.cancelAction()
		self.action = action
		action.start()


	def startPatrol(self, path, delay):
		self.patrolPath = path
		self.patrolDelay = delay
		self.patrolNextPoint = 0
		self.moveToNextPatrolPoint()

	def cancelPatrol(self):
		if self.patrolTimer is not None:
			Atrea.cancelTimer(self.patrolTimer)
		self.cancelMovement()

	def moveToNextPatrolPoint(self):
		self.patrolTimer = Atrea.addTimer(Atrea.getGameTime() + self.patrolDelay, lambda: self.patrolDelayExpired())

	def patrolDelayExpired(self):
		self.patrolTimer = None
		point = self.patrolPath.nodes[self.patrolNextPoint]
		wp = Atrea.Vector3(point['x'], point['y'], point['z'])
		self.addWaypoint(wp, lambda: self.moveToNextPatrolPoint())
		self.patrolNextPoint += 1
		if self.patrolNextPoint >= len(self.patrolPath.nodes):
			self.patrolNextPoint = 0


	def launchReloadAbility(self):
		"""
		Begins reloading the currently equipped weapon.
		@return: True if successful, condition feedback code otherwise
		"""
		reload = DefMgr.get('ability', Constants.ABILITY_ReloadWeapon)
		status = self.abilities.launchAbility(reload, self.entityId, isEntityAbility = False)
		if status is True:
			reloadSeq = self.getItemSequence(Atrea.enums.Item_Reload)
			if reloadSeq is not None:
				self.playSequence(reloadSeq.seqId, self.entityId)
		return status


	def isCrouching(self):
		"""
		@return: True if the entity is crouching, False otherwise
		"""
		return (self.stateField & (1 << Atrea.enums.BSF_Crouching)) != 0


	def onAddedToThreatList(self, entityId):
		"""
		Called by a mob when combatant is added to their threat list
		@param entityId: Mob that has added this combatant to their threat list
		"""
		pass


	def onRemovedFromThreatList(self, entityId):
		"""
		Called by a mob when combatant is removed from their threat list
		@param entityId: Mob that has removed this combatant from their threat list
		"""
		pass


	def beginBusy(self):
		"""
		Begins busy mode (indicates that an action was started)
		"""
		assert not self.busy
		self.busy = True


	def endBusy(self):
		"""
		End busy mode (indicates that an action was finished)
		"""
		assert self.busy
		self.busy = False


	def isBusy(self):
		"""
		Checks if the entity is currently busy
		@return: True if the entity is busy, False otherwise
		"""
		return self.busy


	###################################################################################
	#
	#                             CLIENT RPC METHODS
	#
	###################################################################################

	def setTargetID(self, entityId):
		"""
		Changes the entity currently being targeted by the player.
		@param entityId: Entity ID being targeted
		"""
		self.changeTarget(entityId)
		if hasattr(self, 'debug'):
			self.debug('You selected: ' + str(self.targetId))


	@mustBeAlive
	def setMovementType(self, movementType):
		"""
		Changes the type of movement being used by the client.
		@param movementType: 0 = walking, 1 = running
		"""
		if movementType == 0:
			self.setStateFlag(Atrea.enums.BSF_Walking)
		else:
			self.unsetStateFlag(Atrea.enums.BSF_Walking)


	def toggleCombatDebug(self):
		pass


	def toggleCombatVerboseDebug(self):
		pass


	def confirmationResponse(self, aEffectId, aAccepted):
		print('Being::confirmationResponse: aEffectId:', str(aEffectId), 'aAccepted:', str(aAccepted))


	@mustBeAlive
	def setCrouched(self, enabled):
		"""
		Changes whether the entity is currently crouched or not.
		@param enabled: 1 if crouched, 0 otherwise
		"""
		if enabled:
			self.setStateFlag(Atrea.enums.BSF_Crouching)
		else:
			self.unsetStateFlag(Atrea.enums.BSF_Crouching)


	@mustBeAlive
	def toggleHealDebug(self):
		pass


	@mustBeAlive
	def requestHolsterWeapon(self, aHolster):
		print('Being::requestHolsterWeapon: ', str(aHolster))
