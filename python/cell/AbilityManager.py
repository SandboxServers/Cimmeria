from importlib import import_module
import random
import weakref
import Atrea
import Atrea.enums
import cell.SGWBeing
from common.Config import Config
from common.Logger import error, trace, exc
from common.defs.Ability import Ability
from common.defs.Def import DefMgr
from common.defs.Effect import Effect

class DamageCalc(object):
	"""
	Helper object for calculating damage and various related attributes
	"""

	armorFactorStats = {
		# TODO: does untyped damage have AF?
		Atrea.enums.DT_Untyped: [],
		Atrea.enums.DT_Energy: [Atrea.enums.energyAF, Atrea.enums.energyDensity],
		Atrea.enums.DT_Hazmat: [Atrea.enums.hazmatAF, Atrea.enums.hazmatDensity],
		Atrea.enums.DT_Physical: [Atrea.enums.physicalAF, Atrea.enums.physicalDensity],
		Atrea.enums.DT_Psionic: [Atrea.enums.psionicAF, Atrea.enums.psionicDensity]
	}

	absorptionStats = {
		Atrea.enums.DT_Untyped: [
			Atrea.enums.absorbUntyped,
			Atrea.enums.absorbUntypedEnergy,
			Atrea.enums.absorbUntypedItem
		],
		Atrea.enums.DT_Energy: [
			Atrea.enums.absorbEnergy,
			Atrea.enums.absorbEnergyEnergy,
			Atrea.enums.absorbEnergyItem
		],
		Atrea.enums.DT_Hazmat: [
			Atrea.enums.absorbHazmat,
			Atrea.enums.absorbHazmatEnergy,
			Atrea.enums.absorbHazmatItem
		],
		Atrea.enums.DT_Physical: [
			Atrea.enums.absorbPhysical,
			Atrea.enums.absorbPhysicalEnergy,
			Atrea.enums.absorbPhysicalItem
		],
		Atrea.enums.DT_Psionic: [
			Atrea.enums.absorbPsionic,
			Atrea.enums.absorbPsionicEnergy,
			Atrea.enums.absorbPsionicItem
		]
	}

	def __init__(self, attacker, target):
		"""
		@param attacker: Entity that is attacking
		@type attacker: cell.SGWBeing.SGWBeing
		@param target: Entity being attacked
		@type target: cell.SGWBeing.SGWBeing
		"""
		self.attacker = attacker
		self.target = target


	def calculateResult(self, dontUseQr, ranged : bool):
		"""
		Calculate randomized result code of an attack
		@param dontUseQr: Don't calculate a QR rating
		@param ranged: Is this a ranged or melee attack?
		@return: Result code
		"""
		self.ranged = ranged
		if dontUseQr:
			self.qr = 0.0
		else:
			self.qr = self.calculateQR(ranged)
		self.resultCode = self.resultCodeFromQR(self.qr)
		return self.resultCode


	def calculateDamage(self, baseDamage, damageType, statId):
		"""
		Calculate damaged done on entity
		@param baseDamage: Raw damage value of effect
		@param damageType: See EDamageType enumeration
		@param statId: Stat being damaged (see EStats enumeration)
		@return: Amount of damage done
		"""
		damage = baseDamage * self.qrRand * Config.QR_DAMAGE_MULTIPLIER
		# Early return to avoid needless calculations if we missed
		if damage == 0:
			return {}

		results = []
		statResist = self.calculateStatResistance(statId)
		af = self.calculateArmorFactor(damageType)
		absorbed = self.calculateAbsorbedDamage(damageType)
		damageMult = self.attacker.getStat(Atrea.enums.damage).cur / 100.0 + 1.0
		mitigation = self.target.getStat(Atrea.enums.mitigation).cur - self.attacker.getStat(Atrea.enums.penetration).cur
		afMitigation = round(af * max(0, mitigation) / 100.0)
		self.target.abilities.debug("DmgMul: %f, StatResist: %f, AF: %f, afMitigation: %f, Absorb: %f, QR: %f" % (
			damageMult, statResist, af, afMitigation, absorbed, self.qr))

		resDamage = (damage * damageMult) * (1 - statResist)
		qrDamage = round(resDamage * (1 + self.qr))
		afDamage = max(0, qrDamage - afMitigation)
		absDamage = max(0, afDamage - absorbed)
		if absDamage < afDamage:
			results.append({
				'delta': afDamage - absDamage,
				'resultCode': Atrea.enums.SRC_Absorb
			})
		self.target.abilities.debug("dmgRes: %f, dmgQR: %f, dmgAF: %f, dmgAbs: %f" % (
			resDamage, qrDamage, afDamage, absDamage))
		results.append({
			'delta': -absDamage,
			'resultCode': Atrea.enums.SRC_None
		})
		# TODO: Apply absorbed energy to items
		# TODO #2: When do we get SRC_Immune?
		return results


	def calculateStatResistance(self, statId):
		"""
		Calculates stat resistance multiplier for a given stat on the target
		@param statId: Stat (see EStats enumeration)
		@return: %/100 of damage resisted
		"""
		if statId == Atrea.enums.health:
			res = self.target.getStat(Atrea.enums.fortitude).cur * 0.01
			res += self.target.getStat(Atrea.enums.healthRes).cur * 0.01
			return res
		elif statId == Atrea.enums.focus:
			res = self.target.getStat(Atrea.enums.intelligence).cur * 0.01
			res += self.target.getStat(Atrea.enums.mentalRes).cur * 0.01
			return res
		elif statId == Atrea.enums.energy:
			res = self.target.getStat(Atrea.enums.engagement).cur * 0.01
			res += self.target.getStat(Atrea.enums.kineticRes).cur * 0.01
			return res
		else:
			return 0


	def calculateAbsorbedDamage(self, damageType):
		"""
		Calculates absorbed damage for a given damage type on the target
		@param damageType: See EDamageType enumeration
		@return: Amount of damage absorbed
		"""
		if damageType not in self.absorptionStats:
			raise ValueError("Illegal damage type")

		return sum([self.target.getStat(statId).cur for statId in self.absorptionStats[damageType]])


	def calculateArmorFactor(self, damageType):
		"""
		Calculates armor factor for a given damage type on the target
		@param damageType: See EDamageType enumeration
		@return: AF
		"""
		if damageType not in self.armorFactorStats:
			raise ValueError("Illegal damage type")

		return sum([self.target.getStat(statId).cur for statId in self.armorFactorStats[damageType]])


	def resultCodeFromQR(self, qr : float):
		"""
		Calculates an attack result from the QR rating.
		@param qr: QR value
		@return: Result code (see EResultCode enumeration)
		"""

		# Generate a random number following beta distribution
		# We use beta(base, base + qr) if qr >= 0,
		# and beta(base - qr, base) if qr < 0.
		if qr >= 0:
			result = random.betavariate(Config.QR_ALPHA_BETA, Config.QR_ALPHA_BETA + qr * Config.QR_MULTIPLIER)
		else:
			result = random.betavariate(Config.QR_ALPHA_BETA - qr * Config.QR_MULTIPLIER, Config.QR_ALPHA_BETA)
		self.qrRand = result
		self.target.abilities.debug("Randomized QR rate: %f (qr: %f)" % (result, qr))

		if result < Config.QR_MISS:
			return Atrea.enums.RC_Miss
		elif result < Config.QR_GLANCING:
			return Atrea.enums.RC_Glancing
		elif result < Config.QR_CRITICAL_HIT:
			return Atrea.enums.RC_Hit
		elif result < Config.QR_DOUBLE_CRITICAL_HIT:
			return Atrea.enums.RC_Critical
		else:
			return Atrea.enums.RC_DoubleCritical


	def calculateQR(self, ranged : bool):
		"""
		Calculates QR rating.
		@param ranged: Is this a ranged or melee attack?
		@return: QR rating
		"""
		# TODO: tracking, stabilization, coverQR, coverAcc/Def
		attacker, target = self.attacker, self.target

		qr = 0.0
		if ranged:
			qr += attacker.getStat(Atrea.enums.coordination).cur * 0.05
		else:
			qr += attacker.getStat(Atrea.enums.engagement).cur * 0.05
		qr -= target.getStat(Atrea.enums.perception).cur * 0.05
		qr += attacker.getStat(Atrea.enums.accuracy).cur * 0.01
		qr -= target.getStat(Atrea.enums.defense).cur * 0.01
		qr += attacker.getStat(Atrea.enums.qrMod).cur * 1.0
		qr -= target.getStat(Atrea.enums.qrMod).cur * 1.0
		qr += attacker.getStat(Atrea.enums.awareness).cur * 0.01
		if attacker.isCrouching():
			qr += attacker.getStat(Atrea.enums.crouchingAccuracy).cur * 0.01
		if target.isCrouching():
			qr -= target.getStat(Atrea.enums.crouchingDefense).cur * 0.01
		if ranged:
			attType = "Ranged"
		else:
			attType = "Melee"
		target.abilities.debug("%s QR for %s (%d) -> %s (%d): %f" % (attType, attacker.getName(),
			attacker.entityId, target.getName(), target.entityId, qr))
		return qr


