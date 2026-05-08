import Atrea.enums
from cell.Global import PlayersByName
from common import Constants
from common.defs.Def import DefMgr


def reloadMap(player, target):
	"""
	Reloads the current map on the target
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	target.client.onClientMapLoad(target.space.worldName, target.space.world.clientMap, target.space.worldId,
								  target.position, Atrea.Vector3(0, 0, 0))


def save(player, target):
	"""
	Persists the player entity
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.persist()
	player.feedback('Player data persisted.')


def kill(player, target):
	"""
	Kills the entity targeted by the player
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	target.getStat(Atrea.enums.health).setCurrent(0)
	target.sendDirtyStats()


def revive(player, target):
	"""
	Revives the entity targeted by the player
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	health = target.getStat(Atrea.enums.health)
	health.setCurrent(health.max)
	target.sendDirtyStats()
	target.unsetCombatantStateFlag(Atrea.enums.PLAYER_STATE_Dead)


def clearAbilities(player, target):
	"""
	Removes all abilities from the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	target.removeAbility(target.abilities.abilities)
	player.feedback("Reset abilities for player %s" % target.getName())


def giveStargateAddress(player, target, stargateId, hidden = False):
	"""
	Adds a stargate address to the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  stargateId: int
	@param stargateId: Stargate ID
	@type  hidden: bool
	@param hidden: Hide address from address book
	"""
	stargate = DefMgr.get('stargate', stargateId)
	if stargate is None:
		player.onError("Invalid stargate ID: %d" % stargateId)
		return

	target.addStargateAddress(stargateId, hidden)
	player.feedback("Added stargate address %s for player %s" % (stargate.name, target.getName()))


def removeStargateAddress(player, target, stargateId):
	"""
	Adds a stargate address to the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  stargateId: int
	@param stargateId: Stargate ID
	"""
	stargate = DefMgr.get('stargate', stargateId)
	if stargate is None:
		player.onError("Invalid stargate ID: %d" % stargateId)
		return

	target.removeStargateAddress(stargateId)
	player.feedback("Added stargate address %s for player %s" % (stargate.name, target.getName()))


def giveRespawner(player, target, respawnerId):
	"""
	Adds a respawner to the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  respawnerId: int
	@param respawnerId: Respawner ID
	"""
	respawner = DefMgr.get('respawner', respawnerId)
	if respawner is None:
		player.onError("Invalid respawner ID: %d" % respawnerId)
		return

	target.addRespawner(respawnerId)
	player.feedback("Added respawner %s for player %s" % (respawner.name, target.getName()))


def removeRespawner(player, target, respawnerId):
	"""
	Adds a respawner to the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  respawnerId: int
	@param respawnerId: Respawner ID
	"""
	respawner = DefMgr.get('respawner', respawnerId)
	if respawner is None:
		player.onError("Invalid respawner ID: %d" % respawnerId)
		return

	target.removeRespawner(respawnerId)
	player.feedback("Added respawner address %s for player %s" % (respawner.name, target.getName()))


def giveAbility(player, target, abilityId):
	"""
	Gives an ability to the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  abilityId: int
	@param abilityId: ability ID
	"""
	ability = DefMgr.get('ability', abilityId)
	if ability is None:
		player.onError("Invalid ability ID: %d" % abilityId)
		return

	target.addAbility(abilityId)
	player.feedback("Added ability <%d: %s> for entity %s" % (abilityId, ability.name, target.getName()))


def giveCash(player, target, amount):
	"""
	Gives naquadah to the player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  amount: int
	@param amount: Amount of cash to give
	"""
	target.inventory.addCash(amount)
	target.inventory.flushUpdates()
	player.feedback('Gave %d naquadah to <%s>' % (amount, target.getName()))


def giveItem(player, target, designId, quantity):
	"""
	Gives the specified item to the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  designId: int
	@param designId: Item design ID
	@type  quantity: int
	@param quantity: Number of cash to give
	"""
	design = DefMgr.get('item', designId)
	if design is None:
		player.feedback("Invalid item design ID: %d" % designId)
		return

	if target.inventory.pickedUpItem(designId, quantity):
		target.inventory.flushUpdates()
		player.feedback("Adding %d %s to player %s" % (quantity, design.name, target.getName()))
	else:
		player.onError("Failed to add %d %s to player %s" % (quantity, design.name, target.getName()))


def removeItem(player, target, designId, quantity):
	"""
	Removes the specified item from the targeted player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  designId: int
	@param designId: Item design ID
	@type  quantity: int
	@param quantity: Number of cash to give
	"""
	if target.inventory.removeItem(designId, quantity, True):
		target.inventory.flushUpdates()
		player.feedback("Removed %d from item stack %d for player %s" % (quantity, designId, target.getName()))
	else:
		player.onError("Failed to remove item %d from player %s" % (designId, target.getName()))


def giveTrainingPoints(player, target, amount):
	"""
	Gives training points to a player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  amount: int
	@param amount: Amount of training points to give
	"""
	target.giveTrainingPoints(amount)
	player.feedback('Gave %d training points to <%s>' % (amount, target.getName()))


def giveExperience(player, target, amount):
	"""
	Gives experience points to a player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  amount: int
	@param amount: Amount of exp to give
	"""
	target.giveExperience(amount)
	player.feedback('Gave %d exp to <%s>' % (amount, target.getName()))


def listAbilities(player, target):
	"""
	Lists the abilities the target player has
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.feedback("Ability list of entity %d:" % target.entityId)
	for abilityId in target.abilities.abilities:
		ability = DefMgr.get('ability', abilityId)
		if ability is not None:
			player.feedback("    %s" % ability.name)
		else:
			player.feedback("    <unknown ability %d>" % abilityId)


