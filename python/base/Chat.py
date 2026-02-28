import weakref
from common import Constants
from Atrea import enums, log
from common.Logger import warn, trace


class ChatChannel(object):
	class Membership(object):
		__slots__ = ('player', 'op')
		def __init__(self, player, op):
			self.player = player
			self.op = op


	def __init__(self, channelId, name, password, flags):
		self.id = channelId
		self.name = name
		self.password = password
		self.flags = flags
		# List of all players on the channel (entityId : MembershipData)
		self.players = {}


	def canPlayerSpeak(self):
		"""
		@return: True if ordinary players can speak in this channel, False otherwise
		"""
		return (self.flags & Constants.CHANNEL_FLAG_DisallowPlayerMessages) == 0


	def isCellChannel(self):
		"""
		@return: True if messages in this channel are processed on the CellApp, False otherwise
		"""
		return (self.flags & Constants.CHANNEL_FLAG_OnCell) != 0


	def keepIfEmpty(self):
		"""
		@return: True if channel should be kept alive even if it's empty, False otherwise
		"""
		return (self.flags & Constants.CHANNEL_FLAG_KeepIfEmpty) != 0


	def hasPlayers(self):
		"""
		@return: True if the channel has players in it, False otherwise
		"""
		return len(self.players) > 0


	def addPlayer(self, player, password, op = False):
		"""
		Adds a new player to the channel
		@param player: SGWPlayer instance
		@param password: Password specified when joining
		@param op: Is the new player an op?
		@return: True, if the player was added, False otherwise
		"""
		if player.entityId in self.players:
			warn(enums.LC_Chat, "<%s> Cannot enter channel %s: Player already in channel" % (player.playerName, self.name))
			player.feedback("Cannot enter channel %s: Already joined to channel" % self.name)
			return False

		if self.password != "" and password != self.password:
			warn(enums.LC_Chat, "<%s> Cannot enter channel %d: Invalid password" % (player.playerName, self.name))
			player.feedback("Cannot enter channel %s: Invalid password" % self.name)
			return False

		self.players[player.entityId] = ChatChannel.Membership(weakref.ref(player), op)
		player.onChannelJoined(self.id, self.name)
		return True


	def removePlayer(self, player):
		"""
		Removes a player from the channel
		@param player: SGWPlayer instance
		@return: True, if the player was removed, False otherwise
		"""
		if player.entityId not in self.players:
			return False

		del self.players[player.entityId]
		player.onChannelLeft(self.id, self.name)
		return True


	def removeAll(self):
		"""
		Removes all players from the channel
		"""
		for player in self.players.values():
			if player.player() is not None:
				player.player().onChannelLeft(self.id, self.name)
		self.players.clear()


	def setPlayerOp(self, player, op):
		"""
		Updates whether the player is an operator in this channel
		@param player: SGWPlayer instance
		@param op: Is the player an op?
		@return: True, if the player state was updated, False otherwise
		"""
		if player.entityId not in self.players:
			return False

		self.players[player.entityId].op = op
		return True


	def isOp(self, player):
		"""
		Checks whether the player is an operator in this channel
		@param player: SGWPlayer instance
		@return: True, if the player is an op, False otherwise
		"""
		if player.entityId not in self.players:
			return False

		return self.players[player.entityId].op


	def sendMessage(self, speaker, speakerFlags, message):
		"""
		Sends a message to all players on the channel
		@param speaker: Name of player sending the message
		@param speakerFlags: See ESpeakerFlags enumeration
		@param message: Text to speak
		"""
		trace(enums.LC_Chat, "[%s on %s] %s" % (speaker, self.name, message))
		for membership in self.players.values():
			player = membership.player()
			if player:
				player.client.onPlayerCommunication(speaker, speakerFlags, self.id, message)
			else:
				warn(enums.LC_Chat, "Player on %s is a dead weak reference!" % self.name)


