#pragma once

#include <entity/defs.hpp>
#include <entity/pyutil.hpp>
#include <mercury/nub.hpp>
#include <mercury/base_cell_messages.hpp>

template <typename _T>
class CellEntityManager
{
public:
	static void initialize(uint16_t cellId)
	{
		SGW_ASSERT(!instance_);
		instance_ = new CellEntityManager(cellId);
	}

	static CellEntityManager & instance()
	{
		SGW_ASSERT(instance_);
		return *instance_;
	}

	CellEntityManager(uint16_t cellId)
	{
		SGW_ASSERT(cellId < 0x100);
		firstId_ = Mercury::MinCellEntityId(cellId);
		lastId_ = Mercury::MaxCellEntityId(cellId);
		nextId_ = firstId_;
	}

	~CellEntityManager()
	{
	}


	/**
	 * Returns whether the specified entity ID was allocated by the local CellApp.
	 *
	 * @param entityId ID to check
	 */
	bool isLocalId(uint32_t entityId) const
	{
		return entityId >= firstId_ && entityId <= lastId_;
	}

	typename _T::Ptr create(uint32_t entityId, std::string const & cls, int32_t dbid)
	{
		PyClassDef * def = PyTypeDb::instance().findClass(cls, false);
		if (!def)
			return typename _T::Ptr();

		return create(entityId, def, dbid);
	}

	typename _T::Ptr create(uint32_t entityId, PyClassDef * cls, int32_t dbid)
	{
		SGW_ASSERT(entityId < Mercury::MinLocalEntityId);

		PyGilGuard guard;
		typename _T::Ptr entity = cls->create<_T>(entityId, dbid);
		if (entity)
		{
			SGW_ASSERT(entities_.find(entityId) == entities_.end());
			entities_.insert(std::make_pair(entityId, entity));
		}
		return entity;
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
		uint32_t id;
		if (!freeIds_.empty())
		{
			id = *freeIds_.rbegin();
			freeIds_.pop_back();
		}
		else
		{
			id = nextId_++;
			if (id > lastId_)
				throw std::runtime_error("Entity ID pool exhausted");
		}
		
		PyGilGuard guard;
		typename _T::Ptr entity = cls->create<_T>(id, dbid);
		if (entity)
		{
			SGW_ASSERT(entities_.find(id) == entities_.end());
			entities_.insert(std::make_pair(id, entity));
		}
		else
		{
			// Entity creation failed, reclaim ID
			freeIds_.push_back(id);
		}
		return entity;
	}

	void destroyed(typename _T::Ptr entity)
	{
		uint32_t entityId = entity->entityId();
		auto it = entities_.find(entityId);
		SGW_ASSERT(it != entities_.end());
		SGW_ASSERT(it->second == entity);
	
		entities_.erase(it);
		if (entityId >= firstId_ && entityId <= lastId_)
			freeIds_.push_back(entityId);
	}

	typename _T::Ptr get(uint32_t entityId) const
	{
		auto it = entities_.find(entityId);
		if (it != entities_.end())
			return it->second;
		else
			return typename _T::Ptr();
	}

	std::map<uint32_t, typename _T::Ptr> const & all() const
	{
		return entities_;
	}

private:
	static CellEntityManager * instance_;

	std::map<uint32_t, typename _T::Ptr> entities_;
	std::vector<uint32_t> freeIds_;
	// First and last entity ID that can be assigned by this allocator
	uint32_t firstId_, lastId_;
	uint32_t nextId_;
};

