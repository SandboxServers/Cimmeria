#include <stdafx.hpp>
#include <cellapp/entity/cell_entity.hpp>
#include <cellapp/entity/entity_manager.hpp>
#include <cellapp/entity/movement.hpp>
#include <cellapp/cell_service.hpp>
#include <cellapp/base_client.hpp>
#include <cellapp/space.hpp>
#include <mercury/base_cell_messages.hpp>
#include <boost/python/make_function.hpp>
#include <boost/python/def.hpp>
#include <boost/python/import.hpp>
#include <boost/python/init.hpp>
#include <boost/python/scope.hpp>
#include <boost/python/handle.hpp>
#include <boost/python/call_method.hpp>

void CellEntity::registerClass()
{
	bp::object mod = bp::import(bp::str("Atrea"));
	bp::scope active_scope(mod);
	bp::class_<CellEntity, CellEntity::Ptr> cls =
		bp::class_<CellEntity, CellEntity::Ptr>("CellEntity");

	registerProperties(cls);
	cls
		.add_property("client", &CellEntity::pyClient)
		.add_property("base", &CellEntity::pyBase)
		.add_property("witnesses", &CellEntity::pyWitnesses)
		.add_property("spaceId", &CellEntity::spaceId)
		.add_property("position", &CellEntity::getPosition, &CellEntity::setPosition)
		.add_property("rotation", &CellEntity::getRotation, &CellEntity::setRotation)
		.add_property("velocity", &CellEntity::getVelocity, &CellEntity::setVelocity)
		.add_property("maxSpeed", &CellEntity::getMaxSpeed, &CellEntity::setMaxSpeed)
		.add_property("hasDynamicProperties", &CellEntity::hasDynamicProperties, &CellEntity::setHasDynamicProperties)
		.add_property("cacheable", &CellEntity::isCacheable, &CellEntity::setCacheable)
		.def("dynamicUpdate", &CellEntity::dynamicUpdate)
		.def("createCacheStamp", &CellEntity::createCacheStamp)
		.def("leaveSpace", &CellEntity::leaveSpace)
		.def("enterSpace", &CellEntity::enterSpaceById)
		.def("loadCell", &CellEntity::loadCell)
		.def("destroy", &CellEntity::destroy)
		.def("enableDebugController", &CellEntity::enableDebugController)
		.def("addWaypoint", &CellEntity::addWaypoint)
		.def("cancelMovement", &CellEntity::cancelMovement)
		.def("moveToSpace", &CellEntity::moveToSpace)
		.def("moveToWorld", &CellEntity::moveToWorld)
		.def("hide", &CellEntity::hide);
}

CellEntity::CellEntity()
	: space_(nullptr), controller_(nullptr), maxSpeed_(10.0f), movementType_(0x01), isPlayer_(false),
	  hasDynamicProperties_(true), cacheable_(true), forcedPositionTicks_(0)
{
}

CellEntity::~CellEntity()
{
	SGW_ASSERT(!space_);
	SGW_ASSERT(!base_);
	SGW_ASSERT(!client_);
	SGW_ASSERT(!witnesses_);
	delete controller_;
}


void CellEntity::restored(uint32_t spaceId)
{
	if (spaceId != 0xffffffff)
	{
		Space * space = SpaceManager::instance().get(spaceId);
		if (space)
			enterSpace(space);
		else
			FAULTC(CATEGORY_ENTITY, "Failed to restore entity %d to space %d: No such space", entityId(), spaceId);
	}
}

