#pragma once

#include "Recast.h"
#include "DetourCommon.h"
#include "DetourNavMesh.h"
#include "DetourNavMeshBuilder.h"
#include "DetourNavMeshQuery.h"
#include "common/vec3.hpp"

struct NavigationQueryParams
{
	NavigationQueryParams();

	Vec3 start;
	Vec3 destination;
	Vec3 startExtents;
	Vec3 destinationExtents;
	float minDistance, maxDistance;
	unsigned int maxLengthPolys;
	float smoothingStepSize;
};

class NavigationMesh
{
public:
	NavigationMesh();
	~NavigationMesh();

	bool load(std::string const & path);

	class NavigationPath * findPath(NavigationQueryParams const & params);
	float findPathLength(NavigationQueryParams const & params);

private:
	rcPolyMesh mesh_;
	rcPolyMeshDetail detail_;
	dtNavMesh * navMesh_;
	dtNavMeshQuery query_;
	dtQueryFilter filter_;
};

class NavigationPath
{
public:
	enum { MaxPathLength = 64 };
	const static unsigned int MaxSmoothingSteps = 256;

	~NavigationPath();
	
	void dump();
	void smooth(Vec3 & startingPoint, dtPolyRef startingPoly, Vec3 & dstPoint, dtPolyRef dstPoly, 
		dtNavMeshQuery & query, dtQueryFilter & filter, NavigationQueryParams const & params);

	Vec3 const * points() const;
	int pointsCount() const;

protected:
	NavigationPath();

	bool getSteerTarget(dtNavMeshQuery & query, dtQueryFilter & filter, const Vec3 & startPos, 
		const Vec3 & endPos, float minTargetDist, Vec3 & steerPos, unsigned char & steerPosFlag, 
		dtPolyRef & steerPosRef);
	void fixupCorridor(const dtPolyRef * visited, const int nvisited);
	bool lineSphereIntersect(Vec3 const & center, float radius, Vec3 const & origin, Vec3 const & direction, Vec3 & result);

	dtPolyRef polys_[MaxPathLength];
	int pathLength_;
	Vec3 smoothPath_[MaxSmoothingSteps];
	int smoothLength_;

	friend class NavigationMesh;
};