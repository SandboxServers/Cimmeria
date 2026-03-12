#include <stdafx.hpp>

#include <boost/python/detail/wrap_python.hpp>

#include <cellapp/space.hpp>
#include <cellapp/cell_service.hpp>
#include <cellapp/base_client.hpp>
#include <cellapp/entity/cell_entity.hpp>
#include <cellapp/entity/navigation.hpp>
#include <mercury/base_cell_messages.hpp>


Space::Space(uint32_t spaceId, std::string const & worldName, uint32_t creatorId)
	: spaceId_(spaceId), worldName_(worldName), creatorId_(creatorId), destroyed_(false),
	world_(*SpaceManager::instance().getWorldInfo(worldName))
{
	TRACEC(CATEGORY_SPACE, "Created space %d (worldName: %s, creatorId: %d)", 
		spaceId_, worldName_.c_str(), creatorId_);

	navMesh_ = new NavigationMesh();
	std::string navFileName = "../data/spaces/";
	navFileName += worldName_ + ".nav";
	if (!navMesh_->load(navFileName))
	{
		delete navMesh_;
		navMesh_ = nullptr;
	}
}

Space::~Space()
{
	delete navMesh_;
}

uint32_t Space::spaceId() const
{
	return spaceId_;
}

std::string const & Space::worldName() const
{
	return worldName_;
}

std::map<uint32_t, CellEntity::Ptr> const & Space::entities() const
{
	return entities_;
}

void Space::setCreatorId(uint32_t entityId)
{
	SGW_ASSERT(creatorId_ == 0 && "Space already has a creator entity!");
	creatorId_ = entityId;
}

void Space::findEntitiesByDbid(int32_t databaseId, std::vector<CellEntity::Ptr> & entities)
{
	for (auto dbit = dbEntities_.find(databaseId); dbit != dbEntities_.end() && dbit->first == (uint32_t)databaseId; ++dbit)
	{
		entities.push_back(dbit->second);
	}
}

void Space::destroy()
{
	if (!destroyed_)
	{
		TRACEC(CATEGORY_SPACE, "Destroying space %d", spaceId_);
		destroyed_ = true;
	}
}

bool Space::addEntity(uint32_t entityId, CellEntity::Ptr entity)
{
	if (destroyed_)
	{
		WARNC(CATEGORY_SPACE, "Cannot add entities to destroyed space %d!", spaceId_);
		return false;
	}

	if (entities_.find(entityId) != entities_.end())
	{
		WARNC(CATEGORY_SPACE, "Entity %d is already on space %d!", entityId, spaceId_);
		return false;
	}

	if (!isValidPosition(entity->position()))
	{
		Vec3 const & pos = entity->position();
		WARNC(CATEGORY_SPACE, "Tried to add entity %d to space %d at illegal position (%f, %f, %f)!",
			entityId, spaceId_, pos.x, pos.y, pos.z);
		return false;
	}
	
	entities_.insert(std::make_pair(entityId, entity));
	if (entity->databaseId() != CellEntity::InvalidDatabaseId)
	{
		dbEntities_.insert(std::make_pair(entity->databaseId(), entity));
	}
	return true;
}

bool Space::removeEntity(uint32_t entityId)
{
	auto it = entities_.find(entityId);
	if (it == entities_.end())
	{
		WARNC(CATEGORY_SPACE, "Cannot remove entity: entity %d is not on space %d!", entityId, spaceId_);
		return false;
	}

	auto player = players_.find(entityId);
	if (player != players_.end())
		players_.erase(player);

	int32_t dbid = it->second->databaseId();
	if (dbid != CellEntity::InvalidDatabaseId)
	{
		auto dbit = dbEntities_.find(dbid);
		SGW_ASSERT(dbit != dbEntities_.end());
		while (dbit != dbEntities_.end() && dbit->first == (uint32_t)dbid && dbit->second != it->second)
			++dbit;
		SGW_ASSERT(dbit != dbEntities_.end() && dbit->first == (uint32_t)dbid && dbit->second == it->second);
		dbEntities_.erase(dbit);
	}

	entities_.erase(it);

	if (entityId == creatorId_)
	{
		DEBUG2C(CATEGORY_SPACE, "Space creator has left space %d, destroying it", spaceId_);
		destroy();
	}
	return true;
}

/*
 * Promotes an entity to Player (client controlled) status.
 * The player will receive ticks and vision updates from the Cell,
 * and the entity gets a client RPC mailbox.
 */
