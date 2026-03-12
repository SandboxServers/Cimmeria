#include <stdafx.hpp>
#include <baseapp/mercury/cell_handler.hpp>
#include <mercury/base_cell_messages.hpp>
#include <baseapp/cell_manager.hpp>
#include <baseapp/entity/cached_entity.hpp>

#if defined(_DEBUG)
#define CELL_DEBUG_MESSAGES
// #define CELL_DEBUG_DISTRIBUTION
// #define CELL_DEBUG_ENTITIES
#endif

namespace Mercury
{
	
CellAppConnection * CellAppConnection::Create(Mercury::TcpServer<CellAppConnection> & /*server*/, uint32_t /*connectionId*/)
{
	return new CellAppConnection();
}

CellAppConnection::CellAppConnection()
	: Mercury::UnifiedConnection(0),
	  registered_(false)
{
}


CellAppConnection::~CellAppConnection()
{
	TRACE("");
}


uint32_t CellAppConnection::cellId() const
{
	return cellId_;
}


/**
 * Called when the BaseApp requested the cell to create a cell part for a base entity
 *
 * @param entity   Entity to create
 * @param spaceId   Space ID to restore to
 * @param worldName World name to restore to (if space ID is not specified)
 * @param position  Position of entity on space
 * @param rotation  Rotation of entity on space
 */
void CellAppConnection::sendCreateEntity(BaseEntity * entity, uint32_t spaceId, std::string const & worldName, Vec3 & position, Vec3 & rotation)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_CREATE_ENTITY);
	writer << spaceId << entity->entityId() << entity->databaseId();
	writer << position.x << position.y << position.z;
	writer << rotation.x << rotation.y << rotation.z;
	writer.writeString(entity->classDef()->name());
	writer.writeString(worldName);
	writer << (uint32_t)0;
	writer.endMessage();

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell(%d): CREATE_ENTITY: entityId=%d, dbid=%d, spaceId=%d", 
		cellId_, entity->entityId(), entity->databaseId(), spaceId);
#endif
}


/**
 * Requests the CellApp to destroy the cell part of an entity
 *
 * @param entityId Entity ID to destroy
 */
void CellAppConnection::sendDestroyEntity(uint32_t entityId)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_DESTROY_ENTITY);
	writer << entityId;
	writer.endMessage();

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell(%d): DESTROY_ENTITY: entityId=%d", cellId_, entityId);
#endif
}


/**
 * Requests the CellApp to attach a client controller to an entity
 *
 * @param entityId Entity ID to attach
 */
void CellAppConnection::sendConnectEntity(uint32_t entityId)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_CONNECT_ENTITY);
	writer << entityId;
	writer.endMessage();

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell(%d): CONNECT_ENTITY: entityId=%d", cellId_, entityId);
#endif
}
		

/**
 * Requests the CellApp to detach a client controller from an entity
 *
 * @param entityId Entity ID to detach
 */
void CellAppConnection::sendDisconnectEntity(uint32_t entityId)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_DISCONNECT_ENTITY);
	writer << entityId;
	writer.endMessage();

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell(%d): DISCONNECT_ENTITY: entityId=%d", cellId_, entityId);
#endif
}


/**
 * Notifies the CellApp that the client has moved its controlled entity
 *
 * @param entityId Controlled entity ID
 * @param position Current position of entity
 * @param rotation Current rotation of entity
 * @param velocity Current velocity of entity
 * @param flags    Movement modifier flags (is jumping, running, etc.)
 */
void CellAppConnection::sendEntityMove(uint32_t entityId, Vec3 & position, Vec3 & rotation, Vec3 & velocity, uint8_t flags)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_MOVE);
	writer << entityId <<
		position.x << position.y << position.z <<
		rotation.x << rotation.y << rotation.z <<
		velocity.x << velocity.y << velocity.z <<
		flags;
	writer.endMessage();
}


