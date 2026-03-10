#pragma once

#include <entity/defs.hpp>
#include <entity/pyutil.hpp>
#include <mercury/nub.hpp>
#include <boost/python/class.hpp>
#include <boost/python/handle.hpp>

/*
 * This is the base class of all game entities (Python classes should extend Atrea.Entity)
 * Keeps track of mailboxes, handles RPC calls, replication/backup and persistency.
 */
class Entity
{
public:
	typedef std::shared_ptr<Entity> Ptr;

	Entity();
	~Entity();

	void setup(uint32_t entityId, PyClassDef * cls);
	void rpc(std::string const & method, bp::tuple args);
	void persist();
	bool load(int32_t databaseId);
	uint32_t entityId() const;
	int32_t databaseId() const;
	PyClassDef * classDef() const;
	virtual void destroy();

	void setObject(bp::object self);
	bp::object object() const;

protected:
	bp::object callMethod(std::string const & method);
	bp::object callMethod(std::string const & name, bp::object obj);

	template <typename _T>
	static void registerProperties(bp::class_<_T, typename _T::Ptr> & cls)
	{
		cls.def_readonly("entityId", &Entity::entityId_);
		cls.def_readwrite("databaseId", &Entity::databaseId_);
	}

private:
	virtual void removeFromManager();

	uint32_t entityId_;
	int32_t databaseId_;
	PyClassDef * class_;
	bp::object object_;
	bool destroyed_;
};



template <typename _T>
class EntityManager
{
public:
	EntityManager()
	{
	}

	~EntityManager()
	{
	}

	typename _T::Ptr create(std::string const & cls, int32_t dbid)
	{
		PyClassDef * def = PyTypeDb::instance().findClass(cls, false);
		if (!def)
			return typename _T::Ptr();

		return create(def, dbid);
	}

	typename _T::Ptr create(PyClassDef * cls, int32_t dbid)
	{
		uint32_t index;
		if (!freeIds_.empty())
		{
			index = *freeIds_.rbegin();
			SGW_ASSERT(!entities_[index]);
			freeIds_.pop_back();
		}
		else
		{
			index = (uint32_t)entities_.size();
			entities_.push_back(typename _T::Ptr());
		}

		PyGilGuard guard;
		typename _T::Ptr entity = cls->create<_T>(index + 1, dbid);
		if (entity)
		{
			entities_[index] = entity;
		}
		else
		{
			// Entity creation failed, reclaim ID
			freeIds_.push_back(index);
		}
		return entity;
	}

	void destroyed(typename _T::Ptr entity)
	{
		PyUtil_AssertGIL();
		uint32_t entityId = entity->entityId();
		SGW_ASSERT(entityId != 0 && entityId - 1 < entities_.size());
		SGW_ASSERT(entities_[entityId - 1] == entity);
	
		entities_[entityId - 1] = typename _T::Ptr();
		freeIds_.push_back(entityId - 1);
	}

	typename _T::Ptr get(uint32_t entityId) const
	{
		if (entityId != 0 && entityId <= entities_.size())
			return entities_[entityId - 1];
		else
			return typename _T::Ptr();
	}
	
	std::vector<typename _T::Ptr> const & getAllEntities()
	{
		return entities_;
	}

private:
	static EntityManager * instance_;

	std::vector<typename _T::Ptr> entities_;
	std::vector<uint32_t> freeIds_;
};

