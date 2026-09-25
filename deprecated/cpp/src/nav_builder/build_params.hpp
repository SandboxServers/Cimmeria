#pragma once

// Tunable Recast build parameters for NavBuilder.
//
// Every default below reproduces the value that used to be hard-coded in
// builder.cpp, so `NavBuilder <mode> <in> <out> <fmt>` with no trailing
// arguments builds a byte-identical mesh.
//
// Trailing arguments are `key=value`, `--key=value` or `--key value`.
// Header-only on purpose: the legacy NavBuilder.vcxproj needs no new
// ClCompile entry.

#include <cmath>
#include <cstdlib>
#include <cstring>
#include <sstream>
#include <stdexcept>
#include <string>

// Largest / smallest finite float, spelled out rather than pulled from
// <cfloat> so the header stays self-contained for the legacy vcxproj.
#define NAVBUILDER_FLT_MAX 3.402823466e+38

// Exit codes. 0 = success; every failure path returns one of these.
enum ExitCode
{
	EXIT_OK = 0,
	EXIT_USAGE = 1,
	EXIT_INTERNAL_ERROR = 2,
	EXIT_BUILD_FAILED = 3,
	EXIT_OUTPUT_NOT_WRITABLE = 4
};

// Narrow a derived cell count to `int`, refusing what the conversion
// cannot represent.
//
// Converting an out-of-range floating value to `int` is undefined
// behaviour, and every one of these is user-driven: `cs=1e-30` makes
// `agentRadius / cs` astronomically large without either parameter
// being out of range on its own, and there is no range check anywhere
// between the command line and the cast. Throws, which `main` turns
// into a usage exit rather than a garbage navmesh.
inline int navbuilderToInt(char const * what, double v)
{
	if (!(v == v) || v < -2147483648.0 || v > 2147483647.0)
	{
		std::ostringstream s;
		s << "Derived value '" << what << "' (" << v << ") is outside the int range; "
			<< "check cs / ch and the agent dimensions";
		throw std::runtime_error(s.str());
	}
	return (int)v;
}

struct BuildParams
{
	// Voxel size (metres).
	float cs;
	float ch;
	// Agent (metres). Written verbatim into the .nav header; the server's
	// Detour loader reads them back.
	float agentHeight;
	float agentClimb;
	float agentRadius;
	// Max walkable slope (degrees).
	float slope;
	// Max contour edge length (metres); 0 disables edge splitting.
	float maxEdgeLen;
	// Max contour simplification error (voxels).
	float maxSimplificationError;
	// Region thresholds as a cell *side*; squared before use, exactly like
	// RecastDemo's "Min Region Size" / "Merged Region Size".
	float minRegionSize;
	float mergeRegionSize;
	int maxVertsPerPoly;
	// Detail mesh: multiples of cs / ch respectively (RecastDemo convention).
	float detailSampleDist;
	float detailSampleMaxError;
	// false = rcBuildRegionsMonotone (legacy default), true = watershed.
	bool watershed;
	// Optional horizontal crop, BigWorld metres: minX,minZ,maxX,maxZ.
	bool hasBounds;
	float bounds[4];
	// Tile side in cells; 0 = one rcPolyMesh for the whole map (the XRC
	// single-mesh layout). Anything else writes the tiled "XRCT" layout,
	// one rcPolyMesh per tile, so every Recast index cap applies per tile.
	int tileSize;
	// Worker threads for the tiled build. The output does not depend on it.
	int threads;
	// Tiled only: drop the sub-minRegionSize islands that survive along tile
	// seams (tile_seam_filter.hpp). 0 keeps them, for diagnosis.
	bool seamFilter;

	BuildParams()
		: cs(0.3f), ch(0.2f),
		agentHeight(0.6f), agentClimb(0.9f), agentRadius(0.6f),
		slope(45.0f), maxEdgeLen(12.0f), maxSimplificationError(1.3f),
		minRegionSize(8.0f), mergeRegionSize(20.0f), maxVertsPerPoly(6),
		detailSampleDist(6.0f), detailSampleMaxError(1.0f),
		watershed(false), hasBounds(false), tileSize(0), threads(4), seamFilter(true)
	{
		bounds[0] = bounds[1] = bounds[2] = bounds[3] = 0.0f;
	}