bool CellEntity::enterSpace(Space * space)
{
	if (space_)
	{
		WARNC(CATEGORY_ENTITY, "Entity is already on a space!");
		return false;
	}

	PyGilGuard guard;
	if (base_)
		base_->unbind();
	base_.reset(new Mailbox(classDef()->baseMailbox(), entityId()));
	auto cellMbox = new CellMailbox(classDef()->clientMailbox(), entityId(), Mercury::DISTRIBUTE_LOD);
	cellMbox->setSpace(space->spaceId());
	if (witnesses_)
		witnesses_->unbind();
	witnesses_.reset(cellMbox);
	space_ = space;

	if (!space->addEntity(entityId(), shared_from_this()))
	{
		leaveSpace();
		return false;
	}

	callMethod("enteredSpace", bp::object(space_->spaceId()));

	// Register all local entities with the BaseApp
	// This is needed for world grid updates and message cache updates
	if (isCellOnly())
	{
		CellService::cell_instance().baseClient()->sendCellEntityCreated(
			entityId(), spaceId(), (uint8_t)classDef()->index(), position_, entityFlags()
		);
		cacheStampsReset();
	}

	return true;
}

bool CellEntity::enterSpaceById(uint32_t spaceId)
{
	Space * space = SpaceManager::instance().get(spaceId);
	if (!space)
	{
		WARNC(CATEGORY_ENTITY, "No such space id: %d", spaceId);
		return false;
	}

	return enterSpace(space);
}

bool CellEntity::leaveSpace()
{
	if (!space_)
	{
		WARNC(CATEGORY_ENTITY, "Entity is not on a space!");
		return false;
	}
	
	PyGilGuard guard;
	if (!space_->removeEntity(entityId()))
		return false;

	callMethod("leftSpace", bp::object(space_->spaceId()));

	space_ = nullptr;

	if (base_)
		base_->unbind();
	if (witnesses_)
		witnesses_->unbind();
	base_.reset();
	witnesses_.reset();

	// Unregister this entity with the BaseApp
	// This is needed for world grid updates and message cache updates
	CellService::cell_instance().baseClient()->sendCellEntityDeleted(entityId());

	return true;
}

void CellEntity::connected()
{
	if (!space_)
	{
		WARNC(CATEGORY_ENTITY, "Client connected to entity %d that is not on a space!", entityId());
		return;
	}

	PyGilGuard guard;
	if (client_)
		client_->unbind();
	client_.reset(new CellMailbox(classDef()->clientMailbox(), entityId(), Mercury::DISTRIBUTE_CLIENT));
	
	callMethod("connected");
}

void CellEntity::disconnected()
{
	// Already disconnected; ignore
	if (!client_)
		return;

	if (space_)
	{
		space_->demoteToEntity(entityId());
		isPlayer_ = false;
	}

	PyGilGuard guard;
	if (client_)
		client_->unbind();
	client_.reset();
	
	callMethod("disconnected");
}

void CellEntity::destroy()
{
	if (space_)
		leaveSpace();
	if (client_)
		disconnected();
	Entity::destroy();
}

void CellEntity::createCellPlayer()
{
	if (space_)
	{
		CellService::cell_instance().baseClient()->sendCellPlayerCreated(
			entityId(), space_->spaceId(), position_, rotation_);
	}
	else
		WARNC(CATEGORY_ENTITY, "Cannot create cell player - entity %d is not on a space!", entityId());
}

void CellEntity::tick(uint64_t time, uint32_t gameTimeInTicks)
{
	// Tick movement controller
	// Movement controller returns whether client updates should be sent
	// (e.g. straight movement needs less updates)
	if (controller_)
	{
		// When the forced position timer is active, we ignore the movement
		// controller completely until the timer expires
		if (forcedPositionTicks_)
		{
			forcedPositionTicks_--;
			CellService::cell_instance().baseClient()->sendEntityMove(this, true);
			return;
		}

		controller_->tick(time, gameTimeInTicks);
		if (controller_->isDeleting())
			setController(nullptr);
	}
	else
	{
		// This forces a movement update after every N ticks if no controller is attached
		// Entity ID is added to the server time to avoid jitter
		// (eg. for idleUpdateTicks=10 all entities on every space would send a movement update
		// at the same time in each second, increasing server load for that tick --> jitter)
		// Adding entity ID helps in smoothing network load over time
		int idleTicks = SpaceManager::instance().idleUpdateTicks();
		if (idleTicks != -1 && ((entityId() + time) % (idleTicks + 1)) == 0)
			updatedPosition(ChangedPosition);
	}
}