/**
 * Restores an entity from its BaseApp backup
 *
 * @param entity   Entity to restore
 */
void CellAppConnection::sendRestoreEntity(BaseEntity * entity, uint32_t spaceId, bool recovery,
	std::string const & worldName, Vec3 & position, Vec3 & rotation)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_RESTORE_ENTITY);
	writer << spaceId << entity->entityId() << (int8_t)entity->classDef()->index() << (uint8_t)recovery;
	writer.writeString(worldName);
	writer << position.x << position.y << position.z;
	writer << rotation.x << rotation.y << rotation.z;
	entity->packBackup(writer);
	writer.endMessage();

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell(%d): RESTORE_ENTITY: entityId=%d", cellId_, entity->entityId());
#endif
}


/**
 * Calls an internal RPC method on a cell entity.
 *
 * @param entityId  Entity to call this RPC method on
 * @param messageId Message type to send
 * @param method    RPC arguments serializer
 * @param args      Arguments to send
 */
void CellAppConnection::sendInternalMessage(uint32_t entityId, uint8_t messageId, PyMethod * method, bp::object args)
{
	SGW_ASSERT(method != nullptr);

	Writer writer(*this);
	writer.beginMessage(BASE_CELL_BASEAPP_MESSAGE);
	writer << entityId << messageId;
	PyGilGuard guard;
	method->serializeArgs(writer, args);
	writer.endMessage();
}



/**
 * Calls an internal RPC method on a cell entity.
 *
 * @param entityId  Entity to call this RPC method on
 * @param messageId Message type to send
 * @param args      Serialized arguments to send
 * @param argsSize  Size of arguments in bytes
 */
void CellAppConnection::sendInternalMessage(uint32_t entityId, uint8_t messageId, void * args, std::size_t argsSize)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_BASEAPP_MESSAGE);
	writer << entityId << messageId;
	writer.write(args, argsSize);
	writer.endMessage();
}


/**
 * Calls an exposed RPC method on a cell entity.
 *
 * @param entityId  Entity to call this RPC method on
 * @param messageId Message type to send
 * @param method    RPC arguments serializer
 * @param args      Arguments to send
 */
void CellAppConnection::sendExposedMessage(uint32_t entityId, uint8_t messageId, PyMethod * method, bp::object args)
{
	SGW_ASSERT(method != nullptr);

	Writer writer(*this);
	writer.beginMessage(BASE_CELL_CLIENT_MESSAGE);
	writer << entityId << messageId;
	PyGilGuard guard;
	method->serializeArgs(writer, args);
	writer.endMessage();
}

/**
 * Calls an exposed RPC method on a cell entity.
 *
 * @param entityId  Entity to call this RPC method on
 * @param messageId Message type to send
 * @param args      Serialized arguments to send
 * @param argsSize  Size of arguments in bytes
 */
void CellAppConnection::sendExposedMessage(uint32_t entityId, uint8_t messageId, void * args, std::size_t argsSize)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_CLIENT_MESSAGE);
	writer << entityId << messageId;
	writer.write(args, argsSize);
	writer.endMessage();
}

/**
 * Requests the CellApp to update the status of an entity on the client.
 *
 * @param witnessId Client requesting the update
 * @param entityId  Entity to update
 * @param flags     Requested update types
 */
void CellAppConnection::sendRequestEntityUpdate(uint32_t witnessId, uint32_t entityId, uint8_t flags)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_REQUEST_ENTITY_UPDATE);
	writer << witnessId << entityId << flags;
	writer.endMessage();

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Base -> Cell(%d): REQ_ENTITY_UPDATE: witnessId=%d, entityId=%d, flags=%d", 
		cellId_, witnessId, entityId, (uint32_t)flags);
#endif
}


/**
 * Called when a message was received from the peer
 *
 * @param msg Message stream reader
 */
