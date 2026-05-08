#include <stdafx.hpp>
#include <cellapp/cell_service.hpp>
#include <cellapp/entity/cell_py_util.hpp>
#include <cellapp/entity/entity_manager.hpp>
#include <cellapp/entity/cell_entity.hpp>
#include <cellapp/entity/movement.hpp>
#include <cellapp/entity/navigation.hpp>
#include <cellapp/space.hpp>

void PyUtil_CreateCellPlayer(uint32_t entityId)
{
	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);

	if (!entity)
	{
		WARN("Cannot create cell entity for unknown entityId %d", entityId);
		return;
	}

	entity->createCellPlayer();
}

bp::object PyUtil_CreateCellEntity(std::string const & className, uint32_t dbid)
{
	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().create(className, dbid);
	if (entity)
	{
		return entity->object();
	}
	else
		throw std::runtime_error("Failed to create entity");
}

bool PyUtil_DestroyCellEntity(uint32_t entityId)
{
	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);

	if (!entity)
	{
		WARN("Cannot delete unknown cell entity %d", entityId);
		return false;
	}

	entity->destroy();
	return true;
}

uint32_t PyUtil_CreateSpace(std::string const & worldName, uint32_t creatorId)
{
	Space * space = SpaceManager::instance().create(worldName, creatorId);
	if (space)
		return space->spaceId();
	else
		throw std::runtime_error("Failed to create space");
}

void PyUtil_DestroySpace(uint32_t spaceId)
{
	SpaceManager::instance().destroy(spaceId);
}

uint32_t PyUtil_FindSpace(std::string const & worldName)
{
	Space * space = SpaceManager::instance().get(worldName);
	if (space)
		return space->spaceId();
	else
		return 0xffffffff;
}

bp::object PyUtil_FindEntity(uint32_t entityId)
{
	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (!entity)
		return bp::object();

	return entity->object();
}

bp::object PyUtil_FindEntityOnSpace(uint32_t entityId, uint32_t spaceId)
{
	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (!entity)
		return bp::object();

	if (!entity->space() || entity->space()->spaceId() != spaceId)
		return bp::object();

	return entity->object();
}

bp::object PyUtil_FindEntitiesByDbid(int32_t databaseId, uint32_t spaceId)
{
	Space * space = SpaceManager::instance().get(spaceId);
	if (!space)
	{
		WARN("Cannot find entity on nonexistent space %d", spaceId);
		return bp::object();
	}

	bp::list objects;
	std::vector<CellEntity::Ptr> entities;
	space->findEntitiesByDbid(databaseId, entities);
	for (auto it = entities.begin(); it != entities.end(); ++it)
		objects.append((*it)->object());
	
	return objects;
}

bp::object PyUtil_FindEntities(uint32_t spaceId)
{
	bp::list list;
	auto const & all = CellEntityManager<CellEntity>::instance().all();
	for (auto it = all.begin(); it != all.end(); ++it)
	{
		if (spaceId == 0xffffffff || it->second->space() && it->second->space()->spaceId() == spaceId)
			list.append(it->second->object());
	}
	return list;
}

double PyUtil_GetGameTime()
{
	// TODO: This loses about +/- half tick precision on average but doesnt require an additional syscall.
	// Should it be worked around by calling Service::time()?
	return (double)SpaceManager::instance().time() * SpaceManager::instance().tickRate() / 1000.0;
}

uint32_t PyUtil_AddTimer(double completeTime, bp::object callback)
{
	return SpaceManager::instance().addEntityTimer(completeTime, callback.ptr());
}

void PyUtil_CancelTimer(uint32_t timerId)
{
	SpaceManager::instance().cancelEntityTimer(timerId);
}

bp::object PyUtil_FindPath(uint32_t spaceId, Vector3::Ptr start, Vector3::Ptr destination)
{
	Space * space = SpaceManager::instance().get(spaceId);
	if (!space)
	{
		WARN("No such spaceId: %d", spaceId);
		return bp::object();
	}

	NavigationQueryParams params;
	params.start = Vec3(start->x, start->y, start->z);
	params.destination = Vec3(destination->x, destination->y, destination->z);
	NavigationPath * path = space->findPath(params);
	if (!path)
		return bp::object();

	Vec3 const * pts = path->points();
	int length = path->pointsCount();
	bp::list l;
	for (int i = 0; i < length; i++)
	{
		l.append(Vector3::Ptr(new Vector3(pts[i].x, pts[i].y, pts[i].z)));
	}

	return l;
}

bp::object PyUtil_FindDetailedPath(uint32_t spaceId, Vector3::Ptr start, Vector3::Ptr destination, 
	Vector3::Ptr startExtents, Vector3::Ptr destinationExtents, float minDistance,
	float maxDistance, unsigned int maxPolys, float smoothingStepSize)
{
	Space * space = SpaceManager::instance().get(spaceId);
	if (!space)
	{
		WARN("No such spaceId: %d", spaceId);
		return bp::object();
	}
	
	NavigationQueryParams params;
	params.start = Vec3(start->x, start->y, start->z);
	params.destination = Vec3(destination->x, destination->y, destination->z);
	params.startExtents = Vec3(startExtents->x, startExtents->y, startExtents->z);
	params.destinationExtents = Vec3(destinationExtents->x, destinationExtents->y, destinationExtents->z);
	params.minDistance = minDistance;
	params.maxDistance = maxDistance;
	params.maxLengthPolys = maxPolys;
	params.smoothingStepSize = smoothingStepSize;
	NavigationPath * path = space->findPath(params);
	if (!path)
		return bp::object();

	Vec3 const * pts = path->points();
	int length = path->pointsCount();
	bp::list l;
	for (int i = 0; i < length; i++)
	{
		l.append(Vector3::Ptr(new Vector3(pts[i].x, pts[i].y, pts[i].z)));
	}

	return l;
}

void PyUtil_ReloadClasses()
{
	CellService::cell_instance().loadCellClasses();
}



