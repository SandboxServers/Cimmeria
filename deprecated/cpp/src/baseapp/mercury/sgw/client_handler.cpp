#include <stdafx.hpp>
#include <baseapp/mercury/sgw/client_handler.hpp>
#include <baseapp/mercury/sgw/messages.hpp>
#include <baseapp/entity/base_entity.hpp>
#include <entity/pyutil.hpp>
#include <baseapp/base_service.hpp>
#include <baseapp/cell_manager.hpp>
#include <boost/python/object.hpp>
#include <boost/python/str.hpp>
#include <boost/python/long.hpp>

#if defined(_DEBUG)
#define CLIENT_DEBUG
#endif

namespace Mercury
{
namespace SGW
{

bool ClientHandler::unreliableTickSync_ = false;
bool ClientHandler::unreliableMovement_ = true;

void ClientHandler::initialize()
{
	// FIXME: Make the configuration vars case insensitive
	std::string const & unrSync = BaseService::base_instance().getConfigParameter("unreliable_tick_sync");
	unreliableTickSync_ = (unrSync == "true");

	std::string const & unrMovement = BaseService::base_instance().getConfigParameter("unreliable_movement_update");
	unreliableMovement_ = (unrMovement == "true");
}

ClientHandler::ClientHandler(uint32_t accountId, std::string const & accountName, uint32_t accessLevel)
	: proxyDataId_(1), entitySystemEnabled_(false), connectionSetup_(false), accountId_(accountId),
	accountName_(accountName), accessLevel_(accessLevel)
{
}

ClientHandler::~ClientHandler()
{
}

void ClientHandler::onConnected()
{
	uint8_t updateFreq = (1000 / CellManager::instance().tickRate());
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: UPDATE_FREQUENCY_NOTIFICATION: freq=%d", updateFreq);
#endif
	Bundle & bundle = *channel_->bundle();
	bundle.beginMessage(BASEMSG_UPDATE_FREQUENCY_NOTIFICATION);
	bundle << updateFreq;
	bundle.endMessage();

	uint32_t ticks = CellManager::instance().ticks();
	bundle.beginMessage(BASEMSG_TICK_SYNC);
	bundle << (uint32_t)ticks;
	bundle << (uint32_t)CellManager::instance().tickRate();
	bundle.endMessage();

	bundle.beginMessage(BASEMSG_SET_GAME_TIME, Bundle::FLUSH);
	bundle << (uint32_t)ticks;
	bundle.endMessage();
	// We need to flush here, otherwise the client will crash when processing CREATE_BASE_ENTITY
	channel_->flushBundle(false);
	
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "ClientHandler connected");
#endif
	try
	{
		PyGilGuard guard;
		entity_ = BaseEntityManager<BaseEntity>::instance().create("Account", accountId_);
		if (entity_)
		{
			SGW_ASSERT(entity_->classDef()->index() != -1);
			bp::setattr(entity_->object(), "accountId", bp::long_(accountId_));
			bp::setattr(entity_->object(), "accountName", bp::str(accountName_));
			bp::setattr(entity_->object(), "accessLevel", bp::long_(accessLevel_));
			entity_->setupClient(channel_);
		}
		else
		{
			WARNC(CATEGORY_MESSAGES, "Failed to create controller entity; disconnecting client");
			/*
			 * Notify that client that it's disconnected and close local channel.
			 * The logoff message forces an unclean shutdown on the client side, so 
			 * all received packets should be ignored on the server side as well.
			 */
			disconnectEntity(true);
		}
	}
	catch (bp::error_already_set &)
	{
		WARNC(CATEGORY_MESSAGES, "A Python exception has occurred:");
		PyUtil_ShowErr();
	}

	connectionSetup_ = true;
}

void ClientHandler::onDisconnected()
{
	if (entity_)
		disconnectEntity(false);
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "ClientHandler disconnected");
#endif
	connectionSetup_ = false;
}