void CellAppConnection::onMessageReceived(Reader & msg)
{
	if (!registered_)
	{
		switch (msg.messageId())
		{
		case Mercury::CELL_BASE_AUTHENTICATE:
			onAuthenticationRequest(msg);
			break;

		default:
			FAULT("Illegal message %02x received from CellApp while authenticating", msg.messageId());
			close();
		}
	}
	else
	{
		switch (msg.messageId())
		{
			case Mercury::CELL_BASE_ENTITY_CREATE_ACK:
				onEntityCreateAck(msg);
				break;

			case Mercury::CELL_BASE_ENTITY_DELETE_ACK:
				onEntityDeleteAck(msg);
				break;

			case Mercury::CELL_BASE_SPACE_DATA:
				onSpaceData(msg);
				break;

			case Mercury::CELL_BASE_CREATE_CELL_PLAYER:
				onCellPlayerCreateAck(msg);
				break;

			case Mercury::CELL_BASE_DISCONNECT_ENTITY_ACK:
				onEntityDisconnectAck(msg);
				break;

			case Mercury::CELL_BASE_TICK:
				onGameTick(msg);
				break;

			case Mercury::CELL_BASE_ENTER_AOI:
				onEnteredAoI(msg);
				break;

			case Mercury::CELL_BASE_LEAVE_AOI:
				onLeftAoI(msg);
				break;

			case Mercury::CELL_BASE_MOVE:
				onEntityMove(msg);
				break;

			case Mercury::CELL_BASE_BACKUP_ENTITY:
				onEntityBackup(msg);
				break;

			case Mercury::CELL_BASE_RESTORE_ENTITY_ACK:
				onEntityRestoreAck(msg);
				break;

			case Mercury::CELL_BASE_SWITCH_SPACE:
				onSwitchSpace(msg);
				break;

			case Mercury::CELL_BASE_BASEAPP_MESSAGE:
				onBaseAppMessage(msg);
				break;

			case Mercury::CELL_BASE_CLIENT_MESSAGE:
				onClientMessage(msg);
				break;

			case Mercury::CELL_BASE_NOTIFY_CREATE_CELL_ENTITY:
				onCellEntityCreated(msg);
				break;

			case Mercury::CELL_BASE_NOTIFY_DELETE_CELL_ENTITY:
				onCellEntityDeleted(msg);
				break;

			case Mercury::CELL_BASE_UPDATE_CACHE_STAMP:
				onUpdateCacheStamp(msg);
				break;

			default:
				FAULT("Illegal message 0x%02x received from CellApp", msg.messageId());
				close();
		}
	}
}


/**
 * Callback function called when the connection request was completed.
 *
 * @param errcode Error code; connection was successful if code == errc::success
 */
void CellAppConnection::onConnected(const boost::system::error_code & /*errcode*/)
{
}


/**
 * Callback function called when the connection to the server was closed.
 *
 * @param errcode Reason why the connection was lost
 */
void CellAppConnection::onDisconnected(const boost::system::error_code & /*errcode*/)
{
	if (registered_)
	{
		FAULTC(CATEGORY_MERCURY, "Lost connection to CellApp %d", cellId_);
		CachedEntityManager::instance().deleteCellEntities(cellId_);
		CellManager::instance().unregisterCell(cellId_);
	}
}


void CellAppConnection::unregisterConnection()
{
}


/**
 * Notifies the BaseApp that the cell part of an entity was created.
 */
void CellAppConnection::onEntityCreateAck(Reader & msg)
{
	uint32_t entityId, spaceId;
	msg >> entityId >> spaceId;

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: ENTITY_CREATE_ACK: entityId=%d, spaceId=%d", entityId, spaceId);
#endif

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "Tried to create cell for nonexistent entity %d", entityId);
		return;
	}

	if (spaceId != 0xffffffff)
		entity->cellEntityCreated(spaceId);
	else
		entity->cellEntityCreateFailed();
}


/**
 * Notifies the BaseApp that the cell part of an entity was destroyed.
 */
