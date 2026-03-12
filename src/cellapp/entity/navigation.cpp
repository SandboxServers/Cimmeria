#include "stdafx.hpp"
#include "cellapp/entity/navigation.hpp"

#include <iostream>
#include <fstream>

void xrcLoadPolyMesh(rcPolyMesh & mesh, rcPolyMeshDetail & detail, float & agentHeight, float & agentClimb, float & agentRadius, std::istream & stream)
{
	uint32_t vertices, polys, nvp, borderSize;
	stream.read((char *)&agentHeight, sizeof(agentHeight));
	stream.read((char *)&agentClimb, sizeof(agentClimb));
	stream.read((char *)&agentRadius, sizeof(agentRadius));
	stream.read((char *)&vertices, sizeof(vertices));
	stream.read((char *)&polys, sizeof(polys));
	stream.read((char *)&nvp, sizeof(nvp));
	stream.read((char *)&borderSize, sizeof(borderSize));
	stream.read((char *)&mesh.cs, sizeof(mesh.cs));
	stream.read((char *)&mesh.ch, sizeof(mesh.ch));
	stream.read((char *)&mesh.bmin, sizeof(mesh.bmin));
	stream.read((char *)&mesh.bmax, sizeof(mesh.bmax));

	mesh.nverts = vertices;
	mesh.npolys = mesh.maxpolys = polys;
	mesh.nvp = nvp;
	mesh.borderSize = borderSize;
	mesh.verts = new unsigned short[vertices * 3];
	mesh.polys = new unsigned short[polys * nvp * 2];
	mesh.regs = new unsigned short[polys];
	mesh.flags = new unsigned short[polys];
	mesh.areas = new unsigned char[polys];

	stream.read((char *)mesh.verts, vertices * 3 * sizeof(unsigned short));
	stream.read((char *)mesh.polys, polys * nvp * 2 * sizeof(unsigned short));
	stream.read((char *)mesh.regs, polys * sizeof(unsigned short));
	stream.read((char *)mesh.flags, polys * sizeof(unsigned short));
	stream.read((char *)mesh.areas, polys * sizeof(unsigned char));

	uint32_t detailMeshes, detailVerts, detailTris;
	stream.read((char *)&detailMeshes, sizeof(detailMeshes));
	stream.read((char *)&detailVerts, sizeof(detailVerts));
	stream.read((char *)&detailTris, sizeof(detailTris));

	detail.nmeshes = detailMeshes;
	detail.nverts = detailVerts;
	detail.ntris = detailTris;
	detail.meshes = new unsigned int[4 * detailMeshes];
	detail.verts = new float[3 * detailVerts];
	detail.tris = new unsigned char[4 * detailTris];

	stream.read((char *)detail.meshes, 4 * detailMeshes * sizeof(unsigned int));
	stream.read((char *)detail.verts, 3 * detailVerts * sizeof(float));
	stream.read((char *)detail.tris, 4 * detailTris * sizeof(unsigned char));
}

void xrcFreePolyMesh(rcPolyMesh & mesh, rcPolyMeshDetail & detail)
{
	delete [] mesh.verts;
	delete [] mesh.polys;
	delete [] mesh.regs;
	delete [] mesh.flags;
	delete [] mesh.areas;

	delete [] detail.meshes;
	delete [] detail.verts;
	delete [] detail.tris;
}

inline bool inRange(const float * v1, const float * v2, const float r, const float h)
{
	const float dx = v2[0] - v1[0];
	const float dy = v2[1] - v1[1];
	const float dz = v2[2] - v1[2];
	return (dx * dx + dz * dz) < r * r && fabsf(dy) < h;
}

NavigationQueryParams::NavigationQueryParams()
	// An entity must be standing on top of a polygon, so starting extends are fairly conservative
	: startExtents(0.5f, 0.5f, 0.5f),
	// The destination is usually an entity (eg. the player) that may be in the middle of a jump,
	// standing on handrails, or other crazy positions --> larger destination extents
	destinationExtents(3.0f, 3.0f, 3.0f),
	minDistance(0.0f), maxDistance(0.0f),
	maxLengthPolys(NavigationPath::MaxPathLength),
	smoothingStepSize(0.75f)
{
}

NavigationMesh::NavigationMesh()
	: navMesh_(nullptr)
{
	filter_.setIncludeFlags(0x01);
}