class EffectInstance(object):
	"""
	Effect flags that aren't handled:
		EF_Beneficial_Effect - used by AI to determine if ability is hostile?
		EF_Offline_Time_Counts - count cooldown even when offline
		EF_HasInductionBar - deploy/grenade bar
		EF_RemoveOnDisguiseZeroed - needs stealth system
		EF_ResolveOnAbilityUser - ???
		EF_DisableDisguiseWhenRemoved - needs stealth system
		EF_AlwaysPersist - persists after login/logout, world change, death, ...
		EF_Response = 1048576
		EF_RemoveOnStealthZeroed = 2097152
		EF_CalculateQRFromTarget = calc QR from target *location* (not reverse QR calc!)
		EF_OnlySendToGroup, EF_Response, EF_PromptConfirmationDialog - UNUSED
	"""
	# TODO: Should we use EStatValueType SVT_ constants here?
	STAT_Absolute = 0
	STAT_CurrentPercentage = 1
	STAT_MaxPercentage = 2
	STAT_MinMaxPercentage = 3

	# Maps effects result codes to client kismet event types
	resultEffects = {
		Atrea.enums.RC_None: Atrea.enums.Effect_Pulse_End,
		Atrea.enums.RC_Hit: Atrea.enums.Effect_Hit_Normal,
		Atrea.enums.RC_Miss: Atrea.enums.Effect_Hit_Miss,
		Atrea.enums.RC_Critical: Atrea.enums.Effect_Hit_Crit,
		Atrea.enums.RC_DoubleCritical: Atrea.enums.Effect_Hit_Double_Crit,
		Atrea.enums.RC_Glancing: Atrea.enums.Effect_Hit_Glancing
	}

	pulseTimer = None
	script = None
	manager = None
	# Result code of last QR action
	resultCode = Atrea.enums.RC_None
	# Index of the last effect result that was sent to the client
	lastSentChange = 0
	# Shared QR instance
	damageCalc = None

	def __init__(self, manager, effect : Effect, invokerId : int, instanceId : int):
		"""
		@param manager: ability manager
		@param effect: Template of effect
		@param invokerId: ID of entity that launched the effect
		@param instanceId: Unique ID of this effect instance
		"""
		self.manager = manager
		self.effect = effect
		self.invokerId = invokerId
		self.instanceId = instanceId
		self.duration = effect.pulseDuration * effect.pulseCount
		self.completeTime = Atrea.getGameTime() + self.duration
		self.remainingPulses = effect.pulseCount
		self.statChanges = {}
		self.changes = []
		self.script = None
		if self.effect.scriptName is not None:
			mod = import_module('cell.effects.' + self.effect.scriptName)
			self.script = getattr(mod, self.effect.scriptName)(manager.entity(), self, self.effect.nvps)


	def init(self):
		"""
		Initializes the effect and runs an initial pulse
		"""
		self.doAction("onEffectInit", Atrea.enums.Effect_Init)
		self.pulse()


	def playPulseSequence(self, resultCode):
		sequenceEventId = self.resultEffects[resultCode]
		seq = self.effect.getSequence(sequenceEventId)
		if seq:
			self.manager.playSequence(seq.seqId, self.invokerId, 0)


	def pulse(self):
		"""
		Applies the effect to the targeted entity on each pulse
		"""
		# Pulse count = 0 --> effect is pulsed until it is removed manually
		if self.effect.pulseCount != 0:
			if self.remainingPulses == 0:
				self.pulseTimer = None
				self.manager.removeEffect(self.effect)
				return
			self.remainingPulses -= 1

		self.pulseTimer = Atrea.addTimer(Atrea.getGameTime() + self.effect.pulseDuration, lambda: self.pulse())
		# TODO: Do we need this check? The player will remove effects with EF_ClearOnDeath anyway
		# TODO: This will remove stasis sickness, but it shouldn't !
		if self.manager.entity().isDead():
			self.manager.debug("Not pulsing effect <%s>: entity is dead" % self.effect.name)
			return

		self.doAction("onPulseBegin", Atrea.enums.Effect_Pulse_Begin)
		self.playPulseSequence(self.resultCode)
		self.doAction("onPulseEnd", Atrea.enums.Effect_Pulse_End)


	def remove(self):
		"""
		Reverts all temporary actions applied by the effect
		"""
		if self.pulseTimer:
			Atrea.cancelTimer(self.pulseTimer)

		for statId, difference in self.statChanges.items():
			self.manager.entity().getStat(statId).change(-difference)

		self.doAction("onEffectRemoved", Atrea.enums.Effect_Removed)


	def qrCombatDamage(self, statId, damageType, baseDamage, ranged = True, useSharedQr = True):
		"""
		Deals damage by using the QR system.
		@param statId: Stat to damage (see EStat enumeration)
		@param damageType: Type of damage (see EDamageType enumeration)
		@param baseDamage: Amount of damage (before QA multipliers)
		@param ranged: Is this a ranged or melee attack?
		@param useSharedQr: Use the same randomized QR rating value for all QR calculations for this effect instance
		@return Amount of damage dealt, remaining damage
		"""
		self.manager.debug("qrCombatDamage: statId=%d, damageType=%d, baseDamage=%d" % (statId, damageType, baseDamage))
		invoker = Atrea.findEntity(self.invokerId)
		if useSharedQr:
			if self.damageCalc is None:
				self.damageCalc = DamageCalc(invoker, self.manager.entity())
				self.resultCode = self.damageCalc.calculateResult((self.effect.flags & Atrea.enums.EF_DontUseQR) != 0, ranged)
			results = self.damageCalc.calculateDamage(baseDamage, damageType, statId)
		else:
			damageCalc = DamageCalc(invoker, self.manager.entity())
			self.resultCode = damageCalc.calculateResult((self.effect.flags & Atrea.enums.EF_DontUseQR) != 0, ranged)
			results = damageCalc.calculateDamage(baseDamage, damageType, statId)

		focus, health, remaining, total = 0, 0, 0, 0
		for result in results:
			# calculateDamage() returns multiple result codes
			# Only update stats if the result code in RC_None, as the others indicate that
			# the damage was resisted/absorbed/etc.
			if result['resultCode'] == Atrea.enums.SRC_None:
				changed = self.manager.entity().getStat(statId).change(result['delta'])
				remaining += result['delta'] - changed
				total += result['delta']
				result['delta'] = changed
				# If the effect killed the entity it should be marked as SRC_Mortal
				if statId == Atrea.enums.health and self.manager.entity().isDead():
					result['resultCode'] = Atrea.enums.SRC_Mortal

			if statId == Atrea.enums.health:
				health += result['delta']
			elif statId == Atrea.enums.focus:
				focus += result['delta']

			self.changes.append({
				'StatID': statId,
				'Delta': result['delta'],
				'DamageCode': damageType,
				'StatResultCode': result['resultCode']
			})

		if health or focus:
			self.manager.entity().onAttacked(self.invokerId, health, focus, damageType)

		return {
			'remaining': remaining,
			'total': total,
			'resultCode': self.resultCode
		}


	def changeStat(self, statId, changeType, amount, isPermanent):
		"""
		Changes the value of an entity stat.
		@param statId: Stat to change (see EStat enumeration)
		@param changeType: Type of change (one of the STAT_xxx constants)
		@param amount: Amount (either absolute or percentage)
		@param isPermanent: Is the change permanent or is it canceled when the effect is removed?
		@return: Difference between new and old values
		"""
		if changeType == self.STAT_Absolute:
			difference = self.manager.entity().getStat(statId).change(int(amount))
		elif changeType == self.STAT_CurrentPercentage:
			difference = self.manager.entity().getStat(statId).changeByPercent(int(amount / 100))
		elif changeType == self.STAT_MaxPercentage:
			difference = self.manager.entity().getStat(statId).changeByMaxPercent(int(amount / 100))
		elif changeType == self.STAT_MinMaxPercentage:
			difference = self.manager.entity().getStat(statId).changeByMinMaxPercent(int(amount / 100))
		else:
			raise RuntimeError("Invalid stat change type: %d" % changeType)
		self.manager.debug("Stat %d changed (diff: %d)" % (statId, difference))

		# TODO: Hide non-public stat changes from everyone except the player
		# TODO #2: Do we need to send non-combat stats (not HP/FP) here?
		self.changes.append({
			'StatID': statId,
			'Delta': difference,
			'DamageCode': Atrea.enums.DT_Untyped,
			'StatResultCode': Atrea.enums.SRC_None
		})

		# Store the change amount so we can restore it later
		if not isPermanent:
			if statId in self.statChanges:
				self.statChanges[statId] += difference
			else:
				self.statChanges[statId] = difference
		return difference


	def doAction(self, action, eventId):
		"""
		Executes an effect action and returns effect result code to the client
		@param action: Action name to execute
		@param eventId: Kismet event ID (see ESequenceEventType enumeration)
		"""
		if self.script is not None:
			self.resultCode = Atrea.enums.RC_None
			getattr(self.script, action)()
			if len(self.changes) > self.lastSentChange:
				changesToSend = self.changes[self.lastSentChange:]
				self.lastSentChange = len(self.changes)
				ent = self.manager.entity()
				if ent.client is not None:
					ent.sendStats(ent.client, True, False)
					ent.client.onEffectResults(self.invokerId, self.effect.ability.id, self.effect.id,
						ent.entityId, self.resultCode, changesToSend)
				ent.sendStats(ent.witnesses, True, True)
				if not (self.effect.flags & Atrea.enums.EF_OnlySendToSelf):
					ent.witnesses.onEffectResults(self.invokerId, self.effect.ability.id, self.effect.id,
						ent.entityId, self.resultCode, changesToSend)
				ent.sendDirtyStats()

			# Notify the ability manager that the entity received some damage
			if self.resultCode != Atrea.enums.RC_None:
				self.manager.onDamageReceived(Atrea.findEntity(self.invokerId))

			sequence = self.effect.getSequence(eventId)
			if sequence:
				self.manager.playSequence(sequence.seqId, self.invokerId, 0)



