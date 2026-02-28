###################################################################################
#             Miscellaneous commands used for debugging / development
###################################################################################

import time
import Atrea
import Atrea.enums
from cell.Global import PlayersByName, PathingDebugSubscriptions
from cell.actions.Follow import Follow
from common import debug


def debugVelocity(player, target, velocityX, velocityY, velocityZ):
	"""
	---
	@param player: SGWPlayer
	@param target: Targeted entity
	@type velocityX: float
	@param velocityX: New X velocity of
	@type velocityY: float
	@param velocityY: New Y velocity of entity
	@type velocityZ: float
	@param velocityZ: New Z velocity of entity
	"""
	target.velocity = Atrea.Vector3(velocityX, velocityY, velocityZ)


def debugController(player, target):
	"""
	Enables the debug movement controller on the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	target.enableDebugController()
	player.feedback("Debug controller enabled for entity %d" % target.entityId)


def debugFollow(player, target):
	"""
	Enables the follow movement controller on the entity
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if target.action is None:
		player.feedback("Setting Follow action for %s" % target.getName())
		target.setAction(Follow(target, player))
	else:
		player.feedback("Canceling action for %s" % target.getName())
		target.cancelAction()


def debugPaths(player, target):
	"""
	Enable/disable client-side display of navigation paths
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if not player.pathingDebugEnabled:
		PathingDebugSubscriptions[player.entityId] = player
		player.pathingDebugEnabled = True
		player.feedback("Mob pathing debug enabled")
	else:
		del PathingDebugSubscriptions[player.entityId]
		player.pathingDebugEnabled = False
		player.feedback("Mob pathing debug disabled")


def debugNavigation(player, target):
	"""
	Tests navigation and reachability between two points
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	loc = player.position
	if player.navmeshDebugStart is None:
		player.feedback('Navmesh start marked at (%f, %f, %f)' % (loc.x, loc.y, loc.z))
		player.navmeshDebugStart = loc
	elif player.navmeshDebugEnd is None:
		player.feedback('Navmesh end marked at (%f, %f, %f)' % (loc.x, loc.y, loc.z))
		player.navmeshDebugEnd = loc
		start = time.clock()
		path = Atrea.findPath(player.spaceId, player.navmeshDebugStart, player.navmeshDebugEnd)
		if path is not None:
			end = time.clock()
			player.feedback('Navigation took %f msecs' % ((end - start) * 1000))
			player.client.onShowPath(player.entityId, Atrea.enums.MOB_MOVEMENT_Wander, path)
		else:
			player.onError("Unable to generate a path :(")

		player.navmeshDebugStart = player.navmeshDebugEnd
		player.navmeshDebugEnd = None


def debugAi(player, target):
	"""
	Enable AI debug messages
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if target.enableAiDebug:
		player.feedback('Disabling AI debug for target')
		target.enableAiDebug = False
		target.abilities.setDebuggingPlayer(None)
	else:
		player.feedback('Enabling AI debug for target')
		target.enableAiDebug = True
		target.abilities.setDebuggingPlayer(player)


def debugEvents(player, target):
	"""
	Shows event subscription data for the player
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.feedback("Subscribed events by name:")
	for event, subs in player.subscriptionsByEvent.items():
		if len(subs) > 0:
			player.feedback("   %s: %d subs" % (event, len(subs)))


def debugInventory(player, target):
	"""
	Shows inventory debug data for the player
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if len(target.inventory.itemsPendingDeletion):
		player.feedback("Items pending deletion:")
		for item in target.inventory.itemsPendingDeletion:
			player.feedback("    id=%d, dbid=%d dbstatus=%d bag=%d" % (item.id, item.databaseId, item.dbStatus, item.bagId))

	if len(target.inventory.removedItems):
		player.feedback("Removed items: %s" % str(target.inventory.removedItems))

	seen = {}
	for bagId, bag in target.inventory.bags.items():
		player.feedback("Items in bag %d:" % bagId)
		for item in bag.items:
			if item is not None:
				seen[item.id] = item
				player.feedback("    slot=%d, id=%d, dbid=%d, dbstatus=%d" % (item.slotId, item.id, item.databaseId, item.dbStatus))

	for itemId, item in target.inventory.items.items():
		if itemId not in seen:
			# We've found an item that is in the players inventory, but isn't in any of the bags!
			player.feedback("FLOATING ITEM: bag=%s, slot=%s, id=%d, dbid=%d, dbstatus=%d" %
							(str(item.bagId), str(item.slotId), item.id, item.databaseId, item.dbStatus))


def reloadInventory(player, target):
	"""
	Reloads inventory data from the database
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	target.inventory.loadItems()
	target.inventory.flushUpdates()
	player.feedback("Reloaded inventory of entity %d" % player.entityId)


def reloadScripts(player, target):
	"""
	Reloads all Python scripts on the CellApp
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.feedback("Reloading scripts on CellApp ...")
	start = time.clock()
	debug.reloadModules()
	player.feedback("Reloaded scripts in %f sec" % (time.clock() - start))


def players(player, target):
	"""
	Displays the list of online players
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.feedback("Players online on this CellApp:")
	for player in PlayersByName.values():
		world = player().space.worldName if player().space else "In transition"
		player.feedback(" - %s (%s)" % (player().getName(), world))