void CellAppConnection::onEntityDeleteAck(Reader & msg)
{
	// Entity deletion acknowledgement from cell
	// (cell part of the entity was deleted)
	uint32_t entityId;
	msg >> entityId;

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: ENTITY_DELETE_ACK: entityId=%d", entityId);
#endif

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (entity)
		entity->cellEntityDestroyed();
	else
	{
		// This is not an error condition as the base part may be already deleted
		// when the delete ACK message is received from the cell
		// (e.g. when the client channel is disconnected before logoff)
		TRACEC(CATEGORY_MESSAGES, "No matching base for deleted cell entity %d", entityId);
	}
}


/**
 * Updates space availability information about a space on the BaseApp.
 */
void CellAppConnection::onSpaceData(Reader & msg)
{
	uint32_t spaceId, cellId;
	std::string worldName;
	msg >> spaceId >> cellId;
	msg.readString(worldName);

#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: SPACE_DATA: spaceId=%d, cellId=%d, worldName=%s", 
		spaceId, cellId, worldName.c_str());
#endif
	
	if (cellId == 0xffffffff)
	{
	#if defined(CELL_DEBUG_MESSAGES)
		TRACEC(CATEGORY_MESSAGES, "Forgetting space %d", spaceId);
	#endif
		CellManager::instance().unregisterSpace(spaceId);
	}
	else
	{
		if (cellId != cellId_)
		{
			WARNC(CATEGORY_MESSAGES, "Attempted to add space %d to cell %d (local cell is %d)",
				spaceId, cellId, cellId_);
			return;
		}
		
#if defined(CELL_DEBUG_MESSAGES)	
		TRACEC(CATEGORY_MESSAGES, "Adding space %d to cell %d", spaceId, cellId_);
#endif
		CellManager::instance().registerSpace(this, spaceId, worldName);
	}
}


/**
 * Notifies the BaseApp that the cell part of a player was created.
 */
void CellAppConnection::onCellPlayerCreateAck(Reader & msg)
{
	uint32_t entityId, spaceId;
	float x, y, z, yaw, pitch, roll;
	msg >> entityId >> spaceId >>
		x >> y >> z >>
		yaw >> pitch >> roll;
		
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: CREATE_CELL_PLAYER: entityId=%d, spaceId=%d", entityId, spaceId);
#endif

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARN("Tried to connect cell for nonexistent entity %d", entityId);
		return;
	}
	
	entity->createCellPlayer(spaceId, x, y, z, yaw, roll, pitch);
}


/**
 * Notifies the BaseApp that the cell part of a client controlled entity was disconnected.
 */
void CellAppConnection::onEntityDisconnectAck(Reader & msg)
{
	uint32_t entityId;
	msg >> entityId;
		
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: DISCONNECT_ENTITY_ACK: entityId=%d", entityId);
#endif

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARN("Tried to disconnect cell for nonexistent entity %d", entityId);
		return;
	}

	entity->cellEntityDisconnected();
}


/**
 * Called when the Cell updated its game timer.
 */
void CellAppConnection::onGameTick(Reader & msg)
{
	uint64_t time;
	msg >> time;
		
#if defined(CELL_DEBUG_MESSAGES)
	// TRACEC(CATEGORY_MESSAGES, "Cell -> Base: TICK: time=%d", (uint32_t)time);
#endif

	CellManager::instance().tickCell(cellId(), time);
}


/**
 * Notifies a base entity that another entity entered its AoI (area of interest).
 * TODO: DEPRECATE
 */
