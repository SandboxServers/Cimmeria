import Atrea
import Atrea.enums
from base.Chat import ChannelManager
from base.SGWBeing import SGWBeing
from common import Constants
from common.Config import Config
from common.Logger import trace, warn


class SGWPlayer(SGWBeing):
	account = None
	# TODO: Move these to the ChannelManager.players dict
	afkMessage = None
	dndMessage = None
	# Are we registered in the channel manager?
	chatRegistered = False

	def __init__(self):
		super().__init__()
		self.channelMemberships = []
		
	def destroyed(self):
		if self.chatRegistered:
			# We need to make a copy to make sure that the membership list isn't
			# modified while we're iterating over it
			channels = self.channelMemberships[:]
			for channelId in channels:
				ChannelManager.leaveChannel(self, channelId, True)
			ChannelManager.playerLeft(self)

	def load(self):
		player = Atrea.dbQuery("select player_name, access_level from sgw_player where player_id = " + str(self.databaseId))[0]
		self.playerName = player['player_name']
		self.accessLevel = player['access_level']
		if Config.DEVELOPER_MODE:
			self.accessLevel = 65535
		return True

	def persist(self):
		pass
		
	def attachedToController(self):
		pass
		
	def detachedFromController(self):
		pass
		
	def cellCreated(self, spaceId):
		if self.account is not None:
			Atrea.switchEntity(self.account.entityId, self.entityId)
			self.account = None
		
	def cellCreateFailed(self):
		warn(Atrea.enums.LC_LogInOut, 'Failed to create player entity for <%s> on the cell' % self.playerName)
		if self.account is not None:
			self.account.playCharacterFailed(self)
		else:
			self.disconnect(True)
		
	def cellDestroyed(self):
		pass


	def feedback(self, msg):
		trace(Atrea.enums.LC_Chat, '(%s) <-- %s' % (self.playerName, msg))
		if self.client is not None:
			self.client.onPlayerCommunication('', 0, Atrea.enums.CHAN_feedback, msg)


	def debug(self, msg):
		self.client.onPlayerCommunication('', 0, Atrea.enums.CHAN_feedback, msg)

	def minigameCompleted(self, gameId, resultId):
		trace(Atrea.enums.LC_Minigame, "Minigame %s result for player %s: %d" % (Config.MINIGAME_NAMES[gameId], self.playerName, resultId))
		self.cell.handleMinigameResults(''.encode(), resultId, 0.0, 0, 0, 0)

	def startMinigame(self, gameId, ticket, difficulty, techComp, seed, abilities, intelligence, level):
		"""
		RPC called by the cell entity when it wants to register a new minigame session with the minigame server
		@param gameId: ID of minigame to start (
		@param ticket: Minigame server ticket ID; unused
		@param difficulty: Game difficulty (1-5)
		@param techComp: Tech competency
		@param seed: Random seed to use when generating the game
		@param abilities: Abilities bitmask
		@param intelligence: Intelligence level of player
		@param level: Level of player (NOTE: this field was renamed from the original prevCount in the .def file)
		"""
		url = Atrea.registerMinigameSession(self.entityId, Config.MINIGAME_NAMES[gameId], difficulty, techComp, seed,
		        abilities, intelligence, level, lambda result: self.minigameCompleted(gameId, result))
		if url:
			trace(Atrea.enums.LC_Minigame, "Starting minigame from %s for %s" % (url, self.playerName))
			self.client.onStartMinigame(url)
		else:
			# Failed to create a minigame session
			self.cell.handleMinigameResults(''.encode(), Constants.MINIGAME_RESULT_NotStarted, 0.0, 0, 0, 0)

	def endMinigameForPlayer(self, ticket):
		"""
		RPC called by the cell entity when the client notified it of closing the minigame session via a client RPC
		@param ticket: Minigame server ticket ID; unused
		"""
		Atrea.cancelMinigameSession(self.entityId)

	def cookedDataError(self, categoryId, elementId):
		"""
		Called when the server fails to send a cooked resource to the player.
		@param categoryId: Category of cooked resource
		@param elementId: ID of cooked resource
		"""
		# The client erroneously requests null (0) kismet sequences
		# don't show error messages about this
		if not (categoryId == 6 and elementId == 0):
			warn(Atrea.enums.LC_Resources, "Failed to send cooked resource: categoryId=%d, key=%d" % (categoryId, elementId))
		self.client.onCookedDataError(categoryId, elementId)
		
	def elementDataRequest(self, categoryId, key):
		# trace('Client resource requested; category: ', categoryId, ', key: ', key)
		Atrea.sendResource(self, categoryId, key)
		
	def onClientReady(self):
		# trace("--- CLIENT READY ---")
		self.cell.onClientReady()
		if not self.chatRegistered:
			ChannelManager.playerLoggedIn(self)
			self.chatRegistered = True
		
	def onPlayerReady(self):
		trace(Atrea.enums.LC_Location, "Player <%s> loaded on cell" % self.playerName)
		
	def perfStats(self, fpsAvg, fpsMin, fpsMax, bpsIn, bpsOut, packetsIn, packetsOut, lagAvgMS, lagMinMS, lagMaxMS, resends, appearanceJobs):
		pass

	def logOff(self, disconnect):
		# TODO: Add destroyBaseEntity() native method
		trace(Atrea.enums.LC_Uncategorized, 'Logging the player out...well actually doing nothing')
	
	def cancelLogOff(self):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::cancelLogOff')
	
	def sendDuelChallenge(self, aPlayerName, aSquadDuel):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::sendDuelChallenge %s' % aPlayerName)

	def onSpaceQueueStatus(self):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::onSpaceQueueStatus')
	
	def onSpaceQueueReadyResponse(self, aAccept):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::onSpaceQueueReadyResponse')
	
	def onSpaceQueuedResponse(self, aAccept):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::onSpaceQueuedResponse')


	# Communicator interface
	def onChannelJoined(self, channelId, channelName):
		"""
		Called by the chat channel manager after joining a channel
		@param channelId: Id of channel that the player joined
		@param channelName: Name of channel that the player joined
		@return:
		"""
		if channelId >= Constants.MIN_USER_CHANNEL:
			self.client.onChatJoined(channelName, channelId - Constants.MIN_USER_CHANNEL)
		self.channelMemberships.append(channelId)


	def onChannelLeft(self, channelId, channelName):
		"""
		Called by the chat channel manager after leaving a channel
		@param channelId: Id of channel that the player left
		@param channelName: Name of channel that the player left
		@return:
		"""
		if channelId >= Constants.MIN_USER_CHANNEL:
			self.client.onChatLeft(channelName)
		self.channelMemberships.remove(channelId)


	def chatJoin(self, channelName, channelPassword):
		self.debug("chatJoin: %s" % channelName)
		if not ChannelManager.joinChannel(self, channelName, channelPassword):
			warn(Atrea.enums.LC_Chat, "Failed to join channel %s" % channelName)


	def chatLeave(self, channelId):
		self.debug("chatLeave: %d" % channelId)
		if not ChannelManager.leaveChannel(self, channelId + Constants.MIN_USER_CHANNEL):
			warn(Atrea.enums.LC_Chat, "Failed to leave channel %d" % channelId)


	def sendPlayerCommunication(self, channelId, target, text):
		ChannelManager.sendPlayerMessage(self, channelId, target, text)


	def chatSetAFKMessage(self, afkMessage):
		if len(afkMessage) > 1:
			self.afkMessage = afkMessage
		else:
			self.afkMessage = None


	def chatSetDNDMessage(self, dndMessage):
		if len(dndMessage) > 1:
			self.dndMessage = dndMessage
		else:
			self.dndMessage = None


	def chatIgnore(self, aPlayerName, aFlag):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatIgnore %s' % aPlayerName)


	def chatFriend(self, aPlayerName, aPlayerNick, aFlag):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatFriend %s' % aPlayerName)


	def chatList(self, aChannelID):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatList')


	def chatMute(self, aChannelID, aPlayerName, aFlag):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatMute')


	def chatKick(self, aChannelID, aPlayerName):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatKick')


	def chatOp(self, aChannelID, aPlayerName):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatOp')


	def chatBan(self, aChannelID, aPlayerName, aFlag):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatBan')


	def chatPassword(self, aChannelID, aChannelPassword):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::chatPassword')


	def petition(self, aMessage):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::petition %s' % aMessage)


	def announcePetition(self, aMessage):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::announcePetition %s' % aMessage)


	# OrganizationMember interface
	
	def organizationInvite(self, aOrganizationId, aPlayerName):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::organizationInvite id: %d, name: %s' % (aOrganizationId, aPlayerName))
	
	def organizationInviteByType(self, aOrganizationType, aPlayerName):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::organizationInviteByType type: %d, name: %s' % (aOrganizationType, aPlayerName))

	def organizationKick(self, aOrganizationId, aPlayerName):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::organizationKick id: %d, name: %s' % (aOrganizationId, aPlayerName))

	def organizationRankChange(self, aOrganizationId, aPlayerName, aRank):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::organizationRankChange id: %d, name: %s' % (aOrganizationId, aPlayerName))

	#MinigamePlayer interface
	
	def minigameCallRequest(self, remotePlayerName, tipAmount):
		trace(Atrea.enums.LC_Uncategorized, 'SGWPlayer::Base::minigameCallRequest name: %s, tip: %d' % (remotePlayerName, tipAmount))

	# BlackMarketManager interface

	def BMSearch(self, searchOptions):
		pass

	def BMCreateAuction(self, itemInstanceId, buyoutPrice, auctionLength, startingPrice):
		pass

	def BMPlaceBid(self, sequenceId, bidAmount):
		pass

	def BMCancelAuction(self, sequenceId):
		pass

	def BMStartWatchingItem(self, itemDefId):
		pass

	def BMStopWatchingItem(self, itemDefId):
		pass