class AbilityInstance(object):
	"""
	Instance of an active ability
	@ivar ability     The type of ability being launched
	@ivar warmupTimer Timer tracking warmup state of ability
	"""
	manager = None
	entity = None
	ability = None
	warmupTimer = None
	targetId = None
	targetLoc = None
	target = None

	# Condition feedback codes returned when a position check fails
	positionCheckCodes = {
		Atrea.enums.CAS_POSITION_ABOVE: Atrea.enums.CONDITION_FEEDBACK_PositionCheckAbove,
		Atrea.enums.CAS_POSITION_BELOW: Atrea.enums.CONDITION_FEEDBACK_PositionCheckBelow,
		Atrea.enums.CAS_POSITION_FRONT: Atrea.enums.CONDITION_FEEDBACK_PositionCheckFront,
		Atrea.enums.CAS_POSITION_FLANK: Atrea.enums.CONDITION_FEEDBACK_PositionCheckFlank,
		Atrea.enums.CAS_POSITION_REAR:  Atrea.enums.CONDITION_FEEDBACK_PositionCheckRear
	}

	def __init__(self, manager, entity, ability):
		"""
		@param manager: Ability manager
		@param entity: Entity using this ability
		@param ability: Ability type
		"""
		self.manager = manager
		self.entity = entity
		self.ability = ability


	def setTarget(self, targetId : int, targetLoc : Atrea.Vector3 = None):
		"""
		Sets the entity/location being targeted by this instance.
		@param targetId: Target entity ID
		@param targetLoc: Target position (on the same space as the entity)
		"""
		self.targetId = targetId
		self.targetLoc = targetLoc


	def canUse(self):
		"""
		Checks if all targeting parameters are correct and prerequisites are met.
		@return: True if ability can be used; Condition feedback code (int) otherwise
		"""
		if self.ability.targetTypeId == Atrea.enums.TargetSelf:
			self.target = self.entity
		elif self.ability.targetTypeId == Atrea.enums.TargetTarget:
			target = Atrea.findEntityOnSpace(self.targetId, self.entity.spaceId) if self.targetId else None
			if target is None:
				self.manager.debug("Failed to launch <%s>: Target %d doesnt exist on this space" % (self.ability.name, self.targetId or 0))
				return Atrea.enums.CONDITION_FEEDBACK_InvalidEntity

			if not isinstance(target, cell.SGWBeing.SGWBeing):
				self.manager.debug("Failed to launch <%s>: Target %d is not a Being" % (self.ability.name, self.targetId))
				return Atrea.enums.CONDITION_FEEDBACK_InvalidEntity

			if target.isDead():
				self.manager.debug("Failed to launch <%s>: Target %d is dead" % (self.ability.name, self.targetId))
				return Atrea.enums.CONDITION_FEEDBACK_NotLiving

			if self.ability.requiresWeapons() and not self.manager.entity().hasItemMoniker(self.ability.itemMonikers):
				self.manager.debug("Failed to launch <%s>: Equipped weapon moniker check failed: %s" %
								   (self.ability.name, str(self.ability.itemMonikers)))
				return Atrea.enums.CONDITION_FEEDBACK_WrongWeaponType

			if self.ability.requiredAmmo > 0:
				ammo = self.manager.entity().getAmmoCount()
				if ammo < self.ability.requiredAmmo:
					self.manager.debug("Failed to launch <%s>: %d ammo needed, have %d" %
									   (self.ability.name, self.ability.requiredAmmo, ammo))
					return Atrea.enums.CONDITION_FEEDBACK_AmmoCountLessThan

			if self.ability.flags & Atrea.enums.UseWeaponRange:
				minRange, maxRange = self.manager.entity().getWeaponRange(self.ability.ranged)
			else:
				minRange, maxRange = self.ability.minRange, self.ability.maxRange

			distance = self.entity.distanceTo(target.position)
			if distance < minRange or distance > maxRange != 0:
				self.manager.debug("Failed to launch <%s>: Out of range (min %f, max %f, curr %f)" %
								   (self.ability.name, minRange, maxRange, distance))
				return Atrea.enums.CONDITION_FEEDBACK_OutsideWeaponRange

			facing = self.entity.facingType(target)
			if not (self.ability.positions & (1 << facing)):
				self.manager.debug("Failed to launch <%s>: Invalid facing (got %d, mask %d)" %
								   (self.ability.name, facing, self.ability.positions))
				return self.positionCheckCodes[facing]

			# TODO: Do LOS checks on target
			self.target = target
		else:
			# TODO: Implement all targeting methods
			self.manager.debug("Failed to launch <%s>: TT %d not supported" % (self.ability.name, self.ability.targetTypeId))
			return Atrea.enums.CONDITION_FEEDBACK_Deprecated

		return True


	def launch(self):
		"""
		Attempts to launch the specified ability.
		"""
		self.manager.debug("Launching ability <%s>" % self.ability.name)
		now = Atrea.getGameTime()
		ent = self.entity
		abilityWarmup = self.ability.warmup
		# speedGrenade stat: decreases warmup time of grenade abilities by 1%
		if self.ability.flags & Atrea.enums.SpeedGrenade:
			abilityWarmup *= 1 - ent.getStat(Atrea.enums.speedGrenade).cur / 100.0
		# speedDeploy stat: decreases warmup time of deployable abilities by 1%
		if self.ability.flags & Atrea.enums.SpeedDeploy:
			abilityWarmup *= 1 - ent.getStat(Atrea.enums.speedDeploy).cur / 100.0
		# speedAttack stat: decreases warmup time of attack abilities by 1%
		if self.ability.flags & Atrea.enums.SpeedAttack:
			abilityWarmup *= 1 - ent.getStat(Atrea.enums.speedAttack).cur / 100.0

		abilityCooldown = self.ability.cooldown
		# response stat: decreases the reuse time of abilities with the response ability flag by 1%
		if self.ability.flags & Atrea.enums.Response:
			abilityCooldown *= 1 - ent.getStat(Atrea.enums.response).cur / 100.0

		cooldown = now + abilityCooldown + abilityWarmup
		self.manager.addCooldownTimer(self.ability, cooldown)

		# Only send a timer update if the player has the ability
		# (won't send updates for item abilities that aren't on the ability bar, those aren't visible anyway)
		if self.ability.id in self.manager.abilities:
			# TODO: Do witnesses/squad members need to see the ability cooldown?
			if ent.client is not None:
				ent.client.onTimerUpdate(self.ability.id, Atrea.enums.AbilityCooldown, ent.entityId,
					0, abilityCooldown + abilityWarmup, cooldown)
				for monikerId in self.ability.monikerIds:
					ent.client.onTimerUpdate(monikerId, Atrea.enums.CategoryCooldown, ent.entityId,
						0, abilityCooldown + abilityWarmup, cooldown)

		if abilityWarmup > 0:
			self.manager.debug("Waiting for warmup (now: %f; warmed up at: %f)" % (now, now + abilityWarmup))
			beginSeq = self.ability.getSequence(Atrea.enums.Ability_Begin)
			if beginSeq is not None:
				self.manager.playSequence(beginSeq.seqId, ent.entityId, 0, self.targetId)
			self.warmupAbility = self.ability
			warmup = now + abilityWarmup
			self.warmupTimer = Atrea.addTimer(warmup, lambda: self.afterWarmup())

			# Only send a timer update if the player has the ability
			# (won't send updates for item abilities that aren't on the ability bar, those aren't visible anyway)
			if self.ability.id in self.manager.abilities:
				# TODO: Do witnesses/squad members need to see the ability warmup?
				if ent.client is not None:
					ent.client.onTimerUpdate(self.ability.id, Atrea.enums.AbilityWarmup, ent.entityId,
						0, abilityWarmup, warmup)
		else:
			self.afterWarmup()

		self.entity.fire('ability.launched::' + str(self.ability.id), entity = self.entity, instance = self)
		return True


	def interrupt(self):
		"""
		Attempts to interrupt the ability currently being used (channeled / warmed up).
		@return: True if the ability was interrupted, False otherwise
		"""
		if self.warmupTimer is not None:
			self.manager.debug("Ability <%s> warmup interrupted at %f" % (self.ability.name, Atrea.getGameTime()))
			Atrea.cancelTimer(self.warmupTimer)
			ent = self.manager.entity()
			if ent.client is not None:
				ent.client.onTimerUpdate(self.ability.id, Atrea.enums.AbilityWarmup, ent.entityId, 0, 0, 0)
			interruptSeq = self.ability.getSequence(Atrea.enums.Ability_Interrupt)
			if interruptSeq is not None:
				self.manager.playSequence(interruptSeq.seqId, self.warmupAbilityParams['invokerId'], 0, self.targetId)
			self.warmupTimer = None
			self.warmupAbilityParams = None
			self.entity.fire('ability.interrupted::' + str(self.ability.id), entity = self.entity, instance = self)

		return True


	def afterWarmup(self):
		"""
		Called when the warmup timer for the ability expired and the ability was not canceled
		"""
		self.manager.debug("Ability <%s> warmed up at %f" % (self.ability.name, Atrea.getGameTime()))
		self.warmupTimer = None
		if self.ability.requiredAmmo > 0:
			self.manager.entity().consumeAmmo(self.ability.requiredAmmo)
		endSeq = self.ability.getSequence(Atrea.enums.Ability_End)
		if endSeq is not None:
			self.manager.playSequence(endSeq.seqId, self.entity.entityId, 0, self.targetId)
		targets = self.collectTargets()
		for effect in self.ability.effects:
			for target in targets:
				try:
					target.abilities.addEffect(effect, self.entity.entityId)
				except Exception:
					error(Atrea.enums.LC_Ability, "Exception while adding effect %s: " % effect.name)
					exc(Atrea.enums.LC_Ability)
		# TODO: Only call this if ability is not channeled!
		self.entity.fire('ability.finished::' + str(self.ability.id), entity = self.entity, instance = self)
		self.manager.abilityFinished()


	def collectTargets(self):
		"""
		Returns a list of entities targeted by this ability
		"""
		if self.ability.targetTypeId == Atrea.enums.TargetSelf or self.ability.targetTypeId == Atrea.enums.TargetTarget:
			return [self.target]
		else:
			# TODO: Implement all targeting methods
			assert False



