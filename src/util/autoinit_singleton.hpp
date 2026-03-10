#pragma once

#include <memory>
#include <mutex>

template <class _T>
class autoinit_singleton
{
	public:
		autoinit_singleton(autoinit_singleton const &) = delete;
		autoinit_singleton & operator=(autoinit_singleton const &) = delete;

		static _T & instance()
		{
			std::call_once(m_once_flag, initialize, nullptr);
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
		 static std::once_flag m_once_flag;
		 static std::unique_ptr<_T> m_ptr;
};

template <class _T>
std::unique_ptr<_T> autoinit_singleton<_T>::m_ptr(nullptr);

template <class _T>
std::once_flag autoinit_singleton<_T>::m_once_flag;
