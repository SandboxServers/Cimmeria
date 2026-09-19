#pragma once

// The three Boost.uBLAS names nav_builder uses - matrix<T>, vector<T> and
// prod(matrix, vector) - so the standalone build does not need Boost.
// Accumulation order matches uBLAS (row-major, left to right from zero), so
// results are bit-identical; NavBuilder only ever passes an identity matrix.

#include <vector>
#include <cstddef>

namespace navbuilder_ublas
{
	template <typename T>
	class vector
	{
	public:
		vector() {}
		explicit vector(std::size_t size) : data_(size) {}

		std::size_t size() const { return data_.size(); }
		T & operator[](std::size_t i) { return data_[i]; }
		T const & operator[](std::size_t i) const { return data_[i]; }
		T & operator()(std::size_t i) { return data_[i]; }
		T const & operator()(std::size_t i) const { return data_[i]; }

	private:
		std::vector<T> data_;
	};

	template <typename T>
	class matrix
	{
	public:
		matrix(std::size_t rows, std::size_t cols) : rows_(rows), cols_(cols), data_(rows * cols) {}

		std::size_t size1() const { return rows_; }
		std::size_t size2() const { return cols_; }
		T & operator()(std::size_t r, std::size_t c) { return data_[r * cols_ + c]; }
		T const & operator()(std::size_t r, std::size_t c) const { return data_[r * cols_ + c]; }

	private:
		std::size_t rows_, cols_;
		std::vector<T> data_;
	};

	template <typename T>
	vector<T> prod(matrix<T> const & m, vector<T> const & v)
	{
		vector<T> out(m.size1());
		for (std::size_t r = 0; r < m.size1(); r++)
		{
			T t = T();
			for (std::size_t c = 0; c < m.size2(); c++)
				t += m(r, c) * v[c];
			out[r] = t;
		}
		return out;
	}
}

using namespace navbuilder_ublas;
