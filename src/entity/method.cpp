#include <stdafx.hpp>
#include "entity/method.hpp"
#include "entity/defs.hpp"
#include "entity/entity.hpp"
#include "entity/pyutil.hpp"
#include <boost/algorithm/string.hpp>
#include <boost/python/raw_function.hpp>

using namespace boost::algorithm;

bp::object MailboxRpcMethod::class_;

void MailboxRpcMethod::registerType()
{
	class_ = bp::class_<MailboxRpcMethod>("Method", bp::init<>())
		.def("__call__", bp::raw_function(&MailboxRpcMethod::call, 1));
}

bp::object MailboxRpcMethod::call(bp::tuple args, bp::dict kwargs)
{
	auto self = bp::extract<MailboxRpcMethod *>(args[0])();

	// TODO: Find a better way to pass args from raw_function
	size_t numArgs = bp::len(args) - 1;
	PyObject * handlerArgs = PyTuple_New(numArgs);
	for (unsigned int i = 0; i < numArgs; i++)
	{
		PyObject * argObj = ((bp::object)args[i + 1]).ptr();
		Py_INCREF(argObj);
		PyTuple_SET_ITEM(handlerArgs, i, argObj);
	}
	auto handlerArgsO = bp::object((bp::detail::new_reference)handlerArgs);

	self->handler(self, std::ref(handlerArgsO), std::ref(kwargs));
	return bp::object();
}

PyMethod * PyMethod::fromDefinition(tinyxml2::XMLElement & element, uint16_t messageId, Service::EndpointType endpoint)
{
	PyMethod * method = new PyMethod(element.Name());
	method->messageId_ = messageId;
	method->endpoint_ = endpoint;
	if (element.FirstChildElement("Exposed"))
		method->exposed_ = true;

	for (tinyxml2::XMLElement * argElement = element.FirstChildElement("Arg");
		argElement; argElement = argElement->NextSiblingElement("Arg"))
	{
		Arg arg;
		tinyxml2::XMLElement * argName = argElement->FirstChildElement("ArgName");
		if (argName)
		{
			arg.name = argName->GetText();
			trim(arg.name);
		}

		tinyxml2::XMLNode * comment = argElement->NextSibling();
		if (comment && comment->ToComment())
		{
			arg.comment = comment->Value();
			trim(arg.comment);
		}
		
		arg.type = PyParseType(*argElement);
		method->args_.push_back(arg);
	}

	return method;
}

PyMethod::PyMethod(std::string const & name)
	: messageId_(0), name_(name), exposed_(false),
	endpoint_(Service::ClientMailbox)
{
	cachedMethod_ = new MailboxRpcMethod();
	cachedMethod_->method = this;
}

PyMethod::~PyMethod()
{
}

PyMethod * PyMethod::clone(uint16_t messageId, Handler handler) const
{
	PyMethod * method = new PyMethod(name_);
	method->messageId_ = messageId;
	method->endpoint_ = endpoint_;
	method->cachedMethod_->handler = handler;
	method->args_.insert(method->args_.begin(), args_.begin(), args_.end());
	method->exposed_ = exposed_;
	return method;
}

std::string const & PyMethod::name() const
{
	return name_;
}

bool PyMethod::exposed() const
{
	return exposed_;
}

Service::EndpointType PyMethod::endpointType() const
{
	return endpoint_;
}

uint16_t PyMethod::messageId() const
{
	return messageId_;
}

MailboxRpcMethod * PyMethod::object(Mailbox::Ptr boundMailbox)
{
	cachedMethod_->mailbox = boundMailbox;
	return cachedMethod_;
}