	static const char * usage()
	{
		return
			"Usage: NavBuilder <chunked|whole> <input> <output> <nav|obj> [key=value ...]\n"
			"  cs=0.3 ch=0.2                     voxel size (m)\n"
			"  agentHeight=0.6 agentClimb=0.9 agentRadius=0.6   agent (m)\n"
			"  slope=45                          max walkable slope (deg)\n"
			"  maxEdgeLen=12                     max contour edge (m), 0 = off\n"
			"  maxSimplificationError=1.3        contour simplification (voxels)\n"
			"  minRegionSize=8 mergeRegionSize=20   cell side, squared internally\n"
			"  maxVertsPerPoly=6                 3..6\n"
			"  detailSampleDist=6 detailSampleMaxError=1   multiples of cs / ch\n"
			"  partition=monotone|watershed\n"
			"  bounds=minX,minZ,maxX,maxZ        crop (BigWorld m)\n"
			"  tile=0                            tile side in cells (16..4096); 0 = single mesh\n"
			"  threads=4                         tiled build workers (1..64)\n"
			"  seamFilter=1                      tiled: drop small islands left on tile seams\n"
			"Exit: 0 ok, 1 usage, 2 internal error, 3 Recast build failed, 4 output not writable\n";
	}

	// Parses argv[first..argc). Throws std::runtime_error on a bad argument.
	void parse(int argc, char ** argv, int first)
	{
		for (int i = first; i < argc; i++)
		{
			std::string arg = argv[i];
			if (arg.compare(0, 2, "--") == 0)
				arg = arg.substr(2);

			std::string key, value;
			std::size_t eq = arg.find('=');
			if (eq != std::string::npos)
			{
				key = arg.substr(0, eq);
				value = arg.substr(eq + 1);
			}
			else
			{
				if (i + 1 >= argc)
					throw std::runtime_error("Missing value for parameter '" + arg + "'");
				key = arg;
				value = argv[++i];
			}
			set(key, value);
		}
		validate();
	}

	std::string describe() const
	{
		std::ostringstream s;
		s << "cs=" << cs << " ch=" << ch
			<< " agentHeight=" << agentHeight << " agentClimb=" << agentClimb
			<< " agentRadius=" << agentRadius << " slope=" << slope
			<< " maxEdgeLen=" << maxEdgeLen
			<< " maxSimplificationError=" << maxSimplificationError
			<< " minRegionSize=" << minRegionSize
			<< " mergeRegionSize=" << mergeRegionSize
			<< " maxVertsPerPoly=" << maxVertsPerPoly
			<< " detailSampleDist=" << detailSampleDist
			<< " detailSampleMaxError=" << detailSampleMaxError
			<< " partition=" << (watershed ? "watershed" : "monotone");
		if (hasBounds)
			s << " bounds=" << bounds[0] << "," << bounds[1] << "," << bounds[2] << "," << bounds[3];
		// Only in tiled mode, so a single-mesh build logs the same line it
		// always has.
		if (tileSize > 0)
			s << " tile=" << tileSize << " threads=" << threads << " seamFilter=" << (seamFilter ? 1 : 0);
		return s.str();
	}

private:
	// Parse to `double` and range-check *before* narrowing.
	//
	// `(float)strtod(...)` is the wrong order: converting a double whose
	// value is outside the float range to float is undefined behaviour,
	// so a finite input like `1e39` has already invoked UB by the time
	// any `> 3.4e38f` check runs. In practice MSVC yields +inf and the
	// check catches it; the standard promises nothing.
	static double toDouble(std::string const & key, std::string const & value)
	{
		char * end = nullptr;
		double d = strtod(value.c_str(), &end);
		if (value.empty() || end == nullptr || *end != '\0')
			throw std::runtime_error("Parameter '" + key + "' is not a number: '" + value + "'");
		// strtod happily accepts "nan" and "inf". validate() below compares
		// with <, which is false for NaN, so a non-finite value would slip
		// past every range check and reach floorf/ceilf in builder.cpp.
		if (!(d == d))
			throw std::runtime_error("Parameter '" + key + "' must be finite: '" + value + "'");
		if (d > NAVBUILDER_FLT_MAX || d < -NAVBUILDER_FLT_MAX)
			throw std::runtime_error("Parameter '" + key + "' is out of float range: '" + value + "'");
		return d;
	}

	static float toFloat(std::string const & key, std::string const & value)
	{
		return (float)toDouble(key, value);
	}

	// An integral parameter. `(int)toFloat(...)` truncated silently, so
	// `maxVertsPerPoly=3.9` passed the 3..6 check as 3 — the build then
	// used a value the operator never asked for. And the cast itself ran
	// before validate(), so `maxVertsPerPoly=1e10` was undefined
	// behaviour rather than a rejected argument.
	static int toInt(std::string const & key, std::string const & value)
	{
		double d = toDouble(key, value);
		if (d != (double)(long long)d)
			throw std::runtime_error("Parameter '" + key + "' must be a whole number: '" + value + "'");
		return navbuilderToInt(key.c_str(), d);
	}

