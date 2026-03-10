#pragma once

#include <memory>

template <class _T>
class singleton
{
	public:
		singleton(singleton const &) = delete;
		singleton & operator=(singleton const &) = delete;

		static _T & instance()
		{
			return *m_ptr;
		}

		static void init(_T * inst)
		{
			m_ptr.reset(inst);
		}

	protected:
		~singleton() {}
		 singleton() {}

		 static std::unique_ptr<_T> m_ptr;
};

template <class _T>
std::unique_ptr<_T> singleton<_T>::m_ptr(nullptr);
