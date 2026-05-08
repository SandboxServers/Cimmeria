import re
import Atrea.enums
import inspect
import cell.commands.Crafting
import cell.commands.Entity
import cell.commands.Misc
import cell.commands.Mission
import cell.commands.Net
import cell.commands.Player
import cell.commands.Resource
from common import Constants, Logger
from common.Logger import warn, logLevels, logLevelNames


class Command(object):
	# Registered console commands (name => Command object)
	commands = {}

	def __init__(self, name, function, accessLevel, targetType):
		"""
		@param name: Console command name
		@param function: Ref to python handler function
		@param accessLevel: Required access level
		@param targetType: Required target type
		@return:
		"""
		assert(hasattr(function, '__call__'))
		self.name = name
		self.function = function
		self.accessLevel = accessLevel
		self.targetType = targetType
		self.args = []
		args, varargs, keywords, defaults = inspect.getargspec(function)
		doc = inspect.getdoc(function)
		self.doc = re.search('^([^@]*)', doc).group(1).strip()
		for i in range(2, len(args)):
			argdoc = re.search('@param\\s+' + args[i] + ':\\s*(.*)$', doc, re.MULTILINE)
			argtype = re.search('@type\\s+' + args[i] + ':\\s*(.*)$', doc, re.MULTILINE)
			if argdoc is None:
				warn(Atrea.enums.LC_Chat, 'Missing doc for argument "%s" in command "%s"' % (args[i], name))
			if argtype is None:
				warn(Atrea.enums.LC_Chat, 'Missing type for argument "%s" in command "%s"' % (args[i], name))
			self.args.append((args[i], argdoc.group(1) if argdoc else None, argtype.group(1) if argtype else 'str'))
		self.argsMin = len(self.args) - (len(defaults) if defaults else 0)


	def call(self, player, args):
		"""
		Validates and executes a command
		@param player: SGWPlayer executing the command
		@param args: Arguments passed to the command
		"""
		if player.accessLevel < self.accessLevel:
			player.feedback('%s: An access level of %d or higher is required' % (self.name, self.accessLevel))
			return

		if len(args) < self.argsMin:
			player.feedback('%s: Not enough arguments (got %d, %d expected)' % (self.name, len(args), self.argsMin))
			return

		if len(args) > len(self.args):
			player.feedback('%s: Too many arguments (got %d, %d expected)' % (self.name, len(args), len(self.args)))
			return

		if self.targetType is not None:
			if player.targetId == 0:
				player.feedback('%s: A target is required for this command' % self.name)
				return

			target = player.space.findEntity(player.targetId)
			if target is None:
				player.feedback('%s: Targeted entity %d is unknown' % (self.name, player.targetId))
				return

			bases = inspect.getmro(target.__class__)
			if len([base.__name__  for base in bases if base.__name__ == self.targetType]) == 0:
				player.feedback('%s: An entity of type %s expected; got %s' % (self.name, self.targetType, bases[0].__name__))
				return
		else:
			target = player.space.findEntity(player.targetId) if player.targetId != 0 else None

		validated = {}
		for i in range(0, len(args)):
			arg, doc, type = self.args[i]
			if type == 'str':
				validated[arg] = args[i]
			elif type == 'int':
				try:
					validated[arg] = int(args[i])
				except ValueError:
					player.feedback('%s: %s must be an integer: %s' % (self.name, arg, args[i]))
					return
			elif type == 'float':
				try:
					validated[arg] = float(args[i])
				except ValueError:
					player.feedback('%s: %s must be a float: %s' % (self.name, arg, args[i]))
					return
			elif type == 'bool':
				if args[i] == '0' or args[i].lower() == 'false':
					validated[arg] = False
				elif args[i] == '1' or args[i].lower() == 'true':
					validated[arg] = True
				else:
					player.feedback('%s: %s must be a bool: %s' % (self.name, arg, args[i]))
					return
			else:
				raise RuntimeError("%s: Invalid argument type: %s" % (self.name, type))

		self.function(player, target, **validated)


	@staticmethod
	def exec(player, request : str):
		"""
		Executes a console command
		@param player: SGWPlayer object
		@param request: Command text
		"""
		parts = request.strip().split(' ')
		cmd = parts[0]
		if cmd not in Command.commands:
			player.feedback('Unknown command: %s' % cmd)
			return

		Command.commands[cmd].call(player, parts[1:])
		
		
	@staticmethod
	def add(commands):
		"""
		Adds the specified commands to the console command list
		"""
		for command in commands:
			assert isinstance(command, Command)
			if command.name in Command.commands:
				raise Exception('Console command "%s" already registered!' % command.name)
			Command.commands[command.name] = command


