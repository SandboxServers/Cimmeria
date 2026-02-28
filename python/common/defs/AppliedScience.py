import Atrea
from xml.sax.saxutils import escape
from common.defs.Resource import Resource


class AppliedScience(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['id']
		self.name = row['name']

	def toXml(self):
		""" Creates a cooked XML resource from the applied science definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_APPLIED_SCIENCE>'
		xml += '<ID>' + str(self.id) + '</ID>'
		xml += '<Name>' + escape(self.name) + '</Name>'
		xml += '</COOKED_APPLIED_SCIENCE>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		sciences = Atrea.dbQuery('select * from resources.applied_science')
		for science in sciences:
			defs[science['id']] = AppliedScience(science, defMgr)
