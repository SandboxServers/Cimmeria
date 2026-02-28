import weakref

class Interaction(object):
	def __init__(self, entity):
		self.entity = weakref.ref(entity)


	def onInteract(self, player, interactionSetMapId):
		"""
		Called when the player wants to interact with this entity
		@param player: SGWPlayer entity
		@param interactionSetMapId: ID of interaction map the player requested
		@return Was the interaction handled?
		"""
		return False