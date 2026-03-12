#include <stdafx.hpp>
#include <entity/defs.hpp>
#include <entity/entity.hpp>
#include <entity/mailbox.hpp>
#include <entity/pyutil.hpp>
#include <boost/algorithm/string.hpp>
#include <boost/python/module.hpp>

using namespace boost::algorithm;

std::string trim(const char * str)
{
	std::string s = str;
	trim(s);
	return s;
}

PyTypeInfo * PyParseType(tinyxml2::XMLElement & node)
{
	std::string typeName = node.GetText();
	trim(typeName);
	PyTypeInfo * type = PyTypeDb::instance().findType(typeName);
	if (type)
		return type;

	if (typeName == "ARRAY")
	{
		tinyxml2::XMLElement * childElement = node.FirstChildElement("of");
		SGW_ASSERT(childElement && "Array definition needs <of> element!");
		PyTypeInfo * elementType = PyParseType(*childElement);
		PyTypeInfo * list = new PyTypeInfo(PyTypeInfo::TYPE_LIST);
		SGW_ASSERT(elementType);
		list->addElement(*elementType);
		return list;
	}
	else if (typeName == "FIXED_DICT")
	{
		tinyxml2::XMLElement * properties = node.FirstChildElement("Properties");
		SGW_ASSERT(properties && "Fixed dict definition needs <Properties> element!");
		PyTypeInfo * dict = new PyTypeInfo(PyTypeInfo::TYPE_FIXED_DICT);
		for (tinyxml2::XMLElement * property = properties->FirstChildElement();
			property; property = property->NextSiblingElement())
		{
			tinyxml2::XMLElement * type = property->FirstChildElement("Type");
			SGW_ASSERT(type && "Fixed dict property needs <Type> element!");
			PyTypeInfo * typeinfo = PyParseType(*type);
			typeinfo->name = property->Name();
			dict->addElement(*typeinfo);
			/*tinyxml2::XMLElement * key = property->FirstChildElement("Key");
			if (key)
				dict->addKey(property->Name(), PyParseType(*type));*/
		}
		return dict;
	}
	else
		throw std::runtime_error(std::string("Invalid type encountered: ") + typeName);
}

PyClassDef * PyClassDef::fromDefinitionFile(std::string const & path, std::string const & name, bool isInterface, bool cell)
{
	tinyxml2::XMLDocument doc;
	int error = doc.LoadFile(path.c_str());
	if (error != tinyxml2::XML_NO_ERROR)
	{
		WARN("Failed to parse %s: %d", path.c_str(), error);
		WARN("Detail: %s", doc.GetErrorStr1());
		return nullptr;
	}
	tinyxml2::XMLElement * root = doc.RootElement();
	SGW_ASSERT(root);
	SGW_ASSERT(root->Name() == std::string("root"));

	return new PyClassDef(*root, name, isInterface, cell);
}

bool PyClassDef::isInterface() const
{
	return isInterface_;
}

bool PyClassDef::serverOnly() const
{
	return serverOnly_;
}


int PyClassDef::index() const
{
	return index_;
}


std::string const & PyClassDef::name() const
{
	return name_;
}


MailboxClass * PyClassDef::baseMailbox() const
{
	return baseMailbox_;
}


MailboxClass * PyClassDef::cellMailbox() const
{
	return cellMailbox_;
}


MailboxClass * PyClassDef::clientMailbox() const
{
	return clientMailbox_;
}


PyClassDef * PyClassDef::parent() const
{
	return parent_;
}


PyMethod * PyClassDef::findMethod(EndpointType type, uint32_t messageId) const
{
	auto & methods =
		(type == Service::BaseMailbox) ? internalBaseMethods_ :
		(type == Service::BaseExposedMailbox) ? exposedBaseMethods_ :
		(type == Service::CellMailbox) ? internalCellMethods_ :
		(type == Service::CellExposedMailbox) ? exposedCellMethods_ :
		exposedClientMethods_;

	if (messageId < methods.size())
		return methods[messageId];
	else
		return nullptr;
}


void PyClassDef::setIndex(int index)
{
	index_ = index;
}