void CellEntity::createOnClient(Mailbox::Ptr client)
{
	PyGilGuard guard;
	callMethod("createOnClient", bp::object(client));
}

void CellEntity::setController(class MovementController * controller)
{
	delete controller_;
	controller_ = controller;
	if (controller_)
	{
		controller_->reset();
	}
	else
	{
		// Make sure that we don't have a velocity when not moving
		velocity_.x = velocity_.y = velocity_.z = 0.0f;
	}
}

/*
 * Indicates that an entity that has dynamic properties has
 * entered the players AoI (area of interest).
 */
void CellEntity::enteredAoI(CellEntity::Ptr entity)
{
	PyGilGuard guard;
	callMethod("enteredAoI", entity->object());
}


/*
 * Notifies the BaseApp that the position of the entity changed.
 * 
 * @param flags Movement update hint flags
 */
void CellEntity::updatedPosition(uint32_t flags)
{
	// Nothing changed, don't send an update
	if (!(flags & (ChangedPosition | ChangedRotation | ChangedVelocity)))
		return;

	// TODO: Skip ticks if possible
	// TODO: Change to UpdateAvatar message
	// TODO: Send smaller movement update (no velocity, no direction)
	CellService::cell_instance().baseClient()->sendEntityMove(this);
}


/*
 * Notifies the entity that the cache stamps on the BaseApp were reset
 * and new stamps must be sent.
 */
void CellEntity::cacheStampsReset()
{
	if (cacheable_ && CellService::cell_instance().baseClient()->isReady())
	{
		PyGilGuard guard;
		callMethod("cacheStampsReset");
	}
}

Mailbox::Ptr CellEntity::client() const
{
	return client_;
}

Mailbox::Ptr CellEntity::witnesses() const
{
	return witnesses_;
}

Mailbox::Ptr CellEntity::base() const
{
	return base_;
}

Space * CellEntity::space() const
{
	return space_;
}

MovementController * CellEntity::controller() const
{
	return controller_;
}

bool CellEntity::isPlayer() const
{
	return isPlayer_;
}

/**
 * Does this entity only exist on the CellApp (and not on the base)?
 */
bool CellEntity::isCellOnly() const
{
	return CellEntityManager<CellEntity>::instance().isLocalId(entityId());
}

/*
 * Returns cacheability and state flags of the entity.
 */
uint8_t CellEntity::entityFlags() const
{
	uint8_t flags = 0;
	if (hasDynamicProperties_)
		flags |= Mercury::ENTITY_HAS_DYNAMIC;
	if (!cacheable_)
		flags |= Mercury::ENTITY_NOT_CACHED;
	return flags;
}

uint32_t CellEntity::spaceId() const
{
	if (space_)
		return space_->spaceId();
	else
		return 0xffffffff;
}

Vec3 & CellEntity::position()
{
	return position_;
}

Vec3 & CellEntity::rotation()
{
	return rotation_;
}

Vec3 & CellEntity::velocity()
{
	return velocity_;
}

float CellEntity::maxSpeed() const
{
	return maxSpeed_;
}

void CellEntity::setMovementType(uint8_t type)
{
	movementType_ = type;
}

uint8_t CellEntity::movementType() const
{
	return movementType_;
}

void CellEntity::removeFromManager()
{
	if (space_)
		leaveSpace();
	CellEntityManager<CellEntity>::instance().destroyed(shared_from_this());
}

void CellEntity::setHasDynamicProperties(bool hasProps)
{
	hasDynamicProperties_ = hasProps;
}

bool CellEntity::hasDynamicProperties() const
{
	return hasDynamicProperties_;
}
	
bool CellEntity::isCacheable() const
{
	return cacheable_;
}

void CellEntity::setCacheable(bool cacheable)
{
	cacheable_ = cacheable;
}

