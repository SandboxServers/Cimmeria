#pragma once

#include <common/vec3.hpp>
#include <mercury/stream.hpp>
#include <entity/pyutil.hpp>
#include <soci/soci.h>
#include <boost/algorithm/string.hpp>
#include <boost/python/class.hpp>
#include <boost/python/def.hpp>
#include <boost/python/import.hpp>
#include <boost/python/init.hpp>
#include <boost/python/long.hpp>
#include <boost/python/scope.hpp>

using namespace boost::algorithm;

namespace tinyxml2 {
	class XMLElement;
}

struct Vector3 : public Vec3
{
	typedef std::shared_ptr<Vector3> Ptr;

	inline Vector3(float _x, float _y, float _z)
		: Vec3(_x, _y, _z)
	{}

	inline Vector3()
	{}

	inline Vector3(Vector3 const & v)
		: Vec3(v.x, v.y, v.z)
	{}

	inline Vector3(Vector3::Ptr v)
		: Vec3(v->x, v->y, v->z)
	{}

	inline Vector3(Vec3 const & v)
		: Vec3(v)
	{}
};

bp::object PyEvalStringToObject(std::string const & str);
void PyRegisterModule(bool cell);

class PyTypeInfo
{
public:
	enum BuiltinType
	{
		TYPE_NONE,
		TYPE_BOOL,
		TYPE_INT8,
		TYPE_UINT8,
		TYPE_INT16,
		TYPE_UINT16,
		TYPE_INT32,
		TYPE_UINT32,
		TYPE_INT64,
		TYPE_UINT64,
		TYPE_FLOAT,
		TYPE_DOUBLE,
		TYPE_VECTOR3,
		TYPE_STRING,
		TYPE_UNICODE,
		TYPE_LIST,
		TYPE_FIXED_DICT,
		TYPE_PICKLED
	};

	static const uint32_t MaxListLength = 0x1000;
	static const uint32_t MaxStringLength = 0x1000;
	static const uint32_t MaxPickledLength = 0x100000;

	PyTypeInfo(BuiltinType _type);

	void addElement(PyTypeInfo element);
	
	bp::object fromXml(tinyxml2::XMLElement & element);
	bp::object fromString(std::string const & str) const;


	template <typename _T>
	void pack(_T & stream, bp::object const & object) const
	{
		switch (type)
		{
		case TYPE_NONE:
			{
				break;
			}

		case TYPE_BOOL:
			{
				stream << (uint8_t)(bp::extract<bool>(object) ? 1 : 0);
				break;
			}

		case TYPE_INT8:
			{
				stream << (int8_t)bp::extract<int8_t>(object);
				break;
			}

		case TYPE_UINT8:
			{
				stream << (uint8_t)bp::extract<uint8_t>(object);
				break;
			}

		case TYPE_INT16:
			{
				stream << (int16_t)bp::extract<int16_t>(object);
				break;
			}

		case TYPE_UINT16:
			{
				stream << (uint16_t)bp::extract<uint16_t>(object);
				break;
			}

		case TYPE_INT32:
			{
				stream << (int32_t)bp::extract<int64_t>(object);
				break;
			}

		case TYPE_UINT32:
			{
				stream << (uint32_t)bp::extract<uint32_t>(object);
				break;
			}

		case TYPE_INT64:
			{
				stream << (int64_t)bp::extract<int64_t>(object);
				break;
			}

		case TYPE_UINT64:
			{
				stream << (uint64_t)bp::extract<uint64_t>(object);
				break;
			}

		case TYPE_FLOAT:
			{
				stream << (float)bp::extract<float>(object);
				break;
			}

		case TYPE_DOUBLE:
			{
				stream << (double)bp::extract<double>(object);
				break;
			}

		case TYPE_VECTOR3:
			{
				Vector3 * vector = bp::extract<Vector3 *>(object);
				if (!vector)
					throw std::runtime_error("Failed to pack VECTOR3: argument is not a valid vector object!");
				stream << vector->x << vector->y << vector->z;
				break;
			}

		case TYPE_STRING:
			{
				PyObject * pyo = object.ptr();
				if (!PyBytes_Check(pyo))
					throw std::runtime_error("Failed to pack STRING: argument is not a valid PyBytes instance!");

				Py_ssize_t bytes = PyBytes_GET_SIZE(pyo);
				char * str = PyBytes_AsString(pyo);
				SGW_ASSERT(str);
				stream << (uint8_t)(bytes);
				stream.write(str, (uint32_t)bytes);
				break;
			}

		case TYPE_UNICODE:
			{
				PyObject * pyo = object.ptr();
				if (!PyUnicode_Check(pyo))
					throw std::runtime_error("Failed to pack WSTRING: argument is not a valid PyUnicode instance!");

				if (sizeof(Py_UNICODE) == sizeof(uint16_t))
				{
					size_t bytes = PyUnicode_GET_DATA_SIZE(pyo);
					SGW_ASSERT(PyUnicode_AsUnicode(pyo));
					stream << (uint32_t)(bytes >> 1);
					stream.write(PyUnicode_AS_UNICODE(pyo), (uint32_t)bytes);
				}
				else
				{
					PyObject * bytes = PyUnicode_AsUTF16String(pyo);
					SGW_ASSERT(bytes);
					SGW_ASSERT(PyObject_CheckBuffer(bytes) == 1);
					Py_buffer view;
					int bufOk = PyObject_GetBuffer(bytes, &view, PyBUF_SIMPLE | PyBUF_ND | PyBUF_C_CONTIGUOUS);
					SGW_ASSERT(bufOk == 0);
					// Remove byte order mark (-2 bytes)
					SGW_ASSERT(view.len >= 2);
					stream << (uint32_t)((view.len - 2) >> 1);
					stream.write((uint8_t *)view.buf + 2, (uint32_t)(view.len - 2));
					PyBuffer_Release(&view);
					Py_DECREF(bytes);
				}
				break;
			}

		case TYPE_LIST:
			{
				SGW_ASSERT(elements.size() == 1);
				bp::list l(object);
				Py_ssize_t length = bp::len(l);
				stream << (uint32_t)length;
				for (int i = 0; i < length; i++)
				{
					elements[0].pack(stream, (bp::object)l[i]);
				}
				break;
			}

		case TYPE_FIXED_DICT:
			{
				bp::dict fixedDict(object);
				for (unsigned int i = 0; i < elements.size(); i++)
				{
					if (!fixedDict.has_key(elements[i].name))
						throw std::runtime_error("A mandatory FIXED_DICT attribute is missing");
					bp::object element = fixedDict[elements[i].name];
					elements[i].pack(stream, element);
				}
				break;
			}

		case TYPE_PICKLED:
			{
				PyObject * module = PyImport_ImportModule("pickle");
				PyObject * dump = PyObject_GetAttrString(module, "dumps");
				PyObject * pickled = PyObject_CallFunctionObjArgs(dump, object.ptr(), NULL);
				if (!pickled)
				{
					FAULTC(CATEGORY_MESSAGES, "Exception while pickling object:");
					PyUtil_ShowErr();

					bp::object dummy;
					pickled = PyObject_CallFunctionObjArgs(dump, dummy.ptr(), NULL);
				}

				SGW_ASSERT(PyBytes_Check(pickled));
	
				stream << (uint32_t)PyBytes_GET_SIZE(pickled);
				stream.write(PyBytes_AS_STRING(pickled), (uint32_t)PyBytes_GET_SIZE(pickled));
				Py_DECREF(pickled);
				Py_DECREF(dump);
				Py_DECREF(module);
				break;
			}

		default:
			{
				SGW_ASSERT(false && "Tried to pack illegal type!");
				break;
			}
		}
	}


