import Atrea.enums
from common.Logger import warn, trace


class TradeProposal(object):
	def __init__(self, player):
		self.player = player
		self.version = 0
		self.lockState = Atrea.enums.ETRADELOCKSTATE_None
		self.naquadah = 0
		self.items = []


	def isValid(self):
		"""
		Checks if the trade proposal we've sent to the trade partner can still be fulfilled
		"""
		# Check if we have sufficient cash
		if self.player.inventory.naquadah < self.naquadah:
			return False

		# Check if all items in the proposal are valid
		for itemId, slotId in self.items:
			if self.player.inventory.getItem(itemId) is None:
				return False

		return True


	def update(self, proposal):
		"""
		Updates the trade proposal set by this player
		@param proposal: Local trade proposal
		"""
		if proposal['version'] != self.version + 1:
			warn(Atrea.enums.LC_Item, "<%s> Sent invalid proposal version; current is %d, got %d" %
									  (self.player.getName(), self.version, proposal['version']))
			return False

		items = []
		seenIds = []
		for itemId, slotId in [(item['instanceId'], item['slotId']) for item in proposal['items']]:
			item = self.player.inventory.getItem(itemId)
			if item is None:
				warn(Atrea.enums.LC_Item, "<%s> Sent invalid item in proposal: %d" % (self.player.getName(), itemId))
				return False

			# TODO: Do we need a separate canTrade() ?
			if not item.canSell():
				warn(Atrea.enums.LC_Item, "<%s> Sent unsellable item in proposal: %d" % (self.player.getName(), itemId))
				continue

			if itemId in seenIds:
				warn(Atrea.enums.LC_Item, "<%s> Sent item %d multiple times in proposal!" % (self.player.getName(), itemId))
				continue

			seenIds.append(itemId)
			items.append((itemId, slotId))

		self.version = proposal['version']
		self.naquadah = proposal['cash']
		self.items = items
		self.lockState = Atrea.enums.ETRADELOCKSTATE_None
		return True


	def removeItems(self):
		"""
		Removes all items in the trade proposal from the players inventory
		"""
		for itemId, slotId in self.items:
			item = self.player.inventory.getItem(itemId)
			removed = self.player.inventory.removeItem(itemId, item.quantity, True)
			assert removed


	def makeItemList(self):
		"""
		Returns the list of items
		"""
		items = {}
		for itemId, slotId in self.items:
			item = self.player.inventory.getItem(itemId)
			items[item.typeId] = items.get(item.typeId, 0) + item.quantity

		return items


	def toLocalProposal(self):
		"""
		Returns information the player should know about its local proposal
		"""
		return {
			'version': self.version,
			'items': [{'instanceId': itemId, 'slotId': slotId} for itemId, slotId in self.items],
			'cash': self.naquadah,
			'lockState': self.lockState
		}


	def toRemoteProposal(self):
		"""
		Returns information the player should know about the proposal of its trade partner
		"""
		items = []
		for itemId, slotId in self.items:
			item = self.player.inventory.getItem(itemId).toInvItem()
			item['slotID'] = slotId
			items.append(item)

		return {
			'version': self.version,
			'items': items,
			'cash': self.naquadah,
			'lockState': self.lockState
		}




