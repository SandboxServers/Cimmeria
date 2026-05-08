from cell.SGWSpawnableEntity import SGWSpawnableEntity
import Atrea

class Script(object):
	def __init__(self, owner : SGWSpawnableEntity, persistentData : dict):
		"""
		@param owner: Context the script is executing in (owner entity)
		@param persistentData: Persistent variables stored for this script
		"""
		self.owner = owner
		self.savedVars = persistentData
		if len(persistentData):
			self.restore()

	def teardown(self):
		pass

	def persist(self):
		pass

	def restore(self):
		pass

	def str2vec(self, s : str):
		"""
		Helper function to convert string to Vector3

		@param s: String to convert
		@return: Converted vector
		"""
		parts = s.split(',')
		return Atrea.Vector3(float(parts[0]), float(parts[1]), float(parts[2]))


class EffectScript(Script):
	def __init__(self, owner : SGWSpawnableEntity, effect, nvps):
		"""
		@param owner: Context the script is executing in (owner entity)
		@param effect: Effect instance
		"""
		Script.__init__(self, owner, {})
		self.effect = effect
		self.params = nvps


	def addInstanceParam(self, name, value):
		"""
		Adds a named instance parameter to the script.
		@param name: Parameter name
		@param value: Parameter value (always string)
		"""
		self.params[name] = value


	def onEffectInit(self):
		pass

	def onEffectRemoved(self):
		pass

	def onPulseBegin(self):
		pass

	def onPulseEnd(self):
		pass
