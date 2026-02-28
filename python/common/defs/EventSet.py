from xml.sax.saxutils import quoteattr
import Atrea
from common.defs.Resource import Resource


class EventSet(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['event_set_id']
		self.instances = {}

	def toXml(self):
		""" Creates a cooked XML resource from the kismet event set definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_KISMET_EVENT_SET EventSetID="' + str(self.id) + '">'
		for instance in self.instances.values():
			if instance is not None:
				xml += '<KismetInstance EventID="' + str(instance.eventId) + \
					'" KismetEventSetSeqID="' + str(instance.seqId) + \
					'" KismetScriptName="' + str(instance.kismetScriptName) + '" />'
				for k, v in instance.nvps.items():
					xml += '<KismetScriptNVPs Value="' +quoteattr(v) + '" Name="' +quoteattr(k) + '" />'
		xml += '</COOKED_KISMET_EVENT_SET>'
		return xml

	def getSequence(self, eventId):
		"""
		Returns the kismet sequence for a given event ID
		@param eventId: Event ID (see ESequenceEventType enumeration)
		@return: KismetEventSequenceObject (or None if not found)
		"""
		if eventId in self.instances:
			return self.instances[eventId]
		else:
			return None

	@staticmethod
	def loadAll(defs, defMgr):
		eventSets = Atrea.dbQuery('select * from resources.event_sets')
		for eventSet in eventSets:
			defs[eventSet['event_set_id']] = EventSet(eventSet, defMgr)

		instances = Atrea.dbQuery('select * from resources.event_sets_sequences')
		for instance in instances:
			seq = defMgr.require('sequence', instance['sequence_id'])
			defs[instance['event_set_id']].instances[seq.eventId] = seq

