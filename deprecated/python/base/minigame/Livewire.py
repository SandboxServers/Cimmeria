from math import floor
from random import randrange
import Atrea
import Atrea.enums
from common.Logger import warn


class Livewire(Atrea.Minigame):
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
			# Time remaining at beginning of countdown
			"countdownRate": 20,
			"timerBase": 30,
			"timerMultiplier": 0.588235,
			# Amount of playfield objects remaining
			"playfieldBase": 6,
			# Amount of playfield objects multiplied by techCompetency
			"playfieldTechComp": 0.075472,
			# Amount of goal wires remaining
			"goals": 2,
			# Amount of moving objects
			"movingBase": 2,
			# Amount of moving objects multiplied by techCompetency
			"movingTechComp": 0.188679,
			"moveTimerDivider": 2.5,
			# Amount of obstacle objects
			"obstaclesBase": 2,
			# Amount of obstacle objects multiplied by techCompetency
			"obstaclesTechComp": 0.188679,
			# Timer acceleration when hitting an obstacle wire
			"obstacleTimerBase": 5,
			# Timer acceleration level multiplier when hitting an obstacle wire
			"obstacleTimerLevel": 5,
			"readOut": "I-"
		},
		2: {
			"countdownRate": 17,
			"timerBase": 20,
			"timerMultiplier": 0.588235,
			"playfieldBase": 6,
			"playfieldTechComp": 0.094340,
			"goals": 4,
			"movingBase": 2,
			"movingTechComp": 0.188679,
			"moveTimerDivider": 2,
			"obstaclesBase": 2,
			"obstaclesTechComp": 0.188679,
			"obstacleTimerBase": 6,
			"obstacleTimerLevel": 5,
			"readOut": "S-"
		},
		3: {
			"countdownRate": 15,
			"timerBase": 20,
			"timerMultiplier": 0.392157,
			"playfieldBase": 6,
			"playfieldTechComp": 0.094340,
			"goals": 4,
			"movingBase": 3,
			"movingTechComp": 0.245283,
			"moveTimerDivider": 1.5,
			"obstaclesBase": 3,
			"obstaclesTechComp": 0.245283,
			"obstacleTimerBase": 7,
			"obstacleTimerLevel": 5,
			"readOut": "C-"
		},
		4: {
			"countdownRate": 12,
			"timerBase": 12,
			"timerMultiplier": 0.372549,
			"playfieldBase": 6,
			"playfieldTechComp": 0.150943,
			"goals": 6,
			"movingBase": 5,
			"movingTechComp": 0.320755,
			"moveTimerDivider": 1,
			"obstaclesBase": 3,
			"obstaclesTechComp": 0.245283,
			"obstacleTimerBase": 9,
			"obstacleTimerLevel": 5,
			"readOut": "E-"
		}
	}

	playfieldSounds = ["hackMG/play/low", "hackMG/play/mid", "hackMG/play/high"]
	movingSounds = ["hackMG/move/low", "hackMG/move/mid", "hackMG/move/high"]
	goalSoundNames = ["hackMG/goal/low", "hackMG/goal/mid", "hackMG/goal/high"]
	obstacleSounds = ["hackMG/obsta/low", "hackMG/obsta/mid", "hackMG/obsta/high"]
	playfieldWires = ["pGray", "pRust", "pBronze", "pGreen"]
	playfieldElectrodes = ["pProcessorFan1", "pPurpleBlack1", "pPurpleBlack2", "pOrangeSilver1", "pOrangeSilver2", "pLEDWhite", "pLEDRed", "pLEDBlue", "pLEDGreen", "pLEDYellow"]
	movingWires = ["mYellowStripe", "mPurpleStripe", "mOrangeStripe", "mBlackStripe"]
	movingElectrodes = ["mGear", "mMicroChip1", "mMicroChip2"]
	goalWires = ["gGreenBlack", "gRedSilver", "gBlackGreen"]
	obstacleWires = ["oRed", "oGreen", "oBlack"]
	wireLocationX = 275.6
	wireLocationY = [391, 497, 577, 657, 783]


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
		if not self.initialized:
			return

		message = {
			'abilityBitfield': str(self.abilitiesMask),
			'wireSounds': [wire.sound for wire in self.wires.values()],
			'wireNames': [wire.name for wire in self.wires.values()],
			'glowPoints': self.glowPoints,
			'wireXs': [wire.x for wire in self.wires.values()],
			'wireYs': [wire.y for wire in self.wires.values()],
			'wireDepths': [wire.depth for wire in self.wires.values()],
			'wireLibs': [wire.lib for wire in self.wires.values()],
			'gameReadOut': self.readOut,
			'gameStarted': 'true' if self.started else 'false',
			'countdownRate': str(self.countdownRate),
			'playfieldActive': 'true' if self.playfieldActive else 'false',
			'instcc': str(self.instcc),
			'CA0': str(self.ca[0]),
			'CA1': str(self.ca[1]),
			'CA2': str(self.ca[2]),
			'CA3': str(self.ca[3]),
			'CA4': str(self.ca[4]),
			'timeUnits': str(self.timeUnits),
			'timeRemaining': self.timeRemaining,
			'timerState': str(0.0)
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
		self.setupWires()
		self.sendFullGameState()


	def setupWires(self):
		# TODO: Fix duplicate wire bugs!
		# Add playfield electrodes
		electrodes = floor(self.playfieldTotal * 0.3)
		for i in range(0, electrodes):
			depth = self.findAvailableDepth(101)
			self.wires[depth] = Livewire.Wire('pe' + str(i + 1), self.random(self.playfieldElectrodes),
				depth, self.random(self.playfieldSounds), randrange(375, 900), randrange(250, 700))

		# Add playfield wires
		for i in range(0, self.playfieldTotal - electrodes):
			depth = self.findRandomDepth(1, 100)
			self.wires[depth] = Livewire.Wire('p' + str(i + 1), self.random(self.playfieldWires),
				depth, self.random(self.playfieldSounds), self.wireLocationX, self.random(self.wireLocationY))

		# Add goal wires
		for i in range(0, self.goalTotal):
			depth = self.findRandomDepth(1, 100)
			y = randrange(0, len(self.wireLocationY) - 1)
			self.wires[depth] = Livewire.Wire('g' + str(i + 1), self.random(self.goalWires) + str(randrange(1, 4)),
				depth, self.random(self.goalSoundNames), self.wireLocationX, self.wireLocationY[y])
			self.glowPoints.append("attach" + str(y + 1))

		# Add moving electrodes
		electrodes = floor(self.movingTotal * 0.3)
		for i in range(0, electrodes):
			depth = self.findAvailableDepth(101)
			self.wires[depth] = Livewire.Wire('me' + str(i + 1), self.random(self.movingElectrodes),
				depth, self.random(self.movingSounds), randrange(375, 900), randrange(250, 700))

		# Add moving wires
		for i in range(0, self.movingTotal - electrodes):
			depth = self.findRandomDepth(1, 100)
			self.wires[depth] = Livewire.Wire('m' + str(i + 1), self.random(self.movingWires),
				depth, self.random(self.movingSounds), self.wireLocationX, self.random(self.wireLocationY))

		# Add obstacles
		for i in range(0, self.obstacleTotal):
			depth = self.findRandomDepth(1, 100)
			self.wires[depth] = Livewire.Wire('o' + str(i + 1), self.random(self.obstacleWires) + str(randrange(1, 4)),
				depth, self.random(self.obstacleSounds), self.wireLocationX, self.random(self.wireLocationY))


	def started(self):
		self.init(True)


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


