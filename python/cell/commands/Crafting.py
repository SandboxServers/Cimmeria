###################################################################################
#                              Crafting commands
###################################################################################

from common.defs.Def import DefMgr


def giveAppliedSciencePoints(player, target, amount):
	"""
	Gives applied science points to a player
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  amount: int
	@param amount: Amount of points to give
	"""
	target.crafting.giveAppliedSciencePoints(amount)
	player.feedback('Added %d applied science points to <%s>' % (amount, target.getName()))


def setRacialParadigmLevel(player, target, racialParadigmId, level):
	"""
	Changes the players expertise in a racial paradigm
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  racialParadigmId: int
	@param racialParadigmId: Racial paradigm to update
	@type  level: int
	@param level: New racial paradigm level
	"""
	paradigm = DefMgr.get('racial_paradigm', racialParadigmId)
	if paradigm is not None:
		target.crafting.updateRacialParadigmLevel(racialParadigmId, level)
		player.feedback('Updated racial paradigm <%s> level to %d' % (paradigm.name, level))
	else:
		player.feedback('Invalid racial paradigm ID!')


def learnDiscipline(player, target, disciplineId, expertise = 1):
	"""
	Learns a new discipline
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  disciplineId: int
	@param disciplineId: Discipline to learn
	@type  expertise: int
	@param expertise: Expertise level
	"""
	discipline = DefMgr.get('discipline', disciplineId)
	if discipline is not None:
		if disciplineId in target.crafting.disciplines:
			target.crafting.gainExpertise(disciplineId, expertise)
			player.feedback('Updated discipline <%s> expertise to %d' % (discipline.name, expertise))
		elif target.crafting.learnDiscipline(disciplineId, expertise):
			player.feedback('learned discipline <%s> with expertise level %d' % (discipline.name, expertise))
		else:
			player.feedback('Failed to learn discipline <%s>' % discipline.name)
	else:
		player.feedback('Invalid discipline ID!')


def forgetDiscipline(player, target, disciplineId):
	"""
	Learns a new discipline
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  disciplineId: int
	@param disciplineId: Discipline to learn
	"""
	discipline = DefMgr.get('discipline', disciplineId)
	if discipline is not None:
		if target.crafting.forgetDiscipline(disciplineId):
			player.feedback('Removed discipline <%s>' % discipline.name)
		else:
			player.feedback('Failed to forget discipline <%s>' % discipline.name)
	else:
		player.feedback('Invalid discipline ID!')


def debugAllCraft(player, target):
	"""
	Gives the player every blueprint, max racial paradigm and discipline levels
	@param player: SGWPlayer
	@param target: Targeted player
	"""
	craftingInfo = {
		'items': [],
		'entities': [target.entityId]
	}
	craftingOptions = {
		'crafting': craftingInfo,
		'research': craftingInfo,
		'reverseEngineering': craftingInfo,
		'alloying': craftingInfo
	}
	target.client.onUpdateCraftingOptions(craftingOptions)

	target.crafting.giveBlueprints([b.id for b in DefMgr.getAll('blueprint').values()])
	player.feedback('Added all blueprints to <%s>' % target.getName())

	for paradigm in DefMgr.getAll('racial_paradigm'):
		target.crafting.updateRacialParadigmLevel(paradigm, 7)
	player.feedback('Set all racial paradigm levels to 7 for <%s>' % target.getName())

	for discipline in DefMgr.getAll('discipline'):
		target.crafting.learnDiscipline(discipline, 50)
	player.feedback('Added all disciplines to <%s>' % target.getName())
