###################################################################################
#                         Entity manipulation commands
###################################################################################
from math import pi
import Atrea
import Atrea.enums
import Atrea.enumNames


###################################################################################
#                          SGWSpawnableEntity commands
###################################################################################


def dumpFlags(names, value, strip = 0):
	"""
	Converts a bitmask to a comma separated list of flag names
	@param names: Flag names
	@param value: Value to convert
	@return: Flag names in the specified value
	"""
	flags = []
	for i in range(0, 62):
		flag = 2**i
		if value & flag:
			if flag in names:
				flags.append(names[flag][strip:])
			else:
				flags.append(str(flag))
	return ', '.join(flags)


def dumpFlagsExp(names, value, strip = 0):
	"""
	Converts a bitmask to a comma separated list of flag names
	@param names: Flag names
	@param value: Value to convert
	@return: Flag names in the specified value
	"""
	flags = []
	for i in range(0, max(names.keys()) + 1):
		if value & 2**i:
			if i in names:
				flags.append(names[i][strip:])
			else:
				flags.append(str(2**i))
	return ', '.join(flags)


def entityInfo(player, target, entityId = None):
	"""
	Shows detailed information about the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@param entityId: ID of entity to show
	@type entityId: int
	"""
	if target is None:
		target = Atrea.findEntity(entityId)

	if target is None:
		player.feedback('Could not find entity')
		return

	player.feedback(' ----- ENTITY %s (%d) ----- ' % (target.getName(), target.entityId))
	if target.entityFlags != 0:
		player.feedback('Flags: %s' % dumpFlags(Atrea.enumNames.EEntityFlags, target.entityFlags, 11))
	if target.interactionType != 0:
		player.feedback('Interaction: %s' % dumpFlags(Atrea.enumNames.EInteractionNotificationType, target.interactionType, 4))
	if target.kismetEventSetId != 0:
		player.feedback('Kismet event set: %d' % target.kismetEventSetId)
	if target.beingNameId != 0:
		player.feedback('Name ID: %d' % target.beingNameId)
	player.feedback('Colors: %x, %x, %x' % (target.primaryColorId, target.secondaryColorId, target.skinTint))
	if target.template is not None:
		player.feedback('Template: %s (%d)' % (target.template.templateName, target.template.id))
	if target.tag is not None:
		player.feedback('Tag: %s ' % target.tag)
	if target.staticMeshName is not None:
		player.feedback('Static mesh: %s' % target.staticMeshName)
	if target.bodySet is not None:
		player.feedback('Body set: %s' % target.bodySet)
	if hasattr(target, 'archetype'):
		player.feedback('Archetype: %s (%d)' % (Atrea.enumNames.EArchetype[target.archetype][10:], target.archetype))
	if hasattr(target, 'alignment'):
		player.feedback('Alignment: %s (%d)' % (Atrea.enumNames.EAlignment[target.alignment][10:], target.alignment))
	if hasattr(target, 'faction'):
		player.feedback('Faction: %s (%d)' % (Atrea.enumNames.EFaction[target.faction][8:], target.faction))
	if hasattr(target, 'level'):
		player.feedback('Level: %d' % target.level)
	if hasattr(target, 'beingName') and target.beingName != '':
		player.feedback('Being Name: %s' % target.beingName)
	if hasattr(target, 'stateField'):
		player.feedback('State: %s' % dumpFlagsExp(Atrea.enumNames.EStateField, target.stateField, 4))
	if hasattr(target, 'meleeRange'):
		player.feedback('Melee range: %f' % target.meleeRange)
	if hasattr(target, 'combatantState'):
		player.feedback('Combatant State: %s' % dumpFlags(Atrea.enumNames.ECombatantState, target.combatantState, 13))


def location(player, target, x = None, y = None, z = None):
	"""
	Displays position of the target entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  x: float
	@param x: New X position
	@type  y: float
	@param y: New Z position
	@type  z: float
	@param z: New Z position
	"""
	if z is not None:
		target.position = Atrea.Vector3(x, y, z)
	pos = target.position
	player.feedback("Position of entity %d is: (%f, %f, %f)" % (target.entityId, pos.x, pos.y, pos.z))


def rotation(player, target, x = None, y = None, z = None):
	"""
	Displays position of the target entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  x: float
	@param x: New X rotation
	@type  y: float
	@param y: New Z rotation
	@type  z: float
	@param z: New Z rotation
	"""
	if z is not None:
		target.rotation = Atrea.Vector3(x, y, z)
	rot = target.rotation
	player.feedback("Rotation of entity %d is: (%f, %f, %f)" % (target.entityId, rot.x, rot.y, rot.z))


