#pragma once

#include <baseapp/entity/base_entity.hpp>

bp::object PyUtil_CreateBaseEntity(std::string const & cls);
bp::object PyUtil_CreateBaseEntityFromDb(std::string const & cls, int32_t dbid);
void PyUtil_CreateCellEntity(uint32_t entityId, uint32_t spaceId, uint32_t dbid, 
	float x, float y, float z, float yaw, float pitch, float roll,
	std::string const & worldName);
bp::object PyUtil_FindAllEntities();
void PyUtil_SwitchEntity(uint32_t entityId, uint32_t newEntityId);
void PyUtil_SendResource(BaseEntity::Ptr entity, uint32_t categoryId, uint32_t elementId);

std::string PyUtil_RegisterMinigameSession(uint32_t entityId, std::string const & gameName, uint32_t difficulty,
	uint32_t techCompetency, uint32_t seed, uint32_t abilitiesMask, uint32_t intelligence, uint32_t playerLevel, 
	bp::object callback);
bool PyUtil_CancelMinigameSession(uint32_t entityId);

double PyUtil_GetGameTime();
uint32_t PyUtil_AddTimer(double completeTime, bp::object callback);
void PyUtil_CancelTimer(uint32_t timerId);

void PyUtil_ReloadClasses();
