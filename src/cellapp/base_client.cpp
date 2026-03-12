#include <stdafx.hpp>
#include <cellapp/base_client.hpp>
#include <mercury/base_cell_messages.hpp>
#include <mercury/transparent_filter.hpp>
#include <mercury/nub.hpp>
#include <entity/defs.hpp>
#include <boost/python/wrapper.hpp>
#include <cellapp/space.hpp>
#include <cellapp/cell_service.hpp>
#include <cellapp/entity/cell_entity.hpp>
#include <cellapp/entity/entity_manager.hpp>
#include <cellapp/entity/movement.hpp>

#if defined(_DEBUG)
// Define to show incoming and outgoing messages to/from the BaseApp
#define CELL_DEBUG_MESSAGES
// Define to show entity creation/destruction and feedback messages
// #define CELL_DEBUG_ENTITIES
// Define to show RPC and cached RPC messages
// #define CELL_DEBUG_DISTRIBUTION
#endif

BaseAppClient::BaseAppClient()
	: Mercury::UnifiedConnection(0),
	  recoveryTimer_(Service::instance().ioService()),
	  registered_(false), cacheWriter_(nullptr)
{
}

BaseAppClient::~BaseAppClient()
{
	TRACE("");
}

void BaseAppClient::reconnectIn(uint32_t msecs)
{
	registered_ = false;

	recoveryTimer_.expires_after(std::chrono::milliseconds(msecs));
	auto self = shared_this();
	recoveryTimer_.async_wait([self](const boost::system::error_code & err) {
		self->reconnectTimerExpired(err);
	});
}

void BaseAppClient::startup()
{
	SGW_ASSERT(!isConnected());
	SGW_ASSERT(!registered_);
	reconnect();
}

void BaseAppClient::reconnectTimerExpired(const boost::system::error_code & errcode)
{
	if (errcode == boost::system::errc::success)
	{
		reconnect();
	}
	else
	{
		close();
	}
}

void BaseAppClient::reconnect()
{
	TRACE("Connecting to BaseApp ...");
	connect(
		Service::instance().getConfigParameter("baseapp_address"),
		(uint16_t)atol(Service::instance().getConfigParameter("baseapp_port").c_str())
	);
}


/**
 * Called when an acknowledgement is received to the authentication request
 */
void BaseAppClient::onRegistrationAck(Reader & msg)
{
	uint32_t protocolHash, protocolVersion;
	msg >> protocolHash >> protocolVersion;

	if (protocolHash != Mercury::BaseAppHandshakeHash)
	{
		FAULTC(CATEGORY_MESSAGES, "BaseApp communication error: invalid protocol hash: 0x%08x", protocolHash);
		close();
		return;
	}

	if (protocolVersion != Mercury::BaseCellProtocolVersion)
	{
		FAULTC(CATEGORY_MESSAGES, "BaseApp communication error: protocol version mismatch");
		FAULTC(CATEGORY_MESSAGES, "BaseApp version: %d, CellApp version: %d", protocolVersion, Mercury::BaseCellProtocolVersion);
		close();
		return;
	}
	
	// Only read additional fields after the protocol check was done
	// as these can differ between versions
	uint32_t response, ticks, tickRate;
	msg >> response >> ticks >> tickRate;
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell: AUTHENTICATE_ACK: status=%d", response);
#endif

	if (response != 0)
	{
		FAULTC(CATEGORY_MESSAGES, "BaseApp registration failed: response code: %d", response);
		FAULTC(CATEGORY_MESSAGES, "Will retry connection in %d seconds", ConnectionRecoveryTimeout / 1000);
		close();
		return;
	}

	INFOC(CATEGORY_MESSAGES, "Cell registered with BaseApp (CellId: %d)", CellService::cell_instance().cellId());
	SpaceManager::instance().initTicks(ticks, tickRate);
	DEBUG1C(CATEGORY_MESSAGES, "Synchronized base ticks: %d, tickrate: %d", ticks, tickRate);

	registered_ = true;

	for (auto space : SpaceManager::instance().spaces())
	{
		if (space != nullptr)
		{
			sendSpaceData(space->spaceId(), CellService::cell_instance().cellId(), space->worldName());

			for (auto const & entity : space->entities())
			{
				sendCellEntityCreated(
					entity.second->entityId(), entity.second->spaceId(), 
					(uint8_t)entity.second->classDef()->index(), entity.second->position(),
					entity.second->entityFlags()
				);
				entity.second->cacheStampsReset();
			}
		}
	}
}


