#include <boost/utility.hpp>
#include <boost/thread/once.hpp>
#include <boost/scoped_ptr.hpp>

template <class _T>
class autoinit_singleton : private boost::noncopyable
{
	public:
		static _T & instance()
		{
			boost::call_once(initialize, m_once_flag);
			return *m_ptr;
		}

		static void initialize(_T * inst = nullptr)
		{
			if (inst == nullptr)
				m_ptr.reset(new _T());
			else
				m_ptr.reset(inst);
		}

	protected:
		~autoinit_singleton() {}
		 autoinit_singleton() {}

	private:
		 static boost::once_flag m_once_flag;
		 static boost::scoped_ptr<_T> m_ptr;
};

template <class _T>
boost::scoped_ptr<_T> autoinit_singleton<_T>::m_ptr(nullptr);

template <class _T>
boost::once_flag autoinit_singleton<_T>::m_once_flag = BOOST_ONCE_INIT;

