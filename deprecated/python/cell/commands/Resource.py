###################################################################################
#                              Resource commands
###################################################################################
from random import random
import time
import Atrea
from common.defs.Def import DefMgr


def searchItem(player, target, name, name2 = ''):
	"""
	Search for item designs by name
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  name: str
	@param name: Item name to search for
	@type  name2: str
	@param name2: Item name to search for
	"""
	find = name.lower() + (' ' + name2.lower() if name2 else '')
	for item in DefMgr.getAll('item').values():
		if find in item.name.lower():
			player.feedback('    %d: %s' % (item.id, item.name))


def searchMission(player, target, name, name2 = ''):
	"""
	Search for mission designs by name
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  name: str
	@param name: Name to search for
	@type  name2: str
	@param name2: Name to search for
	"""
	find = name.lower() + (' ' + name2.lower() if name2 else '')
	for mission in DefMgr.getAll('mission').values():
		if find in mission.missionDefn.lower():
			player.feedback('    %d: %s' % (mission.missionId, mission.missionDefn))


def searchTemplate(player, target, name, name2 = ''):
	"""
	Search for entity template designs by name
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  name: str
	@param name: Name to search for
	@type  name2: str
	@param name2: Name to search for
	"""
	find = name.lower() + (' ' + name2.lower() if name2 else '')
	for tpl in DefMgr.getAll('template').values():
		if find in tpl.templateName.lower():
			player.feedback('    %d: %s' % (tpl.id, tpl.templateName))


def reloadResources(player, target, category = None):
	"""
	Reloads all server resources on the CellApp
	@param player: SGWPlayer
	@param target: Targeted entity
	@type  category: str
	@param category: Category to reload
	"""
	player.feedback("Reloading resources on CellApp ...")
	start = time.clock()
	if category is None:
		DefMgr.load()
	else:
		if not DefMgr.hasCategory(category):
			player.feedback("Invalid resource category <%s>" % category)
			return

		DefMgr.loadCategory(category)
	player.feedback("Reloaded resources in %f sec" % (time.clock() - start))


def respawnAll(player, target):
	"""
	Reloads entity templates and respawns all entities
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.feedback("Respawning entities ...")
	start = time.clock()
	DefMgr.loadCategory('template')
	player.space.removeSpawns()
	player.space.loadSpawns()
	player.feedback("Respawned entities in %f sec" % (time.clock() - start))


def autosaveSpawn(player, target, autosave):
	"""
	Enables automatic saving of entites spawned with .spawn
	@param player: SGWPlayer
	@param target: Targeted entity
	@type autosave: bool
	@param autosave: Enable autosaving?
	"""
	player.autosaveSpawns = autosave
	if autosave:
		player.feedback('Auto-saving spawns enabled')
	else:
		player.feedback('Auto-saving spawns disabled')


def spawnEntity(player, target, templateId):
	"""
	Spawns a new entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type templateId: int
	@param templateId: Entity template to spawn
	"""
	template = DefMgr.get('template', templateId)
	if template is None:
		player.feedback('Cannot spawn entity: invalid templateId %d' % templateId)
		return

	player.feedback('Spawning entity of type <%s>' % template.templateName)
	player.space.createEntity(template, player.position, player.rotation)
	if player.autosaveSpawns:
		saveSpawn(player, target)


def spawnRandomEntity(player, target, templateId, xRange, zRange, count = 1):
	"""
	Spawns a new entity
	@param player: SGWPlayer
	@param target: Targeted entity
	@type templateId: int
	@param templateId: Entity template to spawn
	@type xRange: float
	@param xRange: Max X distance from player
	@type zRange: float
	@param zRange: Max Y distance from player
	@type count: int
	@param count: Amount of entities to spawn
	"""
	template = DefMgr.get('template', templateId)
	if template is None:
		player.feedback('Cannot spawn entity: invalid templateId %d' % templateId)
		return

	player.feedback('Spawning %d entities of type <%s>' % (count, template.templateName))
	refpos = player.position
	for i in range(0, count):
		xrand = random() * 2 * xRange - xRange
		zrand = random() * 2 * zRange - zRange
		pos = Atrea.Vector3(refpos.x + xrand, refpos.y, refpos.z + zrand)
		player.space.createEntity(template, pos, player.rotation)


def despawnEntity(player, target):
	"""
	Despawns an entity
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	player.feedback('Despawning entity <%s>' % target.getName())
	Atrea.destroyCellEntity(target.entityId)


def saveSpawn(player, target):
	"""
	Save the new position of the entity to the spawn table
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if target.template is None:
		player.feedback('Cannot save spawn for <%s>; entity doesnt have a template' % target.getName())
		return

	pos = target.position
	rot = target.rotation.y
	world = target.space.worldId
	template = target.template.id
	tag = repr(target.tag) if target.tag is not None else 'null'
	if target.spawnId is None:
		spawn = Atrea.dbQuery('''
			insert into resources.spawnlist
			       (x, y, z, heading, world_id, template_id, tag)
			values (%f, %f, %f, %f, %d, %d, %s)
			returning spawn_id''' %
		    (pos.x, pos.y, pos.z, rot, world, template, tag)
		)
		target.spawnId = int(spawn[0]['spawn_id'])
		player.feedback('Added spawn of <%s> with id %d' % (target.getName(), target.spawnId))
	else:
		Atrea.dbQuery('''
			update resources.spawnlist
			set   x = %f, y = %f, z = %f, heading = %f,
			      world_id = %d, template_id = %d, tag = %s
			where spawn_id = %d''' %
		    (pos.x, pos.y, pos.z, rot, world, template, tag, target.spawnId)
		)
		player.feedback('Updated spawn of <%s> with id %d' % (target.getName(), target.spawnId))



def deleteSpawn(player, target):
	"""
	Deletes the entity from the spawn table
	@param player: SGWPlayer
	@param target: Targeted entity
	"""
	if target.template is None:
		player.feedback('Cannot delete spawn for <%s>; entity doesnt have a template' % target.getName())
		return

	if target.spawnId is None:
		player.feedback('Cannot delete spawn for <%s>; entity is not spawned from a spawnlist' % target.getName())
		return

	Atrea.dbQuery('''
		delete from resources.spawnlist
		where spawn_id = %d''' % target.spawnId
	)
	player.feedback('Deleted spawn of <%s> with id %d' % (target.getName(), target.spawnId))
	target.spawnId = None
