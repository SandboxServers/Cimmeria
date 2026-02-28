import io
import traceback
import sys
import Atrea
import Atrea.enums
import common.Constants
from common.Config import Config

logLevels = {
	Atrea.enums.LC_Uncategorized: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Mission: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Item: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Location: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Ability: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Interact: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Cash: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Mob: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Chat: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_LogInOut: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_CharManagement: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Resources: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Minigame: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Stargate: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Mail: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Crafting: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Event: Config.DEFAULT_LOG_LEVEL,
	Atrea.enums.LC_Visuals: Config.DEFAULT_LOG_LEVEL
}

logLevelNames = {
	Atrea.enums.LC_Uncategorized: '        ',
	Atrea.enums.LC_Mission: 'MISSION ',
	Atrea.enums.LC_Item: 'ITEM    ',
	Atrea.enums.LC_Location: 'LOCATION',
	Atrea.enums.LC_Ability: 'ABILITY ',
	Atrea.enums.LC_Interact: 'INTERACT',
	Atrea.enums.LC_Cash: 'CASH    ',
	Atrea.enums.LC_Mob: 'MOB     ',
	Atrea.enums.LC_Chat: 'CHAT    ',
	Atrea.enums.LC_LogInOut: 'LOGIN   ',
	Atrea.enums.LC_CharManagement: 'CHARMGMT',
	Atrea.enums.LC_Resources: 'RESOURCE',
	Atrea.enums.LC_Minigame: 'MINIGAME',
	Atrea.enums.LC_Stargate: 'STARGATE',
	Atrea.enums.LC_Mail: 'GATEMAIL',
	Atrea.enums.LC_Crafting: 'CRAFTING',
	Atrea.enums.LC_Event: 'EVENT   ',
	Atrea.enums.LC_Visuals: 'VISUALS '
}

# Secondary logging function
LoggerCallback = None

def log(level, event, msg):
	if logLevels[event] <= level:
		Atrea.log(level, logLevelNames[event], str(msg))
		if LoggerCallback is not None:
			LoggerCallback(level, event, str(msg))


def trace(event, msg):
	log(common.Constants.LOG_TRACE, event, msg)

def debug2(event, msg):
	log(common.Constants.LOG_DEBUG2, event, msg)

def debug1(event, msg):
	log(common.Constants.LOG_DEBUG1, event, msg)

def info(event, msg):
	log(common.Constants.LOG_INFO, event, msg)

def warn(event, msg):
	log(common.Constants.LOG_WARN, event, msg)

def error(event, msg):
	log(common.Constants.LOG_ERROR, event, msg)

def critical(event, msg):
	log(common.Constants.LOG_CRITICAL, event, msg)

def exc(event, exc_info = None):
	if exc_info is None:
		exc_info = sys.exc_info()
	sio = io.StringIO()
	traceback.print_exception(exc_info[0], exc_info[1], exc_info[2], None, sio)
	s = sio.getvalue()
	sio.close()
	if s[-1:] == "\n":
		s = s[:-1]
	error(event, s)

