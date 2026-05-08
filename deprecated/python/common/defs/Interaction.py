import Atrea
from xml.sax.saxutils import escape
from common.defs.Resource import Resource


class Interaction(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['interaction_id']
		self.minimapAsset = row['minimap_asset']
		self.minimapPriority = row['minimap_priority']
		self.bodySetAsset = row['body_set_asset']
		self.bodySetPriority = row['body_set_priority']
		self.cursorAsset = row['cursor_asset']
		self.cursorPriority = row['cursor_priority']
		self.static = (row['is_static'] == 1)

	def toXml(self):
		""" Creates a cooked XML resource from the interaction definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_INTERACTIONS>'
		xml += '<ID>' + str(self.id) + '</ID>'
		xml += '<MinimapAsset>' + str(self.minimapAsset) + '</MinimapAsset>'
		xml += '<MinimapPriority>' + str(self.minimapPriority) + '</MinimapPriority>'
		if self.bodySetAsset is not None:
			xml += '<BodySetAsset>' + escape(self.bodySetAsset) + '</BodySetAsset>'
		else:
			xml += '<BodySetAsset/>'
		xml += '<BodySetPriority>' + str(self.bodySetPriority) + '</BodySetPriority>'
		if self.cursorAsset is not None:
			xml += '<CursorAsset>' + escape(self.cursorAsset) + '</CursorAsset>'
		else:
			xml += '<CursorAsset/>'
		xml += '<CursorPriority>' + str(self.cursorPriority) + '</CursorPriority>'
		xml += '<IsStatic>' + str(self.static).lower() + '</IsStatic>'
		xml += '</COOKED_INTERACTIONS>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		interactions = Atrea.dbQuery('select * from resources.interactions')
		for interaction in interactions:
			defs[interaction['interaction_id']] = Interaction(interaction, defMgr)
