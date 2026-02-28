#pragma once

#include <boost/utility.hpp>
#include <boost/thread/once.hpp>
#include <boost/scoped_ptr.hpp>

template <class _T>
class singleton : private boost::noncopyable
{
	public:
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

		 static boost::scoped_ptr<_T> m_ptr;
};

template <class _T>
boost::scoped_ptr<_T> singleton<_T>::m_ptr(nullptr);

