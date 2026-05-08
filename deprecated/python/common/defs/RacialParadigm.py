import Atrea
from xml.sax.saxutils import escape
from common.defs.Resource import Resource


class RacialParadigm(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['id']
		self.name = row['name']

	def toXml(self):
		""" Creates a cooked XML resource from the racial paradigm definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_RACIAL_PARADIGM>'
		xml += '<ID>' + str(self.id) + '</ID>'
		xml += '<Name>' + escape(self.name) + '</Name>'
		xml += '</COOKED_RACIAL_PARADIGM>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		paradigms = Atrea.dbQuery('select * from resources.racial_paradigm')
		for paradigm in paradigms:
			defs[paradigm['id']] = RacialParadigm(paradigm, defMgr)