/**
 * Called when the BaseApp requested the cell to create a cell part for a base entity
 */
void BaseAppClient::onCreateEntityRequest(Reader & msg)
{
	uint32_t spaceId, entityId, dbid, propertyCount;
	Vec3 pos, dir;
	std::string className, worldName;
	msg >> spaceId >> entityId >> dbid
		>> pos.x >> pos.y >> pos.z
		>> dir.x >> dir.y >> dir.z;
	msg.readString(className);
	msg.readString(worldName);
	msg >> propertyCount;
		
#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell: CREATE_ENTITY: spaceId=%d, entityId=%d, dbId=%d, properties=%d",
		spaceId, entityId, dbid, propertyCount);
#endif

	CellEntity::Ptr entity = createEntity(entityId, dbid, spaceId, className, worldName, pos, dir);
	if (!entity)
	{
		// Failed to create entity, send an error response (spaceId 0xffffffff = creation failed)
		sendEntityCreateAck(entityId, 0xffffffff);
	}
	else
	{
		std::string propName;
		for (unsigned int i = 0; i < propertyCount; i++)
		{
			msg.readString(propName);
			bp::object obj = PyTypeDb::instance().findType("PYTHON")->unpack(msg);
			bp::setattr(entity->object(), propName.c_str(), obj);
		}
		sendEntityCreateAck(entityId, entity->spaceId());
	}
}


/**
 * Called when the BaseApp requested the cell to destroy the cell part of an entity
 */
void BaseAppClient::onDestroyEntityRequest(Reader & msg)
{
	uint32_t entityId;
	msg >> entityId;

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell: DESTROY_ENTITY: entityId=%d", entityId);
#endif
		
	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (entity)
	{
		PyGilGuard guard;
		entity->destroy();
		// This needs to be here, otherwise the destructor will be called in
		// the shared_ptr dtor when the GIL is not locked
		entity = CellEntity::Ptr();
	}
	else
		WARNC(CATEGORY_ENTITY, "Failed to destroy nonexistent entity: %d", entityId);

	sendEntityDeleteAck(entityId);
}


/**
 * Called when a client controller is attached to an entity
 * (the player is taking control of that entity)
 */
void BaseAppClient::onConnectEntityRequest(Reader & msg)
{
	uint32_t entityId;
	msg >> entityId;

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell: CONNECT_ENTITY: entityId=%d", entityId);
#endif

	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (entity)
	{
		entity->connected();
	}
	else
		WARNC(CATEGORY_ENTITY, "Failed to connect nonexistent entity: %d", entityId);
}


/**
 * Called when a client controller is detached from an entity
 * (the player is releasing control of that entity)
 */
void BaseAppClient::onDisconnectEntityRequest(Reader & msg)
{
	uint32_t entityId;
	msg >> entityId;

#if defined(CELL_DEBUG_ENTITIES)
	// TRACEC(CATEGORY_MESSAGES, "Base -> Cell: MOVE: entityId=%d", entityId);
#endif

	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (entity)
	{
		Vec3 position, rotation, velocity;
		uint8_t flags;
		msg >> position.x >> position.y >> position.z;
		msg >> rotation.x >> rotation.y >> rotation.z;
		msg >> velocity.x >> velocity.y >> velocity.z;
		msg >> flags;

		if (entity->controller())
			dynamic_cast<PlayerController *>(entity->controller())->playerUpdate(position, rotation, velocity, flags);
		else
			WARNC(CATEGORY_ENTITY, "Entity %d has no movement controller!", entityId);
	}
	else
		WARNC(CATEGORY_ENTITY, "Failed to move nonexistent entity: %d", entityId);
}


/**
 * Called when the entity moved on the client side
 */
