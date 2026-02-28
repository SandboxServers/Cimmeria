import Atrea
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class Sequence(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.seqId = row['sequence_id']
		self.eventId = row['event_id']
		self.kismetScriptName = row['kismet_script_name']
		self.nvps = {}

	def toXml(self):
		""" Creates a cooked XML resource from the kismet event sequence definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?>'
		xml += '<COOKED_KISMET_EVENT_SEQUENCE KismetScriptName=' + quoteattr(self.kismetScriptName) + ' EventID="' + \
			   str(self.eventId) + '" KismetEventSetSeqID="' + str(self.seqId) + '">'
		for name, value in self.nvps.items():
			xml += '<KismetScriptNVPs Value=' + quoteattr(value) + ' Name=' + quoteattr(name) + '/>'
		xml += '</COOKED_KISMET_EVENT_SEQUENCE>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		eventSeqs = Atrea.dbQuery('select * from resources.sequences')
		for eventSeq in eventSeqs:
			defs[eventSeq['sequence_id']] = Sequence(eventSeq, defMgr)
		
		nvps = Atrea.dbQuery('select * from resources.sequences_nvp')
		for nvp in nvps:
			defs[nvp['sequence_id']].nvps[nvp['name']] = nvp['value']
