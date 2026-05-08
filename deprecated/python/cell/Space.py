import copy
from importlib import import_module
import Atrea
import Atrea.enums
from cell.GenericRegion import GenericRegionManager
from cell.RingTransporter import RingTransporterManager
from cell.SGWPlayer import SGWPlayer
from common import Constants
from common.Event import EventParticipant
from common.Logger import trace
from common.defs.Def import DefMgr


class Space(EventParticipant):
	spaceId = 0
	worldId = 0
	worldName = ''
	instanced = False
	regions = None

	def __init__(self, spaceId, worldId, worldName, instanced):
		super().__init__()
		self.spaceId = spaceId
		self.worldId = worldId
		self.world = DefMgr.require('world_info', worldId)
		self.worldName = worldName
		self.instanced = instanced
		self.regions = GenericRegionManager()
		self.regions.load(worldId)
		self.transporters = RingTransporterManager()
		self.transporters.load(self, worldId)

	def playerEntered(self, player):
		self.regions.queryRegions(player)

	def createEntity(self, template, position, rotation = None, tag = None):
		"""
		Creates an instance of an entity template
		@param template: Template to instantiate
		@param position: Position of new entity
		@param rotation: Rotation of new entity
		@param tag: Unique tag name for this entity
		@return: Created entity
		"""
		classNames = {
			'spawnable': 'SGWSpawnableEntity',
			'being': 'SGWBeing',
			'mob': 'SGWMob'
		}

		trace(Atrea.enums.LC_Uncategorized, "Creating entity from template %d" % template.id)
		entity = Atrea.createCellEntity(classNames[template.className], template.id)
		if tag is not None:
			entity.tag = tag
		entity.template = template
		if template.staticMesh:
			entity.staticMeshName = template.staticMesh
		entity.bodySet = template.bodySet
		entity.primaryColorId = template.primaryColorId
		entity.secondaryColorId = template.secondaryColorId
		entity.skinTint = template.skinTint
		if hasattr(entity, 'beingNameId') and template.nameId is not None:
			entity.beingNameId = template.nameId
		if hasattr(entity, 'components'):
			entity.components = copy.deepcopy(template.components)
		if hasattr(entity, 'level') and template.level is not None:
			entity.level = template.level
		if hasattr(entity, 'alignment') and template.alignment is not None:
			entity.alignment = template.alignment
		if hasattr(entity, 'faction') and template.faction is not None:
			entity.faction = template.faction
		if hasattr(entity, 'kismetEventSetId') and template.eventSetId is not None:
			entity.kismetEventSetId = template.eventSetId
		if hasattr(entity, 'interactionType') and template.interactionType is not None:
			entity.interactionType = template.interactionType
		if hasattr(entity, 'entityFlags') and template.flags is not None:
			entity.entityFlags = template.flags
		if hasattr(entity, 'beingName') and template.name is not None:
			entity.beingName = template.name
		if hasattr(entity, 'abilitySet') and template.abilitySet is not None:
			entity.abilitySet = template.abilitySet
		if hasattr(entity, 'ammoTypeId') and template.ammoTypeId is not None:
			entity.ammoTypeId = template.ammoTypeId
		if hasattr(entity, 'stats'):
			entity.getStat(Atrea.enums.health).update(0, 250, 250)
		if template.interactionSet is not None:
			for interaction in template.interactionSet.interactions.values():
				if interaction.id in Constants.STATIC_INTERACTIONS:
					cls = Constants.STATIC_INTERACTIONS[interaction.id]
					mod = import_module('cell.interactions.' + cls)
					handler = getattr(mod, cls)(entity)
					entity.staticInteractions[setMap.id] = handler

		entity.position = position
		if rotation is not None:
			entity.rotation = rotation

		# Start patrol behavior on the entity if a patrol path is specified
		if template.patrolPathId is not None:
			path = DefMgr.get('path', template.patrolPathId)
			# entity.startPatrol(path, template.patrolPointDelay)
		entity.enterSpace(self.spaceId)
		if entity.tag is not None:
			self.fire('entity.spawn.tag::' + str(entity.tag), entity = entity)
		if entity.template is not None:
			self.fire('entity.spawn.template::' + str(entity.template.templateName), entity = entity)
		return entity


	def findEntity(self, entityId):
		"""
		Retrieves the specified entity if it is found on this space.
		@param entityId: ID of entity to retrieve
		@return: Entity (or None if not found)
		"""
		return Atrea.findEntityOnSpace(entityId, self.spaceId)


	def findEntitiesByDbid(self, databaseId):
		"""
		Retrieves the specified entities if any if found on this space.
		@param databaseId: Database ID of entities to retrieve
		@return: Entity list
		"""
		return Atrea.findEntitiesByDbid(databaseId, self.spaceId)


	def removeSpawns(self):
		"""
		Removes all spawned entities from the space.
		"""
		entities = Atrea.findEntities(self.spaceId)
		for entity in entities:
			if not isinstance(entity, SGWPlayer):
				Atrea.destroyCellEntity(entity.entityId)


	def loadSpawns(self):
		"""
		Loads the spawnlist and spawns all entities.
		"""
		spawnlist = Atrea.dbQuery('select * from resources.spawnlist where world_id = %d' % self.worldId)
		for spawn in spawnlist:
			template = DefMgr.get('template', spawn['template_id'])
			pos = Atrea.Vector3()
			pos.x, pos.y, pos.z = spawn['x'], spawn['y'], spawn['z']
			rot = Atrea.Vector3()
			rot.x, rot.y, rot.z = 0, spawn['heading'], 0
			entity = self.createEntity(template, pos, rot, spawn['tag'])
			entity.spawnId = spawn['spawn_id']