class TradeTransaction(object):
	def __init__(self, player1, player2):
		self.proposals = {
			player1.entityId: TradeProposal(player1),
			player2.entityId: TradeProposal(player2)
		}


	def hasPlayer(self, entityId):
		"""
		Checks if the specified player is a participant in this transaction
		@param entityId: Entity ID of trade partner
		"""
		return entityId in self.proposals


	def getProposal(self, entityId):
		"""
		Retrieves the trade proposal of the specified player
		@param entityId: Entity ID of trade partner
		"""
		for partnerId, proposal in self.proposals.items():
			if partnerId == entityId:
				return proposal

		assert False


	def getPartnerProposal(self, entityId):
		"""
		Retrieves the trade proposal of the trade partner of specified player
		@param entityId: Entity ID of trade partner
		"""
		for partnerId, proposal in self.proposals.items():
			if partnerId != entityId:
				return proposal

		assert False


	def updateProposal(self, entityId, proposal):
		"""
		Updates the trade proposal set by the player
		@param entityId: Entity ID of player
		@param proposal: Local trade proposal
		"""
		updated = self.proposals[entityId].update(proposal)
		if updated:
			# Reset the remote lock state to None if we've changed our proposal
			remote = self.getPartnerProposal(entityId)
			remote.lockState = Atrea.enums.ETRADELOCKSTATE_None

			# Notify the players about the new proposal and lock state
			for proposal in self.proposals.values():
				proposal.player.onTradeProposalUpdated()

		return updated


	def updateLockState(self, entityId, localVersionId, remoteVersionId, lockState):
		"""
		Update the state of the trade proposal lock
		@param entityId: Entity ID of trade partner
		@param localVersionId: Last proposal version sent by us
		@param remoteVersionId: Last proposal we've seen from the trade partner
		@param lockState: Trade lock state (one of the ETRADELOCKSTATE_ constants)
		"""
		proposal = self.getProposal(entityId)
		partner = self.getPartnerProposal(entityId)

		if lockState < Atrea.enums.ETRADELOCKSTATE_None or lockState > Atrea.enums.ETRADELOCKSTATE_LockedAndConfirmed:
			warn(Atrea.enums.LC_Item, "<%s> Sent invalid trade lock state %d" %
									  (proposal.player.getName(), lockState))
			return False

		if localVersionId != proposal.version:
			warn(Atrea.enums.LC_Item, "<%s> Sent invalid trade lock version %d (local is %d)" %
									  (proposal.player.getName(), localVersionId, proposal.version))
			return False

		# Prevent the player from changing state to Locked if it hasn't seen the
		# latest proposal version from the trade partner yet
		if lockState != Atrea.enums.ETRADELOCKSTATE_None and partner.version != remoteVersionId:
			lockState = Atrea.enums.ETRADELOCKSTATE_None

		# If one of the players released the trade lock we'll release the lock on the partner as well
		if lockState == Atrea.enums.ETRADELOCKSTATE_None and \
			proposal.lockState != Atrea.enums.ETRADELOCKSTATE_Locked:
			lockState = Atrea.enums.ETRADELOCKSTATE_None
			partner.lockState = Atrea.enums.ETRADELOCKSTATE_None

		# Update lock state and notify both parties
		proposal.lockState = lockState
		proposal.player.onTradeProposalUpdated()
		partner.player.onTradeProposalUpdated()

		# Execute the transaction if the trade was confirmed from both sides
		if proposal.lockState == Atrea.enums.ETRADELOCKSTATE_LockedAndConfirmed and \
			partner.lockState == Atrea.enums.ETRADELOCKSTATE_LockedAndConfirmed:
			self.confirm()
		return True


	def cancel(self):
		p1, p2 = list(self.proposals.values())
		p1.player.onTradeCompleted(Atrea.enums.Completed)
		p2.player.onTradeCompleted(Atrea.enums.Completed)
		return True


	def confirm(self):
		"""
		Performs the transaction specified in the local and remote proposals
		and closes the trade session
		"""
		p1, p2 = list(self.proposals.values())
		if not p1.isValid():
			trace(Atrea.enums.LC_Item, "<%s> Trade failed: proposal not valid" % p1.player.getName())
			p1.player.onTradeCompleted(Atrea.enums.NoLocalCash)
			p2.player.onTradeCompleted(Atrea.enums.NoRemoteCash)
			return False

		if not p2.isValid():
			trace(Atrea.enums.LC_Item, "<%s> Trade failed: proposal not valid" % p2.player.getName())
			p1.player.onTradeCompleted(Atrea.enums.NoRemoteCash)
			p2.player.onTradeCompleted(Atrea.enums.NoLocalCash)
			return False

		p1Items = p1.makeItemList()
		p1Bags = p1.player.inventory.categorizeItemsByBag(p1Items)
		if not p2.player.inventory.canAddItems(p1Bags):
			trace(Atrea.enums.LC_Item, "<%s> Trade failed: not enough space" % p2.player.getName())
			p1.player.onTradeCompleted(Atrea.enums.NoRemoteSpace)
			p2.player.onTradeCompleted(Atrea.enums.NoLocalSpace)
			return False

		p2Items = p2.makeItemList()
		p2Bags = p2.player.inventory.categorizeItemsByBag(p2Items)
		if not p1.player.inventory.canAddItems(p2Bags):
			trace(Atrea.enums.LC_Item, "<%s> Trade failed: not enough space" % p1.player.getName())
			p1.player.onTradeCompleted(Atrea.enums.NoLocalSpace)
			p2.player.onTradeCompleted(Atrea.enums.NoRemoteSpace)
			return False

		trace(Atrea.enums.LC_Item, "<%s> --> %s: Trade successful" % (p1.player.getName(), p2.player.getName()))
		p1.removeItems()
		p2.removeItems()
		p1.player.inventory.addItems(p2Items)
		p2.player.inventory.addItems(p1Items)

		p1.player.onTradeCompleted(Atrea.enums.Completed)
		p2.player.onTradeCompleted(Atrea.enums.Completed)
		return True
