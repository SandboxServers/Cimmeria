#pragma once

#include <entity/entity.hpp>
#include <entity/mailbox.hpp>
#include <common/vec3.hpp>
#include <boost/enable_shared_from_this.hpp>

class CellMailboxClass : public MailboxClass
{
public:
	CellMailboxClass(bp::object & module, PyClassDef * cls);
	virtual ~CellMailboxClass();

private:
	virtual void doRpc(MailboxRpcMethod const * method, bp::object args, bp::object kw);
};

class CellMailbox : public Mailbox
{
public:
	CellMailbox(MailboxClass * cls, uint32_t entityId, uint8_t distributionFlags);
	~CellMailbox();

	uint8_t distributionFlags() const;
	uint32_t recipientId() const;
	uint32_t spaceId() const;
	void setDistributionFlags(uint8_t flags);
	void setRecipient(uint32_t entityId);
	void setSpace(uint32_t spaceId);

private:
	uint8_t distributionFlags_;
	uint32_t recipientId_;
	uint32_t spaceId_;
};

class CellEntity : public Entity, public boost::enable_shared_from_this<CellEntity>
{
public:
	// How many ticks should the client wait until its first avatar
	// position update is acknowledged after a server-side forced position update
	static const unsigned int PositionTicksOnForcedUpdate = 5;

	static const int InvalidDatabaseId = -1;

	typedef boost::shared_ptr<CellEntity> Ptr;

	// Flags that specify the way our position changed since the last update
	enum MovementUpdateFlag
	{
		// The position of the entity changed
		ChangedPosition = 0x01,
		// The velocity of the entity changed
		ChangedVelocity = 0x02,
		// The rotation/orientaton of the entity changed
		ChangedRotation = 0x04,
		// All position parameters were changed
		ChangedAll = ChangedPosition | ChangedVelocity | ChangedRotation
	};

	static void registerClass();

	CellEntity();
	~CellEntity();
	
	
	template <typename _T>
	void backup(_T & stream)
	{
		SGW_ASSERT(classDef() && "Cannot backup server-only entities");
		bool player;
		stream << (uint32_t)43;
		stream << position_.x << position_.y << position_.z;
		stream << rotation_.x << rotation_.y << rotation_.z;
		stream << velocity_.x << velocity_.y << velocity_.z;
		stream << maxSpeed_ << movementType_ << player << (client_ != nullptr);

		PyGilGuard guard;
		auto backup = Entity::callMethod("backup");
		if (backup)
		{
			PyTypeDb::instance().findType("PYTHON")->pack(stream, backup);
		}
		else
		{
			FAULTC(CATEGORY_ENTITY, "Exception while creating a backup of '%s':", classDef()->name().c_str());
			PyUtil_ShowErr();

			PyTypeDb::instance().findType("PYTHON")->pack(stream, bp::object());
		}
	}
	

	template <typename _T>
	void restore(_T & stream, uint32_t spaceId)
	{
		bool connected;
		stream >> position_.x >> position_.y >> position_.z;
		stream >> rotation_.x >> rotation_.y >> rotation_.z;
		stream >> velocity_.x >> velocity_.y >> velocity_.z;
		stream >> maxSpeed_ >> movementType_ >> isPlayer_ >> connected;

		restorePython(stream);
		restored(spaceId);
		if (connected)
			this->connected();
	}


	template <typename _T>
	void restore(_T & stream, uint32_t spaceId, Vec3 const & position, Vec3 const & rotation)
	{
		bool connected;
		Vec3 dummy;
		stream >> dummy.x >> dummy.y >> dummy.z;
		stream >> dummy.x >> dummy.y >> dummy.z;
		stream >> dummy.x >> dummy.y >> dummy.z;
		stream >> maxSpeed_ >> movementType_ >> isPlayer_ >> connected;

		position_ = position;
		rotation_ = rotation;
		velocity_ = Vec3();

		restorePython(stream);
		restored(spaceId);
		if (connected)
			this->connected();
	}


	template <typename _T>
	void restorePython(_T & stream)
	{
		PyGilGuard guard;
		bp::object backup = PyTypeDb::instance().findType("PYTHON")->unpack(stream);
		callMethod("restore", backup);
	}


	void restored(uint32_t spaceId);

	bool enterSpace(class Space * space);
	bool enterSpaceById(uint32_t spaceId);
	bool leaveSpace();
	
	void connected();
	void disconnected();
	virtual void destroy();

	void createCellPlayer();
	void tick(uint64_t time, uint32_t gameTimeInTicks);
	void createOnClient(Mailbox::Ptr client);
	void setController(class MovementController * controller);
	void enteredAoI(Ptr entity);
	void updatedPosition(uint32_t flags);

	/*
	 * Notifies the entity that the cache stamps on the BaseApp were reset
	 * and new stamps must be sent.
	 */
	void cacheStampsReset();

	Mailbox::Ptr client() const;
	Mailbox::Ptr witnesses() const;
	Mailbox::Ptr base() const;
	class Space * space() const;
	class MovementController * controller() const;
	bool isPlayer() const;
	
	/**
	 * Does this entity only exist on the CellApp (and not on the base)?
	 */
	bool isCellOnly() const;

	/*
	 * Returns cacheability and state flags of the entity.
	 */
	uint8_t entityFlags() const;
	uint32_t spaceId() const;
	Vec3 & position();
	Vec3 & rotation();
	Vec3 & velocity();
	float maxSpeed() const;
	void setMovementType(uint8_t type);
	uint8_t movementType() const;
	void moveTo(uint32_t spaceId, std::string const & worldName, Vec3 const & position, Vec3 const & rotation);
	void hide();
	
	bool hasDynamicProperties() const;
	void setHasDynamicProperties(bool hasProps);
	
	bool isCacheable() const;
	void setCacheable(bool cacheable);
	
	Vector3::Ptr getPosition();
	void setPosition(Vector3::Ptr position);
	
	Vector3::Ptr getRotation();
	void setRotation(Vector3::Ptr rotation);
	
	Vector3::Ptr getVelocity();
	void setVelocity(Vector3::Ptr velocity);

	float getMaxSpeed();
	void setMaxSpeed(float speed);

	void loadCell();
	void enableDebugController();
	void addWaypoint(Vector3::Ptr waypoint, bp::object callback);
	void cancelMovement();
	void moveToSpace(uint32_t spaceId, Vector3::Ptr position, Vector3::Ptr rotation);
	void moveToWorld(std::string const & worldName, Vector3::Ptr position, Vector3::Ptr rotation);

private:
	bp::object pyClient() const;
	bp::object pyBase() const;
	bp::object pyWitnesses() const;
	void dynamicUpdate(CellEntity::Ptr entity);

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
	void createCacheStamp(uint32_t propertySetId, bp::object callback, bool invalidate);

	virtual void removeFromManager();

	Mailbox::Ptr client_;
	Mailbox::Ptr witnesses_;
	Mailbox::Ptr base_;
	class Space * space_;
	class MovementController * controller_;
	Vec3 position_, rotation_, velocity_;
	float maxSpeed_;
	uint8_t movementType_;
	bool isPlayer_;
	// Indicates that the entity has properties that can vary depending on the witness
	// (eg. two players see the entity differently)
	bool hasDynamicProperties_;
	// Indicates that the entity can be cached on the BaseApp
	bool cacheable_;
	// The position of the client is force locked to a specified location until
	// this timer expires
	unsigned int forcedPositionTicks_;
};

