import Atrea
import Atrea.enums
from common.Logger import warn
from common.defs.Resource import Resource


class EntityTemplate(Resource):
	def __init__(self, row, defMgr):
		self.id = row['template_id']
		self.staticMesh = row['static_mesh']
		self.bodySet = row['body_set']
		if row['components'] is not None:
			self.components = [comp for comp in row['components'][1:-1].split(',') if comp]
		else:
			self.components = []
		self.flags = row['flags']
		self.interactionType = row['interaction_type']
		self.eventSetId = row['event_set_id']
		self.level = row['level']
		self.alignment = row['alignment']
		self.faction = row['faction']
		self.nameId = row['name_id']
		self.name = row['name']
		self.patrolPathId = row['patrol_path_id']
		self.patrolPointDelay = row['patrol_point_delay']
		self.className = row['class']
		self.templateName = row['template_name']
		self.buyItemList = defMgr.require('item_list', row['buy_item_list']) if row['buy_item_list'] else None
		self.sellItemList = defMgr.require('item_list', row['sell_item_list']) if row['sell_item_list'] else None
		self.repairItemList = defMgr.require('item_list', row['repair_item_list']) if row['repair_item_list'] else None
		self.rechargeItemList = defMgr.require('item_list', row['recharge_item_list']) if row['recharge_item_list'] else None
		self.abilitySet = defMgr.require('ability_set', row['ability_set_id']) if row['ability_set_id'] else None
		self.ammoTypeId = Atrea.enums.__dict__[row['ammo_type']] if row['ammo_type'] else None
		self.lootTable = defMgr.require('loot_table', row['loot_table_id']) if row['loot_table_id'] else None
		self.primaryColorId = row['primary_color_id']
		if self.primaryColorId < 0:
			self.primaryColorId = ~self.primaryColorId + 1
		self.secondaryColorId = row['secondary_color_id']
		if self.secondaryColorId < 0:
			self.secondaryColorId = ~self.secondaryColorId + 1
		self.skinTint = row['skin_tint']
		if self.skinTint < 0:
			self.skinTint = ~self.skinTint + 1
		self.weapon = defMgr.require('item', row['weapon_item_id']) if row['weapon_item_id'] else None
		interactionSetId = row['interaction_set_id']
		self.interactionSet = defMgr.require('interaction_set', interactionSetId) if interactionSetId else None
		trainerId = row['trainer_ability_list_id']
		self.trainerAbilities = defMgr.require('trainer_ability_list', trainerId) if trainerId else None
		self.speakerId = row['speaker_id']

		if self.abilitySet is not None:
			for ability in self.abilitySet.abilities.values():
				if ability.requiresWeapons():
					if self.weapon is None:
						warn(Atrea.enums.LC_Resources, 'Template %d has no weapon but ability %d requires one!' %  (self.id, ability.id))
					elif not ability.canUseWithMonikers(self.weapon.monikerIds):
						warn(Atrea.enums.LC_Resources, 'Template %d cannot use ability %d (requirements: %s, item monikers: %s)!' %
						   (self.id, ability.id, str(ability.monikerIds), str(self.weapon.monikerIds)))


	@staticmethod
	def loadAll(defs, defMgr):
		templates = Atrea.dbQuery('select * from resources.entity_templates')
		for template in templates:
			defs[template['template_id']] = EntityTemplate(template, defMgr)

