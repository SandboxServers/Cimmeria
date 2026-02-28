###################################################################################
#                            Client RPC test commands
###################################################################################

import Atrea.enums
from cell.Minigame import Minigame
from common.Config import Config
from common.defs.Def import DefMgr


def displayDHD(player, target, origin = None):
	"""
	Plays a kismet sequence with the targeted entity as a source and target.
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  origin: int
	@param origin: Override address origin
	"""
	if origin is None:
		gates = DefMgr.getAll('stargate')
		origin = None
		for g in gates.values():
			if g.world.id == player.space.worldId:
				origin = g.addressOrigin
				break

		if origin is None:
			player.onError("Cannot display DHD: No stargate found on this world")
			return True

	target.client.onDisplayDHD(origin)


def sequence(player, target, sequenceId, viewType = Atrea.enums.KISMET_VIEW_EventInvoker):
	"""
	Plays a kismet sequence with the targeted entity as a source and target.
	@param player: SGWPlayer
	@param target: Targeted entity
	@type sequenceId: int
	@param sequenceId: ID of sequence to play
	@type viewType: int
	@param viewType: Kismet view type
	"""
	player.feedback('Playing sequence %d (%d) on <%s>' % (sequenceId, viewType, target.getName()))
	target.playSequence(sequenceId, target.entityId, viewType = viewType)


def sequenceTo(player, target, sequenceId, viewType = Atrea.enums.KISMET_VIEW_EventInvoker):
	"""
	Plays a kismet sequence with the player as a source and the targeted entity as a target.
	@param player: SGWPlayer
	@param target: Targeted entity
	@type sequenceId: int
	@param sequenceId: ID of sequence to play
	@type viewType: int
	@param viewType: Kismet view type
	"""
	if target is None:
		target = player
	player.feedback('Playing sequence %d (%d) from <%s> to <%s>' % (sequenceId, viewType, player.getName(), target.getName()))
	player.playSequence(sequenceId, target.entityId, viewType = viewType)


def sequenceFrom(player, target, sequenceId, viewType = Atrea.enums.KISMET_VIEW_EventInvoker):
	"""
	Plays a kismet sequence with the player as a target and the targeted entity as a source.
	@param player: SGWPlayer
	@param target: Targeted entity
	@type sequenceId: int
	@param sequenceId: ID of sequence to play
	@type viewType: int
	@param viewType: Kismet view type
	"""
	player.feedback('Playing sequence %d (%d) from <%s> to <%s>' % (sequenceId, viewType, target.getName(), player.getName()))
	target.playSequence(sequenceId, player.entityId, viewType = viewType)


def updateTimer(player, target, id, type, totalTime = 1, secondaryId = 0):
	"""
	Starts a timer on the client
	@param player: SGWPlayer
	@param target: Targeted entity
	@type id: int
	@param id: ID of object the timer belongs to
	@type type: int
	@param type: Type of object (see ETimerUpdateType; ability, item, effect, dialog, ...)
	@type secondaryId: int
	@param secondaryId: Instance ID of ability/effect/...
	@type totalTime: float
	@param totalTime: Total duration of the cooldown or effect
	"""
	player.feedback('Starting timer %d (type %d) on <%s>' % (id, type, target.getName()))
	player.client.onTimerUpdate(id, type, target.entityId, secondaryId, totalTime, Atrea.getGameTime() + totalTime)


def timeOfDay(player, target, time, wind, weather):
	"""
	Starts a timer on the client
	@param player: SGWPlayer
	@param target: Targeted entity
	@type time: float
	@param time: Time of day
	@type wind: float
	@param wind: Wind speed
	@type weather: int
	@param weather: Weather type (rain, thunder, ...)
	"""
	player.feedback('Setting time of day to %f, %f, %d' % (time, wind, weather))
	player.client.onTimeofDay(time, wind, weather)


def mapInfo(player, target, sysId, keyId, lifetime, delete = False, sysTypeId = 0):
	"""
	Starts a timer on the client
	@param player: SGWPlayer
	@param target: Targeted entity
	@type sysId: int
	@param sysId: ???
	@type keyId: int
	@param keyId: ???
	@type lifetime: int
	@param lifetime: Instance ID of ability/effect/...
	@type delete: bool
	@param delete: Delete map info?
	@type sysTypeId: int
	@param sysTypeId: Type Id
	"""
	player.feedback('Sending onMapInfo(SysTypeID=%d, SysID=%d, KeyID=%d, Lifetime=%d, Delete=%s' %
					(sysTypeId, sysId, keyId, lifetime, str(delete)))
	target.client.onMapInfo(sysTypeId, sysId, keyId, player.space.worldId, player.position, lifetime, 1 if delete else 0)


def playerCommunication(player, target, message, channel = 'say'):
	"""
	Starts a timer on the client
	@param player: SGWPlayer
	@param target: Speaker entity
	@type message: str
	@param message: Text to speak
	@type channel: int
	@param channel: Channel to speak on
	"""
	channels = {
		'say': 0,
		'emote': 1,
		'yell': 2,
		'team': 3,
		'squad': 4,
		'command': 5,
		'officer': 6,
		'server': 8,
		'feedback': 9,
		'tell': 10,
		'splash': 11,
		'chat': 12
	}

	if channel not in channels:
		player.feedback('Invalid channel name: %s' % channel)
		return

	player.client.onPlayerCommunication(target.getName(), 0, channels[channel], message)


def startMinigame(player, target, gameId, difficulty = 1, techCompetency = 1):
	"""
	Starts a timer on the client
	@param player: SGWPlayer
	@param target: Minigame player entity
	@type gameId: int
	@param gameId: Minigame ID to launch
	@type difficulty: int
	@param difficulty: Game difficulty (1-5)
	@type techCompetency: int
	@param techCompetency: Players tech competency (1-55)
	"""
	if gameId not in Config.MINIGAME_NAMES:
		player.feedback("Unknown minigame id: %d" % gameId)
		return

	player.feedback('Starting minigame %s ...' % Config.MINIGAME_NAMES[gameId])
	request = Minigame("Debug Game", difficulty, gameId, techCompetency, 0x7fff, lambda game, result: player.feedback('Minigame result: %d' % result))
	request.play(target)


def openDialog(player, target, dialogId):
	"""
	Opens a dialog with the targeted entity
	@param player: SGWPlayer
	@param target: Target entity
	@type dialogId: int
	@param dialogId: Dialog ID to open
	"""
	player.displayDialog(target, dialogId)


def clientChallenge(player, target, challenge, type, object, id1, id2):
	"""
	Opens a dialog with the targeted entity
	@param player: SGWPlayer
	@param target: Target entity
	@type challenge: int
	@param challenge:
	@type type: int
	@param type:
	@type object: str
	@param object:
	@type id1: int
	@param id1:
	@type id2: int
	@param id2:
	"""
	player.client.onClientChallenge(challenge, type, object, id1, id2)


