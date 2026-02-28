#pragma once

#include <cellapp/entity/cell_entity.hpp>

class MovementController
{
public:
	enum TickResult
	{
		// The controller did not move the entity
		NoMovement = 0,
		// The entity was moved in a predictable way
		// (velocity and/or direction was not changed)
		LinearMovement = 1,
		// Unpredictable movement change
		// Velocity/direction change, movement start/stop
		NonlinearMovement = 2,
		// Forces player to a certain location
		// Used if the movement controller and the client disagrees about
		// the actual location of the player
		ForcedMovement = 3
	};

	MovementController(CellEntity * entity);
	virtual ~MovementController();

	void deleteLater();
	inline bool isDeleting() const
	{
		return deleting_;
	}

	virtual void reset() = 0;
	virtual void tick(uint64_t time, uint32_t gameTimeInTicks) = 0;

private:
	bool deleting_;
};

class PlayerController : public MovementController
{
public:
	PlayerController(CellEntity * entity);
	virtual ~PlayerController();

	virtual void reset();
	virtual void tick(uint64_t time, uint32_t gameTimeInTicks);
	void playerUpdate(Vec3 & position, Vec3 & rotation, Vec3 & velocity, uint8_t flags);

private:
	// Entity that we're controlling
	CellEntity * entity_;
	// Last game tick
	uint32_t lastTick_;
	// Last time we received a position update from the client or updated the position via tick()
	uint64_t lastUpdateTime_;
};

class DebugController : public MovementController
{
public:
	DebugController(CellEntity * entity);
	virtual ~DebugController();

	virtual void reset();
	virtual void tick(uint64_t time, uint32_t gameTimeInTicks);

private:

	// Entity that we're controlling
	CellEntity * entity_;
	// Last game tick
	uint32_t lastTick_;
};

class WaypointController : public MovementController
{
public:
	WaypointController(CellEntity * entity);
	virtual ~WaypointController();

	virtual void reset();
	virtual void tick(uint64_t time, uint32_t gameTimeInTicks);

	void addWaypoint(Vec3 const & waypoint, bp::object callback);

private:
	void nextPathNode();
	bool nextWaypoint();
	void debugPath();
	void recalculateRotation();

	struct Waypoint
	{
		Vec3 point;
		bp::object callback;
	};

	// Entity that we're controlling
	CellEntity * entity_;
	// Last game tick
	uint32_t lastTick_;
	// Last position update received from client
	Vec3 lastPositionUpdate_;
	// Next waypoint in list
	int32_t nextWaypoint_;
	// List of waypoint to traverse
	std::vector<Waypoint> waypoints_;
	// Next point in navigation path
	int32_t nextPathPoint_;
	// Path to next waypoint
	class NavigationPath * path_;
	// Current destination point
	Vec3 currentDestination_;
	// Distance from current destination
	float currentDistance_;
};