bool Space::promoteToPlayer(uint32_t entityId)
{
	if (destroyed_)
	{
		WARNC(CATEGORY_SPACE, "Cannot add players to destroyed space %d!", spaceId_);
		return false;
	}

	auto it = entities_.find(entityId);
	if (it == entities_.end())
	{
		WARNC(CATEGORY_SPACE, "Cannot add player: entity %d is not on space %d!", entityId, spaceId_);
		return false;
	}

	if (players_.find(entityId) != players_.end())
	{
		WARNC(CATEGORY_SPACE, "Player %d is already on space %d!", entityId, spaceId_);
		return false;
	}

	players_.insert(std::make_pair(entityId, it->second));
	return true;
}

/*
 * Demotes an entity from Player (client controlled) status to entity (locally controlled).
 * The entity won't receive notifications via the BaseApp.
 */
bool Space::demoteToEntity(uint32_t entityId)
{
	auto it = players_.find(entityId);
	if (it == players_.end())
	{
		WARNC(CATEGORY_SPACE, "Player %d is not on space %d!", entityId, spaceId_);
		return false;
	}

	players_.erase(it);
	return true;
}

/*
 * Removes and destroys all players from the space
 * (both from the player list and the entity list)
 * This may destroy the space if the creator entity is removed.
 */
void Space::removePlayers()
{
	PyGilGuard guard;
	while (!players_.empty())
	{
		players_.begin()->second->destroy();
	}
}


void Space::tick(uint32_t gameTimeInTicks)
{
	// We query the time in each space separately to make sure that its
	// reasonably accurate (some drift creeps in by the time we get to
	// the last entity on the space anyway)
	uint64_t time = Service::instance().microTime();
	for (auto it = entities_.begin(); it != entities_.end(); ++it)
	{
		it->second->tick(time, gameTimeInTicks);
	}
}

/*
 * Checks if the specified position is inside the bounds of this space.
 */
bool Space::isValidPosition(Vec3 const & pos) const
{
	return pos.x >= world_.extents.minX && pos.x < world_.extents.maxX &&
		pos.z >= world_.extents.minY && pos.z < world_.extents.maxY;
}

/*
 * Returns true if navigation data (navmesh, heightmap) is available for this space.
 */
bool Space::hasNavigationData() const
{
	return (navMesh_ != nullptr);
}

/*
 * Tries to find a valid ground movement path for an entity using the spaces navmesh.
 */
NavigationPath * Space::findPath(NavigationQueryParams const & params)
{
	if (navMesh_)
		return navMesh_->findPath(params);
	else
	{
		WARNC(CATEGORY_SPACE, "No navigation data loaded for space '%s'", worldName().c_str());
		return nullptr;
	}
}

/**
 * Returns the length of a valid ground movement path for an entity using the spaces navmesh.
 * If no path was found or no navigation mesh is loaded the function returns -1.0f.
 */
float Space::findPathLength(NavigationQueryParams const & params)
{
	if (navMesh_)
		return navMesh_->findPathLength(params);
	else
		return -1.0f;
}

SpaceManager * SpaceManager::instance_ = nullptr;

void SpaceManager::initialize()
{
	SGW_ASSERT(!instance_);
	instance_ = new SpaceManager();
}

SpaceManager & SpaceManager::instance()
{
	SGW_ASSERT(instance_);
	return *instance_;
}

SpaceManager::SpaceManager()
	: time_(0)
{
	idleUpdateTicks_ = atol(Service::instance().getConfigParameter("idle_update_ticks").c_str());
}

SpaceManager::~SpaceManager()
{
	Service::instance().cancelTimer(timerId_);
}


/*
 * Returns a list of all currently running spaces
 */
std::vector<Space *> const & SpaceManager::spaces() const
{
	return spaces_;
}


Space * SpaceManager::create(std::string const & worldName, uint32_t creatorEntityId)
{
	uint32_t id;
	if (!freeIds_.empty())
	{
		id = *freeIds_.rbegin();
		SGW_ASSERT(spaces_[id] == nullptr);
		freeIds_.pop_back();
	}
	else
	{
		id = (uint32_t)spaces_.size();
		spaces_.push_back(nullptr);
	}

	Space * space = new Space(toGlobalId(id), worldName, creatorEntityId);
	spaces_[id] = space;

	auto worldIt = worldSpaces_.find(worldName);
	if (worldIt != worldSpaces_.end())
	{
		SGW_ASSERT(worldIt->second == nullptr);
		worldIt->second = space;
	}

	if (eventCallback_)
		eventCallback_(space, Created);

	try
	{
		PyGilGuard guard;
		auto spaceCreated = (bp::object)bp::import("cell").attr("spaceCreated");
		spaceCreated(space->spaceId(), worldName.c_str(), (worldIt == worldSpaces_.end()) ? 1 : 0);
	}
	catch (bp::error_already_set &)
	{
		WARNC(CATEGORY_SPACE, "Exception while calling 'spaceCreated':");
		PyUtil_ShowErr();
	}

	return space;
}

