#pragma once

#if defined(DEBUG) || defined(ENABLE_RELEASE_ASSERTS)
void PyUtil_AssertGIL();
#endif
void PyUtil_LogVa(LogMessage::Level level, const char * fmt, ...);
void PyUtil_Log(uint32_t loglevel, std::string const & category, std::string const & msg);
void PyUtil_ShowErr(char const * msg = nullptr);
bp::object PyUtil_CreateSubmodule(std::string const & module, std::string const & submodule);

class PyGilGuard
{
public:
	inline PyGilGuard()
	{
		state_ = PyGILState_Ensure();
	}

	inline ~PyGilGuard()
	{
		PyGILState_Release(state_);
	}

private:
	PyGILState_STATE state_;
};

