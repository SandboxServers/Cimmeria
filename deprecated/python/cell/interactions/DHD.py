import Atrea.enums
from cell.interactions.Interaction import Interaction
from common.Logger import trace
from common.defs.Def import DefMgr

class DHD(Interaction):
	def __init__(self, entity):
		super().__init__(entity)


	def onInteract(self, player, interactionSetMapId):
		"""
		Called when the player wants to interact with this entity
		@param player: SGWPlayer entity
		@param interactionSetMapId: ID of interaction map the player requested
		@return Was the interaction handled?
		"""
		# TODO: Check that the player is interacting with a DHD when an onDialGate rpc arrives
		trace(Atrea.enums.LC_Interact, '<%s> interacted with DHD' % player.getName())
		gates = DefMgr.getAll('stargate')
		gate = None
		for g in gates.values():
			if g.world.id == player.space.worldId:
				gate = g
				break

		if gate is None:
			player.onError("Cannot display DHD: No stargate found on this world")
			return True

		player.client.onDisplayDHD(gate.addressOrigin)
		return True
