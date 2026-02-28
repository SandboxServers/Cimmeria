#pragma once

#include <baseapp/mercury/cell_handler.hpp>
#include <baseapp/entity/cached_entity.hpp>
#include <entity/world_grid.hpp>

struct WorldData
{
	// World name / space name
	std::string worldName;
	// Does the world have multiple instances or one shared instance?
	bool instanced;
	// Cell instance handling this world
	Mercury::CellAppConnection * handler;
	// Extents of the world
	struct {
		int minX, minY, maxX, maxY;
	} extents;
};

struct SpaceData
{
	typedef WorldGrid<CachedEntity> Grid;

	inline SpaceData(uint32_t spaceId_, Grid::Params const & params)
		: spaceId(spaceId_), grid(params)
	{}

	uint32_t spaceId;
	// Cell instance handling this space
	Mercury::CellAppConnection * handler;
	WorldData * world;
	// Grid containing entities on this space
	Grid grid;
};

class CellManager
{
	public:
		static void initialize();
		inline static CellManager & instance()
		{
			SGW_ASSERT(instance_);
			return *instance_;
		}

		CellManager();
		~CellManager();

		bool registerCell(Mercury::CellAppConnection * cell);
		void unregisterCell(uint32_t cellId);
		Mercury::CellAppConnection * randomCell();
		/*
		 * Checks if there is at least one CellApp connected to the BaseApp
		 */
		bool hasCells();

		/*
		 * Registers a new space with the cell manager.
		 * Each world instance must be registered before players can enter it.
		 *
		 * @param cell      Cell instance handling this connection
		 * @param spaceId   Unique ID of the space instance
		 * @param worldName Map name / world name
		 */
		void registerSpace(Mercury::CellAppConnection * cell, uint32_t spaceId, std::string const & worldName);

		/*
		 * Unregisters a space instance.
		 * Every base entity on the space will be destroyed and players will be disconnected.
		 *
		 * @param spaceId   Unique ID of the space to unregister
		 */
		void unregisterSpace(uint32_t spaceId);
		SpaceData * findSpace(uint32_t spaceId);
		WorldData * findWorld(std::string const & worldName);
		void tickCell(uint32_t cellId, uint64_t time);
		uint32_t ticks();
		uint32_t tickRate();

	private:
		void loadSpaces(std::string const & configPath);

		std::map<uint32_t, Mercury::CellAppConnection *> cells_;
		std::map<uint32_t, SpaceData *> spaces_;
		std::map<std::string, WorldData> worlds_;
		uint64_t startTime_;
		uint32_t tickRate_;

		static CellManager * instance_;
};

