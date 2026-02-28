import Atrea.enums
from cell.interactions.Interaction import Interaction
from common.Logger import warn, trace
from common.defs.Def import DefMgr


class AbilityTrainer(Interaction):
	def __init__(self, entity):
		super().__init__(entity)


	def onInteract(self, player, interactionSetMapId):
		"""
		Called when the player wants to interact with this entity
		@param player: SGWPlayer entity
		@param interactionSetMapId: ID of interaction map the player requested
		@return Was the interaction handled?
		"""
		if not self.entity().isDead():
			trace(Atrea.enums.LC_Interact, '<%s> interacted with AbilityTrainer' % player.getName())
			player.setTrainer(self)
			self.sendAbilityList(player)


	def sendAbilityList(self, player):
		"""
		Send the trainer's trainable ability list to the player
		@param player: SGWPlayer
		"""
		abilityList = []
		if self.entity().template.trainerAbilities and \
			player.archetype in self.entity().template.trainerAbilities.abilities:
			trainable = self.entity().template.trainerAbilities.abilities[player.archetype]
			for ability in trainable:
				item = {
					'abilityID': ability.id,
					'trainable': 1 if self.canTrainAbility(player, ability) else 0
				}
				abilityList.append(item)

		# TODO: Calculate respec cost
		player.client.onTrainerOpen(self.entity().entityId, abilityList, 1000)


	def canTrainAbility(self, player, ability):
		"""
		Checks if the player meets all prerequisites for training this ability

		@param player: SGWPlayer object
		@param ability: Ability to train
		@return: True if the player can buy the item, False otherwise
		"""
		archetype = DefMgr.get('archetype', player.archetype)
		treeInfo = archetype.abilities[ability.id]
		if player.level < treeInfo.level:
			trace(Atrea.enums.LC_Interact, '<%s> level %d required for ability %d' %
										   (player.getName(), treeInfo.level, ability.id))
			return False

		for prereq in treeInfo.prerequisites:
			if not player.abilities.hasAbility(prereq.id):
				trace(Atrea.enums.LC_Interact, '<%s> ability %d required for ability %d' %
											   (player.getName(), prereq.id, ability.id))
				return False

		return not player.abilities.hasAbility(ability.id)


	def isPrerequisiteAbility(self, player, abilityId):
		"""
		Checks if the specified ability is a prerequisite of another ability

		@param player: Player to check
		@param abilityId: Ability to check
		@return: True if the ability is a prerequisite, False otherwise
		"""
		archetype = DefMgr.get('archetype', player.archetype)
		trainable = self.entity().template.trainerAbilities.abilities[player.archetype]

		for ability in archetype.abilities:
			# Check if the requested ability is a prerequisite, and that we can train the dependent ability
			if abilityId in (ability.id for ability in ability.prerequisites) and \
				ability.id in (ability.id for ability in trainable):
				return True

		return False


	def onTrainAbility(self, player, abilityId):
		"""
		Indicates that the player wants to train an ability
		@param player: SGWPlayer object
		@param abilityId: Ability to train
		"""
		# Check if the requested ability is a valid ability
		definition = DefMgr.get('ability', abilityId)
		if definition is None:
			warn(Atrea.enums.LC_Interact, '<%s> tried to train nonexistent ability %d' % (player.getName(), abilityId))
			return False

		# Check that the trainer has abilities for the requested archetype
		archetypes = self.entity().template.trainerAbilities.abilities
		if player.archetype not in archetypes:
			warn(Atrea.enums.LC_Interact, '<%s> cannot train ability: no abilities for that archetype!' % player.getName())
			return False

		# Check that the trainer is training that ability
		abilities = archetypes[player.archetype]
		if not any(a.id == abilityId for a in abilities):
			warn(Atrea.enums.LC_Interact, '<%s> tried to train unlisted ability %d' % (player.getName(), abilityId))
			return False

		if not self.canTrainAbility(player, definition):
			warn(Atrea.enums.LC_Interact, '<%s> cannot train ability %d' % (player.getName(), abilityId))
			player.onError("Cannot train ability")
			return

		if player.trainingPoints < 1:
			warn(Atrea.enums.LC_Interact, '<%s> doesnt have enough training points' % player.getName())
			player.onError("Not enough training points")
			return

		player.feedback('You have learned the %s ability!' % definition.name)
		player.consumeTrainingPoints(1)
		player.addAbility(abilityId)

		# We should resend the ability list if the player ran out of TP or learned a prerequisite ability
		if player.trainingPoints == 0 or self.isPrerequisiteAbility(player, abilityId):
			self.sendAbilityList(player)


	def onRespec(self, player):
		"""
		Indicates that the player wants to reset all of its abilities
		@param player: SGWPlayer object
		"""
		# TODO: What is the cost of respec?
		# TODO #2: We should store a list of items that were trained (can be unlearned)
		# and that were given by missions/etc (cannot be unlearned)
		player.onError('Not implemented yet!')