void BaseAppClient::onEntityMove(Reader & msg)
{
	uint32_t entityId;
	msg >> entityId;
#if defined(CELL_DEBUG_MESSAGES)
	// TRACEC(CATEGORY_MESSAGES, "Base -> Cell: MOVE: entityId=%d", entityId);
#endif

	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (entity)
	{
		Vec3 position, rotation, velocity;
		uint8_t flags;
		msg >> position.x >> position.y >> position.z;
		msg >> rotation.x >> rotation.y >> rotation.z;
		msg >> velocity.x >> velocity.y >> velocity.z;
		msg >> flags;

		if (entity->controller())
			dynamic_cast<PlayerController *>(entity->controller())->playerUpdate(position, rotation, velocity, flags);
		else
			WARNC(CATEGORY_ENTITY, "Entity %d has no movement controller!", entityId);
	}
	else
		WARNC(CATEGORY_ENTITY, "Failed to move nonexistent entity: %d", entityId);
}


/**
 * Restores an entity from its BaseApp backup
 */
void BaseAppClient::onRestoreEntityRequest(Reader & msg)
{
	uint32_t spaceId, entityId;
	int8_t classId;
	bool recovery;
	std::string worldName;
	Vec3 pos, rot;
	msg >> spaceId >> entityId >> classId >> recovery;
	msg.readString(worldName);
	msg >> pos.x >> pos.y >> pos.z;
	msg >> rot.x >> rot.y >> rot.z;
#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell: RESTORE_ENTITY: entityId=%d, spaceId=%d, recovery=%d", entityId, spaceId, (int)recovery);
#endif

	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (!entity)
	{
		TRACEC(CATEGORY_ENTITY, "Entity %d does not exist, creating before restore", entityId);
		PyClassDef * cls = PyTypeDb::instance().findClass(classId);
		entity = CellEntityManager<CellEntity>::instance().create(entityId, cls, -1);
		if (!entity)
		{
			WARNC(CATEGORY_ENTITY, "Failed to create entity (%d) for restore on world (%s)",
				entityId, worldName.c_str());
			sendEntityCreateAck(entityId, 0xffffffff);
			return;
		}
	}
	else
	{
		WARNC(CATEGORY_ENTITY, "Entity %d already exists before restore", entityId);
	}

	Space * space = findOrCreateSpace(spaceId, worldName, entityId);
	if (space)
	{
		entity->restore(msg, space->spaceId(), pos, rot);
		sendEntityCreateAck(entityId, entity->spaceId());
	}
	else
	{
		WARNC(CATEGORY_ENTITY, "Failed to restore entity (%d) on nonexistent space (%d, world %s)",
			entityId, spaceId, worldName.c_str());
		sendEntityCreateAck(entityId, 0xffffffff);
	}
}


/**
 * Called when an entity message was received from the BaseApp
 */
void BaseAppClient::onBaseAppMessage(Reader & msg)
{
	uint8_t messageId;
	uint32_t entityId;
	msg >> entityId >> messageId;

	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "No such entity: %d", entityId);
		return;
	}

	PyMethod * method = entity->classDef()->findMethod(Service::CellMailbox, messageId);
	if (!method)
	{
		WARNC(CATEGORY_MESSAGES, "No internal cell method exposed for entity '%s', methodId %d",
			entity->classDef()->name().c_str(), messageId);
		return;
	}

	PyGilGuard guard;
	auto args = method->unpackArgs(msg);
	entity->rpc(method->name().c_str(), args);
}


/**
 * Called when an entity message was received from the client
 */
void BaseAppClient::onClientMessage(Reader & msg)
{
	uint8_t messageId;
	uint32_t entityId;
	msg >> entityId >> messageId;

	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "No such entity: %d", entityId);
		return;
	}

	PyMethod * method = entity->classDef()->findMethod(Service::CellExposedMailbox, messageId);
	if (!method)
	{
		WARNC(CATEGORY_MESSAGES, "No public cell method for entity '%s', methodId %d",
			entity->classDef()->name().c_str(), messageId);
		return;
	}

	PyGilGuard guard;
	auto args = method->unpackArgs(msg);
	entity->rpc(method->name().c_str(), args);
}