void ClientHandler::tick(uint64_t time)
{
	// Check if there are resources that must be sent when there is free space in the send buffer
	// and re-queue them when some space is freed up
	while (!queuedResources_.empty() && this->canSendResource())
	{
		ResourceId & id = queuedResources_.back();
#if defined(CLIENT_DEBUG)
		// TRACEC(CATEGORY_MESSAGES, "Re-queued resource (%d, %d)", id.categoryId, id.elementId);
#endif
		sendResource(id.categoryId, id.elementId);
		queuedResources_.pop_back();
	}
}

void ClientHandler::onBaseMessage(Message::Ptr msg)
{
	if (msg->messageId() == CLIENTMSG_AUTHENTICATE)
	{
		// TRACE("Ignored msg CLIENTMSG_AUTHENTICATE");
	}
	else if (msg->messageId() == CLIENTMSG_AVATAR_UPDATE_EXPLICIT)
	{
		uint32_t spaceId, vehicleId;
		Vec3 position, direction, velocity;
		uint8_t yaw, pitch, roll;
		uint8_t flags;
		uint8_t xCell, yCell, zCell, updateId;
		*msg >> spaceId >> vehicleId >>
			position.x >> position.y >> position.z >>
			velocity.x >> velocity.y >> velocity.z >>
			yaw >> pitch >> roll >>
			flags >> xCell >> yCell >> zCell >> updateId;
		direction.x = pitch * 0.024543693f;
		direction.y = yaw * 0.024543693f;
		direction.z = roll * 0.024543693f;
#if defined(CLIENT_DEBUG)
		// TRACE("Client -> Base: AVATAR_UPDATE_EXPLICIT: spaceId=%d, pos=(%f, %f, %f), flags=%02x",
		// 	spaceId, position.x, position.y, position.z, flags);
#endif
		
		if (!entitySystemEnabled_)
		{
			WARNC(CATEGORY_MESSAGES, "Called while the entity system is not initialized!");
			return;
		}
	
		if (!entity_)
		{
			WARNC(CATEGORY_MESSAGES, "Called with no entity connected!");
			return;
		}

		if (entity_->spaceId() == 0xffffffff)
		{
			WARNC(CATEGORY_MESSAGES, "Base entity has no cell entity attached");
			return;
		}

		SpaceData * space = CellManager::instance().findSpace(entity_->spaceId());
		if (!space || !space->handler)
		{
			WARNC(CATEGORY_MESSAGES, "Space %d has no handler", entity_->spaceId());
			return;
		}

		space->handler->sendEntityMove(entity_->entityId(), position, direction, velocity, flags);
	}
	else if (msg->messageId() == CLIENTMSG_REQUEST_ENTITY_UPDATE)
	{
		uint32_t entityId;
		*msg >> entityId;
#if defined(CLIENT_DEBUG)
		TRACEC(CATEGORY_MESSAGES, "Client -> Base: REQUEST_ENTITY_UPDATE: entityId=%d", entityId);
#endif
	}
	else if (msg->messageId() == CLIENTMSG_ENABLE_ENTITIES)
	{
#if defined(CLIENT_DEBUG)
		TRACEC(CATEGORY_MESSAGES, "Client -> Base: ENABLE_ENTITIES");
#endif
		enableEntities();
	}
	else if (msg->messageId() == CLIENTMSG_DISCONNECT)
	{
		uint8_t reason;
		*msg >> reason;
#if defined(CLIENT_DEBUG)
		TRACEC(CATEGORY_MESSAGES, "Client -> Base: DISCONNECT: reason=%d", reason);
#endif
		disconnectEntity(true);
	}
	else
		WARNC(CATEGORY_MESSAGES, "Unknown client message 0x%02x!", msg->messageId());
}

void ClientHandler::onEntityMessage(uint8_t rpcMessageId, uint32_t entityId, Mercury::Message::Ptr msg)
{
	if (!entity_ || !entitySystemEnabled_)
	{
		WARNC(CATEGORY_MESSAGES, "Called while the entity system is not initialized!");
		return;
	}
	
	if (!entity_)
	{
		WARNC(CATEGORY_MESSAGES, "Called with no entity connected!");
		return;
	}

	if (entity_->spaceId() == 0xffffffff)
	{
		WARNC(CATEGORY_MESSAGES, "Base entity has no cell entity attached");
		return;
	}

	SpaceData * space = CellManager::instance().findSpace(entity_->spaceId());
	if (!space || !space->handler)
	{
		WARN("Space %d has no handler", entity_->spaceId());
		return;
	}

	if (entityId != 0)
	{
		WARN("Client tried to send cell RPC message %02x to entity %d", rpcMessageId, entityId);
		return;
	}

	MessageWriter * writer = Service::instance().messageWriter(Service::CellExposedMailbox, entity_->entityId());
	uint32_t length = msg->remaining();
	MessageWriter::DistributionPolicy policy;
	writer->write(Service::CellExposedMailbox, policy, entity_->entityId(), rpcMessageId, msg->fetchBuffer(length).buffer(), length);
}