	template <typename _T>
	bp::object unpack(_T & stream) const
	{
		switch (type)
		{
		case TYPE_NONE:
			{
				return bp::object();
			}

		case TYPE_BOOL:
			{
				uint8_t value;
				stream >> value;
				return bp::object(value ? true : false);
			}

		case TYPE_INT8:
			{
				int8_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_UINT8:
			{
				uint8_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_INT16:
			{
				int16_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_UINT16:
			{
				uint16_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_INT32:
			{
				int32_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_UINT32:
			{
				uint32_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_INT64:
			{
				int64_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_UINT64:
			{
				uint64_t value;
				stream >> value;
				return bp::long_(value);
			}

		case TYPE_FLOAT:
			{
				float value;
				stream >> value;
				return bp::object(value);
			}

		case TYPE_DOUBLE:
			{
				double value;
				stream >> value;
				return bp::object(value);
			}

		case TYPE_VECTOR3:
			{
				Vector3 * vec = new Vector3();
				stream >> vec->x >> vec->y >> vec->z;
				return bp::object(vec);
			}

		case TYPE_STRING:
			{
				uint8_t length;
				stream >> length;
				auto buf = stream.readContiguous(length);
				return bp::object(bp::handle<>(PyBytes_FromStringAndSize(reinterpret_cast<char *>(buf.data()), length)));
			}

		case TYPE_UNICODE:
			{
				uint32_t length;
				stream >> length;
				if (length > MaxStringLength)
					throw std::runtime_error("Failed to unpack WSTRING: string too long!");

				auto buf = stream.readContiguous(length * 2);
				int byteOrder = -1;
				return bp::object(bp::handle<>(PyUnicode_DecodeUTF16(reinterpret_cast<char *>(buf.data()), length * 2, nullptr, &byteOrder)));
			}

		case TYPE_LIST:
			{
				SGW_ASSERT(elements.size() == 1);
				uint32_t length;
				stream >> length;
				if (length > MaxListLength)
					throw std::runtime_error("Failed to unpack ARRAY: array too long!");

				bp::list list;

				for (unsigned int i = 0; i < length; i++)
					list.append(elements[0].unpack(stream));
	
				return list;
			}

		case TYPE_FIXED_DICT:
			{
				bp::dict fixedDict;
				for (unsigned int i = 0; i < elements.size(); i++)
				{
					bp::object element = elements[i].unpack(stream);
					fixedDict[elements[i].name] = element;
				}

				return fixedDict;
			}

		case TYPE_PICKLED:
			{
				uint32_t length;
				stream >> length;
				if (length > MaxPickledLength)
					throw std::runtime_error("Failed to unpack PYTHON: object too large!");

				auto module = bp::handle<>(PyImport_ImportModule("pickle"));
				auto load = bp::handle<>(PyObject_GetAttrString(module.get(), "loads"));
				auto buf = stream.readContiguous(length);
				auto pickled = bp::handle<>(PyByteArray_FromStringAndSize(reinterpret_cast<char *>(buf.data()), length));
				auto obj = bp::handle<>(PyObject_CallFunction(load.get(), "O", pickled.get()));
				if (!obj)
				{
					FAULTC(CATEGORY_MESSAGES, "Exception while unpickling object:");
					PyUtil_ShowErr();
					return bp::object();
				}

				return bp::object(obj);
			}

		default:
			{
				SGW_ASSERT(false && "Tried to unpack illegal type!");
				return bp::object();
			}
		}
	}

public:
	BuiltinType type;
	std::string name;
	std::vector<PyTypeInfo> elements;
};