def facing(player, target):
	"""
	Check facing angle and type to the specified entity
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	posNames = {
		Atrea.enums.CAS_POSITION_ABOVE: 'Above',
		Atrea.enums.CAS_POSITION_BELOW: 'Below',
		Atrea.enums.CAS_POSITION_FRONT: 'Front',
		Atrea.enums.CAS_POSITION_FLANK: 'Flank',
		Atrea.enums.CAS_POSITION_REAR: 'Rear'
	}
	angle = player.facing(target)
	type = player.facingType(target)
	player.feedback('Facing: %f rad / %f deg (%s)' % (angle, angle / pi * 180, posNames[type]))
	player.feedback('Distance: %f' % player.distanceTo(target.position))


def lookAt(player, target):
	"""
	Rotates the targeted entity to look at the player
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	target.lookAt(player.position)


def visible(player, target, visible, entityId = None):
	"""
	Shows/hides the entity targeted by the player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  visible: bool
	@param visible: Is the entity visible?
	@type  entityId: int
	@param entityId: Entity to show/hide
	"""
	if target is None:
		if entityId is not None:
			target = Atrea.findEntity(entityId)
			if target is None:
				player.feedback('Invalid entity ID: %d' % entityId)
				return
		else:
			player.feedback('A target or entity ID must be specified for this command')
			return

	target.setVisible(visible)


def setStaticMesh(player, target, staticMesh):
	"""
	Updates the static mesh name of an entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  staticMesh: str
	@param staticMesh: New static mesh name
	"""
	target.setStaticMeshName(staticMesh)


def setBodySet(player, target, bodySet):
	"""
	Updates the body set of an entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  bodySet: str
	@param bodySet: New body set name
	"""
	target.setBodySet(bodySet)


def setNameId(player, target, nameId):
	"""
	Updates the name of an entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  nameId: int
	@param nameId: New being name ID
	"""
	target.setNameId(nameId)


def setEventSet(player, target, eventSetId):
	"""
	Updates the kismet event set of an entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  eventSetId: int
	@param eventSetId: New event set ID
	"""
	target.setEventSet(eventSetId)


def setInteractionType(player, target, interactionType):
	"""
	Updates the interaction flags of an entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  interactionType: int
	@param interactionType: Interaction flags
	"""
	target.setInteractionType(interactionType)


def interact(player, target):
	"""
	Interacts with an entity, bypassing agression checks and listeners
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	target.onInteract(player)


def initialResponse(player, target, setMapId):
	"""
	Response to the initial interaction dialog
	@param player: SGWPlayer
	@param target: Targeted entity
	@type setMapId: int
	@param setMapId: Intearaction set map ID
	"""
	target.onInitialResponse(player, setMapId)


def setTag(player, target, tag):
	"""
	Updates the name of the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  tag: str
	@param tag: Tag name ('none' resets it)
	"""
	if tag == 'none':
		target.tag = None
	else:
		target.tag = tag


###################################################################################
#                               SGWBeing commands
###################################################################################


def entityShowStats(player, target, stats):
	"""
	Shows selected stats of the targeted entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@param stats: Stat IDs
	"""
	for statId in stats:
		stat = target.getStat(statId)
		player.feedback('%s: %d/%d' % (Atrea.enumNames.EStats[statId], stat.cur, stat.max))


def entityStats(player, target):
	"""
	Shows basic stats
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	statIds = [
		Atrea.enums.health,
		Atrea.enums.focus,
		Atrea.enums.healthRegen,
		Atrea.enums.focusRegen
	]
	entityShowStats(player, target, statIds)


def entityPrimaryStats(player, target):
	"""
	Shows primary stats
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	statIds = [
		Atrea.enums.coordination,
		Atrea.enums.engagement,
		Atrea.enums.fortitude,
		Atrea.enums.morale,
		Atrea.enums.perception,
		Atrea.enums.intelligence
	]
	entityShowStats(player, target, statIds)


def entitySpeedStats(player, target):
	"""
	Shows speed stats
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	statIds = [
		Atrea.enums.movementSpeedMod,
		Atrea.enums.rotationSpeedMod,
		Atrea.enums.speedReload,
		Atrea.enums.speedGrenade,
		Atrea.enums.speedDeploy,
		Atrea.enums.speedAttack
	]
	entityShowStats(player, target, statIds)