void ClientHandler::onBaseEntityMessage(uint8_t rpcMessageId, Mercury::Message::Ptr msg)
{
	if (!entity_ || !entitySystemEnabled_)
	{
		WARNC(CATEGORY_MESSAGES, "Called while the entity system is not initialized!");
		return;
	}
	
	if (entity_)
	{
		PyMethod * method = entity_->classDef()->findMethod(Service::BaseExposedMailbox, rpcMessageId);
		if (method)
		{
			PyGilGuard guard;
			auto args = method->unpackArgs(*msg);
			entity_->rpc(method->name().c_str(), args);
		}
		else
			WARNC(CATEGORY_MESSAGES, "No base method exposed for entity '%s', methodId %d",
				entity_->classDef()->name().c_str(), rpcMessageId);
	}
	else
		WARNC(CATEGORY_MESSAGES, "Called with no entity connected!");
}

Message::Table const & ClientHandler::sendMessages()
{
	return ServerMessageTable;
}

Message::Table const & ClientHandler::receiveMessages()
{
	return ClientMessageTable;
}

/*
 * Sends a custom resource to the client. These mostly contain static definitions 
 * (like item defs, dialogues, static texts, ...).
 * 
 * SGW resource format:
 *    uint8   messageType - only one value is allowed, 0 (MESSAGE_CacheData)
 *    uint32  categoryId  - type of resource, see ResourceManager::categoryNames_
 *    uint32  elementId   - identifier of resource (e.g. item def id)
 *    uint16  proxyDataId - ID of data set (unique for each message)
 *    uint8[] body        - XML document body
 */
void ClientHandler::sendResource(uint32_t categoryId, uint32_t elementId)
{
	// Queue the resource when there is no free space in the tx buffer
	if (!this->canSendResource())
	{
#if defined(CLIENT_DEBUG)
		//TRACEC(CATEGORY_MESSAGES, "Queued resource message for category %d, element %d", categoryId, elementId);
#endif
		ResourceId id;
		id.categoryId = categoryId;
		id.elementId = elementId;
		queuedResources_.push_back(id);
		return;
	}

	std::string const * resource = BaseService::base_instance().resources().get(categoryId, elementId);
	if (!resource)
	{
		if (entity_)
			entity_->cookedDataError(categoryId, elementId);
		// WARNC(CATEGORY_MESSAGES, "Can't send nonexistent resource to client: categoryId=%d, elementId=%d", 
		// 	categoryId, elementId);
		return;
	}

	const unsigned int FragmentSize = 1000;
	unsigned int position = 0;
	uint16_t dataId = proxyDataId_++;
	uint8_t chunkId = 0;

	/*
	 * All fragments have the BASE_FLAG flag set (its meaning is unknown).
	 * The first fragment always has the INITIAL_FRAGMENT flag to indicate that
	 * this is a new resource (as data ID-s can be reused).
	 * The last fragment has a FINAL_FRAGMENT flag.
	 */
	Bundle & bundle = *channel_->bundle();
	while (position < resource->length())
	{
#if defined(CLIENT_DEBUG)
		// TRACEC(CATEGORY_MESSAGES, "Base -> Client: RESOURCE_FRAGMENT: dataId=%d, chunkId=%d", dataId, chunkId);
#endif
		bundle.beginMessage(BASEMSG_RESOURCE_FRAGMENT);
		bundle << dataId << (uint8_t)(chunkId++);
		
		unsigned int msgLen;
		if ((resource->length() - position) > FragmentSize)
			msgLen = FragmentSize;
		else
			msgLen = (unsigned int)resource->length() - position;

		uint8_t flags = RESOURCE_BASE_FLAG;
		if (msgLen < resource->length() - position)
		{
			if (position == 0)
				flags |= RESOURCE_INITIAL_FRAGMENT;
		}
		else
		{
			if (position == 0)
				flags |= RESOURCE_INITIAL_FRAGMENT | RESOURCE_FINAL_FRAGMENT;
			else
				flags |= RESOURCE_FINAL_FRAGMENT;
		}

		bundle << flags;

		/*
		 * Write resource header (messageType, categoryId, elementId)
		 * 0 = MESSAGE_CacheData
		 */
		if (position == 0)
			bundle << (uint8_t)0 << categoryId << elementId;

		/*
		 * This is a hack to work around some bundler limitations
		 * (write() will try to do a single write+expand and causes packets
		 *  with lots of free space to be finalized)
		 */
		while (msgLen)
		{
			unsigned int writeLen = (msgLen > 100) ? 100 : msgLen;
			bundle.write(&(*resource)[position], writeLen);
			msgLen -= writeLen;
			position += writeLen;
		}

		bundle.endMessage();
	}
}

