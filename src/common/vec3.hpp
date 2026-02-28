#pragma once

#include <math.h>

struct Vec3
{
	float x, y, z;

	inline Vec3(float _x, float _y, float _z)
		: x(_x), y(_y), z(_z)
	{}

	inline Vec3()
		: x(0.0f), y(0.0f), z(0.0f)
	{}

	inline Vec3(Vec3 const & v)
		: x(v.x), y(v.y), z(v.z)
	{}

	inline Vec3 & operator = (Vec3 const & v)
	{
		x = v.x;
		y = v.y;
		z = v.z;
		return *this;
	}

	inline Vec3 operator + (Vec3 const & v) const
	{
		Vec3 prod;
		prod.x = x + v.x;
		prod.y = y + v.y;
		prod.z = z + v.z;
		return prod;
	}

	inline Vec3 operator + (float f) const
	{
		Vec3 prod;
		prod.x = x + f;
		prod.y = y + f;
		prod.z = z + f;
		return prod;
	}

	inline Vec3 operator - (Vec3 const & v) const
	{
		Vec3 prod;
		prod.x = x - v.x;
		prod.y = y - v.y;
		prod.z = z - v.z;
		return prod;
	}

	inline Vec3 operator - (float f) const
	{
		Vec3 prod;
		prod.x = x - f;
		prod.y = y - f;
		prod.z = z - f;
		return prod;
	}

	inline Vec3 operator - () const
	{
		Vec3 prod;
		prod.x = -x;
		prod.y = -y;
		prod.z = -z;
		return prod;
	}

	inline Vec3 operator * (Vec3 const & v) const
	{
		Vec3 prod;
		prod.x = x * v.x;
		prod.y = y * v.y;
		prod.z = z * v.z;
		return prod;
	}

	inline Vec3 operator * (float f) const
	{
		Vec3 prod;
		prod.x = x * f;
		prod.y = y * f;
		prod.z = z * f;
		return prod;
	}

	inline Vec3 operator / (float f) const
	{
		Vec3 prod;
		prod.x = x / f;
		prod.y = y / f;
		prod.z = z / f;
		return prod;
	}

	inline float length() const
	{
		return sqrt(x*x + y*y + z*z);
	}

	inline void normalize()
	{
		float len = length();
		x /= len;
		y /= len;
		z /= len;
	}

	inline Vec3 normalized() const
	{
		Vec3 norm;
		float len = length();
		norm.x = x / len;
		norm.y = y / len;
		norm.z = z / len;
		return norm;
	}

	inline Vec3 squared() const
	{
		return (*this * *this);
	}

	inline float dot(Vec3 const & v) const
	{
		return (x*v.x + y*v.y + z*v.z);
	}

	inline float dot() const
	{
		return (x*x + y*y + z*z);
	}

	inline float * raw()
	{
		return &x;
	}

	inline float const * raw() const
	{
		return &x;
	}
};
