#pragma once

#include <entity/python.hpp>
#include <entity/mailbox.hpp>
#include <mercury/bundle.hpp>
#include <common/service.hpp>

class PyMethod;

class MailboxRpcMethod
{
public:
	static void registerType();
	static bp::object class_;

	PyMethod const * method;
	Mailbox::Ptr mailbox;
	std::function<void (MailboxRpcMethod const *, bp::object, bp::object)> handler;

private:
	static bp::object call(bp::tuple args, bp::dict kwargs);
};

class PyMethod
{
public:
	typedef std::function<void (MailboxRpcMethod const *, bp::object, bp::object)> Handler;

	static PyMethod * fromDefinition(tinyxml2::XMLElement & element, uint16_t messageId, Service::EndpointType endpoint);
	
	~PyMethod();


	template <typename _T>
	void serializeArgs(_T & stream, bp::object args) const
	{
		PyUtil_AssertGIL();
		size_t size = bp::len(args);
		for (unsigned int i = 0; i < size; i++)
		{
			args_[i].type->pack(stream, args[i]);
		}
	}


	template <typename _T>
	bp::tuple unpackArgs(_T & msg) const
	{
		PyUtil_AssertGIL();
		PyObject * args = PyTuple_New(args_.size());
		for (unsigned int i = 0; i < args_.size(); i++)
		{
			bp::object arg = args_[i].type->unpack(msg);
			PyObject * argObj = arg.ptr();
			Py_INCREF(argObj);
			PyTuple_SET_ITEM(args, i, argObj);
		}
		return bp::tuple((bp::detail::new_reference)args);
	}

	PyMethod * clone(uint16_t messageId, Handler handler) const;
	std::string const & name() const;
	bool exposed() const;
	Service::EndpointType endpointType() const;
	uint16_t messageId() const;
	MailboxRpcMethod * object() const;
	MailboxRpcMethod * object(Mailbox::Ptr boundMailbox);

private:
	struct Arg
	{
		std::string name;
		std::string comment;
		PyTypeInfo * type;
	};

	PyMethod(std::string const & name);

	uint16_t messageId_;
	std::string name_;
	std::vector<Arg> args_;
	bool exposed_;
	Service::EndpointType endpoint_;
	MailboxRpcMethod * cachedMethod_;
};