/**
 * Called when the CellApp needs to send property updates for an entity
 */
void BaseAppClient::onRequestEntityUpdate(Reader & msg)
{
	uint32_t witnessId, entityId;
	uint8_t flags;
	msg >> witnessId >> entityId >> flags;
	
#if defined(CELL_DEBUG_DISTRIBUTION)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell: REQ_ENTITY_UPDATE: witnessId=%d, entityId=%d, flags=%d", 
		witnessId, entityId, (uint32_t)flags);
#endif

	CellEntity::Ptr witness = CellEntityManager<CellEntity>::instance().get(witnessId);
	if (!witness)
	{
		WARN("Cannot update unknown witness %d!", witnessId);
		return;
	}

	if (!witness->isPlayer())
	{
		WARN("Cannot send updates to non-player witness %d!", witnessId);
		return;
	}

	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().get(entityId);
	if (!witness)
	{
		WARN("Cannot update unknown entity %d!", entityId);
		return;
	}
	
	// Notify all players on the space that the entity entered their AoI
	// CellService::cell_instance().baseClient()->sendEnteredAoI(spaceId(), entity.get(), 0xffffffff);
	
	if (flags & Mercury::UPDATE_STATIC)
	{
		// Update entity properties on all clients
		CellMailbox * mailbox = static_cast<CellMailbox *>(entity->witnesses().get());
		SGW_ASSERT(mailbox->distributionFlags() == Mercury::DISTRIBUTE_LOD);
		mailbox->setDistributionFlags(Mercury::DISTRIBUTE_RECIPIENT);
		mailbox->setRecipient(witnessId);
		entity->createOnClient(entity->witnesses());
		mailbox->setDistributionFlags(Mercury::DISTRIBUTE_LOD);
		mailbox->setRecipient(0);
	}

	// If the entity has non-static properties (eg. interaction flags), notify
	// the player that it should update these properties
	if (flags & Mercury::UPDATE_DYNAMIC && entity->hasDynamicProperties())
	{
		witness->enteredAoI(entity);
	}

	// TODO: UPDATE_UNCACHED
}


/**
 * Sends a registration request to the BaseApp to register this cell
 */
void BaseAppClient::sendAuthenticationRequest()
{
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_AUTHENTICATE);
	request << Mercury::CellAppHandshakeHash << Mercury::BaseCellProtocolVersion
		<< (uint32_t)CellService::cell_instance().cellId();
	request.endMessage();
}


/**
 * Updates space availability information about a space on the BaseApp.
 *
 * @param spaceId   Space to update
 * @param cellId    Cell instance ID running this space (0xffffffff = space is destroyed)
 * @param worldName World name of this space
 */
void BaseAppClient::sendSpaceData(uint32_t spaceId, uint32_t cellId, std::string const & worldName)
{
	SGW_ASSERT(isReady());

#if defined(CELL_DEBUG_MESSAGES)
	TRACE("Cell -> Base: SPACE_DATA: spaceId=%d, cellId=%d, worldName=%s", spaceId, cellId, worldName.c_str());
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_SPACE_DATA);
	request << spaceId << cellId;
	request.writeString(worldName);
	request.endMessage();
}


/**
 * Notifies the BaseApp that the cell part of an entity was created.
 *
 * @param entityId Created entity ID
 * @param spaceId  Space instance ID (0xffffffff = entity create failed)
 */
void BaseAppClient::sendEntityCreateAck(uint32_t entityId, uint32_t spaceId)
{
	SGW_ASSERT(isReady());

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: ENTITY_CREATE_ACK: entityId=%d, spaceId=%d", entityId, spaceId);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_ENTITY_CREATE_ACK);
	request << entityId << spaceId;
	request.endMessage();
}


/**
 * Notifies the BaseApp that the cell part of an entity was destroyed.
 *
 * @param entityId Destroyed entity ID
 */
void BaseAppClient::sendEntityDeleteAck(uint32_t entityId)
{
	SGW_ASSERT(isReady());

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: ENTITY_DELETE_ACK: entityId=%d", entityId);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_ENTITY_DELETE_ACK);
	request << entityId;
	request.endMessage();
}