void ClientHandler::createCellPlayer(uint32_t entityId, uint32_t spaceId,
	float x, float y, float z, float rotX, float rotY, float rotZ)
{
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: SPACE_VIEWPORT_INFO: entityId=%d, spaceId=%d", entityId, spaceId);
#endif
	Bundle & bundle = *channel_->bundle();
	bundle.beginMessage(BASEMSG_SPACE_VIEWPORT_INFO);
	bundle << entityId << entityId << spaceId << (uint8_t)0;
	bundle.endMessage();
	
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: CREATE_CELL_PLAYER: spaceId=%d, pos=(%f, %f, %f)", entityId, x, y, z);
#endif
	bundle.beginMessage(BASEMSG_CREATE_CELL_PLAYER);
	bundle << spaceId << (uint32_t)0 <<
		x << y << z << rotX << rotZ << rotY;
	bundle.endMessage();
	
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: FORCED_POSITION: entityId=%d, spaceId=%d, pos=(%f, %f, %f), rot=(%f, %f, %f)", 
		entityId, spaceId, x, y, z, rotX, rotY, rotZ);
#endif
	bundle.beginMessage(BASEMSG_FORCED_POSITION);
	bundle << entityId << spaceId << (uint32_t)0 <<
		x << y << z << 
		0.0f << 0.0f << 0.0f << 
		rotX << rotZ << rotY <<
		(uint8_t)0x01;
	bundle.endMessage();
}

void ClientHandler::switchEntity(BaseEntity * newEntity)
{
	SGW_ASSERT(entity_ && entity_->controller() && !newEntity->controller());

	disconnectEntity(false);
	entity_ = newEntity->shared_from_this();
	SGW_ASSERT(entity_->classDef()->index() != -1);
	SGW_ASSERT(entity_->client() == nullptr);
	entity_->setupClient(channel_);
}

void ClientHandler::sendBundle(Bundle::Ptr bundle)
{
	channel_->sendBundle(bundle);
}

void ClientHandler::flush()
{
	channel_->flushBundle(true);
}


void ClientHandler::disconnectEntity(bool killConnection)
{
	entitySystemEnabled_ = false;

	if (entity_)
	{
		PyGilGuard guard;
		entity_->detachedFromController();
		entity_->destroy();
		entity_ = BaseEntity::Ptr();
	}
	else
		WARNC(CATEGORY_MESSAGES, "Called with no entity connected!");
	
	// Make sure that this is sent in a separate bundle; otherwise the
	// client will not be able to process the message
	channel_->flushBundle(true);
	Bundle & bundle = *channel_->bundle();
	if (killConnection)
	{
#if defined(CLIENT_DEBUG)
		TRACEC(CATEGORY_MESSAGES, "Base -> Client: LOGGED_OFF");
#endif
		bundle.beginMessage(BASEMSG_LOGGED_OFF, Bundle::FLUSH);
		bundle << (uint8_t)0;
		bundle.endMessage();
		channel_->flushBundle(false);
		channel_->condemn();
	}
	else
	{
#if defined(CLIENT_DEBUG)
		TRACEC(CATEGORY_MESSAGES, "Base -> Client: RESET_ENTITIES");
#endif
		bundle.beginMessage(BASEMSG_RESET_ENTITIES, Bundle::FLUSH);
		bundle << (uint8_t)0;
		bundle.endMessage();
	}
}