def dynamicUpdate(player, target):
	"""
	Requests the CellApp to update dynamic properties on the target entity
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.requestDynamicUpdate(target)


def addDialog(player, target, templateId, setMapId):
	"""
	Adds a new dialog choice for the target entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  templateId: int
	@param templateId: Entity template ID
	@type  setMapId: int
	@param setMapId: Dialog set map ID
	"""
	setMap = DefMgr.get('interaction_set_map', setMapId)
	if setMap is None:
		player.feedback('Invalid dialog set map ID: %d' % setMapId)
		return

	target.addDialog(templateId, setMap, 0)


def removeDialog(player, target, templateId, setMapId):
	"""
	Removes a dialog choice for the target entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  templateId: int
	@param templateId: Entity template ID
	@type  setMapId: int
	@param setMapId: Dialog set map ID
	"""
	setMap = DefMgr.get('interaction_set_map', setMapId)
	if setMap is None:
		player.feedback('Invalid dialog set map ID: %d' % setMapId)
		return

	target.removeDialog(templateId, setMap)


def toggleGodMode(player, target, enabled = True):
	"""
	Enable/disable god mode (invulnerability) on an entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  enabled: bool
	@param enabled: Enable/disable god mode
	"""
	if enabled:
		target.cheatFlags |= Constants.CHEAT_Invulnerable
		player.feedback("God mode ON for entity <%s>" % target.getName())
	else:
		target.cheatFlags &= ~Constants.CHEAT_Invulnerable
		player.feedback("God mode OFF for entity <%s>" % target.getName())


def goto(player, target, name):
	"""
	Moves the player to the location of another player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  name: str
	@param name: Name of target player
	"""
	entity = target or player
	if name not in PlayersByName:
		player.onError("Player is not available on this CellApp")
		return

	destination = PlayersByName[name]()
	if destination is None or destination.space is None:
		player.onError("Player is not on any reachable space")
		return

	pos = destination.position
	entity.teleportTo(Atrea.Vector3(pos.x, pos.y, pos.z), 0.0, destination.space.worldName)
	player.feedback("Teleporting to player <%s>" % destination.getName())


def summon(player, target, name):
	"""
	Summons another player.
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  name: str
	@param name: Name or ID of target player
	"""
	entity = target or player
	if name not in PlayersByName:
		player.onError("Player is not available on this CellApp")
		return

	victim = PlayersByName[name]()
	if victim is None or victim.space is None:
		player.onError("Player is not on any reachable space")
		return

	pos = entity.position
	victim.teleportTo(Atrea.Vector3(pos.x, pos.y, pos.z), 0.0, entity.space.worldName)
	player.feedback("Summoning player <%s>" % victim.getName())


def gotoLocation(player, target, worldName, x, y, z):
	"""
	Moves an entity to the specified location in the world
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  worldName: str
	@param worldName: Destination space name
	@type  x: float
	@param x: X position on destination space
	@type  y: float
	@param y: Y position on destination space
	@type  z: float
	@param z: Z position on destination space
	"""
	entity = target or player
	world = [w for w in DefMgr.getAll('world_info').values() if w.world == worldName]
	if len(world) == 0:
		player.onError("Unable to find world: %s" % worldName)
		return

	entity.teleportTo(Atrea.Vector3(x, y, z), 0.0, worldName)
	player.feedback("Moving entity %s to %s (%f, %f, %f)" % (entity.getName(), worldName, x, y, z))


def gotoXYZ(player, target, x, y, z):
	"""
	Moves an entity to the specified location on the current space
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  x: float
	@param x: X position on the current space
	@type  y: float
	@param y: Y position on the current space
	@type  z: float
	@param z: Z position on the current space
	"""
	entity = target or player
	entity.teleportTo(Atrea.Vector3(x, y, z), 0.0)
	player.feedback("Moving entity %s to (%f, %f, %f)" % (entity.getName(), x, y, z))

