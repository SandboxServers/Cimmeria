#include <stdafx.hpp>
#include <entity/pyutil.hpp>
#include <boost/python/import.hpp>

/*
 * Checks if the GIL is in locked state for the current thread;
 * asserts if it isn't.
 */
void PyUtil_AssertGIL()
{
	PyGILState_STATE state = PyGILState_Ensure();
	PyGILState_Release(state);
	SGW_ASSERT(state == PyGILState_LOCKED);
}

void PyUtil_LogVa(LogMessage::Level level, const char * category, const char * fmt, ...)
{
	va_list args;
	va_start(args, fmt);
	Logger::instance().log(level, category, fmt, args);
	va_end(args);
}

void PyUtil_Log(uint32_t logLevel, std::string const & category, std::string const & msg)
{
	if (msg.length() > 1 || (!msg.empty() && msg[0] != 0x0A && msg[0] != 0x0D && msg[0] != '\t' && msg[0] != ' '))
		PyUtil_LogVa((LogMessage::Level)logLevel, category.c_str(), "%s", msg.c_str());
}

void PyUtil_ShowErr(char const * msg)
{
	PyGilGuard guard;
	PyObject * err = PyErr_Occurred();
	if (err != NULL)
	{
		PyObject *ptype, *pvalue, *ptraceback;

		PyErr_Fetch(&ptype, &pvalue, &ptraceback);
		PyObject * pystr = PyObject_Str(pvalue);
		const char * errorDesc = PyUnicode_AsUTF8(pystr);
		std::string backtrace;

		/* See if we can get a full traceback */
		PyObject * modName = PyUnicode_FromWideChar(L"traceback", 9);
		PyObject * tracebackMod = PyImport_Import(modName);
		Py_DECREF(modName);

		if (tracebackMod == NULL)
			return;

		PyObject * formatExc = PyObject_GetAttrString(tracebackMod, "format_tb");
		if (ptraceback && formatExc && PyCallable_Check(formatExc))
		{
			backtrace = "\n";
			PyObject * formatted = PyObject_CallFunctionObjArgs(formatExc, ptraceback, NULL);
			SGW_ASSERT(formatted);
			SGW_ASSERT(PyList_Check(formatted));
			Py_ssize_t size = PyList_GET_SIZE(formatted);
			for (Py_ssize_t i = 0; i < size; i++)
			{
				PyObject * line = PyList_GET_ITEM(formatted, i);
				pystr = PyObject_Str(line);
				backtrace += PyUnicode_AsUTF8(pystr);
			}
			Py_DECREF(formatted);
		}

		FAULT("%s: %s%s", (msg ? msg : "Python exception"), errorDesc, backtrace.c_str());
		PyErr_Clear();
	}
	else
		FAULT("Unknown Python exception");
}


bp::object PyUtil_CreateSubmodule(std::string const & module, std::string const & submodule)
{
	auto parent = bp::import(module.c_str());
	auto mod = bp::object(bp::handle<>(PyModule_New(submodule.c_str())));
	bp::setattr(parent, submodule.c_str(), mod);

	auto modules = bp::import("sys").attr("modules");
	modules[module + "." + submodule] = mod;
	return mod;
}