bool NavigationMesh::load(std::string const & path)
{
	SGW_ASSERT(!navMesh_);

	std::ifstream navFile;
	navFile.open(path.c_str(), std::ifstream::in | std::ifstream::binary);
	if (navFile.fail())
	{
		FAULTC(CATEGORY_SPACE, "Failed to load navigation mesh from file: %s", path.c_str());
		return false;
	}

	float agentHeight, agentClimb, agentRadius;
	xrcLoadPolyMesh(mesh_, detail_, agentHeight, agentClimb, agentRadius, navFile);

	dtNavMeshCreateParams params;
	memset(&params, 0, sizeof(params));
	params.verts = mesh_.verts;
	params.vertCount = mesh_.nverts;
	params.polys = mesh_.polys;
	params.polyAreas = mesh_.areas;
	params.polyFlags = mesh_.flags;
	params.polyCount = mesh_.npolys;
	params.nvp = mesh_.nvp;
	params.detailMeshes = detail_.meshes;
	params.detailVerts = detail_.verts;
	params.detailVertsCount = detail_.nverts;
	params.detailTris = detail_.tris;
	params.detailTriCount = detail_.ntris;
	params.walkableHeight = agentHeight;
	params.walkableRadius = agentRadius;
	params.walkableClimb = agentClimb;
	rcVcopy(params.bmin, mesh_.bmin);
	rcVcopy(params.bmax, mesh_.bmax);
	params.cs = mesh_.cs;
	params.ch = mesh_.ch;
	params.buildBvTree = true;
		
	unsigned char * navData = nullptr;
	int navDataSize = 0;
	if (!dtCreateNavMeshData(&params, &navData, &navDataSize))
	{
		xrcFreePolyMesh(mesh_, detail_);
		FAULTC(CATEGORY_SPACE, "Could not build Detour navmesh.");
		return false;
	}
		
	navMesh_ = dtAllocNavMesh();
	if (!navMesh_)
	{
		dtFree(navData);
		xrcFreePolyMesh(mesh_, detail_);
		FAULTC(CATEGORY_SPACE, "Could not create Detour navmesh");
		return false;
	}
		
	dtStatus status = navMesh_->init(navData, navDataSize, DT_TILE_FREE_DATA);
	if (dtStatusFailed(status))
	{
		delete navMesh_;
		dtFree(navData);
		xrcFreePolyMesh(mesh_, detail_);
		FAULTC(CATEGORY_SPACE, "Could not init Detour navmesh");
		return false;
	}
		
	status = query_.init(navMesh_, 2048);
	if (dtStatusFailed(status))
	{
		delete navMesh_;
		dtFree(navData);
		xrcFreePolyMesh(mesh_, detail_);
		FAULTC(CATEGORY_SPACE, "Could not init Detour navmesh query");
		return false;
	}

	return true;
}

NavigationMesh::~NavigationMesh()
{
	delete navMesh_;
}

NavigationPath * NavigationMesh::findPath(NavigationQueryParams const & params)
{
	Vec3 startingPt;
	dtPolyRef startingRef;
	dtStatus status = query_.findNearestPoly(params.start.raw(), params.startExtents.raw(), &filter_, &startingRef, startingPt.raw());
	if (dtStatusFailed(status) || !startingRef)
	{
		WARNC(CATEGORY_SPACE, "Failed to find navmesh poly for source location (%f, %f, %f)",
			params.start.x, params.start.y, params.start.z);
		return nullptr;
	}

	Vec3 destinationPt;
	dtPolyRef destinationRef;
	status = query_.findNearestPoly(params.destination.raw(), params.destinationExtents.raw(), &filter_, &destinationRef, destinationPt.raw());
	if (dtStatusFailed(status) || !destinationRef)
	{
		WARNC(CATEGORY_SPACE, "Failed to find navmesh poly for destination location (%f, %f, %f)",
			params.destination.x, params.destination.y, params.destination.z);
		return nullptr;
	}

	NavigationPath * path = new NavigationPath;
	status = query_.findPath(startingRef, destinationRef, startingPt.raw(), destinationPt.raw(), &filter_, 
		path->polys_, &path->pathLength_, std::min(params.maxLengthPolys, (unsigned int)NavigationPath::MaxPathLength));
	if (dtStatusFailed(status) || !path->pathLength_)
	{
		delete path;
		WARNC(CATEGORY_SPACE, "No path found from location (%f, %f, %f) to (%f, %f, %f)",
			params.start.x, params.start.y, params.start.z, 
			params.destination.x, params.destination.y, params.destination.z);
		return nullptr;
	}

	path->smooth(startingPt, startingRef, destinationPt, destinationRef, query_, filter_, params);
	return path;
}

