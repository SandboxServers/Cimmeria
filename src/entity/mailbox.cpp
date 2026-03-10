#include <stdafx.hpp>
#include <entity/mailbox.hpp>
#include <entity/defs.hpp>
#include <entity/pyutil.hpp>
#include <common/service.hpp>
#include <mercury/base_cell_messages.hpp>

MailboxClass::MailboxClass(bp::object & module, PyClassDef * cls, Service::EndpointType mboxType, const char * namePostfix)
	: class_(cls), mailboxType_(mboxType)
{
	class_->rebind(methods_, mboxType, [this](MailboxRpcMethod const * m, bp::object args, bp::object kw) { onRpc(m, args, kw); });
	for (auto it = methods_.begin(); it != methods_.end(); ++it)
		methodsByName_.insert(std::make_pair((*it)->name(), *it));
}


MailboxClass::~MailboxClass()
{
	Py_DECREF(class_);
	for (auto it = methods_.begin(); it != methods_.end(); ++it)
		delete *it;
}


PyMethod * MailboxClass::findMethod(std::string const & name)
{
	auto it = methodsByName_.find(name);
	if (it != methodsByName_.end())
		return it->second;
	else
		return nullptr;
}


void MailboxClass::onRpc(MailboxRpcMethod const * method, bp::object args, bp::object kw)
{
	PyUtil_AssertGIL();
	doRpc(method, args, kw);
}


bp::object Mailbox::staticClass_;

void Mailbox::registerType()
{
	bp::object mod = bp::import("Atrea");
	bp::scope active_scope(mod);
	staticClass_ = bp::class_<Mailbox, Mailbox::Ptr>("Mailbox", bp::no_init)
		.def("__getattr__", &Mailbox::getattr);
}


Mailbox::Mailbox(MailboxClass * cls, uint32_t entityId)
	: class_(cls), entityId_(entityId)
{
}


Mailbox::~Mailbox()
{
}


uint32_t Mailbox::entityId() const
{
	return entityId_;
}


MailboxClass * Mailbox::mailboxClass() const
{
	return class_;
}


/*
 * Detach the mailbox from its current owner entity.
 */
void Mailbox::unbind()
{
	class_ = nullptr;
	entityId_ = 0;
}


void Mailbox::setEntityId(uint32_t entityId)
{
	entityId_ = entityId;
}


/*
 * Python __getattr__ replacement
 * Returns a cached instance of a mailbox method.
 * 
 * @param self Mailbox we're querying
 * @param attr Name of attribute (method name in this case) to query
 */
bp::object Mailbox::getattr(Mailbox * self, std::string const & attr)
{
	PyUtil_AssertGIL();
	// If the entity was destroyed or left the current space, its
	// mailbox bindings will be removed to avoid dangling references
	if (!self->class_)
		throw std::runtime_error("Mailbox is not bound to an entity");

	PyMethod * method = self->class_->findMethod(attr);
	if (method)
	{
		return bp::object(method->object(self->shared_from_this()));
	}
	else
	{
		const char * mboxType = 
			(self->class_->mailboxType() == Service::BaseMailbox) ? "base" :
			(self->class_->mailboxType() == Service::CellMailbox) ? "cell" :
			"client";
		PyErr_Format(PyExc_KeyError, "No such method in mailbox %s (%s): %s",
			self->class_->entityClass()->name().c_str(), mboxType, attr.c_str());
		return bp::object();
	}
}


ClientMailboxClass::ClientMailboxClass(bp::object & module, PyClassDef * cls, Service::EndpointType mboxType)
	: MailboxClass(module, cls, mboxType, "ClientMailbox")
{
}


ClientMailboxClass::~ClientMailboxClass()
{
}


void ClientMailboxClass::doRpc(MailboxRpcMethod const * method, bp::object args, bp::object kw)
{
	MessageWriter * writer = Service::instance().messageWriter(method->method->endpointType(), method->mailbox->entityId());
	MessageWriter::DistributionPolicy policy;
	policy.flags = Mercury::DISTRIBUTE_CLIENT;
	writer->write(method->method->endpointType(), policy, method->mailbox->entityId(), method->method->messageId(), args);
}

