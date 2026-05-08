#include <stdafx.hpp>
#include <baseapp/cell_manager.hpp>
#include <common/service.hpp>
#include <boost/lexical_cast.hpp>

CellManager * CellManager::instance_ = nullptr;

void CellManager::initialize()
{
	SGW_ASSERT(!instance_);
	instance_ = new CellManager();
}

CellManager::CellManager()
{
	startTime_ = Service::instance().microTime();
	tickRate_ = boost::lexical_cast<unsigned int>(Service::instance().getConfigParameter("tick_rate"));
	loadSpaces("../entities/spaces.xml");
}

CellManager::~CellManager()
{
}

bool CellManager::registerCell(Mercury::CellAppConnection * cell)
{
	auto it = cells_.find(cell->cellId());
	if (it != cells_.end())
	{
		WARN("Cell %d already registered", cell->cellId());
		return false;
	}

	cells_.insert(std::make_pair(cell->cellId(), cell));
	return true;
}

void CellManager::unregisterCell(uint32_t cellId)
{
	auto it = cells_.find(cellId);
	if (it == cells_.end())
	{
		WARN("Cell not registered: %d", cellId);
		return;
	}

	// Also remove all spaces belonging to this cell
	for (auto spcIt = spaces_.begin(); spcIt != spaces_.end();)
	{
		if (spcIt->second->handler == it->second)
		{
			std::vector<BaseEntity::Ptr> entities;
			BaseEntityManager<BaseEntity>::instance().getEntities(spcIt->first, entities);

			for (auto entIt = entities.begin(); entIt != entities.end(); ++entIt)
			{
				/*
				 * If the entity has a controller, disconnect the controller as well
				 * (this will kill the client connection)
				 */
				if ((*entIt)->controller())
				{
					TRACE("Disconnecting controller from entity %d", (*entIt)->entityId());
					(*entIt)->controller()->disconnectEntity(true);
				}
			}

			uint32_t spaceId = spcIt->first;
			spcIt++;
			unregisterSpace(spaceId);
		}
		else
			++spcIt;
	}

	cells_.erase(it);
}

Mercury::CellAppConnection * CellManager::randomCell()
{
	// FIXME: Currently this only returns the first cell ID;
	// cell distribution should be randomized by load
	if (!cells_.empty())
		return cells_.begin()->second;
	else
		return nullptr;
}

/*
 * Checks if there is at least one CellApp connected to the BaseApp
 */
bool CellManager::hasCells()
{
	return !cells_.empty();
}

/*
 * Registers a new space with the cell manager.
 * Each world instance must be registered before players can enter it.
 *
 * @param cell      Cell instance handling this connection
 * @param spaceId   Unique ID of the space instance
 * @param worldName Map name / world name
 */
void CellManager::registerSpace(Mercury::CellAppConnection * cell, uint32_t spaceId, std::string const & worldName)
{
	auto it = spaces_.find(spaceId);
	if (it != spaces_.end())
	{
		WARN("Space %d already registered to cell %d (attempted to re-register to %d)", 
			spaceId, it->second->handler->cellId(), cell->cellId());
		return;
	}

	auto worldIt = worlds_.find(worldName);
	if (worldIt == worlds_.end())
	{
		WARN("Cannot register space %d: world name '%s' not registered", 
			spaceId, worldName.c_str());
		return;
	}

	if (!worldIt->second.instanced)
	{
		if (worldIt->second.handler)
		{
			WARN("Space '%s' already exists on cell %d", 
				worldIt->second.handler->cellId(), worldName.c_str());
			return;
		}
		worldIt->second.handler = cell;
	}

	SpaceData::Grid::Params params;
	params.visionDistance = atol(Service::instance().getConfigParameter("grid_vision_distance").c_str());
	params.hysteresis = atol(Service::instance().getConfigParameter("grid_hysteresis").c_str());
	params.sizeX = params.sizeY = atol(Service::instance().getConfigParameter("grid_chunk_size").c_str());
	params.minX = worldIt->second.extents.minX;
	params.minY = worldIt->second.extents.minY;
	params.width = worldIt->second.extents.maxX - worldIt->second.extents.minX;
	params.height = worldIt->second.extents.maxY - worldIt->second.extents.minY;

	SpaceData * space = new SpaceData(spaceId, params);
	space->handler = cell;
	space->world = &worldIt->second;
	spaces_.insert(std::make_pair(spaceId, space));
}

/*
 * Unregisters a space instance.
 * Every base entity on the space will be destroyed and players will be disconnected.
 *
 * @param spaceId   Unique ID of the space to unregister
 */
void CellManager::unregisterSpace(uint32_t spaceId)
{
	auto it = spaces_.find(spaceId);
	if (it != spaces_.end())
	{
		if (!it->second->world->instanced)
			it->second->world->handler = nullptr;
		delete it->second;
		spaces_.erase(it);
	}
	else
		WARN("Space not registered: %d", spaceId);
}

SpaceData * CellManager::findSpace(uint32_t spaceId)
{
	auto it = spaces_.find(spaceId);
	if (it != spaces_.end())
		return it->second;
	else
		return nullptr;
}

WorldData * CellManager::findWorld(std::string const & worldName)
{
	auto it = worlds_.find(worldName);
	if (it != worlds_.end())
		return &it->second;
	else
		return nullptr;
}

void CellManager::tickCell(uint32_t cellId, uint64_t time)
{
	for (auto it = spaces_.begin(); it != spaces_.end(); ++it)
	{
		if (it->second->handler->cellId() == cellId)
		{
			BaseEntityManager<BaseEntity>::instance().tick(it->first, time);
		}
	}
}

uint32_t CellManager::ticks()
{
	return (uint32_t)((Service::instance().microTime() - startTime_) / tickRate());
}

uint32_t CellManager::tickRate()
{
	return tickRate_;
}

/*
 * Loads settings for all worlds that can be loaded on cells
 */
void CellManager::loadSpaces(std::string const & configPath)
{
	tinyxml2::XMLDocument doc;
	int error = doc.LoadFile(configPath.c_str());
	if (error != tinyxml2::XML_NO_ERROR)
	{
		FAULT("Failed to parse base space list '%s': %d", configPath.c_str(), error);
		FAULT("Detail: %s", doc.GetErrorStr1());
		return;
	}

	tinyxml2::XMLElement * root = doc.RootElement();
	SGW_ASSERT(root);
	SGW_ASSERT(root->Name() == std::string("Spaces"));

	for (tinyxml2::XMLElement * space = root->FirstChildElement();
		space; space = space->NextSiblingElement())
	{
		const char * name = space->Attribute("WorldName");
		const char * minX = space->Attribute("MinX");
		const char * maxX = space->Attribute("MaxX");
		const char * minY = space->Attribute("MinY");
		const char * maxY = space->Attribute("MaxY");
		SGW_ASSERT(name);
		SGW_ASSERT(minX);
		SGW_ASSERT(maxX);
		SGW_ASSERT(minY);
		SGW_ASSERT(maxY);

		WorldData data;
		data.worldName = name;
		data.instanced = space->BoolAttribute("Instanced");
		data.handler = nullptr;
		data.extents.minX = atol(minX);
		data.extents.maxX = atol(maxX);
		data.extents.minY = atol(minY);
		data.extents.maxY = atol(maxY);
		SGW_ASSERT(data.extents.minX <= data.extents.maxX);
		SGW_ASSERT(data.extents.minY <= data.extents.maxY);
		worlds_.insert(std::make_pair(data.worldName, data));
	}
}