def entityArmorStats(player, target):
	"""
	Shows armor and resistance stats
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	statIds = [
		Atrea.enums.physicalAF,
		Atrea.enums.energyAF,
		Atrea.enums.hazmatAF,
		Atrea.enums.psionicAF,
		Atrea.enums.kineticRes,
		Atrea.enums.mentalRes,
		Atrea.enums.healthRes,
		Atrea.enums.interruptRes
	]
	entityShowStats(player, target, statIds)


def entityQRStats(player, target):
	"""
	Shows QR system stats
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	statIds = [
		Atrea.enums.accuracy,
		Atrea.enums.defense,
		Atrea.enums.qrMod,
		Atrea.enums.coverQRModifier,
		Atrea.enums.response,
		Atrea.enums.damage,
		Atrea.enums.penetration,
		Atrea.enums.tracking,
		Atrea.enums.stabilization,
		Atrea.enums.awareness,
		Atrea.enums.coverAccuracy,
		Atrea.enums.coverDefense,
		Atrea.enums.crouchingAccuracy,
		Atrea.enums.crouchingDefense,
		Atrea.enums.negation,
		Atrea.enums.mitigation,
		Atrea.enums.recovery,
		Atrea.enums.restoration,
		Atrea.enums.subtlety
	]
	entityShowStats(player, target, statIds)


def entityAbsorbStats(player, target):
	"""
	Shows absorption stats
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	statIds = [
		Atrea.enums.absorbPhysical,
		Atrea.enums.absorbEnergy,
		Atrea.enums.absorbHazmat,
		Atrea.enums.absorbPsionic,
		Atrea.enums.absorbUntyped,
		Atrea.enums.absorbPhysicalItem,
		Atrea.enums.absorbEnergyItem,
		Atrea.enums.absorbHazmatItem,
		Atrea.enums.absorbPsionicItem,
		Atrea.enums.absorbUntypedItem,
		Atrea.enums.absorbPhysicalEnergy,
		Atrea.enums.absorbEnergyEnergy,
		Atrea.enums.absorbHazmatEnergy,
		Atrea.enums.absorbPsionicEnergy,
		Atrea.enums.absorbUntypedEnergy
	]
	entityShowStats(player, target, statIds)


def entityStealthStats(player, target):
	"""
	Shows stealth stats
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	statIds = [
		Atrea.enums.stealthRating,
		Atrea.enums.stealthMovement,
		Atrea.enums.revealRating,
		Atrea.enums.disguiseRating,
		Atrea.enums.disguiseDetection
	]
	entityShowStats(player, target, statIds)


def setLevel(player, target, level):
	"""
	Updates the level of the targeted entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  level: int
	@param level: Level
	"""
	target.setLevel(level)


def setName(player, target, name):
	"""
	Updates the name of the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  name: str
	@param name: Being name
	"""
	target.setName(name)


def setAlignment(player, target, alignment):
	"""
	Updates the alignment of the targeted entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  alignment: str
	@param alignment: Alignment
	"""
	alignments = {
		'undefined': Atrea.enums.ALIGNMENT_Undefined,
		'praxis': Atrea.enums.ALIGNMENT_Praxis,
		'sgu': Atrea.enums.ALIGNMENT_SGU
	}

	if alignment not in alignments:
		player.feedback('Invalid alignment name "%s"' % alignment)
	else:
		target.setAlignment(alignments[alignment])


def setFaction(player, target, faction):
	"""
	Updates the faction of the targeted entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  faction: str
	@param faction: Faction
	"""
	factions = {
		'Undefined' : 0,
		'World_Object' : 1,
		'SGU' : 2,
		'Praxis' : 3,
		'SGU_MissionGiver' : 4,
		'Praxis_MissionGiver' : 5,
		'Neutral_MissionGiver' : 6,
		'Neutral_Ambient' : 7,
		'Aggro_Ambient' : 8,
		'Friendly_Ambient' : 9,
		'Straegis' : 10,
		'Agnos_Laro' : 11,
		'Ancients' : 12,
		'Bataur' : 13,
		'Burtonol' : 14,
		'Furling' : 15,
		'Ihpet_Unas' : 16,
		'Jaffa_Beleth' : 17,
		'Jaffa_Dakara' : 18,
		'Jaffa_DakorFollower' : 19,
		'Jaffa_NinePeaks' : 20,
		'Jaffa_Ra' : 21,
		'Jaffa_Svarog' : 22,
		'Lucia_Ambient' : 23,
		'Lucia_Blue' : 24,
		'Lucia_DrugAddict' : 25,
		'Lucia_DrugDealer' : 26,
		'Lucia_Green' : 27,
		'Lucia_Red' : 28,
		'Lucia_Yellow' : 29,
		'NID' : 30,
		'Nox' : 31,
		'Replicator' : 32,
		'Sha_Friendly' : 33,
		'Sha_Hostile' : 34,
		'Straegis_Prime' : 35,
		'TechConGroup' : 36,
		'Tollan_Ambient' : 37,
		'Tollan_Narim' : 38,
		'Tollan_Travell' : 39,
		'Vokos_Aesir' : 40,
		'Vokos_Alterr' : 41,
		'Vokos_Nix' : 42,
		'Hostile_To_Players' : 43
	}

	if faction not in factions:
		player.feedback('Invalid faction name "%s"' % faction)
	else:
		target.setFaction(factions[faction])


