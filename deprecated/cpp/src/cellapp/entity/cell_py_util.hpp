#pragma once

#include <entity/python.hpp>

void PyUtil_CreateCellPlayer(uint32_t entityId);
bp::object PyUtil_CreateCellEntity(std::string const & className, uint32_t dbid);
bool PyUtil_DestroyCellEntity(uint32_t entityId);
uint32_t PyUtil_CreateSpace(std::string const & worldName, uint32_t creatorId);
void PyUtil_DestroySpace(uint32_t spaceId);
uint32_t PyUtil_FindSpace(std::string const & worldName);
bp::object PyUtil_FindEntity(uint32_t entityId);
bp::object PyUtil_FindEntityOnSpace(uint32_t entityId, uint32_t spaceId);
bp::object PyUtil_FindEntitiesByDbid(int32_t databaseId, uint32_t spaceId);
bp::object PyUtil_FindEntities(uint32_t spaceId);
double PyUtil_GetGameTime();
uint32_t PyUtil_AddTimer(double completeTime, bp::object callback);
void PyUtil_CancelTimer(uint32_t timerId);
bp::object PyUtil_FindPath(uint32_t spaceId, Vector3::Ptr start, Vector3::Ptr destination);
bp::object PyUtil_FindDetailedPath(uint32_t spaceId, Vector3::Ptr start, Vector3::Ptr destination, 
	Vector3::Ptr startExtents, Vector3::Ptr destinationExtents, float minDistance,
	float maxDistance, unsigned int maxPolys, float smoothingStepSize);
void PyUtil_ReloadClasses();


