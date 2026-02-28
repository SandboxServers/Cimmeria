import weakref
import Atrea
import Atrea.enums
from common.Logger import trace, warn
from common.defs.RingTransporterRegion import RingTransporterRegion
from common.defs.Def import DefMgr

class RingTransporter(object):
	STATE_IDLE = 0
	STATE_SEND_WAIT = 1
	STATE_SEND_WARMUP = 2
	STATE_REMOTE_LOAD_WAIT = 3
	STATE_REMOTE_WARMUP = 4
	STATE_COOLDOWN = 5
	STATE_RECV_WAIT = 6
	STATE_RECV_WARMUP = 7

	state = STATE_IDLE
	remoteRegionId = None
	sendPlayers = None
	playersLoaded = None
	numRemotePlayers = 0

	def __init__(self, region : RingTransporterRegion, space):
		super().__init__()
		self.players = {}
		self.regionId = region.id
		self.region = region
		self.pos = Atrea.Vector3(region.x, region.y, region.z)
		self.worldName = region.world.world
		self.destinationIds = region.destinationIds
		self.genericRegion = space.regions.getPointSet(self.region.pointSetId)
		self.genericRegion.callback = self.regionTriggered
		region.instance = self


	def regionTriggered(self, entering : bool, position, entity):
		if entering:
			self.players[entity.entityId] = weakref.ref(entity)
		else:
			if entity.entityId in self.players:
				del self.players[entity.entityId]
			else:
				warn(Atrea.enums.LC_Uncategorized, 'Cannot remove player <%s> from region: Not in region!' % entity.getName())

		if self.state == self.STATE_SEND_WAIT:
			self.__startSending()
		else:
			# TODO: What to do in other states?
			pass


	@staticmethod
	def makeRingRegion(region):
		"""
		Transforms a ring resource to a RegionInfo message
		@param region: Ring region to transform
		@return: RegionInfo structure
		"""
		return {
			'RegionId': region.id,
			'DisplayName': region.displayNameId,
			'WorldId': region.world.id,
			'Position': Atrea.Vector3(region.x, region.y, region.z)
		}


	def interact(self, player):
		"""
		Starts a new ring transport interaction.
		@param player: Player interacting with the ring
		"""
		player.ringSourceId = self.regionId
		regionInfo = self.__class__.makeRingRegion(self.region)

		destinations = []
		for regionId in self.destinationIds:
			destination = DefMgr.get('ring_transporter_region', regionId)
			if destination:
				destinations.append(self.__class__.makeRingRegion(destination))
			else:
				warn(Atrea.enums.LC_Uncategorized, 'Invalid destination region id %d' % regionId)

		player.client.onRingTransporterList(regionInfo, destinations)


	def selectDestination(self, player, regionId : int):
		"""
		Selects the destination region this ring transport will move players to.
		@param player: Player interacting with the ring
		@param regionId: Destination region ID
		@return: True if selection is successful, False if region is invalid or ring transport is busy
		"""
		if regionId not in self.destinationIds:
			warn(Atrea.enums.LC_Uncategorized, 'Invalid destination ring %d selected' % regionId)
			return False

		if regionId == self.regionId:
			warn(Atrea.enums.LC_Uncategorized, 'Source and destination region cannot be the same!')
			return False

		if self.state != self.STATE_IDLE:
			warn(Atrea.enums.LC_Uncategorized, 'Cannot select ring destination: in state %d' % self.state)
			return False

		trace(Atrea.enums.LC_Uncategorized, '<%s> selectDestination() --> STATE_SEND_WAIT' % self.region.tag)
		self.state = self.STATE_SEND_WAIT
		self.remoteRegionId = regionId
		rgnInfo = DefMgr.require('ring_transporter_region', regionId)
		if rgnInfo.world.world == self.worldName:
			self.remoteRegion = player.space.transporters.get(regionId)
			assert(self.remoteRegion is not None)
		else:
			self.remoteRegion = rgnInfo.instance
		player.ringSourceId = None
		self.remoteRegion.remoteWait(self)
		if len(self.players):
			self.__startSending()
		return True


	def playerLoaded(self, player):
		"""
		Called when the player being teleported has finished loading the map.
		@param player: Player being loaded
		"""
		player.destinationRingId = None
		self.playersLoaded.append(weakref.ref(player))
		if len(self.playersLoaded) == self.numRemotePlayers:
			self.__allPlayersLoaded()


	def remoteWait(self, initiator):
		"""
		Called by the initiator ring to start a transport wait sequence
		@param initiator: Initiator ring
		"""
		assert(self.state == self.STATE_IDLE)
		trace(Atrea.enums.LC_Uncategorized, '<%s> remoteWait() --> STATE_RECV_WAIT' % self.region.tag)
		self.state = self.STATE_RECV_WAIT
		self.remoteRegionId = initiator.regionId
		self.remoteRegion = initiator


	def remoteSend(self):
		"""
		Called by the initiator ring to start the transport sequence
		"""
		assert(self.state == self.STATE_RECV_WAIT)
		trace(Atrea.enums.LC_Uncategorized, '<%s> remoteSend() --> STATE_RECV_WARMUP' % self.region.tag)
		self.state = self.STATE_RECV_WARMUP
		self.__beginTransport()


	def remoteCountUpdate(self, num):
		"""
		Called by the remote ring to update the amount of players being transported
		"""
		self.numRemotePlayers = num
		trace(Atrea.enums.LC_Uncategorized, '<%s> remoteCountUpdate(): %d' % (self.region.tag, num))
		if self.numRemotePlayers == 0:
			self.__allPlayersLoaded()


	def remoteTransport(self):
		"""
		Called by the initiator ring to teleport all players
		"""
		assert(self.state == self.STATE_RECV_WARMUP)
		trace(Atrea.enums.LC_Uncategorized, '<%s> remoteTransport() --> STATE_REMOTE_LOAD_WAIT' % self.region.tag)
		self.state = self.STATE_REMOTE_LOAD_WAIT
		self.__doTransport()
		self.remoteRegion.remoteCountUpdate(len(self.sendPlayers))


	def __startSending(self):
		assert(self.state == self.STATE_SEND_WAIT)
		trace(Atrea.enums.LC_Uncategorized, '<%s> startSending() --> STATE_SEND_WARMUP' % self.region.tag)
		self.state = self.STATE_SEND_WARMUP
		self.__beginTransport()
		self.remoteRegion.remoteSend()
		Atrea.addTimer(Atrea.getGameTime() + 3.5, lambda: self.__hideTimerExpired())
		Atrea.addTimer(Atrea.getGameTime() + 4.0, lambda: self.__warmupTimerExpired())


	def __beginTransport(self):
		"""
		Starts ring transport sequence, locks player movement
		"""
		self.sendPlayers = self.players.copy()
		self.players.clear()
		sequence = self.region.eventSet.getSequence(Atrea.enums.Region_Teleport_Out)
		# FIXME: Only play the transporter sequence on the first player for now
		# This is because playing it for each player will mess up the ring matinee
		if sequence is not None and len(self.sendPlayers):
			player = list(self.sendPlayers.values())[0]()
			player.playSequence(sequence.seqId, player.entityId, viewType = Atrea.enums.KISMET_VIEW_EventInvoker)

		for player in self.sendPlayers.values():
			trace(Atrea.enums.LC_Uncategorized, '<%s> Starting transport for player %s' % (self.region.tag, player().getName()))
			player().onTeleportOut(self.regionId, self.remoteRegionId)
			player().setStateFlag(Atrea.enums.BSF_MovementLock)


	def __doTransport(self):
		"""
		Starts ring transport sequence, locks player movement
		"""
		# Prepare the arrived player list
		self.playersLoaded = []
		# Teleport players to their new location
		d = self.remoteRegion
		for player in self.sendPlayers.values():
			player().destinationRingId = self.remoteRegionId
			player().teleportTo(d.pos, worldName = d.worldName)


	def __allPlayersLoaded(self):
		"""
		Called when every player that was teleported from the remote side
		was loaded on the local side
		"""
		assert(self.state == self.STATE_REMOTE_LOAD_WAIT)
		trace(Atrea.enums.LC_Uncategorized, '<%s> allPlayersLoaded() --> STATE_REMOTE_WARMUP' % self.region.tag)
		self.state = self.STATE_REMOTE_WARMUP

		if len(self.playersLoaded):
			# Play ring transport "teleport in" sequence
			sequence = self.region.eventSet.getSequence(Atrea.enums.Region_Teleport_In)
			if sequence is not None:
				# FIXME: Only play the transporter sequence on the first player for now
				# This is because playing it for each player will mess up the ring matinee
				player = self.playersLoaded[0]()
				player.playSequence(sequence.seqId, player.entityId, viewType = Atrea.enums.KISMET_VIEW_EventInvoker)

			Atrea.addTimer(Atrea.getGameTime() + 3.0, lambda: self.__remoteWarmupTimerExpired())
		else:
			self.__remoteWarmupTimerExpired()


	def __hideTimerExpired(self):
		"""
		Turns players invisible while the teleport effect is still running
		"""
		assert(self.state == self.STATE_SEND_WARMUP)
		for player in self.sendPlayers.values():
			player().setVisible(False)


	def __warmupTimerExpired(self):
		"""
		Called when the warmup timer of the ring transporter expired
		(some time after the TeleportIn sequence finished playing and players are now invisible)
		"""
		assert(self.state == self.STATE_SEND_WARMUP)
		trace(Atrea.enums.LC_Uncategorized, '<%s> warmupTimerExpired() --> STATE_REMOTE_LOAD_WAIT' % self.region.tag)
		self.state = self.STATE_REMOTE_LOAD_WAIT
		self.__doTransport()
		self.remoteRegion.remoteCountUpdate(len(self.sendPlayers))
		self.remoteRegion.remoteTransport()



	def __remoteWarmupTimerExpired(self):
		"""
		Called when the remote warmup timer of the ring transporter expired
		(in the middle of the TeleportIn sequence when players should be made visible)
		"""
		assert(self.state == self.STATE_REMOTE_WARMUP)
		trace(Atrea.enums.LC_Uncategorized, '<%s> remoteWarmupTimerExpired() --> STATE_COOLDOWN' % self.region.tag)
		self.state = self.STATE_COOLDOWN
		if len(self.playersLoaded):
			for player in self.playersLoaded:
				player().setVisible(True)

			Atrea.addTimer(Atrea.getGameTime() + 2.5, lambda: self.__cooldownTimerExpired())
		else:
			self.__cooldownTimerExpired()


	def __cooldownTimerExpired(self):
		"""
		Called when the cooldown timer of the ring transporter expired
		(after the TeleportIn sequence when players should be able to move)
		"""
		assert(self.state == self.STATE_COOLDOWN)
		trace(Atrea.enums.LC_Uncategorized, '<%s> cooldownTimerExpired() --> STATE_IDLE' % self.region.tag)
		self.state = self.STATE_IDLE
		for player in self.playersLoaded:
			player().unsetStateFlag(Atrea.enums.BSF_MovementLock)
			player().onTeleportIn(self.regionId)



class RingTransporterManager(object):
	def __init__(self):
		self.regions = {}

	def load(self, space, worldId):
		regions = DefMgr.getAll('ring_transporter_region')
		for region in regions.values():
			if region.world.id == worldId:
				self.regions[region.id] = RingTransporter(region, space)


	def get(self, id):
		return self.regions.get(id)