void CellEntity::setPosition(Vector3::Ptr position)
{
	position_.x = position->x;
	position_.y = position->y;
	position_.z = position->z;

	if (isPlayer_)
		forcedPositionTicks_ = PositionTicksOnForcedUpdate;
}

Vector3::Ptr CellEntity::getPosition()
{
	return Vector3::Ptr(new Vector3(position_.x, position_.y, position_.z));
}

void CellEntity::setRotation(Vector3::Ptr rotation)
{
	rotation_.x = rotation->x;
	rotation_.y = rotation->y;
	rotation_.z = rotation->z;
}

Vector3::Ptr CellEntity::getRotation()
{
	return Vector3::Ptr(new Vector3(rotation_.x, rotation_.y, rotation_.z));
}

void CellEntity::setVelocity(Vector3::Ptr velocity)
{
	velocity_.x = velocity->x;
	velocity_.y = velocity->y;
	velocity_.z = velocity->z;
}

Vector3::Ptr CellEntity::getVelocity()
{
	return Vector3::Ptr(new Vector3(velocity_.x, velocity_.y, velocity_.z));
}

void CellEntity::setMaxSpeed(float speed)
{
	maxSpeed_ = speed;
}

float CellEntity::getMaxSpeed()
{
	return maxSpeed_;
}

void CellEntity::moveTo(uint32_t spaceId, std::string const & worldName, Vec3 const & position, Vec3 const & rotation)
{
	BaseAppClient * client = CellService::cell_instance().baseClient();
	client->sendEntityBackup(this);
	// TODO: Disconnect entity?
	client->sendSwitchSpace(entityId(), spaceId, worldName, position, rotation);
}

void CellEntity::hide()
{
	if (!space())
	{
		WARNC(CATEGORY_ENTITY, "Tried to hide entity %d that is not on a space!", entityId());
		return;
	}

	// Notify all players on the space that the entity left their AoI
	CellService::cell_instance().baseClient()->sendLeftAoI(space()->spaceId(), entityId(), false);
}

bp::object CellEntity::pyClient() const
{
	if (client())
		return bp::object(client());
	else
		return bp::object();
}

bp::object CellEntity::pyBase() const
{
	if (base())
		return bp::object(base());
	else
		return bp::object();
}

bp::object CellEntity::pyWitnesses() const
{
	if (witnesses())
		return bp::object(witnesses());
	else
		return bp::object();
}

void CellEntity::dynamicUpdate(CellEntity::Ptr entity)
{
	// Hijack the entities mailbox to only send messages to us
	CellMailbox * mailbox = static_cast<CellMailbox *>(entity->witnesses().get());
	SGW_ASSERT(mailbox && mailbox->distributionFlags() == Mercury::DISTRIBUTE_LOD);
	SGW_ASSERT(mailbox->entityId() == entity->entityId());
	mailbox->setDistributionFlags(Mercury::DISTRIBUTE_RECIPIENT);
	mailbox->setRecipient(entityId());

	// Ask the entity to do a dynamic update
	PyGilGuard guard;
	callMethod("onDynamicUpdate", entity->object());

	// Restore the mailbox to its original state
	mailbox->setDistributionFlags(Mercury::DISTRIBUTE_LOD);
}

/*
 * Creates a new revision of a cached property set on the BaseApp and notifies witnesses.
 * Calls the specified callback function and redirects all mailboxes for the duration of 
 * the call to the cache message set.
 * WARNING: Do not call RPC methods on the mailboxes of other entities for the duration of the call.
 *
 * @param propertySetId Property set to update
 * @param callback      Cache creation callback
 * @param invalidate    Invalidate previous messages?
 */