	void set(std::string const & key, std::string const & value)
	{
		if (key == "cs") cs = toFloat(key, value);
		else if (key == "ch") ch = toFloat(key, value);
		else if (key == "agentHeight") agentHeight = toFloat(key, value);
		else if (key == "agentClimb") agentClimb = toFloat(key, value);
		else if (key == "agentRadius") agentRadius = toFloat(key, value);
		else if (key == "slope") slope = toFloat(key, value);
		else if (key == "maxEdgeLen") maxEdgeLen = toFloat(key, value);
		else if (key == "maxSimplificationError") maxSimplificationError = toFloat(key, value);
		else if (key == "minRegionSize") minRegionSize = toFloat(key, value);
		else if (key == "mergeRegionSize") mergeRegionSize = toFloat(key, value);
		else if (key == "maxVertsPerPoly") maxVertsPerPoly = toInt(key, value);
		else if (key == "detailSampleDist") detailSampleDist = toFloat(key, value);
		else if (key == "detailSampleMaxError") detailSampleMaxError = toFloat(key, value);
		else if (key == "tile") tileSize = toInt(key, value);
		else if (key == "threads") threads = toInt(key, value);
		else if (key == "seamFilter")
		{
			if (value == "1") seamFilter = true;
			else if (value == "0") seamFilter = false;
			else throw std::runtime_error("seamFilter must be 0 or 1, got '" + value + "'");
		}
		else if (key == "partition")
		{
			if (value == "watershed") watershed = true;
			else if (value == "monotone") watershed = false;
			else throw std::runtime_error("partition must be 'monotone' or 'watershed', got '" + value + "'");
		}
		else if (key == "bounds")
		{
			std::size_t pos = 0;
			for (int n = 0; n < 4; n++)
			{
				std::size_t comma = value.find(',', pos);
				if ((comma == std::string::npos) != (n == 3))
					throw std::runtime_error("bounds needs exactly four values: minX,minZ,maxX,maxZ");
				bounds[n] = toFloat(key, value.substr(pos, comma == std::string::npos ? comma : comma - pos));
				pos = comma + 1;
			}
			hasBounds = true;
		}
		else
			throw std::runtime_error("Unknown parameter '" + key + "'");
	}

	void validate() const
	{
		if (!(cs > 0.0f) || !(ch > 0.0f))
			throw std::runtime_error("cs and ch must be > 0");
		if (!(agentHeight > 0.0f) || agentClimb < 0.0f || agentRadius < 0.0f)
			throw std::runtime_error("agentHeight must be > 0; agentClimb and agentRadius must be >= 0");
		if (slope < 0.0f || slope >= 90.0f)
			throw std::runtime_error("slope must be in [0, 90)");
		if (maxEdgeLen < 0.0f || maxSimplificationError < 0.0f)
			throw std::runtime_error("maxEdgeLen and maxSimplificationError must be >= 0");
		if (minRegionSize < 0.0f || mergeRegionSize < 0.0f)
			throw std::runtime_error("minRegionSize and mergeRegionSize must be >= 0");
		if (maxVertsPerPoly < 3 || maxVertsPerPoly > 6)
			throw std::runtime_error("maxVertsPerPoly must be 3..6 (Detour's DT_VERTS_PER_POLYGON is 6)");
		if (hasBounds && (!(bounds[0] < bounds[2]) || !(bounds[1] < bounds[3])))
			throw std::runtime_error("bounds must satisfy minX < maxX and minZ < maxZ");
		// Below 16 cells the walkableRadius + 3 border dwarfs the tile;
		// above 4096 a tile is no smaller than a whole map.
		if (tileSize != 0 && (tileSize < 16 || tileSize > 4096))
			throw std::runtime_error("tile must be 0 (single mesh) or 16..4096 cells");
		if (threads < 1 || threads > 64)
			throw std::runtime_error("threads must be 1..64");

		// The derived cell counts are what actually reach Recast, and
		// every one is a quotient or a square of parameters that are
		// individually in range: `cs=1e-30` passes `cs > 0` and still
		// sends `agentRadius / cs` far past INT_MAX, and
		// `minRegionSize` squared overflows above ~46341. Checking here
		// rather than at the cast site in builder.cpp is what makes a
		// bad argument exit 1 (usage) instead of 2 (internal error).
		navbuilderToInt("walkableHeight", std::ceil((double)agentHeight / (double)ch));
		navbuilderToInt("walkableClimb", std::floor((double)agentClimb / (double)ch));
		navbuilderToInt("walkableRadius", std::ceil((double)agentRadius / (double)cs));
		navbuilderToInt("maxEdgeLen", (double)maxEdgeLen / (double)cs);
		navbuilderToInt("minRegionArea", (double)minRegionSize * (double)minRegionSize);
		navbuilderToInt("mergeRegionArea", (double)mergeRegionSize * (double)mergeRegionSize);
	}
};
