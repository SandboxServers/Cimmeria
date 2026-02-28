#include <stdafx.hpp>
#include <baseapp/entity/base_entity.hpp>
#include <baseapp/cell_manager.hpp>
#include <entity/entity.hpp>
#include <boost/python/def.hpp>
#include <boost/python/import.hpp>
#include <boost/python/init.hpp>
#include <boost/python/scope.hpp>
#include <boost/python/handle.hpp>

template <>
BaseEntityManager<BaseEntity> * BaseEntityManager<BaseEntity>::instance_ = nullptr;

void BaseEntity::registerClass()
{
	bp::object mod = bp::import(bp::str("Atrea"));
	bp::scope active_scope(mod);
	bp::class_<BaseEntity, BaseEntity::Ptr> cls =
		bp::class_<BaseEntity, BaseEntity::Ptr>("BaseEntity");

	registerProperties(cls);
	cls.add_property("client", &BaseEntity::pyClient);
	cls.add_property("cell", &BaseEntity::pyCell);
	cls.def("disconnect", &BaseEntity::disconnect);
	cls.def("destroy", &BaseEntity::destroy);
}

BaseEntity::BaseEntity()
	: controller_(nullptr), spaceId_(0xffffffff), backup_(nullptr), pythonBackup_(nullptr)
{
	TRACE("Creating BaseEntity");
}

BaseEntity::~BaseEntity()
{
	TRACE("Deleting BaseEntity");
	delete [] backup_;
	delete [] pythonBackup_;
}

bool BaseEntity::hasBackup()
{
	return (backup_ != nullptr);
}

/*
 * Updates the client address of the entity and creates the client mailbox for
 * the specified address.
 */
void BaseEntity::setupClient(Mercury::BaseChannel::Ptr channel)
{
	clientChannel_ = channel;
	if (channel)
	{
		if (client_)
			client_->unbind();
		client_.reset(new Mailbox(classDef()->clientMailbox(), entityId()));
	}
}

/*
 * Notifies the entity a client controller was attached from the entity.
 */
void BaseEntity::attachedToController(Controller * controller)
{
	controller_ = controller;
	PyGilGuard guard;
	callMethod("attachedToController");
}

/*
 * Notifies the entity that client controller was detached from the entity.
 */
void BaseEntity::detachedFromController()
{
	PyGilGuard guard;
	callMethod("detachedFromController");
	controller_ = nullptr;
}

/*
 * Destroys the cell part of the entity (if it has one)
 */
void BaseEntity::destroyCellPart(bool removeLocally)
{
	if (spaceId_ != 0xffffffff)
	{
		// The base entity still has a cell entity attached
		// Ask the CellApp to destroy it
		SpaceData * space = CellManager::instance().findSpace(spaceId_);
		if (space && space->handler)
			space->handler->sendDestroyEntity(entityId());
		else
			WARNC(CATEGORY_ENTITY, "No cell handler found for spaceId %d", spaceId_);

		if (removeLocally)
		{
			PyGilGuard guard;
			updateSpace(0xffffffff);
		}
	}
	else
		WARNC(CATEGORY_ENTITY, "Entity %d has no cell part!", entityId());
}

/*
 * Notifies the entity that the cell entity was created for this base entity.
 * Calling this creates the cell mailbox, sets the specified space ID and calls
 * the "cellCreated" method on the python entity object.
 */
void BaseEntity::cellEntityCreated(uint32_t spaceId)
{
	if (cell_)
	{
		WARNC(CATEGORY_ENTITY, "Base entity %d (%s) already has a cell entity!", entityId(), classDef()->name().c_str());
		return;
	}

	PyGilGuard guard;
	updateSpace(spaceId);
	
	callMethod("cellCreated", bp::object(spaceId));
}

/*
 * Notifies the entity that the base failed to create the cell part of the entity.
 * (Most likely no CellApp is connected or DBID is invalid)
 * Calling this calls the "cellCreateFailed" method on the python object.
 */
void BaseEntity::cellEntityCreateFailed()
{
	if (cell_)
	{
		WARNC(CATEGORY_ENTITY, "BUG: Base entity %d (%s) has a cell entity when entity creation failed!", entityId(), classDef()->name().c_str());
		return;
	}

	PyGilGuard guard;
	callMethod("cellCreateFailed");
}

/*
 * Notifies the entity that the Cell part of the entity was destroyed.
 * Calling this deletes the cell mailbox, resets space ID and calls
 * the "cellDestroyed" method on the python entity object.
 */
void BaseEntity::cellEntityDestroyed()
{
	if (!cell_)
	{
		WARNC(CATEGORY_ENTITY, "BUG: Destroyed nonexistent cell of base entity %d (%s)!", entityId(), classDef()->name().c_str());
		return;
	}

	PyGilGuard guard;
	updateSpace(0xffffffff);
	callMethod("cellDestroyed");
}

/*
 * Notifies the entity that the cell entity is connected
 * and is ready to receive/send messages from/to the client.
 */
void BaseEntity::cellEntityConnected()
{
	PyGilGuard guard;
	callMethod("cellConnected");
}

void BaseEntity::cellEntityDisconnected()
{
	PyGilGuard guard;
	callMethod("cellDisconnected");
}

/*
 * Creates a cell player and a player viewport on the client.
 * Should be called after the client has finished loading the map.
 */
void BaseEntity::createCellPlayer(uint32_t spaceId, float x, float y, float z, 
	float yaw, float pitch, float roll)
{
	PyGilGuard guard;
	updateSpace(spaceId);

	if (controller_)
		controller_->createCellPlayer(entityId(), spaceId, x, y, z, yaw, pitch, roll);
}

/*
 * Notifies the entity that the server failed to generate or send a cooked resource
 */
void BaseEntity::cookedDataError(uint32_t categoryId, uint32_t elementId)
{
	PyGilGuard guard;
	object().attr("cookedDataError")(categoryId, elementId);
}

Mailbox::Ptr BaseEntity::client() const
{
	return client_;
}

Mailbox::Ptr BaseEntity::cell() const
{
	return cell_;
}

Controller * BaseEntity::controller() const
{
	return controller_;
}

uint32_t BaseEntity::spaceId() const
{
	return spaceId_;
}

void BaseEntity::disconnect(bool killConnection)
{
	if (controller_)
		controller_->disconnectEntity(killConnection);
}

bp::object BaseEntity::pyClient() const
{
	if (client())
		return bp::object(client());
	else
		return bp::object();
}

bp::object BaseEntity::pyCell() const
{
	if (cell())
		return bp::object(cell());
	else
		return bp::object();
}

void BaseEntity::updateSpace(uint32_t spaceId)
{
	PyUtil_AssertGIL();
	if (spaceId != spaceId_)
	{
		if (spaceId == 0xffffffff)
		{
			BaseEntityManager<BaseEntity>::instance().removeFromSpace(shared_from_this(), spaceId_);
			if (cell_)
				cell_->unbind();
			cell_.reset();

			spaceId_ = 0xffffffff;
		}
		else
		{
			BaseEntityManager<BaseEntity>::instance().addToSpace(shared_from_this(), spaceId);

			PyGilGuard guard;
			if (cell_)
				cell_->unbind();
			cell_.reset(new Mailbox(classDef()->cellMailbox(), entityId()));
			spaceId_ = spaceId;
		}
	}
}

void BaseEntity::removeFromManager()
{
	if (spaceId_ != 0xffffffff)
		destroyCellPart(true);
	BaseEntityManager<BaseEntity>::instance().destroyed(shared_from_this());
}
