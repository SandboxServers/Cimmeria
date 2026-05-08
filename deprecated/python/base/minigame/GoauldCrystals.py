from math import floor
from random import randrange
import Atrea
import Atrea.enums
from common.Logger import warn


class GoauldCrystals(Atrea.Minigame):
	# Was the game initialized?
	initialized = False
	# Is the player currently playing?
	started = False
	consumablesUsed = False
	consumableCheatBonus = False
	playfieldActive = False
	moveMode = 0

	# Game time remaining (seconds)
	timeRemaining = 0
	countdownRate = 0
	paramTimer = 0
	playfieldTotal = 0
	goalTotal = 0
	movingTotal = 0
	moveTimer = 0
	obstacleTotal = 0
	obstacleTimer = 0
	timeUnits = 0
	# Name displayed in minigame UI
	readOut = ''
	# Cheat abilities the player can use
	abilityBitfield = 0
	instcc = -1
	# Current timer mode
	# 0 = Normal (countdown)
	# 1 = Stopped (countdown for stopTimeCheatTime)
	# 2 = Reversed (countup + countdown for upTimeCheatTime)
	timerState = 0
	# How many times was this minigame reset?
	resets = 0
	# How many goal wires were cut?
	goalCut = 0
	# Time remaining in "stop time" cheat countdown (seconds)
	stopTimeCheatTime = 0
	# Time remaining in "up time" cheat countdown (seconds)
	upTimeCheatTime = 0
	oneStepCheatTime = 0

	# How often should the minigame be ticked?
	tickRate = 0.25

	difficultyLevels = {
		1: {
			"bonusIcons": [1, 2, 3],
			"goalIcons": [4, 5, 6],
			"goalTotals": {
				0: [45, 15, 6],
				1: [45, 23, 12],
				2: [63, 30, 18],
				3: [72, 38, 18],
				4: [81, 38, 24],
				5: [90, 50, 24],
				6: [105, 63, 30],
				7: [120, 75, 30],
				8: [135, 88, 40],
				9: [150, 100, 48],
				10: [180, 125, 60]
			},
			"timeRemaining": 120,
			"playfieldSize": [8, 8]
		},
		2: {
			"bonusIcons": [1, 2, 3],
			"goalIcons": [4, 5, 6, 7],
			"goalTotals": {
				0: [63, 30, 18],
				1: [72, 38, 18],
				2: [81, 38, 24],
				3: [90, 50, 24],
				4: [105, 63, 30],
				5: [120, 75, 30],
				6: [135, 88, 40],
				7: [150, 100, 48],
				8: [180, 125, 60],
				9: [198, 130, 66],
				10: [216, 140, 72]
			},
			"timeRemaining": 120,
			"playfieldSize": [8, 8]
		},
		3: {
			"bonusIcons": [1, 2, 3],
			"goalIcons": [4, 5, 6, 7],
			"goalTotals": {
				0: [63, 30, 18],
				1: [72, 38, 18],
				2: [81, 38, 24],
				3: [90, 50, 24],
				4: [105, 63, 30],
				5: [120, 75, 30],
				6: [135, 88, 40],
				7: [150, 100, 48],
				8: [180, 125, 60],
				9: [198, 130, 66],
				10: [216, 140, 72]
			},
			"timeRemaining": 100,
			"playfieldSize": [9, 9]
		},
		4: {
			"bonusIcons": [1, 2, 3],
			"goalIcons": [4, 5, 6, 8, 7],
			"goalTotals": {
				0: [63, 30, 23, 6],
				1: [72, 38, 23, 12],
				2: [81, 38, 30, 18],
				3: [90, 50, 30, 24],
				4: [105, 63, 38, 30],
				5: [120, 75, 45, 30],
				6: [135, 88, 53, 36],
				7: [150, 100, 60, 42],
				8: [180, 125, 75, 48],
				9: [198, 130, 83, 54],
				10: [216, 140, 90, 60]
			},
			"timeRemaining": 80,
			"playfieldSize": [10, 10]
		}
	}



	class Wire(object):
		__slots__ = ('name', 'lib', 'depth', 'sound', 'x', 'y', 'cut')
		def __init__(self, name, lib, depth, sound, x, y):
			self.name = name
			self.lib = lib
			self.depth = depth
			self.sound = sound
			self.x = x
			self.y = y
			self.cut = False


	def __init__(self):
		super().__init__()
		self.ca = [0, 0, 0, 0, 0]
		self.wires = {}
		self.glowPoints = []


	def random(self, items):
		"""
		Returns a random item from the specified array
		@param items: Array of randomized items
		@return: Random item
		"""
		return items[randrange(0, len(items) - 1)]


	def updatedDifficulty(self):
		"""
		Called when the difficulty level or some other related parameters
		(tech competency, level, ...) was changed

		Updates parameters that are dependent on the difficulty level (timers, wire counts, ...)
		"""
		level = self.difficultyLevels[self.difficulty]
		self.paramTimer = floor(level['timerBase'] + self.techCompetency * level['timerMultiplier'])
		self.playfieldTotal = floor(level['playfieldBase'] + self.techCompetency * level['playfieldTechComp'])
		self.goalTotal = level['goals']
		self.movingTotal = floor(level['movingBase'] + self.techCompetency * level['movingTechComp'])
		self.moveTimer = self.techCompetency / level['moveTimerDivider']
		self.obstacleTotal = floor(level['obstaclesBase'] + self.techCompetency * level['obstaclesTechComp'])
		self.obstacleTimer = level['obstacleTimerBase'] + (self.techCompetency / 5) * level['obstacleTimerLevel']
		self.countdownRate = 1 / level['countdownRate']
		self.timeRemaining = level['countdownRate'] + 0.25
		self.readOut = level['readOut'] + str(self.techCompetency)
		timerType = randrange(0, 2)
		if timerType == 0:
			self.timeUnits = self.paramTimer + self.techCompetency / 1.7
		elif timerType == 1:
			self.timeUnits = 17 + self.techCompetency * 4.5 ** 1.25
		elif timerType == 2:
			self.timeUnits = 6 * (self.obstacleTimer / 10 * self.goalTotal) + (21 + self.playerLevel)
		else:
			assert False


	def findAvailableDepth(self, depth):
		"""
		Returns the first depth level that doesn't have any wires and is >= level
		@param depth: Minimum depth level
		@return: Free depth level
		"""
		while depth in self.wires:
			depth += 1
		return depth


	def findRandomDepth(self, minDepth, maxDepth):
		"""
		Returns a random depth level that doesn't have any wires and is between min and max
		@param minDepth: Minimum depth level
		@param maxDepth: maximum depth level
		@return: Free depth level
		"""
		depth = randrange(minDepth, maxDepth)
		while depth in self.wires:
			depth = randrange(minDepth, maxDepth)
		return depth


	###############################################################################################
	#                                SERVER -> CLIENT MESSAGES
	###############################################################################################


	def sendCmd(self, command, args):
		"""
		Sends an extension message to the client

		@param command: Message to send
		@param args: Dictionary of parameters
		"""
		args['_cmd'] = command
		self.send(args)


	def sendFullGameState(self):
		#if not self.initialized:
		#	return

		message = {
			'abilityBitfield': str(self.abilitiesMask),
			'instcc': str(self.instcc),
			'CA0': str(self.ca[0]),
			'CA1': str(self.ca[1]),
			'CA2': str(self.ca[2]),
			'CA3': str(self.ca[3]),
			'CA4': str(self.ca[4]),

			'numResets': '0',
			'timeRemaining': 120,
			'timerState': 0,
			'isSpectator': False,
			'gameArray': [randrange(1, 8) for i in range(8*8)],
			'playfieldWidth': 8,
			'playfieldHeight': 8,
			'goalIcons': [1, 2, 3],
			'goalTotals': 1,
			'goalScores': 1,
			'techcomp': 1,
			'difficulty': 1,
			'minigamelevel': 1
		}
		self.sendCmd('fullgamestate', message)


	def sendTimerUpdate(self):
		message = {
			'timeRemaining': self.timeRemaining - 0.25,
			'timerState': self.timerState
		}
		self.sendCmd('timerupdate', message)


	def addTime(self, timeDiff):
		self.timeRemaining += timeDiff
		message = {
			'timeDiff': timeDiff
		}
		self.sendCmd('addtime', message)


	def cutTime(self, timeDiff):
		self.timeRemaining -= self.timeRemaining * 0.25
		self.sendCmd('cuttime', {})


	def sendConvertToSeconds(self):
		self.sendCmd('converttoseconds', {})


	def sendRevertTime(self):
		self.sendCmd('reverttime', {})


	def sendCountdownUpdate(self):
		message = {
			'countdownRate': self.countdownRate
		}
		self.sendCmd('countdownupdate', message)


	def sendOpenMessage(self):
		self.sendCmd('opendoor', {})


	def sendCloseMessage(self, wireName):
		message = {
			'wirename': wireName
		}
		self.sendCmd('closedoor', message)


	def sendWireDestroyed(self, wireName):
		message = {
			'wirename': wireName
		}
		self.sendCmd('destroy', message)


	def sendWireCreated(self, wire):
		message = {
			'wireObj': {
				'_name': wire.name,
				'_lib': wire.lib,
				'depth': wire.depth,
				'sound': wire.sound,
				'x': wire.x,
				'y': wire.y
			}
		}
		self.sendCmd('create', message)


	def sendWiresFlash(self, wires):
		message = {
			'wirearray': wires
		}
		self.sendCmd('flash', message)


	def sendWireOver(self, wirename):
		message = {
			'wirename': wirename
		}
		self.sendCmd('over', message)


	def sendWireOut(self, wirename):
		message = {
			'wirename': wirename
		}
		self.sendCmd('out', message)


	def sendPlayfieldActive(self, active):
		message = {
			'playfieldActive': active
		}
		self.sendCmd('playfieldactive', message)


	def sendResetGame(self):
		self.sendCmd('resetgame', {})


	def sendFailure(self):
		self.sendCmd('failure', {})


	def sendVictory(self):
		self.sendCmd('victory', {})


	###############################################################################################
	#                            CLIENT -> SERVER MESSAGE HANDLERS
	###############################################################################################


	def onOpenDoor(self):
		self.sendOpenMessage()
		self.started = True
		Atrea.addTimer(Atrea.getGameTime() + self.tickRate, self.tick)
		if self.timerState == 0:
			self.timerState = 1
			self.lastTime = Atrea.getGameTime()
			self.sendTimerUpdate()
		return True


	def onCloseDoor(self):
		self.sendCloseMessage('')
		return True


	def onProcessReset(self):
		self.started = False
		self.sendResetGame()
		self.resets += 1
		self.init(False)
		return True


	def onProcessOver(self, wireName):
		self.sendWireOver(wireName)
		return True


	def onProcessOut(self, wireName):
		self.sendWireOut(wireName)
		return True


	def onProcessMove(self, wireName, countdownUpdate):
		if not self.started:
			warn(Atrea.enums.LC_Minigame, 'Wire "%s" cut when game was not started!' % wireName)
			return False

		wires = [wire for wire in self.wires.values() if wire.name == wireName]
		if len(wires) != 1:
			warn(Atrea.enums.LC_Minigame, 'Tried to cut unknown wire "%s"' % wireName)
			return False

		if wires[0].cut:
			warn(Atrea.enums.LC_Minigame, 'Wire "%s" was already cut!')
			return False

		wires[0].cut = True

		if wireName[0] == 'g':
			self.goalCut += 1
		elif wireName[0] == 'p':
			if not self.playfieldActive:
				warn(Atrea.enums.LC_Minigame, 'Wire "%s" cut when playfield is not active!' % wireName)
				return False
		elif wireName[0] == 'o':
			self.countdownRate += self.countdownRate * self.obstacleTimer / 100
			self.sendCountdownUpdate()
		elif wireName[0] == 'm':
			self.countdownRate += self.countdownRate * self.moveTimer / 100
			self.sendCountdownUpdate()
		else:
			warn(Atrea.enums.LC_Minigame, 'Tried to cut illegal wire "%s"' % wireName)
			return False

		self.sendWireDestroyed(wireName)
		if self.goalCut == self.goalTotal:
			self.sendVictory()
			self.victory()
			# TODO: Save consumablesUsed / instcc?
		return True


	def onActivateItemCheat(self, cheatNum):
		return True


	def onActivateAbilityCheat(self, cheatNum):
		return True


	###############################################################################################
	#                                  INTERNAL FUNCTIONS
	###############################################################################################


	def tick(self):
		if not self.started:
			return

		timeDelta = Atrea.getGameTime() - self.lastTime
		self.lastTime = Atrea.getGameTime()
		if self.timerState == 1:
			self.timeRemaining -= timeDelta
		elif self.timerState == 2:
			self.stopTimeCheatTime -= timeDelta
			if self.stopTimeCheatTime <= 0:
				self.timeRemaining += self.stopTimeCheatTime
				self.timerState = 1
				self.sendTimerUpdate()
		elif self.timerState == 3:
			self.upTimeCheatTime -= timeDelta
			self.timeRemaining += timeDelta
			if self.upTimeCheatTime <= 0:
				self.timeRemaining += self.upTimeCheatTime * 2
				self.timerState = 1
				self.sendTimerUpdate()

		if self.moveMode == 2:
			self.oneStepCheatTime -= timeDelta
			if self.oneStepCheatTime <= 0:
				self.moveMode = 0

		if self.timeRemaining <= 0 and self.timerState >= 1:
			self.timerState = 0
			self.started = False
			self.sendFailure()

		Atrea.addTimer(Atrea.getGameTime() + self.tickRate, self.tick)


	def init(self, firstInit):
		self.initialized = True
		self.started = False
		self.consumablesUsed = False
		self.consumableCheatBonus = False
		self.playfieldActive = False
		self.moveMode = 0
		self.timerState = 0
		self.updatedDifficulty()
		if firstInit:
			self.initialTime = self.timeRemaining
		self.sendFullGameState()


	def started(self):
		# self.init(True)
		self.sendFullGameState()


	def aborted(self):
		self.started = False
		self.sendFailure()


	def message(self, cmd, args):
		if cmd == 'opendoor':
			return self.onOpenDoor()
		elif cmd == 'closedoor':
			return self.onCloseDoor()
		elif cmd == 'processreset':
			return self.onProcessReset()
		elif cmd == 'processover':
			return self.onProcessOver(args['wirename'])
		elif cmd == 'processout':
			return self.onProcessOut(args['wirename'])
		elif cmd == 'processmove':
			return self.onProcessMove(args['wirename'], args['countdownupdate'])
		elif cmd == 'activateitemcheat':
			return self.onActivateItemCheat(args['cheatNum'])
		elif cmd == 'activateabilitycheat':
			return self.onActivateAbilityCheat(args['cheatNum'])
		else:
			warn(Atrea.enums.LC_Minigame, 'Got invalid Livewire message "%s"!' % cmd)
			return False


