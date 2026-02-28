import Atrea
import Atrea.enums
from xml.sax.saxutils import quoteattr
from common.Logger import warn
from common.defs.Resource import Resource


class CharacterCreation(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.charDefId = row['char_def_id']
		self.alignmentId = Atrea.enums.__dict__[row['alignment']]
		self.archetypeId = Atrea.enums.__dict__[row['archetype']]
		self.bodySet = row['body_set']
		self.gender = Atrea.enums.__dict__[row['gender']]
		self.startingWorld = row['starting_world']
		self.startingX = row['starting_x']
		self.startingY = row['starting_y']
		self.startingZ = row['starting_z']
		self.visualGroups = {}
		self.abilities = []

	def getAllChoices(self, visGroupChoices):
		""" Returns a list of all visual choices (either required or optional)
		and associated data (components, item IDs).
		The function returns an error code if a required visual group is missing,
		or an invalid choice was specified for a group
		"""
		choices = {}
		# Add all choices to the returned list
		for choice in visGroupChoices:
			if choice['VisGroupId'] not in self.visualGroups:
				warn(Atrea.enums.LC_CharManagement, "Invalid visual group %d for charDef %d" % (choice['VisGroupId'], self.charDefId))
				return 10003 # ERROR_CharacterCreationUnspecifiedError
			group = self.visualGroups[choice['VisGroupId']]
			if group.type != Atrea.enums.VIS_Optional:
				warn(Atrea.enums.LC_CharManagement, "Choice not allowed for forced for visual group %d" % choice['VisGroupId'])
				return 10003 # ERROR_CharacterCreationUnspecifiedError
			if choice['ChoiceId'] not in group.choices:
				warn(Atrea.enums.LC_CharManagement, "Invalid choice %d for visual group %d" % (choice['ChoiceId'], choice['VisGroupId']))
				return 10003 # ERROR_CharacterCreationUnspecifiedError
			choices[group.id] = group.choices[choice['ChoiceId']]

		# Check that all required visual groups have a choice selected
		for groupId in self.visualGroups:
			if groupId not in choices:
				if self.visualGroups[groupId].type == Atrea.enums.VIS_Forced:
					# For forced visual choices take the first choice from the list
					choices[groupId] = next(iter(self.visualGroups[groupId].choices.values()))
				else:
					warn(Atrea.enums.LC_CharManagement, "Missing choice for visual group %d, charDef %d" % (groupId, self.charDefId))
					return 10000 # ERROR_CharacterCreationNotEnoughInformation

		return choices
		
	def toXml(self):
		xml = '<Defs BodySet="' + self.bodySet + '" GenderId="' + str(self.gender) + '" ArchetypeId="' + \
			str(self.archetypeId) + '" AlignmentId="' + str(self.alignmentId) + \
			'" CharDefId="' + str(self.charDefId) + '">'
		for group in self.visualGroups.values():
			xml += group.toXml()
		xml += '</Defs>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		rows = Atrea.dbQuery('select * from resources.char_creation')
		for r in rows:
			defs[r['char_def_id']] = CharacterCreation(r, defMgr)

		abilities = Atrea.dbQuery('select * from resources.char_creation_abilities')
		for ability in abilities:
			abilityRes = defMgr.require('ability', ability['ability_id'])
			defs[ability['char_def_id']].abilities.append(abilityRes)

		visGroups = Atrea.dbQuery('select * from resources.char_creation_visgroups')
		visRefs = {}
		for grp in visGroups:
			group = VisualGroup(grp)
			# We need to keep a list of visual group IDs when loading because
			# sgw_res_char_creation_choices uses visGroupId to refer to its group
			visRefs[grp['vis_group_id']] = group
			defs[grp['char_def_id']].visualGroups[grp['vis_group_id']] = group

		choices = Atrea.dbQuery('select * from resources.char_creation_choices')
		for choice in choices:
			# Check if the item in the choice is valid
			if choice['item_id']:
				defMgr.require('item', choice['item_id'])
			visRefs[choice['vis_group_id']].addChoice(choice)
			
class VisualGroup(object):
	def __init__(self, row):
		self.id = row['vis_group_id']
		self.text = row['text']
		self.type = Atrea.enums.__dict__[row['vis_type']]
		self.choices = {}
		
	def toXml(self):
		xml = '<VisualGroups VisType="' + str(self.type) + '" Text=' + quoteattr(self.text) + ' VisGroupId="' + str(self.id) + '">'
		for choiceId, choice in self.choices.items():
			itemId = choice['item_id']
			if itemId is None:
				itemId = 0
			
			xml += '<Choices ItemId="' + str(itemId) + '" Component=' + quoteattr(choice['component']) + \
				' ChoiceId="' + str(choiceId) + '" />'
		xml += '</VisualGroups>'
		return xml
		
	def addChoice(self, choice):
		self.choices[choice['choice_id']] = {
			'component': choice['component'],
			'item_id': choice['item_id'],
			'item_durability': choice['item_durability'],
			'item_bound': choice['item_bound'] == 1
		}