void ClientHandler::gameTick(uint64_t time)
{
	if (entity_)
	{
#if defined(CLIENT_DEBUG)
		// TRACEC(CATEGORY_MESSAGES, "BASEMSG_TICK_SYNC: time=%d", (uint32_t)time);
#endif
		Bundle & bundle = unreliableTickSync_ ? *channel_->unreliableBundle() : *channel_->bundle();
		bundle.beginMessage(BASEMSG_TICK_SYNC);
		bundle << (uint32_t)time << (uint32_t)CellManager::instance().tickRate();
		bundle.endMessage();
	}
}

void ClientHandler::createEntity(uint32_t entityId, uint8_t classId, Vec3 & position, Vec3 & rotation)
{
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: CREATE_ENTITY: entityId=%d, classId=%d", entityId, classId);
#endif
	Bundle & bundle = *channel_->bundle();
	bundle.beginMessage(BASEMSG_CREATE_ENTITY);
	bundle << entityId << (uint8_t)0xff << classId << (uint8_t)0x00 << (uint8_t)0x00;
	bundle.endMessage();

	Vec3 velocity;
	// TODO: Is the flag 0x01 valid?
	moveEntity(entityId, position, rotation, velocity, 0x01);
}

void ClientHandler::enterAoI(uint32_t entityId)
{
	Bundle & bundle = *channel_->bundle();
	// SGW specific - SpawnableEntity::onVisible(true)
	bundle.beginEntityMessage(0x08, entityId);
	bundle << (uint8_t)0x01;
	bundle.endMessage();
}

void ClientHandler::leaveAoI(uint32_t entityId, bool deleteEntity)
{
	if (deleteEntity && this->entity_ && entityId == this->entity_->entityId())
		return;

#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: ENTITY_INVISIBLE: entityId=%d", entityId);
#endif
	Bundle & bundle = *channel_->bundle();
	bundle.beginMessage(BASEMSG_ENTITY_INVISIBLE);
	// 0xff = No alias
	bundle << entityId << (uint8_t)0xff;
	bundle.endMessage();
	
	if (deleteEntity)
	{
#if defined(CLIENT_DEBUG)
		TRACEC(CATEGORY_MESSAGES, "Base -> Client: LEAVE_AOI: entityId=%d", entityId);
#endif
		bundle.beginMessage(BASEMSG_LEAVE_AOI);
		// Only send CacheStamp 0
		bundle << entityId << (uint32_t)0;
	}
	bundle.endMessage();
}

void ClientHandler::moveEntity(uint32_t entityId, Vec3 const & position, Vec3 const & rotation, Vec3 const & velocity, uint8_t flags)
{
#if defined(CLIENT_DEBUG)
	// TRACEC(CATEGORY_MESSAGES, "Base -> Client: DETAILED_POSITION: entityId=%d", entityId);
#endif
	Bundle & bundle = unreliableMovement_ ? *channel_->unreliableBundle() : *channel_->bundle();
	bundle.beginMessage(BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL);
	bundle << entityId << position.x << position.y << position.z;
	packXYZ(bundle, velocity);
	bundle << (uint8_t)flags <<
		(uint8_t)(rotation.y / 0.024543693f) <<
		(uint8_t)(rotation.x / 0.024543693f) <<
		(uint8_t)(rotation.z / 0.024543693f);
	bundle.endMessage();
}


void ClientHandler::forcedPosition(uint32_t entityId, uint32_t spaceId, Vec3 const & position, Vec3 const & rotation, Vec3 const & velocity, uint8_t flags)
{
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: FORCED_POSITION: entityId=%d, spaceId=%d, pos=(%f, %f, %f), rot=(%f, %f, %f)", 
		entityId, spaceId, position.x, position.y, position.z, rotation.x, rotation.y, rotation.z);
