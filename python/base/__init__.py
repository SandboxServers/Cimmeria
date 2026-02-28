import Atrea
import Atrea.enums
from base.SGWEntity import SGWEntity
from base.SGWSpawnableEntity import SGWSpawnableEntity
from common.Logger import info, warn
from common.defs.Def import DefMgr

def baseStarted():
	info(Atrea.enums.LC_Uncategorized, "Loading resources ...")
	DefMgr.load(True)

def createResource(categoryId, elementId):
	if categoryId not in DefMgr.ResourceCategories:
		warn(Atrea.enums.LC_Resources, "Requested invalid resource category %d" % categoryId)
		return None
	
	return DefMgr.makeResource(DefMgr.ResourceCategories[categoryId], elementId)