bool SpaceManager::destroy(uint32_t spaceId)
{
	uint32_t index = spaceId & 0xffff;
	if (!isLocalId(spaceId) || index >= spaces_.size() || !spaces_[index])
	{
		WARNC(CATEGORY_SPACE, "Cannot delete nonexistent space %d", spaceId);
		return false;
	}

	Space * space = spaces_[index];
	auto worldIt = worldSpaces_.find(space->worldName());

	try
	{
		PyGilGuard guard;
		auto spaceDestroyed = (bp::object)bp::import("cell").attr("spaceDestroyed");
		spaceDestroyed(space->spaceId(), space->worldName().c_str(), (worldIt == worldSpaces_.end()) ? 1 : 0);
	}
	catch (bp::error_already_set &)
	{
		WARNC(CATEGORY_SPACE, "Exception while calling 'spaceDestroyed':");
		PyUtil_ShowErr();
	}

	if (eventCallback_)
		eventCallback_(space, Deleted);

	if (worldIt != worldSpaces_.end())
	{
		SGW_ASSERT(worldIt->second == space);
		worldIt->second = nullptr;
	}

	space->destroy();
	spaces_[index] = nullptr;
	return true;
}

Space * SpaceManager::get(uint32_t spaceId) const
{
	uint32_t index = spaceId & 0xffff;
	if (!isLocalId(spaceId) || index >= spaces_.size())
		return nullptr;

	return spaces_[index];
}

Space * SpaceManager::get(std::string const & worldName) const
{
	auto it = worldSpaces_.find(worldName);
	if (it != worldSpaces_.end())
		return it->second;
	else
		return nullptr;
}

WorldData const * SpaceManager::getWorldInfo(std::string const & worldName) const
{
	auto it = worlds_.find(worldName);
	if (it != worlds_.end())
		return &it->second;
	else
		return nullptr;
}

/*
 * Removes all player entities from registered cells
 * (leaves non-player entities untouched)
 * This may delete spaces that were instantiated for players!
 */
void SpaceManager::removePlayers()
{
	for (unsigned int spaceId = 0; spaceId < spaces_.size(); spaceId++)
	{
		if (spaces_[spaceId] != nullptr)
			spaces_[spaceId]->removePlayers();
	}
}

void SpaceManager::setEventHandler(EventCallback cb)
{
	eventCallback_ = cb;
}

/*
 * Loads the list of worlds that must be instantiated on this cell instance.
 * Different cells may load a different set of spaces (but two cells shouldn't
 * instantiate the same world if the world is not marked as instanced on the BaseApp).
 */
void SpaceManager::loadSpaces(std::string const & cellPath, std::string const & spacesPath)
{
	tinyxml2::XMLDocument spcDoc;
	int error = spcDoc.LoadFile(spacesPath.c_str());
	if (error != tinyxml2::XML_NO_ERROR)
	{
		FAULT("Failed to parse base space list '%s': %d", spacesPath.c_str(), error);
		FAULT("Detail: %s", spcDoc.GetErrorStr1());
		return;
	}

	tinyxml2::XMLElement * root = spcDoc.RootElement();
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
		data.extents.minX = atol(minX);
		data.extents.maxX = atol(maxX);
		data.extents.minY = atol(minY);
		data.extents.maxY = atol(maxY);
		SGW_ASSERT(data.extents.minX <= data.extents.maxX);
		SGW_ASSERT(data.extents.minY <= data.extents.maxY);
		worlds_.insert(std::make_pair(data.worldName, data));
	}


	tinyxml2::XMLDocument doc;
	error = doc.LoadFile(cellPath.c_str());
	if (error != tinyxml2::XML_NO_ERROR)
	{
		FAULTC(CATEGORY_SPACE, "Failed to parse cell space list '%s': %d", cellPath.c_str(), error);
		FAULTC(CATEGORY_SPACE, "Detail: %s", doc.GetErrorStr1());
		return;
	}

	root = doc.RootElement();
	SGW_ASSERT(root);
	SGW_ASSERT(root->Name() == std::string("Spaces"));

	for (tinyxml2::XMLElement * space = root->FirstChildElement();
		space; space = space->NextSiblingElement())
	{
		const char * name = space->Attribute("WorldName");
		SGW_ASSERT(name);
		auto it = worlds_.find(name);
		if (it == worlds_.end())
		{
			FAULTC(CATEGORY_SPACE, "Unknown space '%s' in cell space configuration", name);
			continue;
		}
		
		if (it->second.instanced)
		{
			FAULTC(CATEGORY_SPACE, "Cannot create instanced space '%s' on startup", name);
			continue;
		}

		INFOC(CATEGORY_SPACE, "Creating local space for '%s'", name);
		worldSpaces_.insert(std::make_pair(name, nullptr));
		create(name);
	}
}