void CellAppConnection::onEnteredAoI(Reader & msg)
{
	uint32_t witnessId, spaceId, entityId;
	uint8_t classId;
	Vec3 pos, rot;
	msg >> witnessId >> spaceId >> entityId >> classId;
	msg >> pos.x >> pos.y >> pos.z;
	msg >> rot.x >> rot.y >> rot.z;
		
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: ENTER_AOI: witnessId=%d, entityId=%d", witnessId, entityId);
#endif

	if (witnessId != 0xffffffff)
	{
		BaseEntity::Ptr witness = BaseEntityManager<BaseEntity>::instance().get(witnessId);
		if (witness)
		{
			if (witness->controller())
				witness->controller()->createEntity(entityId, classId, pos, rot);
		}
		else
			WARNC(CATEGORY_MESSAGES, "Unknown witness id: %d", witnessId);
	}
	else
	{
		std::vector<BaseEntity::Ptr> entities;
		BaseEntityManager<BaseEntity>::instance().getEntities(spaceId, entities);
		for (auto it = entities.begin(); it != entities.end(); ++it)
		{
			if ((*it)->entityId() != entityId && (*it)->controller())
				(*it)->controller()->createEntity(entityId, classId, pos, rot);
		}
	}
}


/**
 * Notifies a base entity that another entity left its AoI (area of interest).
 * TODO: DEPRECATE
 */
void CellAppConnection::onLeftAoI(Reader & msg)
{
	uint8_t deleteEntity;
	uint32_t spaceId, entityId;
	msg >> spaceId >> entityId >> deleteEntity;
		
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: LEAVE_AOI: entityId=%d, delete=%d", entityId, deleteEntity);
#endif

	std::vector<BaseEntity::Ptr> entities;
	BaseEntityManager<BaseEntity>::instance().getEntities(spaceId, entities);
	for (auto it = entities.begin(); it != entities.end(); ++it)
	{
		if ((*it)->controller())
			(*it)->controller()->leaveAoI(entityId, deleteEntity == 1);
	}
}


/**
 * Notifies clients that a Cell entity has moved / is moving.
 */
void CellAppConnection::onEntityMove(Reader & msg)
{
	CachedEntity::MovementUpdate update;
	uint32_t spaceId, entityId;
	msg >> spaceId >> entityId;
	msg >> update.position.x >> update.position.y >> update.position.z;
	msg >> update.rotation.x >> update.rotation.y >> update.rotation.z;
	msg >> update.velocity.x >> update.velocity.y >> update.velocity.z;
	msg >> update.flags >> update.serverFlags;
		
#if defined(CELL_DEBUG_MESSAGES)
	// TRACEC(CATEGORY_MESSAGES, "Cell -> Base: MOVE: entityId=%d, pos=(%f, %f, %f)", entityId, pos.x, pos.y, pos.z);
#endif

	// Look up the cached entity for this update
	// This is needed to get a witness list
	CachedEntity::Ptr cached = CachedEntityManager::instance().getEntity(entityId);
	if (!cached)
	{
		WARN("Tried to add move for unknown entity %d", entityId);
		return;
	}

	cached->addMove(update);
}


/**
 * Creates a backup of the cell state of the entity and sends it to the BaseApp.
 */
void CellAppConnection::onEntityBackup(Reader & msg)
{
	uint32_t spaceId, entityId;
	msg >> spaceId >> entityId;
		
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: BACKUP_ENTITY: entityId=%d, spaceId=%d", entityId, spaceId);
#endif
	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "Cannot backup entity %d without a base part", entityId);
		return;
	}

	entity->unpackBackup(msg);
}


/**
 * Notifies the BaseApp that the restore of a cell entity was successful.
 */
void CellAppConnection::onEntityRestoreAck(Reader & msg)
{
	uint32_t entityId;
	msg >> entityId;
		
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: RESTORE_ENTITY_ACK: entityId=%d", entityId);
#endif
}


/**
 * Requests the BaseApp to start moving an entity to a different space.
 */
