import Atrea
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource


class Container(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['container_id']
		self.name = row['name']
		self.equipped = (row['is_equipped'] == 1)

	def toXml(self):
		""" Creates a cooked XML resource from the container definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?><COOKED_CONTAINER ID="' + str(self.id) + \
		      '" Name=' + quoteattr(self.name) + ' IsEquipped="' + str(self.equipped).lower() + '" />'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		containers = Atrea.dbQuery('select * from resources.containers')
		for container in containers:
			defs[container['container_id']] = Container(container, defMgr)
