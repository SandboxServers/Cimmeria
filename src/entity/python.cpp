#include <stdafx.hpp>
#include <entity/python.hpp>
#include <entity/pyutil.hpp>
#include <entity/defs.hpp>
#include <entity/entity.hpp>

bp::object PyEvalStringToObject(std::string const & str)
{
	bp::handle<> main_module(bp::borrowed(PyImport_AddModule("__main__")));
	bp::handle<> main_namespace(bp::borrowed(PyModule_GetDict(main_module.get())));
	PyObject * value = PyRun_String(str.c_str(), Py_eval_input, main_namespace.get(), main_namespace.get());
	if (!value)
	{
		PyUtil_ShowErr();
		SGW_ASSERT(value);
	}
	return bp::object(bp::handle<>(value));
}

void PyRegisterModule(bool cell)
{
	Py_Initialize();
#if PY_VERSION_HEX < 0x03090000
	PyEval_InitThreads();
#endif
	PyImport_AddModule("Atrea");
	
	{
		bp::object module = bp::import("Atrea");
		bp::scope active_scope(module);
		bp::class_<Vector3, ::Vector3::Ptr>("Vector3", bp::init<>())
			.def(bp::init<float, float, float>())
			.def(bp::init<Vector3::Ptr>())
			.def("length", &Vector3::length)
			.def("normalize", &Vector3::normalize)
			.def_readwrite("x", &Vector3::x)
			.def_readwrite("y", &Vector3::y)
			.def_readwrite("z", &Vector3::z);
		
		bp::def("log", &PyUtil_Log);
		PyTypeDb::instance().initialize("../entities/", cell);
		Mailbox::registerType();
		MailboxRpcMethod::registerType();
	}

	PyEval_SaveThread();
}

PyTypeInfo::PyTypeInfo(BuiltinType _type)
	: type(_type)
{
}

void PyTypeInfo::addElement(PyTypeInfo element)
{
	elements.push_back(element);
}

bp::object PyTypeInfo::fromXml(tinyxml2::XMLElement & element)
{
	std::string text = element.GetText();
	trim(text);
	return fromString(text);
}

bp::object PyTypeInfo::fromString(std::string const & str) const
{
	switch (type)
	{
	case TYPE_BOOL:
		return bp::object(str == "true" || str == "TRUE");

	case TYPE_INT8:
	case TYPE_UINT8:
	case TYPE_INT16:
	case TYPE_UINT16:
	case TYPE_INT32:
	case TYPE_UINT32:
	case TYPE_INT64:
	case TYPE_UINT64:
		return bp::long_(atol(str.c_str()));

	case TYPE_FLOAT:
	case TYPE_DOUBLE:
		return bp::long_(atof(str.c_str()));

	case TYPE_VECTOR3:
		{
			std::size_t separator1 = str.find_first_of(' ');
			SGW_ASSERT(separator1 != std::string::npos && separator1 > 0);
			std::size_t separator2 = str.find_first_of(' ', separator1 + 1);
			SGW_ASSERT(separator2 != std::string::npos && separator2 > separator1);

			Vector3 * vector = new Vector3();
			vector->x = (float)atof(str.substr(0, separator1).c_str());
			vector->y = (float)atof(str.substr(separator1 + 1, separator2).c_str());
			vector->z = (float)atof(str.substr(separator2 + 1).c_str());
			return bp::object(Vector3::Ptr(vector));
		}

	case TYPE_STRING:
		return bp::object(bp::handle<>(PyBytes_FromString(str.c_str())));

	case TYPE_UNICODE:
		return bp::object(bp::handle<>(PyUnicode_FromString(str.c_str())));

	case TYPE_LIST:
		{
			bp::object obj = PyEvalStringToObject(str);
			SGW_ASSERT(PyList_Check(obj.ptr()));
			return obj;
		}

	case TYPE_FIXED_DICT:
		{
			bp::object obj = PyEvalStringToObject(str);
			SGW_ASSERT(PyDict_Check(obj.ptr()));
			return obj;
		}

	case TYPE_PICKLED:
		return PyEvalStringToObject(str);

	default:
		throw std::runtime_error("fromString() not supported for this type!");
	}
}