#endif
	Bundle & bundle = *channel_->bundle();
	bundle.beginMessage(BASEMSG_FORCED_POSITION);
	bundle << entityId << spaceId << (uint32_t)0 <<
		position.x << position.y << position.z << 
		velocity.x << velocity.y << velocity.z << 
		rotation.x << rotation.y << rotation.z <<
		flags;
	bundle.endMessage();
}


void ClientHandler::enableEntities()
{
	if (entitySystemEnabled_)
	{
		WARNC(CATEGORY_MESSAGES, "The entity system is already enabled");
		return;
	}

	if (!entity_)
	{
		WARNC(CATEGORY_MESSAGES, "No controller entity attached to client!");
		return;
	}
	
	PyGilGuard guard;
#if defined(CLIENT_DEBUG)
	TRACEC(CATEGORY_MESSAGES, "Base -> Client: CREATE_BASE_PLAYER: entityId=%d, classId=%d", 
		entity_->entityId(), entity_->classDef()->index());
#endif
	Bundle & bundle = *channel_->bundle();
	bundle.beginMessage(BASEMSG_CREATE_BASE_PLAYER);
	bundle << entity_->entityId();
	bundle << (uint8_t)entity_->classDef()->index();
	bundle << (uint8_t)0; // Python property count
	bundle.endMessage();

	entity_->attachedToController(this);

	/*
	 * Check if there is a Cell entity instance associated with the
	 * base entity; if there is, request the cell to send its position data
	 * (needed for CREATE_CELL_PLAYER message)
	 */
	if (entity_->spaceId() != 0xffffffff)
	{
		SpaceData * space = CellManager::instance().findSpace(entity_->spaceId());
		if (space && space->handler)
			space->handler->sendConnectEntity(entity_->entityId());
		else
			WARNC(CATEGORY_MESSAGES, "No cell handler found for space %d!", entity_->spaceId());
	}

	entitySystemEnabled_ = true;
}

void ClientHandler::forwardMessage(uint32_t entityId, uint8_t messageId, bp::object args)
{
	BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId);
	if (!entity)
	{
		WARN("Cannot send message to nonexistent entity %d!", entityId);
		return;
	}
	
	PyMethod * method = entity->classDef()->findMethod(Service::ClientMailbox, messageId);

	Mercury::Bundle & bundle = *channel_->bundle();
	bundle.beginEntityMessage(messageId, entityId);
	PyGilGuard guard;
	method->serializeArgs(bundle, args);
	bundle.endMessage();
}

void ClientHandler::forwardMessage(uint32_t entityId, uint8_t messageId, void * buffer, std::size_t size)
{
	Mercury::Bundle & bundle = *channel_->bundle();
	bundle.beginEntityMessage(messageId, entityId);
	bundle.write(buffer, size);
	bundle.endMessage();
}

void ClientHandler::packXYZ(Mercury::Bundle & bundle, Vec3 const & v)
{
	float x, y, z;
	uint32_t packed1 = 0;
	uint8_t packed2 = 0;

	if (v.x < 0.0f)
	{
		x = -v.x;
		packed1 |= 0x00800000;
	}
	else
		x = v.x;
	x += 2.0f;
	packed1 |= (reinterpret_cast<uint32_t &>(x) >> 3) & 0x007FF000;

	if (v.z < 0.0f)
	{
		z = -v.z;
		packed1 |= 0x00000800;
	}
	else
		z = v.z;
	z += 2.0f;
	packed1 |= (reinterpret_cast<uint32_t &>(z) >> 15) & 0x000007FF;

	if (v.y < 0.0f)
	{
		y = -v.y;
		packed2 |= 0x80;
	}
	else
		y = v.y;
	y += 2.0f;

	uint32_t yDelta = (reinterpret_cast<uint32_t &>(y) >> 12) & 0x00007FFF;
	packed1 |= (yDelta & 0xFF) << 24;
	packed2 |= (uint8_t)((yDelta & 0x7F00) >> 8);

	bundle << packed1 << packed2;
}

bool ClientHandler::canSendResource()
{
	return channel_->sendQueueSize() + channel_->bundle()->numPackets() < ResourceTxQueueSize;
}

}
}