class ChatChannelManager(object):
	def __init__(self):
		self.channels = {
			enums.CHAN_say: ChatChannel(enums.CHAN_say, "say", "", Constants.CHANNEL_FLAG_OnCell),
			enums.CHAN_emote: ChatChannel(enums.CHAN_emote, "emote", "", Constants.CHANNEL_FLAG_OnCell),
			enums.CHAN_yell: ChatChannel(enums.CHAN_yell, "yell", "", Constants.CHANNEL_FLAG_OnCell),
			enums.CHAN_team: ChatChannel(enums.CHAN_team, "team", "", Constants.CHANNEL_FLAG_OnCell),
			enums.CHAN_squad: ChatChannel(enums.CHAN_squad, "squad", "", Constants.CHANNEL_FLAG_OnCell),
			enums.CHAN_command: ChatChannel(enums.CHAN_command, "command", "", Constants.CHANNEL_FLAG_OnCell),
			enums.CHAN_officer: ChatChannel(enums.CHAN_officer, "officer", "", Constants.CHANNEL_FLAG_OnCell),
			enums.CHAN_server: ChatChannel(enums.CHAN_server, "server", "", Constants.CHANNEL_FLAG_DisallowPlayerMessages),
			enums.CHAN_feedback: ChatChannel(enums.CHAN_feedback, "feedback", "", Constants.CHANNEL_FLAG_OnCell | Constants.CHANNEL_FLAG_DisallowPlayerMessages),
			enums.CHAN_tell: ChatChannel(enums.CHAN_tell, "tell", "", 0),
			enums.CHAN_splash: ChatChannel(enums.CHAN_splash, "splash", "", Constants.CHANNEL_FLAG_OnCell | Constants.CHANNEL_FLAG_DisallowPlayerMessages)
		}
		self.nextChannelId = Constants.MIN_USER_CHANNEL
		# List of all players in game (playerName : SGWPlayer weakref)
		self.players = {}
		# Add some default channels
		self.createChannel("chat", "", Constants.CHANNEL_FLAG_KeepIfEmpty)
		self.createChannel("roleplay", "", Constants.CHANNEL_FLAG_KeepIfEmpty)
		self.createChannel("alliance", "", Constants.CHANNEL_FLAG_KeepIfEmpty)


	def getSpeakerFlags(self, player):
		flags = 0
		if player.dndMessage is not None:
			flags |= enums.SPEAKER_DND
		if player.accessLevel > 0:
			flags |= enums.SPEAKER_GM
		# TODO: Add PlatoonLeader, Petition flag checks
		return flags


	def isPlayerOnline(self, playerName):
		"""
		@param playerName: Name of player to check
		@return: True if the player is online, False otherwise
		"""
		return playerName in self.players


	def playerLoggedIn(self, player):
		"""
		Indicates that a player entered the game
		@param player: SGWPlayer instance
		"""
		self.players[player.playerName] = weakref.ref(player)
		# Subscribe the player to the server channel
		self.channels[enums.CHAN_server].addPlayer(player, "")


	def playerLeft(self, player):
		"""
		Indicates that a player left the game
		@param player: SGWPlayer instance
		"""
		del self.players[player.playerName]


	def createChannel(self, name, password, flags = 0):
		"""
		Creates a new chat channel using the specified name and password
		@param name: Channel name
		@param password: Channel password (empty if no password)
		@param flags: Channel flags
		@return: Created channel
		"""
		channelId = self.nextChannelId
		self.nextChannelId += 1
		self.channels[channelId] = ChatChannel(channelId, name, password, flags)
		return self.channels[channelId]


	def deleteChannel(self, channelId):
		"""
		Deletes a chat channel
		@param channelId: Channel Id
		@return: True if channel was deleted, False otherwise
		"""
		if channelId not in self.channels:
			warn(enums.LC_Chat, "Cannot delete channel: Invalid channel id %d" % channelId)
			return False

		if channelId < Constants.MIN_USER_CHANNEL:
			warn(enums.LC_Chat, "Cannot delete system channel %d" % channelId)
			return False

		self.channels[channelId].removeAll()
		del self.channels[channelId]
		return True


	def joinChannel(self, player, channelName, password):
		"""
		Adds a player to the channel
		@param player: SGWPlayer instance
		@param channelName: Name of channel
		@param password: Password specified when joining
		@return: True if player was added, False otherwise
		"""
		for channel in self.channels.values():
			if channel.name == channelName and channel.id >= Constants.MIN_USER_CHANNEL:
				return channel.addPlayer(player, password)

		warn(enums.LC_Chat, "Cannot join channel %s: Channel not found" % channelName)
		return False


	def leaveChannel(self, player, channelId, force = False):
		"""
		Removes a player from the channel
		@param player: SGWPlayer instance
		@param channelId: Id of channel to leave
		@return: True if player was removed, False otherwise
		"""
		if channelId in self.channels and (channelId >= Constants.MIN_USER_CHANNEL or force):
			channel = self.channels[channelId]
			if channel.removePlayer(player):
				if not channel.hasPlayers() and not channel.keepIfEmpty() and channelId >= Constants.MIN_USER_CHANNEL:
					warn(enums.LC_Chat, "Deleting empty channel %d" % channelId)
					self.deleteChannel(channel.id)
				return True
			else:
				return False

		warn(enums.LC_Chat, "Cannot leave channel %d: Channel not found" % channelId)
		return False


	def requestCreateChannel(self, player, name, password):
		"""
		Called when a player requested the channel manager to create a new chat channel
		@param player: Player creating the channel
		@param name: Channel name
		@param password: Channel password (empty if no password)
		@return: Created channel
		"""
		for channel in self.channels:
			if channel.name == name:
				warn(enums.LC_Chat, '<%s> tried to create existing channel "%s"' % (player.playerName, name))
				player.feedback('Channel "%s" already exists' % name)
				return False

		# TODO: Limit the number of channels a player can create
		channel = self.createChannel(name, password, Constants.CHANNEL_FLAG_OnCell)
		assert channel.addPlayer(player, password, True)
		return channel


	def requestDeleteChannel(self, player, channelId):
		"""
		Called when a player requested the channel manager to delete a chat channel
		@param player: Player deleting the channel
		@param channelId: Channel Id
		@return: True if channel was deleted, False otherwise
		"""
		if channelId not in self.channels:
			warn(enums.LC_Chat, "<%s> Cannot delete channel: Invalid channel id %d" % (player.playerName, channelId))
			return False

		if channelId < Constants.MIN_USER_CHANNEL:
			warn(enums.LC_Chat, "Cannot delete system channel %d" % channelId)
			return False

		if not self.channels[channelId].isOp(player):
			warn(enums.LC_Chat, "<%s> Cannot delete channel: Not an op" % player.playerName)
			player.feedback("Cannot delete channel: Not an op")
			return False

		return self.deleteChannel(channelId)


	def sendPlayerMessage(self, player, channelId, target, message):
		"""
		Sends a player message on the given channel
		@param player: SGWPlayer sending the message
		@param channelId: ID of channel to send on
		@param target: Player name (target of communication)
		@param message: Text to speak
		@return: True if message was sent, False otherwise
		"""
		if channelId not in self.channels:
			warn(enums.LC_Chat, "Invalid channel id: %d" % channelId)
			return False

		channel = self.channels[channelId]
		flags = self.getSpeakerFlags(player)
		# The player cannot speak on certain reserved channels (eg. feedback, server, splash)
		if not channel.canPlayerSpeak():
			warn(enums.LC_Chat, "Players cannot speak on this channel: %d" % channelId)
			return False

		# Distribution of cell messages is handled on the cell, so we forward them there without checking anything
		if channel.isCellChannel():
			player.cell.processPlayerCommunication(player.playerName, flags, target, channelId, message)
			return True

		# Tell channel needs to be handled specially, as messages sent there are only sent to
		# a single recipient instead of everyone in the channel
		if channelId == enums.CHAN_tell:
			if target == "":
				warn(enums.LC_Chat, "Messages on CHAN_tell must have a target!")
				return False

			recipient = None
			if target in self.players:
				# Dereference the players weak ref; this may or may not return None.
				recipient = self.players[target]()

			if recipient is None:
				trace(enums.LC_Chat, "Tell recipient %s not found" % target)
				player.client.onPlayerCommunication('', 0, enums.CHAN_feedback, "Player %s doesn't exist" % target)
				return False

			trace(enums.LC_Chat, "[%s to %s] %s" % (target, player.playerName, message))
			player.client.onTellSent(target, message)
			recipient.client.onPlayerCommunication(player.playerName, flags, enums.CHAN_tell, message)
			return True

		# Process the message normally and distribute it to every player on the channel
		channel.sendMessage(player.playerName, flags, message)
		return True

ChannelManager = ChatChannelManager()