void CellAppConnection::onSwitchSpace(Reader & msg)
{
	uint32_t spaceId, entityId;
	msg >> entityId >> spaceId;
	std::string worldName;
	Vec3 position, rotation;
	msg.readString(worldName);
	msg >> position.x >> position.y >> position.z;
	msg >> rotation.x >> rotation.y >> rotation.z;
		
#if defined(CELL_DEBUG_MESSAGES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: SWITCH_SPACE: entityId=%d", entityId);
#endif

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "Cannot move entity %d without a base part", entityId);
		return;
	}

	if (!entity->hasBackup())
	{
		WARNC(CATEGORY_MESSAGES, "Cannot move entity %d because it isn't backed up!", entityId);
		return;
	}

	entity->destroyCellPart(false);
	sendRestoreEntity(entity.get(), spaceId, false, worldName, position, rotation);
}


/**
 * Calls an RPC method on a base entity.
 */
void CellAppConnection::onBaseAppMessage(Reader & msg)
{
	uint32_t entityId;
	uint8_t messageId;
	msg >> entityId >> messageId;

#if defined(CELL_DEBUG_DISTRIBUTION)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: BASE_RPC: messageId=%02x, entityId=%d", messageId, entityId);
#endif

	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "No such entity: %d", entityId);
		return;
	}

	PyMethod * method = entity->classDef()->findMethod(Service::BaseMailbox, messageId);
	if (!method)
	{
		WARNC(CATEGORY_MESSAGES, "No internal base method for entity '%s', methodId %d",
			entity->classDef()->name().c_str(), messageId);
		return;
	}

	PyGilGuard guard;
	auto args = method->unpackArgs(msg);
	entity->rpc(method->name().c_str(), args);
}


/**
 * Calls an RPC method on a client entity.
 */
void CellAppConnection::onClientMessage(Reader & msg)
{
	uint32_t entityId;
	uint8_t messageId;
	MessageWriter::DistributionPolicy policy;
	msg >> entityId >> messageId >> policy.flags;

	if (policy.flags & Mercury::DISTRIBUTE_RECIPIENT)
	{
		msg >> policy.recipientId;
	}

	if (policy.flags & (Mercury::DISTRIBUTE_LOD | Mercury::DISTRIBUTE_SPACE))
	{
		msg >> policy.spaceId;
	}

#if defined(CELL_DEBUG_DISTRIBUTION)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: DISTRIBUTE: flags=%02x, recipient=%d, messageId=%02x", 
		policy.flags, policy.recipientId, messageId);
#endif

	MessageWriter * writer = Service::instance().messageWriter(Service::ClientMailbox, entityId);
	std::size_t size = msg.remaining();
	writer->write(Service::ClientMailbox, policy, entityId, messageId, msg.readContiguous(size).data(), size);
}


/**
 * Called when a cell-only entity was created on the CellApp
 */
void CellAppConnection::onCellEntityCreated(Reader & msg)
{
	uint32_t entityId, spaceId;
	uint8_t classId, flags;
	Vec3 position;
	msg >> entityId >> spaceId >> classId >> flags;
	msg >> position.x >> position.y >> position.z;
		
#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: CELL_ENTITY_CREATE: entityId=%d, spaceId=%d, pos=(%f, %f, %f)", 
		entityId, spaceId, position.x, position.y, position.z);
#endif

	CachedEntity::Ptr entity = CachedEntityManager::instance().addEntity(entityId, classId, flags);
	if (entity)
	{
		entity->enterSpace(spaceId, position);
	}
}

		
/**
 * Called when a cell-only entity was deleted on the CellApp
 */
void CellAppConnection::onCellEntityDeleted(Reader & msg)
{
	uint32_t entityId;
	msg >> entityId;
		
#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: CELL_ENTITY_DELETE: entityId=%d)", entityId);
#endif

	CachedEntityManager::instance().deleteEntity(entityId);
}


/**
 * Called when the cell has updated the cached messages of an entity
 */
