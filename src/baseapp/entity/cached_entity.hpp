#pragma once

#include <common/vec3.hpp>
#include <util/singleton.hpp>
#include <entity/world_grid.hpp>
#include <mercury/base_cell_messages.hpp>

/*
 * A Cell entity with cached parts stored on the Base
 */
class CachedEntity : public WorldGridMember<CachedEntity>, public boost::enable_shared_from_this<CachedEntity>
{
public:
	// Describes the version of the data a witness knows about an entity
	struct KnownVersion
	{
		inline KnownVersion()
			: generationId(0)
		{
			for (unsigned i = 0; i < Mercury::MaxPropertySets; i++)
			{
				versions[i] = 0;
			}
		}

		// This ID is incremented every time the entity ID is reused
		// (make sure that we don't accidentally consider a new entity cached)
		uint32_t generationId;
		// Last seen version of each property set
		uint32_t versions[Mercury::MaxPropertySets];
	};

	struct MovementUpdate
	{
		Vec3 position, rotation, velocity;
		uint8_t flags;
		uint8_t serverFlags;
	};

	typedef boost::shared_ptr<CachedEntity> Ptr;
	typedef boost::weak_ptr<CachedEntity> WeakPtr;

	static void staticInit();

	CachedEntity(uint32_t entityId, uint8_t classId, uint32_t generationId, uint8_t flags);
	~CachedEntity();

	inline uint32_t entityId() const
	{
		return entityId_;
	}

	inline uint8_t classId() const
	{
		return classId_;
	}

	inline uint8_t flags() const
	{
		return flags_;
	}

	inline Vec3 & lastPosition()
	{
		return lastPosition_;
	}

	bool isAWitness() const;

	bool isOnCell(uint16_t cellId);
	void enterSpace(uint32_t spaceId, Vec3 const & position);
	void leaveSpace();

	void addMove(MovementUpdate const & update);
	WorldGridPosition gridPosition() const;

	/*
	 * World grid callback called when a witnessed entity becomes invisible.
	 *
	 * @param entity    Entity that became invisible to us
	 * @param destroyed Was the entity destroyed?
	 */
	void onEntityInvisible(WeakPtr entity, bool destroyed);

	/*
	 * World grid callback called when a witnessed entity becomes visible.
	 *
	 * @param entity Entity that became visible to us
	 */
	void onEntityVisible(WeakPtr entity);

	/*
	 * Called when the cached entity is being deleted from the entity manager.
	 */
	void onDelete();

	void invalidateCacheStamps();
	void invalidateCacheStamps(uint32_t propertySetId);
	void beginCacheStamp(uint32_t propertySetId);
	void addCacheMessage(uint32_t propertySetId, uint8_t messageId, uint8_t flags, void * args, std::size_t argsLength);
	void endCacheStamp();

	/*
	 * Notifies the witness entity about any cache updates that have occured since the
	 * entity has last seen us. If no reference version is available, a full update is sent.
	 *
	 * @param witness    Entity witnessing us
	 * @param refVersion Last cache version the witness has seen
	 */
	void sendDeltas(CachedEntity & witness, KnownVersion const & refVersion) const;

	/*
	 * Notifies the witness entity about any cache updates that have occured since the
	 * entity has last seen us. If no reference version is available, a full update is sent.
	 *
	 * @param witness       Entity witnessing us
	 * @param propertySetId Property set to send
	 * @param refVersion    Last cache version the witness has seen
	 */
	void sendDeltas(CachedEntity & witness, uint32_t propertySetId, uint32_t refVersion) const;
	
	/*
	 * Updates the reference version so it contains the latest version numbers for this entity.
	 * Should be called after receiving a delta or a propagated update.
	 *
	 * @param refVersion Cache version to update
	 */
	void updateRef(KnownVersion & refVersion) const;

private:
	static const uint32_t InvalidPropertySetId = 0xffffffff;
	static bool CacheOnClient;

	// A cached entity RPC message
	struct CacheMessage
	{
		CacheMessage(uint8_t _messageId, uint8_t _flags, void * _args, uint16_t _argsLength)
			: messageId(_messageId), flags(_flags), args(new uint8_t[_argsLength]), argsLength(_argsLength)
		{
			memcpy(args, _args, argsLength);
		}

		~CacheMessage()
		{
			delete [] args;
		}

		CacheMessage(CacheMessage const & msg)
			: minVersion(msg.minVersion), maxVersion(msg.maxVersion),
			messageId(msg.messageId), flags(msg.flags),
			argsLength(msg.argsLength), args(msg.args)
		{
			// FIXME: Fix buffer ownership semantics for messages
			msg.args = nullptr;
		}

		CacheMessage & operator = (CacheMessage & msg)
		{
			minVersion = msg.minVersion;
			maxVersion = msg.maxVersion;
			messageId = msg.messageId;
			flags = msg.flags;
			argsLength = msg.argsLength;
			args = msg.args;
			msg.args = nullptr;
			return *this;
		}

		// First and last propset version that can see this message
		uint32_t minVersion, maxVersion;
		// RPC message ID
		uint8_t messageId;
		// Distribution/merge flags
		uint8_t flags;
		// Length of message arguments
		uint16_t argsLength;
		// Message arguments
		mutable uint8_t * args;
	};

	struct CacheStamp
	{
		std::vector<unsigned int> messages;
	};

	struct PropertySet
	{
		PropertySet()
			: firstStampId(1)
		{}

		void invalidate()
		{
			firstStampId += (uint32_t)stamps.size();
			stamps.clear();
			messages.clear();
		}

		void addStamp()
		{
			stamps.resize(stamps.size() + 1);
		}

		void addMessage(uint8_t messageId, uint8_t flags, void * args, std::size_t argsLength)
		{
			CacheMessage msg(messageId, flags, args, (uint16_t)argsLength);
			msg.minVersion = lastVersion();
			msg.maxVersion = 0xffffffff;
			messages.push_back(msg);
			stamps.rbegin()->messages.push_back((unsigned int)messages.size() - 1);
		}

		uint32_t lastVersion() const
		{
			return firstStampId + (uint32_t)stamps.size() - 1;
		}

		uint32_t firstStampId;
		std::vector<CacheMessage> messages;
		std::vector<CacheStamp> stamps;
	};
	
	// Unique ID of this entity
	uint32_t entityId_;
	// Entity type ID on the cell
	uint8_t classId_;
	// Space holding this entity
	struct SpaceData * space_;
	// Last position received from the CellApp
	Vec3 lastPosition_;
	// Reuse counter of this cached entity ID
	uint32_t generationId_;
	// Cell entity cacheability flags (Mercury::CellEntityFlags)
	uint8_t flags_;
	// Cache data for each property set
	PropertySet propertySets_[Mercury::MaxPropertySets];
	// Which property set ID are we currently writing to?
	uint32_t writingPropertySet_;
	// Last known state of entities
	// We only allocate this at runtime if the entity is a witness to avoid memory overhead
	std::unique_ptr<std::map<uint32_t, KnownVersion> > knownEntities_;
};


class CachedEntityManager : public singleton<CachedEntityManager>
{
public:
	CachedEntity::Ptr getEntity(uint32_t entityId) const;
	CachedEntity::Ptr addEntity(uint32_t entityId, uint8_t classId, uint8_t flags);
	void deleteEntity(uint32_t entityId);
	void deleteCellEntities(uint16_t cellId);

private:
	struct EntityInfo
	{
		CachedEntity::Ptr entity;
		uint32_t generationId;
	};

	std::map<uint32_t, EntityInfo> entities_;
};