float NavigationMesh::findPathLength(NavigationQueryParams const & params)
{
	NavigationPath * path = findPath(params);
	if (!path)
		return -1.0f;

	float len = 0.0f;
	Vec3 const * curr = &params.start;
	Vec3 const * points = path->points();
	for (int i = 0; i < path->pointsCount(); i++)
	{
		len += (points[i] - *curr).length();
		curr = points + i;
	}
	return len;
}

NavigationPath::NavigationPath()
{
}

NavigationPath::~NavigationPath()
{
}

void NavigationPath::dump()
{
	Vec3 & start = smoothPath_[0];
	Vec3 & end = smoothPath_[smoothLength_ - 1];
	TRACEC(CATEGORY_SPACE, " ***** DUMPING NAVPATH FROM (%f, %f, %f) to (%f, %f, %f) ***** ",
		start.x, start.y, start.z, end.x, end.y, end.z);

	std::cout << "POLYS: ";
	for (int i = 0; i < pathLength_; i++)
	{
		std::cout << polys_[i] << " ";
	}
	std::cout << std::endl;

	std::cout << "POINTS: ";
	for (int i = 0; i < smoothLength_; i++)
	{
		Vec3 & p = smoothPath_[i];
		std::cout << "(" << p.x << "; " << p.y << "; " << p.z << ") -> ";
	}
	std::cout << std::endl;
}

void NavigationPath::fixupCorridor(const dtPolyRef * visited, const int nvisited)
{
	int furthestPath = -1;
	int furthestVisited = -1;
	
	// Find furthest common polygon.
	for (int i = pathLength_ - 1; i >= 0; --i)
	{
		bool found = false;
		for (int j = nvisited - 1; j >= 0; --j)
		{
			if (polys_[i] == visited[j])
			{
				furthestPath = i;
				furthestVisited = j;
				found = true;
			}
		}
		if (found)
			break;
	}

	// If no intersection found just return current path. 
	if (furthestPath == -1 || furthestVisited == -1)
		return;
	
	// Concatenate paths.	

	// Adjust beginning of the buffer to include the visited.
	const int req = nvisited - furthestVisited;
	const int orig = rcMin(furthestPath + 1, pathLength_);
	int size = rcMax(0, pathLength_ - orig);
	if (req + size > MaxPathLength)
		size = MaxPathLength - req;
	if (size)
		memmove(polys_ + req, polys_ + orig, size * sizeof(dtPolyRef));
	
	// Store visited
	for (int i = 0; i < req; ++i)
		polys_[i] = visited[(nvisited - 1) - i];				
	
	pathLength_ = req + size;
}