void CellAppConnection::onUpdateCacheStamp(Reader & msg)
{
	uint32_t entityId, propertySetId;
	uint8_t invalidate;
	msg >> entityId >> propertySetId >> invalidate;
		
#if defined(CELL_DEBUG_ENTITIES)
	TRACEC(CATEGORY_MESSAGES, "Cell -> Base: UPDATE_CACHE_STAMP: entityId=%d, propSetId=%d, invalidate=%s", 
		entityId, propertySetId, invalidate ? "true" : "false");
#endif

	if (propertySetId >= Mercury::MaxPropertySets)
	{
		FAULTC(CATEGORY_MESSAGES, "Cache stamp received for illegal propset %d", propertySetId);
		return;
	}

	CachedEntity::Ptr entity = CachedEntityManager::instance().getEntity(entityId);
	if (!entity)
	{
		WARNC(CATEGORY_MESSAGES, "Cache stamp received for unknown entity %d", entityId);
		return;
	}

	if (invalidate)
	{
		entity->invalidateCacheStamps(propertySetId);
	}

	if (msg.remaining() == 0)
	{
		WARN("Received empty cache update for entity %d, propset %d", entityId, propertySetId);
		return;
	}

	entity->beginCacheStamp(propertySetId);
	while (msg.remaining())
	{
		uint8_t messageId, flags;
		uint16_t length;
		msg >> messageId >> flags >> length;
		
#if defined(CELL_DEBUG_ENTITIES)
		TRACEC(CATEGORY_MESSAGES, "Cell -> Base: CACHE_MESSAGE: messageId=%d, flags=%d, length=%d", 
			messageId, flags, length);
#endif

		entity->addCacheMessage(propertySetId, messageId, flags, msg.readContiguous(length).data(), length);
	}
	entity->endCacheStamp();
}


/**
 * Called when an authentication/registration request was received from the CellApp
 */
void CellAppConnection::onAuthenticationRequest(Reader & msg)
{
	uint32_t protocolHash, protocolVersion;
	msg >> protocolHash >> protocolVersion;

	if (protocolHash != Mercury::CellAppHandshakeHash)
	{
		FAULTC(CATEGORY_MERCURY, "CellApp communication error: invalid protocol hash: %08x", protocolHash);
		sendRegistrationAck(Mercury::REGISTRATION_HASH_MISMATCH);
		close();
		return;
	}

	if (protocolVersion != Mercury::BaseCellProtocolVersion)
	{
		FAULTC(CATEGORY_MESSAGES, "CellApp communication error: protocol version mismatch");
		FAULTC(CATEGORY_MESSAGES, "BaseApp version: %d, CellApp version: %d", Mercury::BaseCellProtocolVersion, protocolVersion);
		sendRegistrationAck(Mercury::REGISTRATION_PROTOCOL_MISMATCH);
		close();
		return;
	}
	
	// Only read additional fields after the protocol check was done
	// as these can differ between versions
	msg >> cellId_;
	DEBUG2C(CATEGORY_MERCURY, "Connect request from CellApp ID %08x", cellId_);
	
	if (CellManager::instance().registerCell(this))
	{
		sendRegistrationAck(Mercury::REGISTRATION_OK);
		registered_ = true;
		INFOC(CATEGORY_MERCURY, "Connected to CellApp %d at (%s)", cellId_, socket().remote_endpoint().address().to_string().c_str());
	}
	else
	{
		sendRegistrationAck(Mercury::REGISTRATION_FAILED);
		close();
		FAULTC(CATEGORY_MESSAGES, "CellApp (%d) failed to register", cellId_);
	}
}


/**
 * Sends an acknowledgement to the CellApps authentication request
 */
void CellAppConnection::sendRegistrationAck(uint32_t resultCode)
{
	Writer writer(*this);
	writer.beginMessage(BASE_CELL_AUTHENTICATE_ACK);
	writer << Mercury::BaseAppHandshakeHash << Mercury::BaseCellProtocolVersion;
	writer << (uint32_t)resultCode;
	if (resultCode == 0)
	{
		writer << CellManager::instance().ticks() <<
			CellManager::instance().tickRate();
	}
	else
	{
		writer << (uint32_t)0 << (uint32_t)0;
	}
	writer.endMessage();
}

}