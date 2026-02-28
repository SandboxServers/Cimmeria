#include <stdafx.hpp>
#include <entity/entity.hpp>
#include <boost/python/def.hpp>
#include <boost/python/import.hpp>
#include <boost/python/init.hpp>
#include <boost/python/scope.hpp>

#if defined(_DEBUG)
// Define to show entity method calls from C++
// #define DEBUG_ENTITY_CALLS
#endif

Entity::Entity()
	: databaseId_(-1), class_(nullptr), destroyed_(false)
{
}

Entity::~Entity()
{
	if (class_)
		TRACEC(CATEGORY_ENTITY, "Entity destroyed (entityId %d, class %s)", 
			entityId_, class_->name().c_str());
}

void Entity::setup(uint32_t entityId, PyClassDef * cls)
{
	entityId_ = entityId;
	class_ = cls;
}

void Entity::rpc(std::string const & method, bp::tuple args)
{
	PyUtil_AssertGIL();
	try
	{
		object_.attr(method.c_str())(*args);
	}
	catch (bp::error_already_set &)
	{
		FAULTC(CATEGORY_ENTITY, "Exception while calling while calling RPC '%s.%s':", 
			class_->name().c_str(), method.c_str());
		PyUtil_ShowErr();
	}
}

void Entity::persist()
{
	TRACEC(CATEGORY_ENTITY, "Persisting entity %d (%s)", entityId_, class_->name().c_str());
	callMethod("persist");
}

bool Entity::load(int32_t databaseId)
{
	databaseId_ = databaseId;

	TRACEC(CATEGORY_ENTITY, "Loading entity %d (%s)", entityId_, class_->name().c_str());
	return !callMethod("load").is_none();
}

uint32_t Entity::entityId() const
{
	return entityId_;
}

int32_t Entity::databaseId() const
{
	return databaseId_;
}

PyClassDef * Entity::classDef() const
{
	return class_;
}

void Entity::destroy()
{
	if (!destroyed_)
	{
		PyGilGuard guard;
		// Call pre-destroy hook (mainly to remove refs)
		callMethod("destroyed");

		persist();
		destroyed_ = true;
		/* if (toObject(this)->ob_refcnt != 1)
		{
			TRACE("Not destroying entity %d (still %d alive references)", entityId_, 
				toObject(this)->ob_refcnt);
		} */
		// This should be the last instruction, since
		// it can remove all references to this entity
		removeFromManager();
	}
}

void Entity::setObject(bp::object self)
{
	SGW_ASSERT(!object_);
	object_ = self;
}

bp::object Entity::object() const
{
	SGW_ASSERT(object_);
	return object_;
}

bp::object Entity::callMethod(std::string const & method)
{
	PyUtil_AssertGIL();
#if defined(DEBUG_ENTITY_CALLS)
	TRACE("Calling %s.%s()", classDef()->name().c_str(), method.c_str());
#endif

	try
	{
		return object().attr(method.c_str())();
	}
	catch (bp::error_already_set &)
	{
		WARN("Error while calling entity method %s.%s:", method.c_str());
		PyUtil_ShowErr();
		return bp::object();
	}
}

bp::object Entity::callMethod(std::string const & method, bp::object obj)
{
	PyUtil_AssertGIL();
#if defined(DEBUG_ENTITY_CALLS)
	TRACE("Calling %s.%s()", classDef()->name().c_str(), method.c_str());
#endif

	try
	{
		return object().attr(method.c_str())(obj);
	}
	catch (bp::error_already_set &)
	{
		FAULT("Exception while calling entity method %s.%s:", classDef()->name().c_str(), method.c_str());
		PyUtil_ShowErr();
		return bp::object();
	}
}

void Entity::removeFromManager()
{
}
