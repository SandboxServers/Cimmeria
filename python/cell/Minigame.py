import weakref
from common.Config import Config

class Minigame(object):
	def __init__(self, name, difficulty, gameId, techCompetency, archetypes, callback):
		"""
		Creates a new minigame play request
		@param name: Name of minigame session
		@param difficulty: Difficulty level (1-5)
		@param gameId: Id of minigame to play (see Config.MINIGAME_NAMES)
		@param techCompetency: ??? (1)
		@param archetypes: Bitmask of archetypes that can play the game (mask of shifted EArchetype values)
		@param callback: Handler function called when the minigame is finished
		"""
		self.name = name
		assert(1 <= difficulty <= 5)
		self.difficulty = difficulty
		assert(gameId in Config.MINIGAME_NAMES)
		self.gameId = gameId
		self.gameName = Config.MINIGAME_NAMES[gameId]
		self.techCompetency = techCompetency
		self.archetypes = archetypes
		self.callback = callback
		self.player = None
		self.seed = None


	def setSeed(self, seed):
		"""
		Sets the random seed used when playing the minigame
		@param seed: Integer in the range 0..0x7fffffff
		"""
		assert(seed is None or 0 <= seed <= 0x7fffffff)
		self.seed = seed


	def play(self, player):
		"""
		Starts the minigame session for the player
		@param player: SGWPlayer object
		@return: True if the minigame was started, False otherwise
		"""
		assert(self.player is None)
		if player.requestStartMinigame(self):
			self.player = weakref.ref(player)
			return True
		else:
			return False


	def onMinigameResult(self, resultId):
		"""
		Called by the player when the minigame session is finished
		@param resultId: Result code (see MINIGAME_RESULT_ constants in Constants.py)
		"""
		self.callback(self, resultId)
		self.player = None