def setSpeed(player, target, speed):
	"""
	Updates the movement and rotation speed of the targeted entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  speed: int
	@param speed: Speed (100 = default)
	"""
	target.getStat(Atrea.enums.movementSpeedMod).setCurrent(speed)
	target.getStat(Atrea.enums.rotationSpeedMod).setCurrent(speed)
	target.sendDirtyStats()
	player.feedback('Set speed of entity %d to %f' % (target.entityId, speed))


def addComponent(player, target, component):
	"""
	Adds a body component to the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  component: str
	@param component: Component name
	"""
	target.addBodyComponent(component)


def removeComponent(player, target, component):
	"""
	Removes a body component from the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  component: str
	@param component: Component name
	"""
	target.removeBodyComponent(component)


def setStateFlag(player, target, flag):
	"""
	Sets a state flag on the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  flag: str
	@param flag: BSF_ state flag name
	"""
	if hasattr(Atrea.enums, 'BSF_' + flag):
		target.setStateFlag(getattr(Atrea.enums, 'BSF_' + flag))
	else:
		player.feedback('Invalid state flag: ' + flag)


def unsetStateFlag(player, target, flag):
	"""
	Unsets a state flag on the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  flag: str
	@param flag: BSF_ state flag name
	"""
	if hasattr(Atrea.enums, 'BSF_' + flag):
		target.unsetStateFlag(getattr(Atrea.enums, 'BSF_' + flag))
	else:
		player.feedback('Invalid state flag: ' + flag)


def setCombatantFlag(player, target, flag):
	"""
	Sets a combatant state flag on the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  flag: str
	@param flag: PLAYER_STATE_ flag name
	"""
	if hasattr(Atrea.enums, 'PLAYER_STATE_' + flag):
		target.setCombatantStateFlag(getattr(Atrea.enums, 'PLAYER_STATE_' + flag))
	else:
		player.feedback('Invalid combatant state flag: ' + flag)


def unsetCombatantFlag(player, target, flag):
	"""
	Unsets a combatant state flag on the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  flag: str
	@param flag: PLAYER_STATE_ flag name
	"""
	if hasattr(Atrea.enums, 'PLAYER_STATE_' + flag):
		target.unsetCombatantStateFlag(getattr(Atrea.enums, 'PLAYER_STATE_' + flag))
	else:
		player.feedback('Invalid combatant state flag: ' + flag)


def setHealth(player, target, health):
	"""
	Updates the health of the targeted entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  health: int
	@param health: Health
	"""
	target.getStat(Atrea.enums.health).setCurrent(health)
	target.sendDirtyStats()
	player.feedback("Set health of entity %d to %d" % (target.entityId, health))


def setFocus(player, target, focus):
	"""
	Updates the focus of the targeted entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  focus: int
	@param focus: Focus
	"""
	target.getStat(Atrea.enums.focus).setCurrent(focus)
	target.sendDirtyStats()
	player.feedback("Set focus of entity %d to %d" % (target.entityId, focus))


###################################################################################
#                               SGWMob commands
###################################################################################


def setAggression(player, target, level):
	"""
	Updates the aggression level of the targeted mob
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  level: int
	@param level: Aggression level
	"""
	target.setAggression(level)


def generateThreat(player, target, threat):
	"""
	Generates threat on the targeted mob
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  threat: int
	@param threat: Amount of threat generated
	"""
	target.threatGenerated(player.entityId, threat)


def combatInfo(player, target):
	"""
	Checks the target for combat-related AI issues
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if target.template is None:
		player.feedback(' - Entity is not spawned from a template')
	elif target.template.weapon is None:
		player.feedback(' - Entity has no weapon')

	if target.abilitySet is None:
		player.feedback(' - Entity has no ability set')
	else:
		types = {
			Atrea.enums.ABILITY_TYPE_Undefined: 0,
			Atrea.enums.ABILITY_TYPE_Buff: 0,
			Atrea.enums.ABILITY_TYPE_Debuff: 0,
			Atrea.enums.ABILITY_TYPE_Heal: 0,
			Atrea.enums.ABILITY_TYPE_DOT: 0,
			Atrea.enums.ABILITY_TYPE_DD: 0
		}
		for ability in target.abilitySet.abilities.values():
			types[ability.typeId] += 1

		if types[Atrea.enums.ABILITY_TYPE_DD] == 0:
			player.feedback(' - Entity has no damaging abilities')


