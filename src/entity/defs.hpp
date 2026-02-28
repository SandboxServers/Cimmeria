#pragma once

#include <entity/python.hpp>
#include <entity/method.hpp>
#include <entity/mailbox.hpp>
#include <mercury/nub.hpp>
#include <common/service.hpp>

PyTypeInfo * PyParseType(tinyxml2::XMLElement & node);

class PyClassDef
{
public:
	static PyClassDef * fromDefinitionFile(std::string const & path, std::string const & name, bool isInterface, bool cell);

	typedef Service::EndpointType EndpointType;
	
	~PyClassDef();

	bool isInterface() const;
	bool serverOnly() const;
	int index() const;
	std::string const & name() const;
	class MailboxClass * baseMailbox() const;
	class MailboxClass * cellMailbox() const;
	class MailboxClass * clientMailbox() const;
	class PyClassDef * parent() const;
	PyMethod * findMethod(EndpointType type, uint32_t messageId) const;

	void setIndex(int index);
	void setClientMailbox(class MailboxClass * mailbox);
	void rebind(std::vector<PyMethod *> & bindings, EndpointType type, PyMethod::Handler handler);

	template <typename _T>
	void expose(std::string const & parentModule)
	{
		SGW_ASSERT(!isInterface_);

		std::string modName = parentModule + "." + name_;
		bp::object module = bp::import(modName.c_str());
		bp::object cls = module.attr(name_.c_str());

		if (!cls)
		{
			FAULT("No Python class found for entity %s", name_.c_str());
			PyErr_Clear();
			return;
		}

		class_ = cls;
	
		bp::object parentMod = bp::import(parentModule.c_str());
		SGW_ASSERT(parentMod);
		// Create Python mailbox classes for base, cell & client
		baseMailbox_ = new ClientMailboxClass(parentMod, this, Service::BaseMailbox);
		cellMailbox_ = new ClientMailboxClass(parentMod, this, Service::CellMailbox);
		clientMailbox_ = new ClientMailboxClass(parentMod, this, Service::ClientMailbox);
	}

	/*
	 * Creates an instance of the Python class defined by this ClassDef
	 */
	template <typename _T>
	typename _T::Ptr create(uint32_t entityId, int32_t dbid = -1)
	{
		PyUtil_AssertGIL();
		if (!class_)
		{
			WARN("Entity '%s' has no python implementation", name_.c_str());
			return typename _T::Ptr();
		}

		bp::object entity;
		try
		{
			entity = class_();
		}
		catch (bp::error_already_set &)
		{
			// This is handled by the (!entity) condition
		}

		if (!entity)
		{
			FAULT("Failed to create entity '%s':", name_.c_str());
			PyUtil_ShowErr();
			return typename _T::Ptr();
		}
		
		typename _T::Ptr obj = bp::extract<typename _T::Ptr>(entity);
		obj->setObject(entity);
		obj->setup(entityId, this);

		if (dbid != -1)
		{
			if (!obj->load(dbid))
			{
				return typename _T::Ptr();
			}
		}
	
		return obj;
	}

private:
	PyClassDef(tinyxml2::XMLElement & root, std::string const & name, bool isInterface, bool cell);

	void inherit(PyClassDef * cls);
	PyMethod * findMethod(EndpointType type, std::string const & name);
	
	int index_;
	std::string name_;
	bool isInterface_;
	bool serverOnly_;
	PyClassDef * parent_;
	bool persistent_;
	bool hasIdentifier_;
	std::vector<PyClassDef *> interfaces_;
	// Is the def loaded on a cell?
	bool cell_;

	class MailboxClass * baseMailbox_, * cellMailbox_, * clientMailbox_;
	bp::object class_;
	
	std::map<std::string, PyMethod *> baseMethods_;
	std::map<std::string, PyMethod *> cellMethods_;
	std::map<std::string, PyMethod *> clientMethods_;
	
	std::vector<PyMethod *> internalBaseMethods_;
	std::vector<PyMethod *> internalCellMethods_;
	std::vector<PyMethod *> exposedBaseMethods_;
	std::vector<PyMethod *> exposedCellMethods_;
	std::vector<PyMethod *> exposedClientMethods_;
};

class PyTypeDb
{
public:
	static PyTypeDb & instance();

	void initialize(std::string const & entitiesPath, bool cell);

	template <typename _T>
	void setupClasses(std::string const & parentModule)
	{
		std::vector<PyClassDef *> defs(256, nullptr);
		for (auto it = classes_.begin(); it != classes_.end(); ++it)
		{
			if (!it->second->isInterface() && it->second->index() != -1)
				defs[it->second->index()] = it->second;
		}
		for (int i = (int)defs.size() - 1; i >= 0; i--)
		{
			if (defs[i])
				defs[i]->expose<_T>(parentModule);
		}
	}

	void registerType(std::string const & name, PyTypeInfo * type);
	void registerClass(std::string const & name, bool isInterface, PyClassDef * cls);
	void registerConstant(std::string const & enumeration, std::string const & name, bp::object value);

	PyTypeInfo * findType(std::string const & name);
	PyClassDef * findClass(std::string const & name, bool isInterface);
	PyClassDef * findClass(int classId);
	PyClassDef * loadClass(std::string const & name, bool isInterface);
	std::multimap<std::string, PyClassDef *> const & classes() const;

private:
	PyTypeDb();

	void loadAliases(std::string const & path);
	void loadEntities();
	void loadEnumerations(std::string const & path);

	std::string entitiesPath_;
	std::map<std::string, PyTypeInfo *> types_;
	std::multimap<std::string, PyClassDef *> classes_;
	bool cell_;
	bp::object enumsModule_;
	bp::object enumNamesModule_;

	static PyTypeDb * instance_;
};

