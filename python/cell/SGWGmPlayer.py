import Atrea
import Atrea.enums
from cell.Global import PathingDebugSubscriptions
from cell.Minigame import Minigame
from cell.SGWPlayer import SGWPlayer
from common.Config import Config
from common.defs.Def import DefMgr

def pathingDebugMove(entityId, path):
	for witness in PathingDebugSubscriptions.values():
		witness.client.onShowPath(entityId, Atrea.enums.MOB_MOVEMENT_Follow, path)
	

class SGWGmPlayer(SGWPlayer):
	def __init__(self):
		super().__init__()
		self.autosaveSpawns = False
		self.navmeshDebugStart = None
		self.navmeshDebugEnd = None
		self.pathingDebugEnabled = False


	def destroyed(self):
		super().destroyed()
		if self.pathingDebugEnabled:
			del PathingDebugSubscriptions[self.entityId]


	def gmSetFlag(self, aFlagId, aForceVal):
		self.client.onUpdateKnownCrafts([2, 3, 4, 5, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77])
		if self.targetId == 0:
			entities = []
		else:
			entities = [self.targetId]
		item = self.inventory.getBag(Atrea.enums.INV_Main).getItem(5)
		if item:
			items = [item.id]
		else:
			items = []
		self.client.onUpdateCraftingOptions({
			'crafting': {'items': items, 'entities': entities},
			'research': {'items': items, 'entities': entities},
			'reverseEngineering': {'items': items, 'entities': entities},
			'alloying': {'items': items, 'entities': entities}
		})


	def gmSetTarget(self, nameOrId):
		target = self.gmGetTargetBeing()
		if target is not None:
			try:
				targetId = int(nameOrId)
				target.changeTarget(targetId)
				self.debug("Target of entity %d set to %d" % (target.entityId, targetId))
			except ValueError:
				self.onError("Illegal target ID: %s" % nameOrId)


	# DEBUG COMMANDS
	def gmDebugAbility(self, abilityId):
		target = self.gmGetTargetBeing()
		if target:
			prev = target.abilities.setDebuggingPlayer(self)
			ability = DefMgr.get('ability', abilityId)
			target.abilities.launchAbility(ability, target.targetId, isEntityAbility=False)
			target.abilities.setDebuggingPlayer(prev)


	def gmDebugHeal(self):
		entity = self.gmGetTargetBeing()
		if entity is not None:
			entity.getStat(Atrea.enums.health).changeByMinMaxPercent(1.0)
			entity.getStat(Atrea.enums.focus).changeByMinMaxPercent(1.0)
			entity.sendDirtyStats()
			self.debug("Healed entity %d" % entity.entityId)


	def debugMinigameEnded(self, game, resultId):
		self.debug('Result code of minigame: %d' % resultId)


	def gmDebugStartMinigame(self, gameId):
		if gameId not in Config.MINIGAME_NAMES:
			self.onError("Unknown minigame id: %d" % gameId)
			return

		self.debug('Starting minigame %s ...' % Config.MINIGAME_NAMES[gameId])
		request = Minigame("Debug Game", 1, gameId, 1, 0xff, lambda game, result: self.debugMinigameEnded(game, result))
		request.play(self)


	def gmDebugBehaviorsOnMob(self):
		"""
		Enable/disable AI trace messages for the targeted mob.
		"""
		entity = self.gmGetTargetMob()
		if entity is not None:
			entity.enableAiDebug = not entity.enableAiDebug
			if entity.enableAiDebug:
				self.debug('Enabled behavior debug for %d <%s>' % (entity.entityId, entity.getName()))
			else:
				self.debug('Disabled behavior debug for %d <%s>' % (entity.entityId, entity.getName()))


	def onPhysics(self, bTurnOn):
		print('SGWGmPlayer::onPhysics: ', str(bTurnOn))