void PyClassDef::setClientMailbox(class MailboxClass * mailbox)
{
	clientMailbox_ = mailbox;
}


/*
 * Creates a class from an entity definition XML node
 */
PyClassDef::PyClassDef(tinyxml2::XMLElement & root, std::string const & name, bool isInterface, bool /*cell*/)
	: index_(-1), name_(name), isInterface_(isInterface), serverOnly_(false), parent_(nullptr)
{
	tinyxml2::XMLElement * serverOnly = root.FirstChildElement("ServerOnly");
	if (serverOnly)
		serverOnly_ = true;

	// Check if parent class exists & inherit methods & props from it
	tinyxml2::XMLElement * parent = root.FirstChildElement("Parent");
	// Interfaces can't have parent classes
	SGW_ASSERT(!(isInterface && parent));
	if (parent)
	{
		std::string parentName = parent->GetText();
		trim(parentName);
		parent_ = PyTypeDb::instance().loadClass(parentName, false);
		SGW_ASSERT(parent_ && "Couldn't find parent class for entity");
		inherit(parent_);
	}

	tinyxml2::XMLElement * implements = root.FirstChildElement("Implements");
	// Interfaces can't implement any interfaces
	SGW_ASSERT(!(isInterface && implements && implements->FirstChildElement()));
	if (implements)
	{
		/*
		 * Perform inheritance for interfaces as well
		 * From an import POV, the only difference between interfaces and classes is
		 * that interfaces cannot have a parent class
		 */
		for (tinyxml2::XMLElement * impl = implements->FirstChildElement("Interface"); impl; impl = impl->NextSiblingElement("Interface"))
		{
			std::string intfName = impl->GetText();
			trim(intfName);
			PyClassDef * intf = PyTypeDb::instance().loadClass(intfName, true);
			SGW_ASSERT(intf && "Couldn't find parent interface for entity");
			interfaces_.push_back(intf);
			inherit(intf);
		}
	}

	tinyxml2::XMLElement * bases = root.FirstChildElement("BaseMethods");
	if (bases)
	{
		for (tinyxml2::XMLElement * base = bases->FirstChildElement(); base; base = base->NextSiblingElement())
		{
			PyMethod * method = PyMethod::fromDefinition(*base, (uint16_t)internalBaseMethods_.size(), Service::BaseMailbox);
			baseMethods_.insert(std::make_pair(base->Name(), method));
			internalBaseMethods_.push_back(method);
			if (method->exposed())
				exposedBaseMethods_.push_back(method);
		}
	}

	tinyxml2::XMLElement * cells = root.FirstChildElement("CellMethods");
	if (cells)
	{
		for (tinyxml2::XMLElement * cell = cells->FirstChildElement(); cell; cell = cell->NextSiblingElement())
		{
			PyMethod * method = PyMethod::fromDefinition(*cell, (uint16_t)internalCellMethods_.size(), Service::CellMailbox);
			cellMethods_.insert(std::make_pair(cell->Name(), method));
			internalCellMethods_.push_back(method);
			if (method->exposed())
				exposedCellMethods_.push_back(method);
		}
	}
	
	tinyxml2::XMLElement * clients = root.FirstChildElement("ClientMethods");
	if (clients)
	{
		for (tinyxml2::XMLElement * client = clients->FirstChildElement(); client; client = client->NextSiblingElement())
		{
			PyMethod * method = PyMethod::fromDefinition(*client, (uint16_t)exposedClientMethods_.size(), Service::ClientMailbox);
			clientMethods_.insert(std::make_pair(client->Name(), method));
			// All client methods are implicitly exposed
			exposedClientMethods_.push_back(method);
		}
	}
}

PyClassDef::~PyClassDef()
{
	/*
	 * Clean up method defs
	 * These are either allocated by inheriting methods or constructing from
	 * a class def, so it's safe to delete them.
	 *
	 * The only requirement is that python instances must not be called
	 * after destruction
	 */
	for (auto it = baseMethods_.begin(); it != baseMethods_.end(); ++it)
		delete it->second;

	for (auto it = cellMethods_.begin(); it != cellMethods_.end(); ++it)
		delete it->second;

	for (auto it = clientMethods_.begin(); it != clientMethods_.end(); ++it)
		delete it->second;
}