def help(player, target, command = None):
	"""
	Shows help about console commands
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  command: str
	@param command: Get help about this command
	"""
	if command is None:
		commands = [cmd for cmd in Command.commands.values() if cmd.accessLevel <= player.accessLevel]
	else:
		commands = [cmd for cmd in Command.commands.values()
					if cmd.name.find(command) != -1 and cmd.accessLevel <= player.accessLevel]

	commands.sort(key=lambda c : c.name)
	for cmd in commands:
		player.feedback('%s: %s' % (cmd.name, cmd.doc))
		if len(commands) <= 3:
			for index in range(0, len(cmd.args)):
				arg = cmd.args[index]
				if index <= cmd.argsMin:
					player.feedback('    %s (%s): %s' % (arg[0], arg[2], arg[1]))
				else:
					player.feedback('    [%s] (%s): %s' % (arg[0], arg[2], arg[1]))

	if len(commands) == 0:
		player.feedback('No command found with that name')


def setLogLevel(player, target, level, category = None):
	"""
	Updates log level for one or more event category
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  level: str
	@param level: Log level; trace, debug1-2, info, warn, error
	@type  category: str
	@param category: Mission, item, event, etc.
	"""
	levelNames = {
		'trace': Constants.LOG_TRACE,
		'debug2': Constants.LOG_DEBUG2,
		'debug1': Constants.LOG_DEBUG1,
		'info': Constants.LOG_INFO,
		'warn': Constants.LOG_WARN,
		'error': Constants.LOG_ERROR
	}

	if level not in levelNames:
		player.feedback('Invalid log level name: %s' % level)
		return

	if category is None:
		for categoryId in logLevels:
			logLevels[categoryId] = levelNames[level]
		player.feedback('Set log level of all categories to %s' % level)
	else:
		for categoryId, name in logLevelNames.items():
			if category == name.rstrip().lower():
				logLevels[categoryId] = levelNames[level]
				player.feedback('Set log level of category %s to %s' % (category, level))
				return

		player.feedback('Invalid log category name: %s' % category)


