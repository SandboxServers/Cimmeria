import Atrea
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class Text(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['moniker_id']
		self.monikerName = row['moniker_name']
		self.language = row['language']
		self.flags = row['flags']
		self.text = row['text']

	def toXml(self):
		""" Creates a cooked XML resource from the text definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_TEXT ' + \
		      'MonikerID="' + str(self.id) + '" MonikerName=' + quoteattr(self.monikerName) + \
		      ' Language="' + str(self.language) + '" Flags="' + str(self.flags) + \
		      '" Text=' + quoteattr(self.text) + ' />'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		texts = Atrea.dbQuery('select * from resources.texts')
		for text in texts:
			defs[text['moniker_id']] = Text(text, defMgr)
