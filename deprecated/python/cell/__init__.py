from Atrea import dbQuery
import Atrea.enums
from cell.Item import Item
from cell.Space import Space
from cell.SpaceManager import SpaceManager
from common import Constants
from common.Logger import info
from common.defs.Def import DefMgr
from cell.SGWGmPlayer import pathingDebugMove

def cellStarted():
	""" Called when cell instance startup completed """
	info(Atrea.enums.LC_Uncategorized, "CellApp started")


def cellStarting():
	"""
	Called when the cell instance has finished setting up the Python mappings,
	but hasn't started networking or spaces. This is the place to load shared resources.
	"""
	info(Atrea.enums.LC_Uncategorized, "Loading resources ...")
	DefMgr.load()
	registerInteractions()
	
	
def registerInteractions():
	"""
	Register legacy static interaction handlers
	"""
	for setMapId in Constants.STATIC_INTERACTIONS:
		cb = lambda setMap, instigator, target: target.staticInteractions[setMap.id].onInteract(instigator, setMap.id)
		DefMgr.get('interaction_set_map', setMapId).handlers.append(cb)
	

def spaceCreated(spaceId, worldName, instanced):
	"""
	Called after a space is created on the cell
	@param spaceId: ID of newly created space
	@param worldName: Name of map assigned to this space
	@param instanced: Is the world instanced or is there only a single instance?
	"""
	world = dbQuery("select world_id from resources.worlds where world = '" + str(worldName) + "'")[0]
	space = Space(spaceId, world['world_id'], worldName, instanced)
	SpaceManager.registerSpace(space)
	space.loadSpawns()


def spaceDestroyed(spaceId, worldName, instanced):
	""" Called before a space is destroyed on the cell """
	space = SpaceManager.get(spaceId)
	SpaceManager.unregisterSpace(space)


def pathingDebug(spaceId, entityId, path):
	pathingDebugMove(entityId, path)
