import Atrea
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class SpecialWords(Resource):
	versioning = True

	def __init__(self, rows):
		self.words = []
		for word in rows:
			self.words.append({
				'word': word['special_word'],
				'flags': word['flags'],
				'profanity': word['profanity'],
				'substring': word['substring']
			})

	def toXml(self):
		""" Creates a cooked XML resource from the applied science definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_SPECIAL_WORDS>'
		for word in self.words:
			xml += '<SpecialWord SpecialWord="' + quoteattr(word['word']) + '" Flags="' + str(word['flags']) + '" ' \
				   'Profanity="' + str(word['profanity']) + '" Substring="' + str(word['substring']) + '" />'
		xml += '</COOKED_SPECIAL_WORDS>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		words = Atrea.dbQuery('select * from resources.special_words')
		defs[0] = SpecialWords(words)
