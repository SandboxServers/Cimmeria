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

#include <cstdlib>
#include <cstring>
#include <sstream>
#include <stdexcept>
#include <string>

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

	BuildParams()
		: cs(0.3f), ch(0.2f),
		agentHeight(0.6f), agentClimb(0.9f), agentRadius(0.6f),
		slope(45.0f), maxEdgeLen(12.0f), maxSimplificationError(1.3f),
		minRegionSize(8.0f), mergeRegionSize(20.0f), maxVertsPerPoly(6),
		detailSampleDist(6.0f), detailSampleMaxError(1.0f),
		watershed(false), hasBounds(false)
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
		return s.str();
	}

private:
	static float toFloat(std::string const & key, std::string const & value)
	{
		char * end = nullptr;
		float f = (float)strtod(value.c_str(), &end);
		if (value.empty() || end == nullptr || *end != '\0')
			throw std::runtime_error("Parameter '" + key + "' is not a number: '" + value + "'");
		return f;
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
		else if (key == "maxVertsPerPoly") maxVertsPerPoly = (int)toFloat(key, value);
		else if (key == "detailSampleDist") detailSampleDist = toFloat(key, value);
		else if (key == "detailSampleMaxError") detailSampleMaxError = toFloat(key, value);
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
	}
};