class AbilityManager(object):
	currentAbility = None
	autoCycleAbility = None
	autoCycle = False
	debugPlayer = None

	def __init__(self, entity):
		self.entity = weakref.ref(entity)
		self.monikerCooldowns = {}
		self.cooldownTimers = {}
		self.abilities = []
		self.effectInstances = {}
		self.monikers = {}


	def backup(self):
		"""
		Creates a backup of the ability manager state
		"""
		return self.abilities


	def restore(self, backup):
		"""
		Restores the ability manager from a backed up state
		"""
		self.abilities = backup
		# TODO: Restore timers and instances


	def setDebuggingPlayer(self, player):
		"""
		Sets the player that receives ability debug messages from this manager
		@param player: SGWPlayer
		@return: Previous debugging player (or None if no player was attached)
		"""
		old = self.debugPlayer
		if player is None:
			self.debugPlayer = None
		else:
			self.debugPlayer = weakref.ref(player)
		return old


	def debug(self, msg : str):
		"""
		Sends a debug message to the player
		@param msg: Message to send
		"""
		if self.debugPlayer and self.debugPlayer():
			self.debugPlayer().debug(msg)
		else:
			ent = self.entity()
			if ent.client is not None:
				ent.debug(msg)


	def hasAbility(self, abilityId):
		"""
		Checks if the entity has the specified ability in its ability list
		@param abilityId: Ability ID  to check
		"""
		return any(a == abilityId for a in self.abilities)


	def add(self, abilities):
		"""
		Adds the specified abilities to the list of known abilities
		@param abilities: Ability ID or list of ability IDs
		"""
		if type(abilities) == int:
			if abilities not in self.abilities:
				self.abilities.append(abilities)
		else:
			self.abilities = list(set(self.abilities + abilities))


	def remove(self, abilities):
		"""
		Removes the specified abilities from the list of known abilities
		@param abilities: Ability ID or list of ability IDs
		"""
		if type(abilities) == int:
			if abilities in self.abilities:
				self.abilities.remove(abilities)
		else:
			self.abilities = list(set(self.abilities) - set(abilities))


	def canUseAbility(self, ability : Ability, isEntityAbility : bool = True):
		"""
		Checks if the specified ability can be launched by the entity.
		@param ability: Ability to test
		@param isEntityAbility: Should the entity possess the ability?
		@return: True if ability can be used; Condition feedback code (int) otherwise
		"""
		if self.entity().isDead():
			self.debug("Failed to launch ability <%s>: Ability user is dead" % ability.name)
			return Atrea.enums.CONDITION_FEEDBACK_NotLiving

		if self.currentAbility:
			self.debug("Failed to launch ability <%s>: Entity is already using an ability" % ability.name)
			return Atrea.enums.CONDITION_FEEDBACK_Deprecated

		if isEntityAbility and ability.id not in self.abilities:
			self.debug("Failed to launch ability <%s>: Entity doesn't have ability" % ability.name)
			return Atrea.enums.CONDITION_FEEDBACK_EntityDoesNotHaveAbility

		if self.isCoolingDown(ability):
			self.debug("Failed to launch ability <%s>: Cooldown not ready" % ability.name)
			return Atrea.enums.CONDITION_FEEDBACK_WeaponCooldownNotReady

		if ability.targetCollectionMethod != Atrea.enums.TCM_Single:
			self.debug("Failed to launch ability <%s>: TCM %d not supported" % (ability.name, ability.targetCollectionMethod))
			return Atrea.enums.CONDITION_FEEDBACK_Deprecated

		# TODO: Check ammo count

		return True


	def updateEffectTimer(self, instance : EffectInstance):
		ent = self.entity()
		if ent.client is not None:
			ent.client.onTimerUpdate(instance.effect.id, Atrea.enums.DurationEffect, ent.entityId,
				instance.effect.id, instance.duration, instance.completeTime)
		ent.witnesses.onTimerUpdate(instance.effect.id, Atrea.enums.DurationEffect, ent.entityId,
			instance.effect.id, instance.duration, instance.completeTime)


	def isCoolingDown(self, ability):
		"""
		Checks if a cooldown timer is currently running for this ability.
		@param ability: Ability to check
		@return True if ability is cooling down, False otherwise
		"""
		if ability.id in self.cooldownTimers:
			return True

		for monikerId in ability.monikerIds:
			if monikerId in self.monikerCooldowns:
				return True

		return False


	def addMonikerTimer(self, monikerId, completeTime):
		"""
		Starts a cooldown timer for an ability group (moniker).
		@param completeTime: Timer belongs to this moniker
		@param completeTime: Game time when the ability completes
		"""
		assert monikerId not in self.monikerCooldowns
		self.monikerCooldowns[monikerId] = Atrea.addTimer(completeTime - 0.001, lambda : self.monikerCooledDown(monikerId))


	def addCooldownTimer(self, ability, completeTime):
		"""
		Starts a cooldown timer for an ability.
		@param ability: Timer belongs to this ability
		@param completeTime: Game time when the ability completes
		"""
		assert(ability.id not in self.cooldownTimers)
		# Moniker timers must be added first to make sure that they expire before the
		# ability manager tries to auto cycle the ability
		for monikerId in ability.monikerIds:
			self.addMonikerTimer(monikerId, completeTime)
		self.cooldownTimers[ability.id] = Atrea.addTimer(completeTime, lambda: self.abilityCooledDown(ability))


	def abilityFinished(self):
		"""
		Notifies the ability manager that the current ability is finished and the
		entity can start performing a new ability.
		"""
		assert self.currentAbility is not None
		self.currentAbility = None
		self.entity().onAbilityFinished()


	def playSequence(self, sequenceId : int, sourceId : int, instanceId : int, targetId = None):
		"""
		Plays a kismet sequence for all witnesses
		@param sequenceId: Kismet event sequence ID to play
		@param sourceId: Source entity ID
		@param instanceId: Effect instance ID
		@param targetId: Target instance ID
		"""
		ent = self.entity()
		trace(Atrea.enums.LC_Visuals, "Playing sequence %d (from %d to %d)" % (sequenceId, sourceId, targetId or ent.entityId))
		if ent.client is not None:
			ent.client.onSequence(sequenceId, sourceId, targetId or ent.entityId, 1, 0, [],
				Atrea.enums.KISMET_VIEW_Witness, instanceId)
		ent.witnesses.onSequence(sequenceId, sourceId, targetId or ent.entityId, 1, 0, [],
			Atrea.enums.KISMET_VIEW_Witness, instanceId)


	def hasMoniker(self, monikerId):
		"""
		Checks if the entity has specified moniker
		@param monikerId: Moniker to check
		@return True if the moniker is present, False otherwise
		"""
		return monikerId in self.monikers and len(self.monikers[monikerId]) > 0


	def removeMoniker(self, monikerId):
		"""
		Removes all effects from the entity that have the specified moniker
		@param monikerId: Moniker to remove
		"""
		if monikerId not in self.monikers:
			return

		for effect in self.monikers[monikerId][:]:
			self.debug("Clearing effect <%s> due to monikerId %d" % (effect.effect.name, monikerId))
			self.removeEffect(effect)


	def hasEffect(self, effectId):
		"""
		Checks if the entity has an active instance of the specified effect
		@param effectId: Effect to check
		@return True if the effect is present, False otherwise
		"""
		return effectId in self.effectInstances


	def removeEffect(self, effect : Effect):
		"""
		Removes an effect from the entity
		@param effect: Effect to remove
		"""
		if effect.id in self.effectInstances:
			self.debug("Removing effect <%s> from %s (%d)" % (effect.name, self.entity().getName(),
				self.entity().entityId))
			instance = self.effectInstances[effect.id]
			instance.remove()
			del self.effectInstances[effect.id]


	def removeEffectWithFlag(self, effectFlags):
		"""
		Removes all active effects that have at least one of the effect flags set.
		@param effectFlags: Effect flags mask (see EEffectFlag enumeration)
		"""
		# This needs to make a copy of the effect list as values() only provides a view to the dict
		# and would raise an exception if the dict is modified while iterating
		for effect in list(filter(lambda e: e.effect.flags & effectFlags, self.effectInstances.values())):
			self.debug("Clearing effect <%s> due to effect flag %d" % (effect.effect.name, effectFlags))
			self.removeEffect(effect.effect)


	def addEffect(self, effect : Effect, invokerId : int):
		"""
		@param effect: Effect to add
		@param invokerId: ID of entity that launched the effect
		"""
		self.removeEffect(effect)
		self.debug("Adding effect <%s> on %s (%d)" % (effect.name, self.entity().getName(), self.entity().entityId))
		instance = EffectInstance(self, effect, invokerId, effect.id)
		self.effectInstances[effect.id] = instance
		self.updateEffectTimer(instance)
		instance.init()


	def abilityCooledDown(self, ability : Ability):
		"""
		Called when the cooldown timer for an ability expired
		@param ability: Ability whose timer expired
		"""
		del self.cooldownTimers[ability.id]
		self.entity().onAbilityCooledDown(ability)

		if self.autoCycle:
			target = self.entity().space.findEntity(self.entity().targetId)
			if target is None or target.isDead():
				self.autoCycle = False
				self.entity().stoppedAutoCycling()
			else:
				self.launchAbility(self.autoCycleAbility, self.entity().targetId, autoCycle = True, isEntityAbility = False)


	def monikerCooledDown(self, monikerId):
		"""
		Called when the cooldown timer for an ability moniker expired
		@param monikerId: Moniker whose timer expired
		"""
		del self.monikerCooldowns[monikerId]
		# TODO: Notify entity about moniker cooldowns (or ability cooldowns tied to monikers?)


	def interruptAbility(self):
		"""
		Interrupts the ability currently being used (channeled / warmed up).
		"""
		if self.currentAbility is not None and self.currentAbility.interrupt():
			if self.currentAbility.ability.id in self.cooldownTimers:
				# TODO: Should we cancel moniker timers as well?
				Atrea.cancelTimer(self.cooldownTimers[self.currentAbility.ability.id])
				del self.cooldownTimers[self.currentAbility.ability.id]
			self.currentAbility = None
			self.entity().onAbilityInterrupted()


	def useAbility(self, abilityId : int, targetId : int, targetLoc : Atrea.Vector3 = None):
		"""
		Attempts to launch the specified ability.
		@param abilityId: Ability ID of ability to launch
		@param targetId: Players currently selected target
		@param targetLoc: Target location on ground
		@return: True if successful, condition feedback code if failed
		"""
		ability = DefMgr.get('ability', abilityId)
		if ability is None:
			self.debug("Failed to launch ability <%d>: no such ability" % abilityId)
			return Atrea.enums.CONDITION_FEEDBACK_EntityDoesNotHaveAbility

		# Interrupt auto cycling of active ability
		# TODO: Cycling should be interrupted when entity moves (?)
		if self.autoCycle:
			self.autoCycle = False

		return self.launchAbility(ability, targetId, targetLoc)


	def launchAbility(self, ability : Ability, targetId : int, targetLoc : Atrea.Vector3 = None,
					  autoCycle : bool = False, isEntityAbility : bool = True):
		"""
		Attempts to launch the specified ability.
		@param ability: Ability to launch
		@param targetId: Players currently selected target
		@param targetLoc: Target location on ground
		@param autoCycle: Should auto-cycling be enabled?
		@param isEntityAbility: Should the entity possess the ability?
		@return: True if successful, condition feedback code if failed
		"""
		self.debug("About to launch ability <%s>" % ability.name)
		params = self.canUseAbility(ability, isEntityAbility)
		if type(params) == int:
			return params

		instance = AbilityInstance(self, self.entity(), ability)
		instance.setTarget(targetId, targetLoc)
		code = instance.canUse()
		if code is not True:
			return code

		if ability.flags & Atrea.enums.Deactivate_AutoCycle:
			self.autoCycle = False
			self.entity().stoppedAutoCycling()
		elif autoCycle:
			self.autoCycle = autoCycle
			self.autoCycleAbility = ability

		self.currentAbility = instance
		instance.launch()
		return True


	def onDamageReceived(self, attacker):
		"""
		Called when the entity received damage from an effect.
		"""
		self.removeEffectWithFlag(Atrea.enums.EF_ClearOnDamage)
		if attacker:
			self.entity().onDamageReceived(attacker)


	def onDead(self):
		"""
		Called when the entity died.
		"""
		self.removeEffectWithFlag(Atrea.enums.EF_ClearOnDeath)
		self.interruptAbility()


	def onRevived(self):
		"""
		Called when the entity is revived
		"""
		self.removeEffectWithFlag(Atrea.enums.EF_ClearOnRez)


	def onBandolierSlotChange(self):
		"""
		Called when the active bandolier slot was changed or the active item was swapped/removed
		"""
		self.removeEffectWithFlag(Atrea.enums.EF_RemoveOnBandolierSlotChange)
		self.interruptAbility()
		if self.autoCycle:
			self.autoCycle = False