/**
 * Notifies the BaseApp that the cell part of an entity was created.
 *
 * @param entityId  Player entity ID
 * @param spaceId   ID of space the player entered
 * @param position  Position of player
 * @param rotation  Rotation of player
 */
void BaseAppClient::sendCellPlayerCreated(uint32_t entityId, uint32_t spaceId,
	Vec3 const & position, Vec3 const & rotation)
{
	SGW_ASSERT(isReady());

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: CREATE_CELL_PLAYER: entityId=%d, spaceId=%d, loc=(%f, %f, %f)", 
		entityId, spaceId, position.x, position.y, position.z);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_CREATE_CELL_PLAYER);
	request << entityId << spaceId <<
		position.x << position.y << position.z << 
		rotation.x << rotation.y << rotation.z;
	request.endMessage();
}


/**
 * Notifies the BaseApp about current game time on the cell.
 *
 * @param time Current game time
 */
void BaseAppClient::sendGameTick(uint64_t time)
{
	if (!isReady())
		return;

#if defined(CELL_DEBUG_MESSAGES)
	// TRACEC(CATEGORY_MESSAGES, "Cell -> Base: TICK: time=%ld", time);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_TICK);
	request << time;
	request.endMessage();
}


/**
 * Notifies a base entity that another entity entered its AoI (area of interest).
 *
 * @param spaceId   Space to update
 * @param entity    Entering entity
 * @param witnessId Entity ID of witness
 */
void BaseAppClient::sendEnteredAoI(uint32_t spaceId, CellEntity * entity, uint32_t witnessId)
{
	if (!isReady())
		return;

	Vec3 & pos = entity->position();
	Vec3 & rot = entity->rotation();
#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: ENTER_AOI: witnessId=%d, entityId=%d, spaceId=%d, loc=(%f, %f, %f)", 
		witnessId, entity->entityId(), spaceId, pos.x, pos.y, pos.z);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_ENTER_AOI);
	request << witnessId << spaceId << entity->entityId() <<
		(uint8_t)entity->classDef()->index() <<
		pos.x << pos.y << pos.z <<
		rot.x << rot.y << rot.z;
	request.endMessage();
}


/**
 * Notifies a base entity that another entity left its AoI (area of interest).
 *
 * @param spaceId      Space to update
 * @param entityId     Entity leaving the AoI
 * @param deleteEntity Was the entity deleted or just made invisible?
 */
void BaseAppClient::sendLeftAoI(uint32_t spaceId, uint32_t entityId, bool deleteEntity)
{
	if (!isReady())
		return;

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: LEAVE_AOI: entityId=%d, spaceId=%d, delete=%d", 
		spaceId, entityId, deleteEntity ? 1 : 0);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_LEAVE_AOI);
	request << spaceId << entityId << (uint8_t)(deleteEntity ? 1 : 0);
	request.endMessage();
}


/**
 * Notifies clients that an entity has moved / is moving.
 *
 * @param spaceId      Space to update
 * @param entity       Entering entity
 * @param deleteEntity Was the entity deleted or just made invisible?
 */
void BaseAppClient::sendEntityMove(CellEntity * entity, bool forced)
{
	if (!isReady())
		return;

	Vec3 & pos = entity->position();
	Vec3 & rot = entity->rotation();
	Vec3 & vel = entity->velocity();
#if defined(CELL_DEBUG_ENTITIES)
	// TRACEC(CATEGORY_MESSAGES, "Cell -> Base: MOVE: entityId=%d, spaceId=%d, loc=(%f, %f, %f)", 
	// 	entity->entityId(), entity->space()->spaceId(), pos.x, pos.y, pos.z);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_MOVE);
	uint8_t forceFlag = forced ? 1 : 0;
	request << entity->space()->spaceId() << entity->entityId() <<
		pos.x << pos.y << pos.z <<
		rot.x << rot.y << rot.z <<
		vel.x << vel.y << vel.z <<
		entity->movementType() <<
		forceFlag;
	request.endMessage();
}


/**
 * Creates a backup of the cell state of the entity and sends it to the BaseApp.
 *
 * @param entity Entity to back up
 */