void NavigationPath::smooth(Vec3 & startingPoint, dtPolyRef startingPoly, Vec3 & dstPoint,
	dtPolyRef /*dstPoly*/, dtNavMeshQuery & query, dtQueryFilter & filter,
	NavigationQueryParams const & params)
{
	// Iterate over the path to find smooth path on the detail mesh surface.
	// memcpy(polys, path->path_, sizeof(dtPolyRef) * path->pathLength_); 
	// int npolys = path->pathLength_;
				
	Vec3 iterPos, targetPos;
	query.closestPointOnPoly(startingPoly, startingPoint.raw(), iterPos.raw(), nullptr);
	query.closestPointOnPoly(polys_[pathLength_ - 1], dstPoint.raw(), targetPos.raw(), nullptr);

	static const float SLOP = 0.1f;
				
	smoothLength_ = 0;
				
	smoothPath_[smoothLength_] = iterPos;
	smoothLength_++;
	
	// Move towards target a small advancement at a time until target reached or
	// when ran out of memory to store the path.
	while (pathLength_ && (unsigned int)smoothLength_ < MaxSmoothingSteps)
	{
		// Find location to steer towards.
		Vec3 steerPos;
		unsigned char steerPosFlag;
		dtPolyRef steerPosRef;
					
		if (!getSteerTarget(query, filter, iterPos, targetPos, SLOP, steerPos, steerPosFlag, steerPosRef))
			break;
					
		bool endOfPath = (steerPosFlag & DT_STRAIGHTPATH_END) ? true : false;
					
		// Find movement delta.
		Vec3 delta;
		delta = steerPos - iterPos;

		float len = dtMathSqrtf(dtVdot(delta.raw(), delta.raw()));

		// If the steer target is end of path or off-mesh link, do not move past the location.
		if (endOfPath && len < params.smoothingStepSize)
			len = 1;
		else
			len = params.smoothingStepSize / len;

		float moveTgt[3];
		dtVmad(moveTgt, iterPos.raw(), delta.raw(), len);
					
		// Move
		Vec3 result;
		dtPolyRef visited[16];
		int nvisited = 0;
		query.moveAlongSurface(polys_[0], iterPos.raw(), moveTgt, &filter,
			result.raw(), visited, &nvisited, 16);
		
		fixupCorridor(visited, nvisited);
		float h = 0;
		query.getPolyHeight(polys_[0], result.raw(), &h);
		result.y = h;
		iterPos = result;

		// Handle end of path when close enough.
		if (endOfPath && inRange(iterPos.raw(), steerPos.raw(), SLOP, 1.0f))
		{
			// Reached end of path.
			iterPos = targetPos;
			if ((unsigned int)smoothLength_ < MaxSmoothingSteps)
			{
				smoothPath_[smoothLength_] = iterPos;
				smoothLength_++;
			}
			break;
		}
					
		// Store results.
		if ((unsigned int)smoothLength_ < MaxSmoothingSteps)
		{
			smoothPath_[smoothLength_] = iterPos;
			smoothLength_++;
			float distance = (iterPos - dstPoint).length();
			if (params.maxDistance > 0.3f && distance < params.maxDistance)
			{
				Vec3 const & prev = smoothPath_[smoothLength_ - 2];
				Vec3 point;
				if (lineSphereIntersect(dstPoint, params.maxDistance, prev, (iterPos - prev).normalized(), point))
					smoothPath_[smoothLength_ - 1] = point;
				break;
			}
			else if (params.minDistance > 0.3f && distance < params.minDistance)
			{
				Vec3 const & prev = smoothPath_[smoothLength_ - 2];
				Vec3 point;
				if (lineSphereIntersect(dstPoint, params.minDistance, prev, (iterPos - prev).normalized(), point))
					smoothPath_[smoothLength_ - 1] = point;
				break;
			}
		}
	}
}

bool NavigationPath::lineSphereIntersect(Vec3 const & center, float radius, Vec3 const & origin, Vec3 const & direction, Vec3 & result)
{
	float dv = direction.dot(origin - center);
	float root = dv*dv - (origin - center).dot() + radius*radius;
	if (root < 0.0f)
		// Line and sphere does not intersect
		return false;

	if (root > 0.01f)
	{
		// Compute each point and return the one that is
		// closer to the line origin
		float result1 = -dv + sqrt(root);
		float result2 = -dv - sqrt(root);
		if ((direction * result1).dot() > (direction * result2).dot())
			result = origin + direction * result2;
		else
			result = origin + direction * result1;
	}
	else
		// The points are very close to each other, return their average as
		// the intersection point
		result = origin + (direction * -dv);
	return true;
}

Vec3 const * NavigationPath::points() const
{
	return smoothPath_;
}

int NavigationPath::pointsCount() const
{
	return smoothLength_;
}

bool NavigationPath::getSteerTarget(dtNavMeshQuery & query, dtQueryFilter & /*filter*/, const Vec3 & startPos,
	const Vec3 & endPos, float minTargetDist, Vec3 & steerPos, unsigned char & steerPosFlag,
	dtPolyRef & steerPosRef)							 
{
	// Find steer target.
	static const int MAX_STEER_POINTS = 3;
	float steerPath[MAX_STEER_POINTS * 3];
	unsigned char steerPathFlags[MAX_STEER_POINTS];
	dtPolyRef steerPathPolys[MAX_STEER_POINTS];
	int steerPathLength = 0;
	query.findStraightPath(startPos.raw(), endPos.raw(), polys_, pathLength_, 
		steerPath, steerPathFlags, steerPathPolys, &steerPathLength, MAX_STEER_POINTS);
	if (!steerPathLength)
		return false;

	// Find vertex far enough to steer to.
	int ns = 0;
	while (ns < steerPathLength)
	{
		// Stop when point is further than slop away.
		if (!inRange(&steerPath[ns*3], startPos.raw(), minTargetDist, 1000.0f))
			break;
		ns++;
	}
	// Failed to find good point to steer to.
	if (ns >= steerPathLength)
		return false;
	
	dtVcopy(steerPos.raw(), &steerPath[ns * 3]);
	steerPos.y = startPos.y;
	steerPosFlag = steerPathFlags[ns];
	steerPosRef = steerPathPolys[ns];
	
	return true;
}
