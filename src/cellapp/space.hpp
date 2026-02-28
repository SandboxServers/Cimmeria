#pragma once

#include <entity/entity.hpp>
#include <cellapp/entity/cell_entity.hpp>
#include <common/vec3.hpp>
	
struct WorldData
{
	// World name / space name
	std::string worldName;
	// Does the world have multiple instances or one shared instance?
	bool instanced;
	// Extents of the world
	struct {
		int minX, minY, maxX, maxY;
	} extents;
};

class Space
{
public:
	Space(uint32_t spaceId, std::string const & worldName, uint32_t creatorId = 0);
	~Space();

	uint32_t spaceId() const;
	std::string const & worldName() const;
	std::map<uint32_t, CellEntity::Ptr> const & entities() const;
	void setCreatorId(uint32_t entityId);

	void findEntitiesByDbid(int32_t databaseId, std::vector<CellEntity::Ptr> & entities);

	void destroy();
	bool addEntity(uint32_t entityId, CellEntity::Ptr entity);
	bool removeEntity(uint32_t entityId);

	bool promoteToPlayer(uint32_t entityId);
	bool demoteToEntity(uint32_t entityId);
	void removePlayers();

	bool isValidPosition(Vec3 const & pos) const;
	bool hasNavigationData() const;
	class NavigationPath * findPath(struct NavigationQueryParams const & params);
	float findPathLength(struct NavigationQueryParams const & params);

	void tick(uint32_t gameTimeInTicks);

private:
	uint32_t spaceId_;
	// Area name
	std::string worldName_;
	// Entity that created this space
	uint32_t creatorId_;
	// Is the space being destroyed?
	bool destroyed_;
	// (entityId, entity) map of entities in the space
	std::map<uint32_t, CellEntity::Ptr> entities_;
	// (databaseId, [entity, ...]) map of entities in the space
	std::multimap<uint32_t, CellEntity::Ptr> dbEntities_;
	// (entityId, entity) map of players in the space
	// This borrows the object reference from entities_!
	std::map<uint32_t, CellEntity::Ptr> players_;
	// Navigation mesh for this space
	class NavigationMesh * navMesh_;
	// World information and extents
	WorldData const & world_;
};

class SpaceManager
{
public:
	typedef Mercury::TimerMgr<double> TimerMgr;

	// List of space events
	enum Event
	{
		// A new space was created
		Created,
		// A space was destroyed
		Deleted
	};
	
	typedef boost::function<void (Space *, Event)> EventCallback;

	static void initialize();
	static SpaceManager & instance();

	SpaceManager();
	~SpaceManager();
	
	std::vector<Space *> const & spaces() const;
	Space * create(std::string const & worldName, uint32_t creatorEntityId = 0);
	bool destroy(uint32_t spaceId);
	Space * get(uint32_t spaceId) const;
	Space * get(std::string const & worldName) const;
	WorldData const * getWorldInfo(std::string const & worldName) const;
	void removePlayers();

	void setEventHandler(EventCallback cb);
	void loadSpaces(std::string const & cellPath, std::string const & spacesPath);
	void initTicks(uint32_t time, uint32_t tickRate);

	bool isLocalId(uint32_t spaceId) const;
	uint32_t toGlobalId(uint32_t spaceId) const;

	uint32_t time() const;
	uint32_t tickRate() const;
	int idleUpdateTicks() const;

	TimerMgr::TimerId addEntityTimer(double completeTime, PyObject * callback);
	void cancelEntityTimer(TimerMgr::TimerId timerId);

private:
	static SpaceManager * instance_;

	// Callback for space creation and destruction events
	EventCallback eventCallback_;
	std::map<std::string, WorldData> worlds_;
	std::vector<Space *> spaces_;
	std::vector<uint32_t> freeIds_;
	// Current game time on all local spaces (in ticks)
	uint32_t time_;
	// Tickrate of cell (in Nub timer ticks)
	uint32_t tickRate_;
	// When was the last tick (in milliseconds)
	uint64_t lastTick_;
	// ID of tick timer
	Mercury::Nub::Timer::TimerId timerId_;
	// List of worlds that can only have one world instance
	std::map<std::string, Space *> worldSpaces_;
	// Game ticker (entities can queue events in it)
	TimerMgr timer_;
	// Amount of ticks to skip after an idle movement update (-1 disables idle movement updates)
	int idleUpdateTicks_;

	void updateTickTimer(uint64_t now, unsigned int ticks);
	void tick(Service::TimerMgr & mgr, uint64_t timerTime, uint64_t now);
	void expireEntityTimer(void * callback);
};