void BaseAppClient::sendEntityBackup(CellEntity * entity)
{
	SGW_ASSERT(isReady());

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: BACKUP_ENTITY: entityId=%d, spaceId=%d", 
		entity->entityId(), entity->space()->spaceId());
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_BACKUP_ENTITY);
	request << entity->space()->spaceId() << entity->entityId();
	entity->backup(request);
	request.endMessage();
}


/**
 * Notifies the BaseApp that the restore of a cell entity was successful.
 *
 * @param entityId Entity that was restored
 */
void BaseAppClient::sendEntityRestoreAck(uint32_t entityId)
{
	SGW_ASSERT(isReady());

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: RESTORE_ENTITY_ACK: entityId=%d", entityId);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_RESTORE_ENTITY_ACK);
	request << entityId;
	request.endMessage();
}


/**
 * Requests the BaseApp to start moving an entity to a different space.
 *
 * @param entityId  Entity to move
 * @param spaceId   Destination space ID
 * @param worldName Destination world name
 * @param position  Destination position
 * @param rotation  Destination orientation
 */
void BaseAppClient::sendSwitchSpace(uint32_t entityId, uint32_t spaceId, std::string const & worldName, 
	Vec3 const & position, Vec3 const & rotation)
{
	SGW_ASSERT(isReady());

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: SWITCH_SPACE: entityId=%d, spaceId=%d", 
		entityId, spaceId);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_SWITCH_SPACE);
	request << entityId << spaceId;
	request.writeString(worldName);
	request << position.x << position.y << position.z;
	request << rotation.x << rotation.y << rotation.z;
	request.endMessage();
}


/**
 * Calls an RPC method on a base entity.
 *
 * @param entityId  Recipient of RPC message
 * @param messageId Message type to send
 * @param method    RPC arguments serializer
 * @param args      Arguments to send
 */
void BaseAppClient::sendBaseAppMessage(uint32_t entityId, uint8_t messageId, PyMethod & method, bp::object args)
{
	if (!isReady())
		return;

#if defined(CELL_DEBUG_DISTRIBUTION)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: BASE_RPC: entityId=%d, message=%s", 
		entityId, method.name().c_str());
#endif
	SGW_ASSERT(!cacheWriter_);
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_BASEAPP_MESSAGE);
	request << entityId << messageId;
	method.serializeArgs(request, args);
	request.endMessage();
}


/**
 * Calls an RPC method on a client entity.
 *
 * @param entityId     Entity to call this RPC method on
 * @param messageId    Message type to send
 * @param distribution Message distribution policy
 * @param method       RPC arguments serializer
 * @param args         Arguments to send
 */
void BaseAppClient::sendClientMessage(uint32_t entityId, uint8_t messageId, 
	MessageWriter::DistributionPolicy distribution, PyMethod & method, bp::object args)
{
	if (!isReady())
		return;

	if (cacheWriter_)
	{
#if defined(CELL_DEBUG_DISTRIBUTION)
		TRACEC(CATEGORY_MESSAGES, "Cell -> Base: CACHE_MESSAGE: entityId=%d, message=%s, dist=%02x", 
			entityId, method.name().c_str(), distribution.flags);
#endif
		SGW_ASSERT(cacheEntityId_ == entityId && distribution.flags == Mercury::DISTRIBUTE_LOD);
		Writer & request = *cacheWriter_;
		// TODO: Write message flags!
		request << messageId << (uint8_t)0;
		// Write a dummy message length, then seek back and write the real one after serialization
		std::size_t sizePos = request.position();
		request << (uint16_t)0;
		method.serializeArgs(request, args);
		// Message should be serializable to the client (length field is 16 bits)
		SGW_ASSERT(request.position() - sizePos - 2 <= 0xffff);
		request.writeAt(sizePos, (uint16_t)(request.position() - sizePos - 2));
	}
	else
	{
#if defined(CELL_DEBUG_DISTRIBUTION)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: CELL_RPC: entityId=%d, message=%s, dist=%02x", 
		entityId, method.name().c_str(), distribution.flags);
#endif

		Writer request(*this);
		request.beginMessage(Mercury::CELL_BASE_CLIENT_MESSAGE);
		request << entityId << messageId << distribution.flags;
		if (distribution.flags & Mercury::DISTRIBUTE_RECIPIENT)
		{
			request << distribution.recipientId;
		}

		if (distribution.flags & (Mercury::DISTRIBUTE_LOD | Mercury::DISTRIBUTE_SPACE))
		{
			request << distribution.spaceId;
		}

		method.serializeArgs(request, args);
		request.endMessage();
	}
}