/*
 * Inherit methods from parent class / interface.
 * This is needed to avoid expensive tree lookups when calling class / using props,
 * and to handle cases where an interface is implemented in multiple hierarchies and
 * the same message would have multiple IDs.
 */
void PyClassDef::inherit(PyClassDef * cls)
{
	for (auto it = cls->internalBaseMethods_.begin(); it != cls->internalBaseMethods_.end(); ++it)
	{
		// Clone the method to assign a new internal message ID to it
		// (and a new method object for Python)
		// The method WILL have a different message ID from the Client,
		// but we won't send Base/Cell messages to the Client, so it doesn't matter
		PyMethod * method = (*it)->clone((uint16_t)internalBaseMethods_.size(), PyMethod::Handler());
		baseMethods_.insert(std::make_pair(method->name(), method));
		internalBaseMethods_.push_back(method);
		if (method->exposed())
			exposedBaseMethods_.push_back(method);
	}
	
	// Do the same method cloning for Cell messages
	for (auto it = cls->internalCellMethods_.begin(); it != cls->internalCellMethods_.end(); ++it)
	{
		PyMethod * method = (*it)->clone((uint16_t)internalCellMethods_.size(), PyMethod::Handler());
		cellMethods_.insert(std::make_pair(method->name(), method));
		internalCellMethods_.push_back(method);
		if (method->exposed())
			exposedCellMethods_.push_back(method);
	}
	
	for (auto it = cls->exposedClientMethods_.begin(); it != cls->exposedClientMethods_.end(); ++it)
	{
		// Client methods are only called from Base/Cell (server side),
		// so they only have exposed methods and one message ID on both sides
		PyMethod * method = (*it)->clone((uint16_t)exposedClientMethods_.size(), PyMethod::Handler());
		clientMethods_.insert(std::make_pair(method->name(), method));
		exposedClientMethods_.push_back(method);
	}
}

PyMethod * PyClassDef::findMethod(EndpointType type, std::string const & name)
{
	std::map<std::string, PyMethod *> & methods =
		(type == Service::BaseMailbox) ? baseMethods_ :
		(type == Service::CellMailbox) ? cellMethods_ :
		clientMethods_;

	auto it = methods.find(name);
	if (it != methods.end())
		return it->second;
	else
		return nullptr;
}


/*
 * Clones all internal methods for the specified mailbox type and binds
 * method handlers to "handler".
 */
void PyClassDef::rebind(std::vector<PyMethod *> & bindings, EndpointType type, PyMethod::Handler handler)
{
	std::vector<PyMethod *> & methods =
		(type == Service::BaseMailbox) ? internalBaseMethods_ :
		(type == Service::CellMailbox) ? internalCellMethods_ :
		exposedClientMethods_;

	for (auto it = methods.begin(); it != methods.end(); ++it)
		bindings.push_back((*it)->clone((*it)->messageId(), handler));
}


PyTypeDb * PyTypeDb::instance_ = nullptr;

PyTypeDb & PyTypeDb::instance()
{
	if (!instance_)
		instance_ = new PyTypeDb();

	return *instance_;
}

PyTypeDb::PyTypeDb()
	: cell_(false)
{
}

#define PY_REGISTER_TYPE(T, TN) { \
	PyTypeInfo * type = new PyTypeInfo(PyTypeInfo::TN); \
	registerType(#T, type); \
}