void SpaceManager::initTicks(uint32_t time, uint32_t tickRate)
{
	time_ = time;
	// TODO: Improve ticking to use ASIO timers instead of service timers (?)
	tickRate_ = tickRate;
	uint64_t now = Service::instance().microTime();
	lastTick_ = now;
	updateTickTimer(now, 1);
}

/*
 * Checks if the space ID belongs to this cell
 */
bool SpaceManager::isLocalId(uint32_t spaceId) const
{
	return (spaceId >> 16) == CellService::cell_instance().cellId();
}

/*
 * Converts a local space index to a global space ID
 */
uint32_t SpaceManager::toGlobalId(uint32_t spaceId) const
{
	SGW_ASSERT(!(spaceId & 0xffff0000));
	return spaceId | (CellService::cell_instance().cellId() << 16);
}

uint32_t SpaceManager::time() const
{
	return time_;
}

uint32_t SpaceManager::tickRate() const
{
	return tickRate_;
}

int SpaceManager::idleUpdateTicks() const
{
	return idleUpdateTicks_;
}

SpaceManager::TimerMgr::TimerId SpaceManager::addEntityTimer(double completeTime, PyObject * callback)
{
	PyUtil_AssertGIL();
	Py_INCREF(callback);
	return timer_.addTimer(
		[this](auto &, auto, auto, void * ud) { expireEntityTimer(ud); },
		completeTime, callback);
}

void SpaceManager::cancelEntityTimer(SpaceManager::TimerMgr::TimerId timerId)
{
	if (timer_.isValidTimer(timerId))
	{
		PyUtil_AssertGIL();
		PyObject * obj = reinterpret_cast<PyObject *>(timer_.cancelTimer(timerId));
		Py_DECREF(obj);
	}
	else
		WARNC(CATEGORY_SPACE, "Attempted to cancel nonexistent timer %d", timerId);
}

/*
 * Updates the space timer to tick at the next timeslot.
 */
void SpaceManager::updateTickTimer(uint64_t now, unsigned int ticks)
{
	// Calculate timer drift (difference from expected tick time)
	int drift = (int)(lastTick_ - now);
	// If the drift is more than two ticks (we can't continue ticking normally
	// because the completion time of the next tick has already passed) we need to skip
	// some ticks to be able to continue ticking normally
	if (drift <= -(int)(tickRate_ * 2))
	{
		unsigned int skipTicks = -drift / tickRate_;
		drift += skipTicks * tickRate_;
		WARNC(CATEGORY_SPACE, "Game tick skipped! Ticked at %d, expected %d (drift %d ms, skipped %d ticks)", 
			(int32_t)now, (int32_t)lastTick_, drift, skipTicks);
		lastTick_ += skipTicks * tickRate_;
		time_ += skipTicks;
	}
	timerId_ = Service::instance().addTimer(
		[this](auto & mgr, uint64_t timerTime, uint64_t now, auto *) { tick(mgr, timerTime, now); },
		now + (ticks * tickRate_) + drift);
	lastTick_ += ticks * tickRate_;
}

void SpaceManager::tick(Service::TimerMgr & /*mgr*/, uint64_t /*timerTime*/, uint64_t now)
{
	updateTickTimer(now, 1);
	time_++;
	BaseAppClient * client = CellService::cell_instance().baseClient();
	if (client)
		client->sendGameTick(time_);
	for (unsigned int spaceId = 0; spaceId < spaces_.size(); spaceId++)
	{
		if (spaces_[spaceId] != nullptr)
			spaces_[spaceId]->tick(time_);
	}
	timer_.tick((double)time_ * tickRate_ / 1000.0);
}

void SpaceManager::expireEntityTimer(void * callback)
{
	PyGilGuard guard;
	PyObject * obj = reinterpret_cast<PyObject *>(callback);
	PyObject * args = PyTuple_New(0);
	PyObject * result = PyObject_Call(obj, args, nullptr);
	Py_XDECREF(result);
	Py_DECREF(args);
	Py_DECREF(obj);
	if (result == nullptr)
	{
		FAULTC(CATEGORY_SPACE, "Error while calling timer handler:");
		PyUtil_ShowErr();
	}
}

