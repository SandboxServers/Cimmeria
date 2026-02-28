#include <stdafx.hpp>
#include <cellapp/entity/movement.hpp>
#include <cellapp/entity/navigation.hpp>
#include <cellapp/space.hpp>
#include <cellapp/cell_service.hpp>
#include <cellapp/base_client.hpp>

MovementController::MovementController(CellEntity * entity)
	: deleting_(false)
{
}

MovementController::~MovementController()
{
}

void MovementController::deleteLater()
{
	deleting_ = true;
}

PlayerController::PlayerController(CellEntity * entity)
	: MovementController(entity), entity_(entity), lastTick_(0)
{
}

PlayerController::~PlayerController()
{
}

void PlayerController::reset()
{
	lastUpdateTime_ = Service::instance().microTime();
	lastTick_ = SpaceManager::instance().time();
}

void PlayerController::tick(uint64_t time, uint32_t gameTimeInTicks)
{
	// TODO: Add partial ticking
	// TODO: Add lag compensation for clients
	// TODO: Tick jumping players properly

	// Don't tick the entity if it isn't moving
	if (entity_->velocity().length() < 0.0001)
		return;

	float timeMul = (float)(time - lastUpdateTime_) / 1000;
	entity_->position() = entity_->position() + entity_->velocity() * timeMul;
	entity_->updatedPosition(CellEntity::ChangedPosition);
	lastTick_ = gameTimeInTicks;
	lastUpdateTime_ = time;
}

void PlayerController::playerUpdate(Vec3 & position, Vec3 & rotation, Vec3 & velocity, uint8_t flags)
{
	if (!entity_->space()->isValidPosition(position))
	{
		WARN("Player %d sent invalid position (%f, %f, %f)", 
			entity_->entityId(), position.x, position.y, position.z);
		return;
	}

	entity_->position() = position;
	entity_->rotation() = rotation;
	entity_->velocity() = velocity;
	entity_->setMovementType((flags == 1) ? 1 : 0);
	lastUpdateTime_ = Service::instance().microTime();
	// TODO: Set proper update flags
	entity_->updatedPosition(CellEntity::ChangedAll);
}


DebugController::DebugController(CellEntity * entity)
	: MovementController(entity), entity_(entity), lastTick_(0)
{
	entity_->velocity() = Vec3(0.5f, 0.0f, 0.0f);
}

DebugController::~DebugController()
{
}

void DebugController::reset()
{
	lastTick_ = SpaceManager::instance().time();
}

void DebugController::tick(uint64_t time, uint32_t gameTimeInTicks)
{
	float timeMul = (float)(gameTimeInTicks - lastTick_) * (float)SpaceManager::instance().tickRate() / 1000;
	TRACE("DebugCtrl: ticked %d, timeMul %f", gameTimeInTicks - lastTick_, timeMul);
	lastTick_ = gameTimeInTicks;
	entity_->position() = entity_->position() + entity_->velocity() * timeMul;
	entity_->updatedPosition(CellEntity::ChangedPosition);
}

WaypointController::WaypointController(CellEntity * entity)
	: MovementController(entity), entity_(entity), nextWaypoint_(-1), path_(nullptr)
{
}

WaypointController::~WaypointController()
{
	delete path_;

	// Need to acquire the GIL here to delete the python callback objects attached to the waypoints
	PyGilGuard guard;
	waypoints_.clear();
}

void WaypointController::reset()
{
	lastTick_ = SpaceManager::instance().time();
}

void WaypointController::tick(uint64_t time, uint32_t gameTimeInTicks)
{
	bool linear = true;
	if (nextWaypoint_ == -1)
	{
		nextPathNode();
		linear = false;
	}

	if (nextWaypoint_ >= (int)waypoints_.size())
	{
		TRACE("Waypoint controller reached target; destroying myself");
		entity_->velocity() = Vec3();
		entity_->updatedPosition(CellEntity::ChangedAll);
		deleteLater();
		return;
	}

	// Time multiplier
	float timeMul = (float)(gameTimeInTicks - lastTick_) * (float)SpaceManager::instance().tickRate() / 1000;
	currentDistance_ -= entity_->velocity().length() * timeMul;
	if (currentDistance_ > 0.01)
		entity_->position() = entity_->position() + entity_->velocity() * timeMul;
	else
	{
		float diff = abs(currentDistance_ / entity_->velocity().length() * 1000);
		TRACE("Requesting next waypoint from wplist; dropped time: %f ms", diff);
		entity_->position() = currentDestination_;
		nextPathNode();
		linear = false;
	}

	lastTick_ = gameTimeInTicks;
	if (linear)
		entity_->updatedPosition(CellEntity::ChangedPosition);
	else
		entity_->updatedPosition(CellEntity::ChangedAll);
}

