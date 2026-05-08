import Atrea
import Atrea.enums
import sys
import imp
from common.Logger import trace, warn

scalarTypes = [bool, int, float, bytes, bytearray, str]
listTypes = [list, dict, set, frozenset, tuple]
ignoreAttrs = ["client", "base", "cell", "witnesses"]
reloadableModules = ["base", "cell", "common"]

def reloadObject(obj, known):
	"""
	Reassigns the __class__ of the object after reloading its module
	(so the object will use the new class prototype instead of the old)
	@param obj: Object to reload
	"""
	if obj in known:
		return

	known[obj] = True
	known['reloaded'] += 1
	cls = getattr(sys.modules[obj.__class__.__module__], obj.__class__.__name__)
	obj.__class__ = cls

	# Check each attribute of the class
	# If the attribute is an object, reload it as well
	if hasattr(obj, '__dict__'):
		reloadDict(obj.__dict__, known)

def reloadDict(d, known):
	"""
	Reload the class of all objects in the dict
	@param d: dict
	"""
	for attr, value in d.items():
		if attr not in ignoreAttrs:
			reloadValue(value, known)

def reloadList(l, known):
	"""
	Reload the class of all objects in the dict
	@param l: list
	"""
	for value in l:
		reloadValue(value, known)

def reloadValue(value, known):
	"""
	Checks and/or reloads the value.
	If the value has a reloadable class ("base", "cell" or "common" module), its class is reload.
	If the value is a container (list/dict) its elements are inspected recursively.
	@param value: Value to inspect
	@param known: List of already inspected objects
	"""
	if isinstance(value, object) and type(value) not in scalarTypes:
		if type(value) in listTypes:
			if isinstance(value, dict):
				reloadDict(value, known)
			elif isinstance(value, list):
				reloadList(value, known)
		else:
			module = value.__class__.__module__.split('.', 1)[0]
			if module in reloadableModules:
				reloadObject(value, known)

def reloadModules():
	# FIXME: This needs to be called twice to actually reload the objects and produces weird side effects
	# (eg. isinstance(being, SGWBeing) == false)
	exceptions = [
		# Don't reload DefMgr as the server will lose all resource refs when its reloaded
		'common.defs.Def',
		# Global BaseApp state
		'base.Global',
		# Global CellApp state
		'cell.Global',
		'cell.SpaceManager'
	]

	for name, module in sys.modules.items():
		if (name.startswith('base.') or name.startswith('cell.') or name.startswith('common.')) and name not in exceptions:
			imp.reload(module)

	# This is needed to update the newly created object types with Base/Cell extender code
	# Otherwise the new and old class dtors won't be compatible
	Atrea.reloadClasses()

	if hasattr(Atrea, 'findEntities'):
		# Entity enumerator for the CellApp
		entities = Atrea.findEntities(0xffffffff)
	elif hasattr(Atrea, 'findAllEntities'):
		# Entity enumerator for the BaseApp
		entities = Atrea.findAllEntities()
	else:
		entities = None
		warn(Atrea.enums.LC_Uncategorized, "Cannot reload entities: No enumerator found")

	if entities is not None:
		trace(Atrea.enums.LC_Uncategorized, "Replacing class of %d entities ..." % len(entities))
		known = {'reloaded': 0}
		for entity in entities:
			reloadObject(entity, known)
		trace(Atrea.enums.LC_Uncategorized, "Replaced class of %d objects" % known['reloaded'])

	trace(Atrea.enums.LC_Uncategorized, "Reload completed")


