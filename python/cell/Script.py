from cell.SGWSpawnableEntity import SGWSpawnableEntity
import Atrea

# Imported lazily inside EffectScript methods to avoid circular imports at
# module load time (ContentEngine imports from cell.Script indirectly).
def _get_engine():
    try:
        from cell.ContentEngine import engine
        return engine
    except ImportError:
        return None

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
	_LIFECYCLE_EVENT_TYPES = {
		'onEffectInit':    'effect_init',
		'onEffectRemoved': 'effect_removed',
		'onPulseBegin':    'effect_pulse_begin',
		'onPulseEnd':      'effect_pulse_end',
	}

	def __init__(self, owner : SGWSpawnableEntity, effect, nvps):
		"""
		@param owner: Context the script is executing in (owner entity)
		@param effect: Effect instance
		"""
		Script.__init__(self, owner, {})
		self.effect = effect
		self.params = nvps
		# Wrap each lifecycle method so that the engine dispatch fires after
		# the subclass override runs.  This works even when the subclass does
		# not call super(), because we patch the bound method on the instance.
		for method_name, event_type in self._LIFECYCLE_EVENT_TYPES.items():
			original = getattr(self, method_name)
			def _make_wrapper(orig, etype):
				def _wrapper():
					orig()
					self._dispatch_effect_event(etype)
				return _wrapper
			setattr(self, method_name, _make_wrapper(original, event_type))


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

	# ------------------------------------------------------------------
	# Lifecycle dispatch hooks — called by the framework after the
	# subclass overrides above.  Routes events to ContentEngine when it
	# is handling this effect.
	# ------------------------------------------------------------------

	def _dispatch_effect_event(self, event_type):
		eng = _get_engine()
		if eng and hasattr(self, 'effect') and self.effect is not None:
			effect_id = getattr(self.effect, 'effectId', None) or getattr(self.effect, 'effect_id', None)
			if effect_id is not None:
				eng.on_effect_event(event_type, effect_id, self)