void PyTypeDb::initialize(std::string const & entitiesPath, bool cell)
{
	cell_ = cell;
	entitiesPath_ = entitiesPath;
	
	PY_REGISTER_TYPE(BOOL, TYPE_BOOL);
	PY_REGISTER_TYPE(INT8, TYPE_INT8);
	PY_REGISTER_TYPE(UINT8, TYPE_UINT8);
	PY_REGISTER_TYPE(INT16, TYPE_INT16);
	PY_REGISTER_TYPE(UINT16, TYPE_UINT16);
	PY_REGISTER_TYPE(INT32, TYPE_INT32);
	PY_REGISTER_TYPE(UINT32, TYPE_UINT32);
	PY_REGISTER_TYPE(INT64, TYPE_INT64);
	PY_REGISTER_TYPE(UINT64, TYPE_UINT64);
	PY_REGISTER_TYPE(FLOAT, TYPE_FLOAT);
	PY_REGISTER_TYPE(DOUBLE, TYPE_DOUBLE);
	PY_REGISTER_TYPE(STRING, TYPE_STRING);
	PY_REGISTER_TYPE(WSTRING, TYPE_UNICODE);
	PY_REGISTER_TYPE(VECTOR3, TYPE_VECTOR3);
	PY_REGISTER_TYPE(PYTHON, TYPE_PICKLED);

	// FIXME: Implement proper python types for these
	PyTypeInfo * mailbox = new PyTypeInfo(PyTypeInfo::TYPE_UINT32);
	registerType("MAILBOX", mailbox);

	loadEnumerations(entitiesPath_ + "/defs/enumerations.xml");
	loadEnumerations(entitiesPath_ + "/custom_enumerations.xml");
	loadAliases(entitiesPath_ + "/defs/alias.xml");
	loadAliases(entitiesPath_ + "/custom_alias.xml");
	loadEntities();
}

void PyTypeDb::registerType(std::string const & name, PyTypeInfo * type)
{
	SGW_ASSERT(!findType(name));
	types_.insert(std::make_pair(name, type));
}

void PyTypeDb::registerClass(std::string const & name, bool isInterface, PyClassDef * cls)
{
	SGW_ASSERT(!findClass(name, isInterface));
	classes_.insert(std::make_pair(name, cls));
}

void PyTypeDb::registerConstant(std::string const & enumeration, std::string const & name, bp::object value)
{
	if (enumsModule_.is_none())
		enumsModule_ = PyUtil_CreateSubmodule("Atrea", "enums");
	
	if (enumNamesModule_.is_none())
		enumNamesModule_ = PyUtil_CreateSubmodule("Atrea", "enumNames");

	bp::setattr(enumsModule_, name.c_str(), value);

	if (!PyObject_HasAttrString(enumNamesModule_.ptr(), enumeration.c_str()))
		setattr(enumNamesModule_, enumeration.c_str(), bp::dict());
	
	auto enumDict = (bp::object)enumNamesModule_.attr(enumeration.c_str());
	if (!bp::extract<bp::dict>(value).check())
		enumDict[value] = name;
}

PyTypeInfo * PyTypeDb::findType(std::string const & name)
{
	auto it = types_.find(name);
	if (it != types_.end())
		return it->second;
	else
		return nullptr;
}

PyClassDef * PyTypeDb::findClass(std::string const & name, bool isInterface)
{
	auto it = classes_.find(name);
	while (it != classes_.end())
	{
		if (it->first == name && it->second->isInterface() == isInterface)
			return it->second;
		else
			++it;
	}

	return nullptr;
}

PyClassDef * PyTypeDb::findClass(int classId)
{
	for (auto it = classes_.begin(); it != classes_.end(); ++it)
	{
		if (it->second->index() == classId)
			return it->second;
	}

	return nullptr;
}

PyClassDef * PyTypeDb::loadClass(std::string const & name, bool isInterface)
{
	PyClassDef * cls = findClass(name, isInterface);
	if (cls)
		return cls;

	std::string path = entitiesPath_ + "/defs/" + (isInterface ? "interfaces/" : "") + name + ".def";
	cls = PyClassDef::fromDefinitionFile(path, name, isInterface, cell_);
	if (cls)
		registerClass(name, isInterface, cls);
	return cls;
}

std::multimap<std::string, PyClassDef *> const & PyTypeDb::classes() const
{
	return classes_;
}