/*
 * Adds a new waypoint to the list of waypoints for this controller.
 * The optional callback function is called when the waypoint is reached.
 */
void WaypointController::addWaypoint(Vec3 const & waypoint, bp::object callback)
{
	PyUtil_AssertGIL();

	Waypoint pt;
	pt.point = waypoint;
	pt.callback = callback;
	waypoints_.push_back(pt);
}

void WaypointController::nextPathNode()
{
	float maxSpeed = entity_->maxSpeed();

	if (entity_->space()->hasNavigationData())
	{
		/*
		 * Navmesh-based navigation has two waypoint levels:
		 * the first level consists of user-specified waypoints (the waypoints_ array)
		 * the second level is built by the navigation code itself (the path_ array)
		 * When the entity reaches a waypoint, it may reach a path point or a real waypoint, 
		 * so we need to handle both.
		 */
		if (!path_ || ++nextPathPoint_ >= path_->pointsCount())
		{
			// We ran out of path nodes, jump to next waypoint
			if (!nextWaypoint())
				return;
			
			// Create a path to the next waypoint in the list
			TRACE("Planning entity path ...");
			NavigationQueryParams params;
			params.start = entity_->position();
			params.destination = waypoints_[nextWaypoint_].point;
			path_ = entity_->space()->findPath(params);
			nextPathPoint_ = 0;
			debugPath();
			if (!path_ || !path_->pointsCount())
				return;
		}
		
		// Start moving to the next path point in a straight line
		Vec3 const & p = path_->points()[nextPathPoint_];
		TRACE("Moving to next path node %d (%f, %f, %f)", nextPathPoint_, p.x, p.y, p.z);
		currentDestination_ = p;
		currentDistance_ = (currentDestination_ - entity_->position()).length();
		Vec3 & velocity = entity_->velocity();
		if (currentDistance_ > 0.01f)
		{
			velocity = (currentDestination_ - entity_->position()).normalized() * maxSpeed;
			TRACE("Velocity is (%f, %f, %f)", velocity.x, velocity.y, velocity.z);
			recalculateRotation();
		}
		else
			velocity = Vec3();
	}
	else
	{
		// We ran out of path nodes, jump to next waypoint
		if (!nextWaypoint())
			return;
		
		// Start moving to the next waypoint in a straight line
		TRACE("Space has no navigation data, setting a direct path");
		currentDestination_ = waypoints_[nextWaypoint_].point;
		currentDistance_ = (currentDestination_ - entity_->position()).length();
		entity_->velocity() = (currentDestination_ - entity_->position()).normalized() * maxSpeed;
		recalculateRotation();
	}
}

bool WaypointController::nextWaypoint()
{
	// Only check for callback if we already passed a waypoint
	if (nextWaypoint_ != -1)
	{
		// If the waypoint has a Python callback, execute the callback
		auto cb = waypoints_[nextWaypoint_].callback;
		if (!cb.is_none())
		{
			// FIXME: Is acquiring the GIL here OK?
			PyGilGuard guard;
			try
			{
				cb();
			}
			catch (bp::error_already_set &)
			{
				FAULT("Python exception while calling waypoint controller callback:");
				PyUtil_ShowErr();
			}
		}
	}

	if (++nextWaypoint_ >= (int)waypoints_.size())
	{
		TRACE("Ran out of waypoints!");
		debugPath();
		return false;
	}

	return true;
}

void WaypointController::debugPath()
{
	PyGilGuard guard;
	bp::list l;
	if (path_)
	{
		Vec3 const * pts = path_->points();
		int length = path_->pointsCount();
		for (int i = 0; i < length; i++)
		{
			l.append(Vector3::Ptr(new Vector3(pts[i].x, pts[i].y, pts[i].z)));
		}
	}

	try
	{
		auto pathingDebug = bp::import("cell").attr("pathingDebug");
		pathingDebug(entity_->space()->spaceId(), entity_->entityId(), l);
	}
	catch (bp::error_already_set &)
	{
		WARN("Exception while calling 'pathingDebug':");
		PyUtil_ShowErr();
	}
}

void WaypointController::recalculateRotation()
{
	Vec3 dir(entity_->position() - currentDestination_);
	dir.y = 0.0f;
	dir.normalize();
	entity_->rotation().y = acos(-dir.z);
	if (dir.x > 0)
		entity_->rotation().y = 2*3.14159f - entity_->rotation().y;
	TRACE("Rotation: (%f, %f) -> angle: %f", dir.x, dir.z, entity_->rotation().y);

	// Ignore X and Z rotation (looks weird for most entities and official didn't use them either)
	entity_->rotation().x = 0.0f;
	entity_->rotation().z = 0.0f;
}
