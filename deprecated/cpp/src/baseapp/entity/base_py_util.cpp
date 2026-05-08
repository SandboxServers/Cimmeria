#include <stdafx.hpp>
#include <baseapp/entity/base_py_util.hpp>
#include <baseapp/entity/base_entity.hpp>
#include <baseapp/entity/ticker.hpp>
#include <baseapp/mercury/cell_handler.hpp>
#include <baseapp/cell_manager.hpp>
#include <baseapp/minigame.hpp>
#include <baseapp/base_service.hpp>
#include <boost/lexical_cast.hpp>

bp::object PyUtil_CreateBaseEntity(std::string const & cls)
{
	return PyUtil_CreateBaseEntityFromDb(cls, -1);
}

bp::object PyUtil_CreateBaseEntityFromDb(std::string const & cls, int32_t dbid)
{
	BaseEntity::Ptr ent = BaseEntityManager<BaseEntity>::instance().create(cls, dbid);
	if (!ent)
	{
		// FIXME: This does not trigger as we return a non-null value
		PyErr_Format(PyExc_ValueError, "No such entity class: %s", cls.c_str());
		return bp::object();
	}

	return ent->object();
}

void PyUtil_CreateCellEntity(uint32_t entityId, uint32_t spaceId, uint32_t dbid, 
	float x, float y, float z, float yaw, float pitch, float roll,
	std::string const & worldName)
{
	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_ENTITY, "Couldn't create cell entity for nonexistent entityId %d", entityId);
		return;
	}

	Mercury::CellAppConnection * handler = nullptr;
	if (spaceId != 0xffffffff)
	{
		SpaceData * space = CellManager::instance().findSpace(spaceId);
		if (space)
		{
			handler = space->handler;
			if (space->world->worldName != worldName)
			{
				WARNC(CATEGORY_ENTITY, "createCellEntity(): world name '%s' differs from world '%s' of space %d",
					worldName.c_str(), spaceId, space->world->worldName.c_str());
			}
		}
	}
	else
	{
		WorldData * world = CellManager::instance().findWorld(worldName);
		if (world)
		{
			if (world->instanced)
				handler = CellManager::instance().randomCell();
			else
			{
				if (world->handler)
					handler = world->handler;
				else
				{
					WARNC(CATEGORY_ENTITY, "createCellEntity(): no space instance running for world '%s'",
						worldName.c_str());
				}
			}
		}
		else
		{
			WARNC(CATEGORY_ENTITY, "createCellEntity(): unknown world name '%s'", worldName.c_str());
		}
	}

	if (!handler)
	{
		WARNC(CATEGORY_ENTITY, "No cell handler found for spaceId %d", spaceId);
		entity->cellEntityCreateFailed();
		return;
	}

	Vec3 pos, rot;
	pos.x = x;
	pos.y = y;
	pos.z = z;
	rot.x = yaw;
	rot.y = pitch;
	rot.z = roll;
	handler->sendCreateEntity(entity.get(), spaceId, worldName, pos, rot);
}

bp::object PyUtil_FindAllEntities()
{
	bp::list list;
	auto const & all = BaseEntityManager<BaseEntity>::instance().getAllEntities();
	for (auto it = all.begin(); it != all.end(); ++it)
	{
		if (*it)
			list.append((*it)->object());
	}
	return list;
}

void PyUtil_SwitchEntity(uint32_t entityId, uint32_t newEntityId)
{
	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_ENTITY, "Couldn't switch from nonexistent entityId %d", entityId);
		return;
	}
	
	BaseEntity::Ptr newEntity = BaseEntityManager<BaseEntity>::instance().get(newEntityId);
	if (!newEntity)
	{
		WARNC(CATEGORY_ENTITY, "Couldn't switch to nonexistent entityId %d", newEntityId);
		return;
	}
	
	if (entity->controller())
	{
		entity->controller()->switchEntity(newEntity.get());
	}
	else
		WARNC(CATEGORY_ENTITY, "Entity %d doesn't have a controller", entityId);
}

void PyUtil_SendResource(BaseEntity::Ptr entity, uint32_t categoryId, uint32_t elementId)
{
	Controller * controller = entity->controller();
	if (controller)
		controller->sendResource(categoryId, elementId);
	else
		WARNC(CATEGORY_ENTITY, "Attempted to send resource to an entity without controller (%d)", 
			entity->entityId());
}

void PyUtil_MinigameCallback(Minigame * minigame, MinigameCompletionStatus status, bp::object callback)
{
	PyGilGuard guard;
	auto args = bp::make_tuple((int)status);
	try
	{
		callback(bp::object((long)status));
	}
	catch (bp::error_already_set &)
	{
		FAULTC(CATEGORY_ENTITY, "Error while calling minigame completion handler:");
		PyUtil_ShowErr();
	}
}

std::string PyUtil_RegisterMinigameSession(uint32_t entityId, std::string const & gameName, uint32_t difficulty,
	uint32_t techCompetency, uint32_t seed, uint32_t abilitiesMask, uint32_t intelligence, uint32_t playerLevel, 
	bp::object callback)
{
	MinigameRequestManager::QueueEntry * entry = BaseService::base_instance().minigameManager().queue(
		entityId, gameName, difficulty, techCompetency, seed, abilitiesMask, intelligence, playerLevel, 
		boost::bind(&PyUtil_MinigameCallback, _1, _2, callback));
	if (!entry)
		return "";

	return std::string("http://unused/") + Service::instance().getConfigParameter("minigame_external_address") + "/" +
		Service::instance().getConfigParameter("minigame_external_port") + "/" + gameName + "/" + 
		boost::lexical_cast<std::string>(entityId) + "/" + entry->key;
}

bool PyUtil_CancelMinigameSession(uint32_t entityId)
{
	return BaseService::base_instance().minigameManager().cancel(entityId);
}

double PyUtil_GetGameTime()
{
	// TODO: This loses about +/- half tick precision on average but doesnt require an additional syscall.
	// Should it be worked around by calling Service::time()?
	return (double)CellManager::instance().ticks() * CellManager::instance().tickRate() / 1000.0;
}

uint32_t PyUtil_AddTimer(double completeTime, bp::object callback)
{
	return Ticker::instance().addEntityTimer(completeTime, callback.ptr());
}

void PyUtil_CancelTimer(uint32_t timerId)
{
	Ticker::instance().cancelEntityTimer(timerId);
}

void PyUtil_ReloadClasses()
{
	PyTypeDb::instance().setupClasses<BaseEntity>("base");
}