/**
 * Notifies the BaseApp that a local entity was created on the cell.
 *
 * @param entityId  ID of new entity
 * @param spaceId   Space the entity is bound to
 * @param classId   Entity type ID
 * @param position  Position of the entity
 * @param flags     Entity flags (Mercury.CellEntityFlags)
 */
void BaseAppClient::sendCellEntityCreated(uint32_t entityId, uint32_t spaceId, uint8_t classId, 
										  Vec3 const & position, uint8_t flags)
{
	if (!isReady())
		return;

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: NOTIFY_CREATE_CELL_ENTITY: entityId=%d, spaceId=%d", 
		entityId, spaceId);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_NOTIFY_CREATE_CELL_ENTITY);
	request << entityId << spaceId << classId << flags;
	request << position.x << position.y << position.z;
	request.endMessage();
}


/**
 * Notifies the BaseApp that a local entity was destroyed on the cell.
 *
 * @param entityId  ID of entity to delete
 */
void BaseAppClient::sendCellEntityDeleted(uint32_t entityId)
{
	if (!isReady())
		return;

#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: NOTIFY_DELETE_CELL_ENTITY: entityId=%d", entityId);
#endif
	Writer request(*this);
	request.beginMessage(Mercury::CELL_BASE_NOTIFY_DELETE_CELL_ENTITY);
	request << entityId;
	request.endMessage();
}

/**
 * Begin sending a cache stamp update to the BaseApp.
 * Messages in the update must be added using sendClientMessage().
 * To finish the update, call endCacheStamp().
 * 
 * @param entityId      ID of entity to update
 * @param propertySetId ID of the property set to update (0-based index)
 * @param invalidate    Invalidate currently cached updates in this set?
 */
void BaseAppClient::beginCacheStamp(uint32_t entityId, uint32_t propertySetId, bool invalidate)
{
	SGW_ASSERT(!cacheWriter_ && isReady());

#if defined(CELL_DEBUG_DISTRIBUTION)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: UPDATE_CACHE_STAMP: entityId=%d, propset=%d, invd=%s", 
		entityId, propertySetId, invalidate ? "true" : "false");
#endif

	cacheWriter_ = new Writer(*this);
	cacheWriter_->beginMessage(Mercury::CELL_BASE_UPDATE_CACHE_STAMP);
	*cacheWriter_ << entityId << propertySetId << (uint8_t)(invalidate ? 1 : 0);
	cacheEntityId_ = entityId;
}


/**
 * Finish sending a cache stamp update to the BaseApp.
 */
void BaseAppClient::endCacheStamp()
{
	SGW_ASSERT(cacheWriter_);
	cacheWriter_->endMessage();
	delete cacheWriter_;
	cacheWriter_ = nullptr;
}


/**
 * Called when a message was received from the peer
 *
 * @param msg Message stream reader
 */
void BaseAppClient::onMessageReceived(Reader & msg)
{
	switch (msg.messageId())
	{
		case Mercury::BASE_CELL_AUTHENTICATE_ACK:
			onRegistrationAck(msg);
			break;

		case Mercury::BASE_CELL_CREATE_ENTITY:
			onCreateEntityRequest(msg);
			break;

		case Mercury::BASE_CELL_DESTROY_ENTITY:
			onDestroyEntityRequest(msg);
			break;

		case Mercury::BASE_CELL_CONNECT_ENTITY:
			onConnectEntityRequest(msg);
			break;

		case Mercury::BASE_CELL_DISCONNECT_ENTITY:
			onDisconnectEntityRequest(msg);
			break;

		case Mercury::BASE_CELL_MOVE:
			onEntityMove(msg);
			break;

		case Mercury::BASE_CELL_RESTORE_ENTITY:
			onRestoreEntityRequest(msg);
			break;

		case Mercury::BASE_CELL_BASEAPP_MESSAGE:
			onBaseAppMessage(msg);
			break;

		case Mercury::BASE_CELL_CLIENT_MESSAGE:
			onClientMessage(msg);
			break;

		case Mercury::BASE_CELL_REQUEST_ENTITY_UPDATE:
			onRequestEntityUpdate(msg);
			break;

		default:
			FAULT("Illegal message received from BaseApp: %02x", msg.messageId());
			close();
	}
}