void PyTypeDb::loadAliases(std::string const & path)
{
	tinyxml2::XMLDocument doc;
	int loaded = doc.LoadFile(path.c_str());
	SGW_ASSERT(loaded == tinyxml2::XML_NO_ERROR);
	tinyxml2::XMLElement * root = doc.RootElement();
	SGW_ASSERT(root);
	SGW_ASSERT(root->Name() == std::string("root"));

	for (tinyxml2::XMLElement * alias = root->FirstChildElement();
		alias; alias = alias->NextSiblingElement())
	{
		registerType(alias->Name(), PyParseType(*alias));
	}
}

void PyTypeDb::loadEntities()
{
	std::string entitiesPath = entitiesPath_ + "/entities.xml";
	tinyxml2::XMLDocument doc;
	int loaded = doc.LoadFile(entitiesPath.c_str());
	SGW_ASSERT(loaded == tinyxml2::XML_NO_ERROR);
	tinyxml2::XMLElement * root = doc.RootElement();
	SGW_ASSERT(root);
	SGW_ASSERT(root->Name() == std::string("root"));

	int index = 0;
	for (tinyxml2::XMLElement * entity = root->FirstChildElement();
		entity; entity = entity->NextSiblingElement())
	{
		PyClassDef * cls = loadClass(entity->Name(), false);
		if (!cls)
		{
			WARN("PyTypeDb: Failed to load class %s", entity->Name());
			throw std::runtime_error("Failed to load class defs");
		}
		if (!cls->serverOnly())
			cls->setIndex(index++);
	}
}

void PyTypeDb::loadEnumerations(std::string const & path)
{
	tinyxml2::XMLDocument doc;
	int loaded = doc.LoadFile(path.c_str());
	SGW_ASSERT(loaded == tinyxml2::XML_NO_ERROR);
	tinyxml2::XMLElement * root = doc.RootElement();
	SGW_ASSERT(root);
	SGW_ASSERT(root->Name() == std::string("root"));

	for (tinyxml2::XMLElement * enumeration = root->FirstChildElement();
		enumeration; enumeration = enumeration->NextSiblingElement())
	{
		// We'll ignore enumeration type here as all enums are ints and
		// Python only has one int type (PyLong) anyway
		tinyxml2::XMLElement * tokens = enumeration->FirstChildElement("Tokens");
		if (tokens)
		{
			for (tinyxml2::XMLElement * token = tokens->FirstChildElement();
				token; token = token->NextSiblingElement())
			{
				tinyxml2::XMLElement * nameNode = token->FirstChildElement("Name");
				tinyxml2::XMLElement * valueNode = token->FirstChildElement("Value");
				SGW_ASSERT(nameNode && valueNode);
				// Name and value needs to be trimmed as they may contain
				// trailing spaces
				std::string name = nameNode->GetText();
				std::string value = valueNode->GetText();
				trim(name);
				trim(value);
				// Work around invalid INT32 values in enumerations.xml
				if (value.find_first_of('.') == std::string::npos)
				{
#ifdef _MSC_VER
					int64_t intval = _strtoi64(value.c_str(), nullptr, 10);
#else
					int64_t intval = strtoll(value.c_str(), nullptr, 10);
#endif
					registerConstant(enumeration->Name(), name.c_str(), bp::object(intval));
				}
			}
		}
		else
		{
			tinyxml2::XMLElement * vtype = enumeration->FirstChildElement("ValueType");
			if (vtype)
			{
				bp::dict table;
				for (tinyxml2::XMLElement * row = enumeration->FirstChildElement("Row");
					row; row = row->NextSiblingElement())
				{
					tinyxml2::XMLElement * keyNode = row->FirstChildElement("Key");
					tinyxml2::XMLElement * dataNode = row->FirstChildElement("Data");
					SGW_ASSERT(keyNode && dataNode);
					// Key and data needs to be trimmed as they may contain leading/trailing spaces
					std::string key = keyNode->GetText();
					std::string data = dataNode->GetText();
					trim(key);
					trim(data);

					int keyVal = atol(key.c_str());
					std::istringstream stream(data);
					std::string item;
					bp::list values;
					while (std::getline(stream, item, ','))
					{
						values.append(atol(item.c_str()));
					}

					table[keyVal] = values;
				}

				registerConstant(enumeration->Name(), enumeration->Name(), table);
			}
		}
	}
}