def setClientLogging(player, target):
	"""
	Enable/disable sending of server log messages to the client
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if Logger.LoggerCallback is None:
		Logger.LoggerCallback = lambda level,evt,msg: player.onDebug(msg)
		player.feedback('Enabled client logging')
	else:
		Logger.LoggerCallback = None
		player.feedback('Disabled client logging')






###################################################################################
#                           Console commands table
###################################################################################


Command.add([
	# List of commands that can be accessed through the chat channel / server console
	# Format: Command(Name, Function, RequiredAccessLevel, RequiredTargetType)
	Command('help', help, 0, None),
	Command('loglevel', setLogLevel, 1, None),
	Command('logclient', setClientLogging, 1, None),

	Command('kill', cell.commands.Player.kill, 1, 'SGWBeing'),
	Command('revive', cell.commands.Player.revive, 1, 'SGWBeing'),
	Command('clearabilities', cell.commands.Player.clearAbilities, 1, 'SGWPlayer'),
	Command('giveaddress', cell.commands.Player.giveStargateAddress, 1, 'SGWPlayer'),
	Command('giveability', cell.commands.Player.giveAbility, 1, 'SGWPlayer'),
	Command('givecash', cell.commands.Player.giveCash, 1, 'SGWPlayer'),
	Command('giveitem', cell.commands.Player.giveItem, 1, 'SGWPlayer'),
	Command('giverespawner', cell.commands.Player.giveRespawner, 1, 'SGWPlayer'),
	Command('givetp', cell.commands.Player.giveTrainingPoints, 1, 'SGWPlayer'),
	Command('givexp', cell.commands.Player.giveExperience, 1, 'SGWPlayer'),
	Command('god', cell.commands.Player.toggleGodMode, 1, 'SGWPlayer'),
	Command('listabilities', cell.commands.Player.listAbilities, 1, 'SGWPlayer'),
	Command('dynamicupdate', cell.commands.Player.dynamicUpdate, 1, 'SGWSpawnableEntity'),
	Command('adddialog', cell.commands.Player.addDialog, 1, 'SGWPlayer'),
	Command('removeaddress', cell.commands.Player.removeStargateAddress, 1, 'SGWPlayer'),
	Command('removedialog', cell.commands.Player.removeDialog, 1, 'SGWPlayer'),
	Command('removeitem', cell.commands.Player.removeItem, 1, 'SGWPlayer'),
	Command('removerespawner', cell.commands.Player.removeRespawner, 1, 'SGWPlayer'),
	Command('reloadmap', cell.commands.Player.reloadMap, 1, 'SGWPlayer'),
	Command('save', cell.commands.Player.save, 1, None),
	Command('goto', cell.commands.Player.goto, 1, None),
	Command('summon', cell.commands.Player.summon, 1, None),
	Command('gotolocation', cell.commands.Player.gotoLocation, 1, None),
	Command('gotoxyz', cell.commands.Player.gotoXYZ, 1, None),

	Command('info', cell.commands.Entity.entityInfo, 1, None),
	Command('location', cell.commands.Entity.location, 1, 'SGWSpawnableEntity'),
	Command('rotation', cell.commands.Entity.rotation, 1, 'SGWSpawnableEntity'),
	Command('facing', cell.commands.Entity.facing, 1, 'SGWSpawnableEntity'),
	Command('lookat', cell.commands.Entity.lookAt, 1, 'SGWSpawnableEntity'),
	Command('visible', cell.commands.Entity.visible, 1, None),
	Command('staticmesh', cell.commands.Entity.setStaticMesh, 1, 'SGWSpawnableEntity'),
	Command('bodyset', cell.commands.Entity.setBodySet, 1, 'SGWSpawnableEntity'),
	Command('nameid', cell.commands.Entity.setNameId, 1, 'SGWSpawnableEntity'),
	Command('eventset', cell.commands.Entity.setEventSet, 1, 'SGWSpawnableEntity'),
	Command('interactiontype', cell.commands.Entity.setInteractionType, 1, 'SGWSpawnableEntity'),
	Command('interact', cell.commands.Entity.interact, 1, 'SGWSpawnableEntity'),
	Command('initialresponse', cell.commands.Entity.initialResponse, 1, 'SGWSpawnableEntity'),
	Command('tag', cell.commands.Entity.setTag, 1, 'SGWSpawnableEntity'),

	Command('level', cell.commands.Entity.setLevel, 1, 'SGWBeing'),
	Command('name', cell.commands.Entity.setName, 1, 'SGWBeing'),
	Command('alignment', cell.commands.Entity.setAlignment, 1, 'SGWBeing'),
	Command('faction', cell.commands.Entity.setFaction, 1, 'SGWBeing'),
	Command('speed', cell.commands.Entity.setSpeed, 1, 'SGWBeing'),
	Command('addcomponent', cell.commands.Entity.addComponent, 1, 'SGWBeing'),
	Command('delcomponent', cell.commands.Entity.removeComponent, 1, 'SGWBeing'),
	Command('setstate', cell.commands.Entity.setStateFlag, 1, 'SGWBeing'),
	Command('unsetstate', cell.commands.Entity.unsetStateFlag, 1, 'SGWBeing'),
	Command('setcombatant', cell.commands.Entity.setCombatantFlag, 1, 'SGWBeing'),
	Command('unsetcombatant', cell.commands.Entity.unsetCombatantFlag, 1, 'SGWBeing'),
	Command('health', cell.commands.Entity.setHealth, 1, 'SGWBeing'),
	Command('focus', cell.commands.Entity.setFocus, 1, 'SGWBeing'),
	Command('stats', cell.commands.Entity.entityStats, 1, 'SGWBeing'),
	Command('primarystats', cell.commands.Entity.entityPrimaryStats, 1, 'SGWBeing'),
	Command('speedstats', cell.commands.Entity.entitySpeedStats, 1, 'SGWBeing'),
	Command('armorstats', cell.commands.Entity.entityArmorStats, 1, 'SGWBeing'),
	Command('qrstats', cell.commands.Entity.entityQRStats, 1, 'SGWBeing'),
	Command('absorbstats', cell.commands.Entity.entityAbsorbStats, 1, 'SGWBeing'),
	Command('stealthstats', cell.commands.Entity.entityStealthStats, 1, 'SGWBeing'),

	Command('aggression', cell.commands.Entity.setAggression, 1, 'SGWMob'),
	Command('threaten', cell.commands.Entity.generateThreat, 1, 'SGWMob'),
	Command('combatinfo', cell.commands.Entity.combatInfo, 1, 'SGWMob'),

	Command('missionaccept', cell.commands.Mission.acceptMission, 1, 'SGWPlayer'),
	Command('missionabandon', cell.commands.Mission.abandonMission, 0, 'SGWPlayer'),
	Command('missionadvance', cell.commands.Mission.advanceMission, 1, 'SGWPlayer'),
	Command('missionclear', cell.commands.Mission.clearMission, 1, 'SGWPlayer'),
	Command('missionclearactive', cell.commands.Mission.clearActiveMissions, 1, 'SGWPlayer'),
	Command('missionclearhistory', cell.commands.Mission.clearMissionHistory, 1, 'SGWPlayer'),
	Command('missioncomplete', cell.commands.Mission.completeMission, 1, 'SGWPlayer'),
	Command('missionfail', cell.commands.Mission.failMission, 1, 'SGWPlayer'),
	Command('missionlist', cell.commands.Mission.displayMissionList, 1, 'SGWPlayer'),
	Command('missionlistfull', cell.commands.Mission.displayFullMissionList, 1, 'SGWPlayer'),
	Command('missiondetails', cell.commands.Mission.displayMissionDetails, 1, 'SGWPlayer'),
	Command('missionreload', cell.commands.Mission.reloadMission, 1, 'SGWPlayer'),
	Command('missionreset', cell.commands.Mission.resetMission, 1, 'SGWPlayer'),
	Command('missionrewards', cell.commands.Mission.displayMissionRewards, 1, 'SGWPlayer'),

	Command('appliedscience', cell.commands.Crafting.giveAppliedSciencePoints, 1, 'SGWPlayer'),
	Command('racialparadigm', cell.commands.Crafting.setRacialParadigmLevel, 1, 'SGWPlayer'),
	Command('learndiscipline', cell.commands.Crafting.learnDiscipline, 1, 'SGWPlayer'),
	Command('forgetdiscipline', cell.commands.Crafting.forgetDiscipline, 1, 'SGWPlayer'),
	Command('allcraft', cell.commands.Crafting.debugAllCraft, 1, 'SGWPlayer'),

	Command('searchitem', cell.commands.Resource.searchItem, 1, None),
	Command('searchmission', cell.commands.Resource.searchMission, 1, None),
	Command('searchtemplate', cell.commands.Resource.searchTemplate, 1, None),
	Command('reloadres', cell.commands.Resource.reloadResources, 1, None),
	Command('respawnall', cell.commands.Resource.respawnAll, 1, None),
	Command('autosavespawn', cell.commands.Resource.autosaveSpawn, 1, None),
	Command('spawn', cell.commands.Resource.spawnEntity, 1, None),
	Command('spawnrandom', cell.commands.Resource.spawnRandomEntity, 1, None),
	Command('despawn', cell.commands.Resource.despawnEntity, 1, 'SGWSpawnableEntity'),
	Command('savespawn', cell.commands.Resource.saveSpawn, 1, 'SGWSpawnableEntity'),
	Command('delspawn', cell.commands.Resource.deleteSpawn, 1, 'SGWSpawnableEntity'),

	Command('net_dhd', cell.commands.Net.displayDHD, 1, None),
	Command('net_seq', cell.commands.Net.sequence, 1, 'SGWSpawnableEntity'),
	Command('net_seqto', cell.commands.Net.sequenceTo, 1, None),
	Command('net_seqfrom', cell.commands.Net.sequenceFrom, 1, 'SGWSpawnableEntity'),
	Command('net_timer', cell.commands.Net.updateTimer, 1, 'SGWSpawnableEntity'),
	Command('net_timeofday', cell.commands.Net.timeOfDay, 1, 'SGWPlayer'),
	Command('net_mapinfo', cell.commands.Net.mapInfo, 1, 'SGWPlayer'),
	Command('net_speak', cell.commands.Net.playerCommunication, 1, 'SGWSpawnableEntity'),
	Command('net_minigame', cell.commands.Net.startMinigame, 1, 'SGWPlayer'),
	Command('net_dialog', cell.commands.Net.openDialog, 1, None),
	Command('net_challenge', cell.commands.Net.clientChallenge, 1, None),

	Command('debug_velocity', cell.commands.Misc.debugVelocity, 1, 'SGWSpawnableEntity'),
	Command('debug_controller', cell.commands.Misc.debugController, 1, 'SGWSpawnableEntity'),
	Command('debug_follow', cell.commands.Misc.debugFollow, 1, 'SGWSpawnableEntity'),
	Command('debug_paths', cell.commands.Misc.debugPaths, 1, 'SGWSpawnableEntity'),
	Command('debug_nav', cell.commands.Misc.debugNavigation, 1, 'SGWSpawnableEntity'),
	Command('debug_events', cell.commands.Misc.debugEvents, 1, 'SGWSpawnableEntity'),
	Command('debug_ai', cell.commands.Misc.debugAi, 1, 'SGWMob'),
	Command('debug_inven', cell.commands.Misc.debugInventory, 1, 'SGWPlayer'),
	Command('debug_invreload', cell.commands.Misc.reloadInventory, 1, 'SGWPlayer'),
	Command('reloadscripts', cell.commands.Misc.reloadScripts, 1, None),
	Command('players', cell.commands.Misc.players, 1, None)
])