/**
 * Callback function called when the connection request was completed.
 *
 * @param errcode Error code; connection was successful if code == errc::success
 */
void BaseAppClient::onConnected(const boost::system::error_code & errcode)
{
	// The connection was established successfully
	if (errcode == boost::system::errc::success)
	{
		TRACE("Sending authentication request to BaseApp");
		sendAuthenticationRequest();
	}
	else
	{
		FAULT("Connection to BaseApp failed: %s", errcode.message().c_str());
		reconnectIn(ConnectionRecoveryTimeout);
	}
}


/**
 * Callback function called when the connection to the server was closed.
 *
 * @param errcode Reason why the connection was lost
 */
void BaseAppClient::onDisconnected(const boost::system::error_code & /*errcode*/)
{
	FAULTC(CATEGORY_MERCURY, "Lost connection to BaseApp");
	FAULTC(CATEGORY_MERCURY, "Disconnected; removing all player entities");
	SpaceManager::instance().removePlayers();
	reconnectIn(ConnectionRecoveryTimeout);
	// FIXME: Handle disconnection while sending a stamp
	SGW_ASSERT(!cacheWriter_);
}

void BaseAppClient::unregisterConnection()
{
}


CellEntity::Ptr BaseAppClient::createEntity(uint32_t entityId, uint32_t dbid, uint32_t spaceId, 
											std::string const & className, std::string const & worldName,
											Vec3 const & position, Vec3 const & rotation)
{
	Space * space = findOrCreateSpace(spaceId, worldName, entityId);
	if (!space)
	{
		WARNC(CATEGORY_ENTITY, "Failed to create entity (%d) on nonexistent space (%d, world %s)",
			entityId, spaceId, worldName.c_str());
		return CellEntity::Ptr();
	}
	
	CellEntity::Ptr entity = CellEntityManager<CellEntity>::instance().create(entityId, className, dbid);
	if (!entity)
	{
		WARNC(CATEGORY_ENTITY, "Failed to create entity (entityId=%d, spaceId=%d, dbid=%d)",
			entityId, spaceId, dbid);
		SpaceManager::instance().destroy(space->spaceId());
		return CellEntity::Ptr();
	}

	entity->position() = position;
	entity->rotation() = rotation;
	entity->enterSpace(space);
	return entity;
}

Space * BaseAppClient::findOrCreateSpace(uint32_t spaceId, std::string const & worldName, uint32_t creatorId)
{
	Space * space = nullptr;
	// If an exact spaceId was specified, try to find the space
	if (spaceId != 0xffffffff)
	{
		space = SpaceManager::instance().get(spaceId);
		if (!space)
		{
			WARNC(CATEGORY_ENTITY, "Failed to find nonexistent space (%d, world %s)",
				spaceId, worldName.c_str());
		}
	}
	else
	{
		// No spaceId specified
		// The space is either a non-instanced existing space on this cell
		// or a new instanced space that we should create
		space = SpaceManager::instance().get(worldName);
		if (!space)
		{
			WorldData const * world = SpaceManager::instance().getWorldInfo(worldName);
			if (world)
			{
				if (world->instanced)
				{
					// No such space, create an instance
					space = SpaceManager::instance().create(worldName);
					space->setCreatorId(creatorId);
				}
				else
				{
					WARN("Tried to create space for world '%s' that is not instanced!", worldName.c_str());
				}
			}
			else
			{
				WARN("Tried to create space for invalid world '%s'!", worldName.c_str());
			}
		}
	}

	return space;
}
