import Atrea
import Atrea.enums
from common.Logger import warn
from common.defs.Resource import Resource


class Blueprint(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['blueprint_id']
		self.discipline = defMgr.get('discipline', row['discipline_id'])
		self.alloy = (row['is_alloy'] == 1)
		# Item ID of product
		self.product = defMgr.require('item', row['product_id'])
		self.quantity = row['quantity']
		self.requiresElementaryComponents = (row['requires_elementary_components'] == 1)
		self.componentSets = {}


	def validate(self):
		"""
		Make sure that properties are sane and raise a warning if they aren't
		"""
		if not self.alloy and self.requiresElementaryComponents:
			warn(Atrea.enums.LC_Resources, 'Crafting blueprint %d must not require elementary components' % self.id)

		if self.alloy and len(self.componentSets) > 1:
			warn(Atrea.enums.LC_Resources, 'Alloying blueprint %d must have only one component set' % self.id)

		if len(self.componentSets) == 0:
			warn(Atrea.enums.LC_Resources, 'Blueprint %d must have at least one component set' % self.id)
		elif self.alloy and len(next(iter(self.componentSets.values()))) > 1:
			warn(Atrea.enums.LC_Resources, 'Alloying blueprint %d must have only one component' % self.id)


	def toXml(self):
		""" Creates a cooked XML resource from the blueprint definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_BLUPRINT>'
		xml += '<ID>' + str(self.id) + '</ID>'
		xml += '<ProductID>' + str(self.product.id) + '</ProductID>'
		xml += '<Quantity>' + str(self.quantity) + '</Quantity>'
		xml += '<DisciplineID>' + str(self.discipline.id) + '</DisciplineID>'
		xml += '<IsAlloy>' + str(self.alloy).lower() + '</IsAlloy>'
		xml += '<RequiresElementaryComponents>' + str(self.requiresElementaryComponents).lower() + '</RequiresElementaryComponents>'
		for componentSet in self.componentSets.values():
			xml += '<BlueprintComponentSets>'
			for component in componentSet:
				xml += '<BlueprintComponentList ItemID="' + str(component['item'].id) + \
				       '" Quantity="' + str(component['quantity']) + '" />'
			xml += '</BlueprintComponentSets>'
		xml += '</COOKED_BLUPRINT>'
		return xml


	@staticmethod
	def loadAll(defs, defMgr):
		blueprints = Atrea.dbQuery('select * from resources.blueprints')
		for blueprint in blueprints:
			defs[blueprint['blueprint_id']] = Blueprint(blueprint, defMgr)

		components = Atrea.dbQuery('select * from resources.blueprints_components')
		for component in components:
			bp = defs[component['blueprint_id']]
			# componentSetId is an internal identifier and is only used to distinguish
			# components sets from each other in the table; the client doesn't see this value
			if component['component_set_id'] not in bp.componentSets:
				bp.componentSets[component['component_set_id']] = []
			bp.componentSets[component['component_set_id']].append({
				'item': defMgr.require('item', component['item_id']),
			    'quantity': component['quantity']})

		for blueprint in defs.values():
			blueprint.validate()
