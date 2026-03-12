#pragma once

#include <entity/entity.hpp>

template <typename _T>
class BaseEntityManager : public EntityManager<_T>
{
public:
	static void initialize()
	{
		SGW_ASSERT(!instance_);
		instance_ = new BaseEntityManager<_T>();
	}

	static BaseEntityManager<_T> & instance()
	{
		SGW_ASSERT(instance_);
		return *instance_;
	}

	void addToSpace(typename _T::Ptr entity, uint32_t spaceId)
	{
		spaceMap_.insert(std::make_pair(spaceId, entity));
	}

	void removeFromSpace(typename _T::Ptr entity, uint32_t spaceId)
	{
		for (auto it = spaceMap_.find(spaceId); it != spaceMap_.end() && it->first == spaceId; ++it)
		{
			if (it->second == entity)
			{
				spaceMap_.erase(it);
				return;
			}
		}

		WARN("Entity not on space %d!", spaceId);
	}

	void destroyed(typename _T::Ptr entity)
	{
		uint32_t spaceId = entity->spaceId();
		if (spaceId != 0xffffffff)
			removeFromSpace(entity, spaceId);

		EntityManager<_T>::destroyed(entity);
	}

	void tick(uint32_t spaceId, uint64_t time)
	{
		for (auto it = spaceMap_.find(spaceId); it != spaceMap_.end() && it->first == spaceId; ++it)
		{
			if (it->second->controller())
				it->second->controller()->gameTick(time);
		}
	}
	
	void getEntities(uint32_t spaceId, std::vector<typename _T::Ptr> & entities)
	{
		for (auto it = spaceMap_.find(spaceId); it != spaceMap_.end() && it->first == spaceId; ++it)
		{
			entities.push_back(it->second);
		}
	}

private:
	static BaseEntityManager<_T> * instance_;

	// (spaceId, entity) map
	std::multimap<uint32_t, typename _T::Ptr> spaceMap_;
};

template <typename _T>
BaseEntityManager<_T> * BaseEntityManager<_T>::instance_ = nullptr;