void CellEntity::createCacheStamp(uint32_t propertySetId, bp::object callback, bool invalidate)
{
	if (propertySetId >= Mercury::MaxPropertySets)
	{
		WARN("Illegal property set id %d", propertySetId);
		return;
	}
	
	if (!cacheable_)
	{
		WARN("Called on non-cacheable entity %d", entityId());
		return;
	}
	
	CellService::cell_instance().baseClient()->beginCacheStamp(entityId(), propertySetId, invalidate);
	// Call the cache message generation callback that will fill the cache update buffer
	PyGilGuard guard;
	try
	{
		callback();
	}
	catch (bp::error_already_set &)
	{
		FAULTC(CATEGORY_ENTITY, "Exception in createCacheStamp callback:");
		PyUtil_ShowErr();
	}
	CellService::cell_instance().baseClient()->endCacheStamp();
}

void CellEntity::loadCell()
{
	SGW_ASSERT(space_);
	SGW_ASSERT(!isCellOnly());
	setController(new PlayerController(this));
	space_->promoteToPlayer(entityId());
	isPlayer_ = true;

	CellService::cell_instance().baseClient()->sendCellEntityCreated(
		entityId(), spaceId(), (uint8_t)classDef()->index(), position_, entityFlags()
	);
	cacheStampsReset();
}

void CellEntity::enableDebugController()
{
	setController(new DebugController(this));
}

void CellEntity::addWaypoint(Vector3::Ptr waypoint, bp::object callback)
{
	if (isPlayer_)
	{
		WARNC(CATEGORY_ENTITY, "Cannot set waypoints on a player");
		return;
	}

	WaypointController * controller = dynamic_cast<WaypointController *>(controller_);
	if (!controller)
	{
		controller = new WaypointController(this);
		setController(controller);
	}

	controller->addWaypoint(Vec3(waypoint->x, waypoint->y, waypoint->z), callback);
}

void CellEntity::cancelMovement()
{
	if (isPlayer_)
	{
		WARNC(CATEGORY_ENTITY, "Cannot cancel movement on a player");
		return;
	}

	setController(nullptr);
}

void CellEntity::moveToSpace(uint32_t spaceId, Vector3::Ptr position, Vector3::Ptr rotation)
{
	moveTo(spaceId, "", Vec3(position->x, position->y, position->z), 
		Vec3(rotation->x, rotation->y, rotation->z));
}

void CellEntity::moveToWorld(std::string const & worldName, Vector3::Ptr position, Vector3::Ptr rotation)
{
	moveTo(0xffffffff, worldName, Vec3(position->x, position->y, position->z), 
		Vec3(rotation->x, rotation->y, rotation->z));
}


CellMailboxClass::CellMailboxClass(bp::object & module, PyClassDef * cls)
	: MailboxClass(module, cls, Service::ClientMailbox, "CellMailbox")
{
}


CellMailboxClass::~CellMailboxClass()
{
}


void CellMailboxClass::doRpc(MailboxRpcMethod const * method, bp::object args, bp::object /*kw*/)
{
	MessageWriter * writer = Service::instance().messageWriter(method->method->endpointType(), method->mailbox->entityId());
	MessageWriter::DistributionPolicy policy;
	CellMailbox * mbox = static_cast<CellMailbox *>(method->mailbox.get());
	policy.flags = mbox->distributionFlags();
	policy.recipientId = mbox->recipientId();
	policy.spaceId = mbox->spaceId();
	writer->write(method->method->endpointType(), policy, method->mailbox->entityId(), method->method->messageId(), args);
}


CellMailbox::CellMailbox(MailboxClass * cls, uint32_t entityId, uint8_t distributionFlags)
	: Mailbox(cls, entityId), distributionFlags_(distributionFlags), recipientId_(0xffffffff)
{
}

CellMailbox::~CellMailbox()
{
}

uint8_t CellMailbox::distributionFlags() const
{
	return distributionFlags_;
}

uint32_t CellMailbox::recipientId() const
{
	return recipientId_;
}

uint32_t CellMailbox::spaceId() const
{
	return spaceId_;
}

void CellMailbox::setDistributionFlags(uint8_t flags)
{
	distributionFlags_ = flags;
}

void CellMailbox::setRecipient(uint32_t entityId)
{
	recipientId_ = entityId;
}

void CellMailbox::setSpace(uint32_t spaceId)
{
	spaceId_ = spaceId;
}

