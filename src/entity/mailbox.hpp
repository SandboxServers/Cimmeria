#pragma once

#include <mercury/nub.hpp>
#include <common/service.hpp>

class PyClassDef;
class PyMethod;
class MailboxRpcMethod;
class Mailbox;

class MailboxClass
{
public:
	MailboxClass(bp::object & module, PyClassDef * cls, Service::EndpointType mboxType, const char * namePostfix);
	virtual ~MailboxClass();

	inline PyClassDef * entityClass() const
	{
		return class_;
	}

	inline Service::EndpointType mailboxType() const
	{
		return mailboxType_;
	}

	PyMethod * findMethod(std::string const & name);

private:
	void onRpc(MailboxRpcMethod const * method, bp::object args, bp::object kw);
	virtual void doRpc(MailboxRpcMethod const * method, bp::object args, bp::object kw) = 0;

	PyClassDef * class_;
	Service::EndpointType mailboxType_;
	std::vector<PyMethod *> methods_;
	std::map<std::string, PyMethod *> methodsByName_;
};

class Mailbox : public boost::enable_shared_from_this<Mailbox>
{
public:
	typedef boost::shared_ptr<Mailbox> Ptr;
	static void registerType();

	Mailbox(MailboxClass * cls, uint32_t entityId);
	~Mailbox();

	uint32_t entityId() const;
	MailboxClass * mailboxClass() const;

	void unbind();
	void setEntityId(uint32_t entityId);
private:
	MailboxClass * class_;
	uint32_t entityId_;
	
	static bp::object getattr(Mailbox * self, std::string const & attr);
	static bp::object staticClass_;
};

class ClientMailboxClass : public MailboxClass
{
public:
	ClientMailboxClass(bp::object & module, PyClassDef * cls, Service::EndpointType mboxType);
	virtual ~ClientMailboxClass();

private:
	virtual void doRpc(MailboxRpcMethod const * method, bp::object args, bp::object kw);
};

