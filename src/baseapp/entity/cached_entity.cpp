#include <stdafx.hpp>
#include <baseapp/entity/cached_entity.hpp>
#include <baseapp/cell_manager.hpp>

#if defined(_DEBUG)
#define DEBUG_CACHE_UPDATES
// #define DEBUG_CACHE_MESSAGES
#endif

bool CachedEntity::CacheOnClient = true;

void CachedEntity::staticInit()
{
	CacheOnClient = Service::instance().getConfigParameter("cache_on_client") == "true";
}

CachedEntity::CachedEntity(uint32_t entityId, uint8_t classId, uint32_t generationId, uint8_t flags)
	: entityId_(entityId), classId_(classId), space_(nullptr),
	generationId_(generationId), flags_(flags), writingPropertySet_(InvalidPropertySetId)
{
	if (isAWitness())
	{
		knownEntities_.reset(new std::map<uint32_t, KnownVersion>());
	}
}

CachedEntity::~CachedEntity()
{
}

bool CachedEntity::isAWitness() const
{
	// TODO: Is this OK in all cases? Can base entities be non-witnesses?
	return Mercury::IsBaseEntity(entityId_);
}

bool CachedEntity::isOnCell(uint16_t cellId)
{
	return space_ != nullptr && space_->handler->cellId() == cellId;
}

void CachedEntity::enterSpace(uint32_t spaceId, Vec3 const & position)
{
	SGW_ASSERT(!space_);
	SpaceData * space = CellManager::instance().findSpace(spaceId);
	SGW_ASSERT(space);
	space_ = space;
	lastPosition_ = position;
	space->grid.add(shared_from_this());
}

void CachedEntity::leaveSpace()
{
	SGW_ASSERT(space_ != nullptr && &space_->grid == grid_);
	space_->grid.remove(shared_from_this());
	space_ = nullptr;
}

void CachedEntity::addMove(MovementUpdate const & update)
{
	if (space_)
	{
		WorldGridPosition position((int)update.position.z, (int)update.position.x);
		if (!space_->grid.validPosition(position))
		{
			WARN("Tried to add illegal move (%f, %f, %f) for entity %d", entityId(),
				update.position.x, update.position.y, update.position.z);
			return;
		}

		if (update.serverFlags & 0x01)
		{
			if (!isAWitness())
			{
				WARN("CachedEntity::addMove(): Forced update received for non-witness entity %d!", entityId());
				return;
			}

			BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(entityId());
			if (entity && entity->controller())
			{
				entity->controller()->forcedPosition(
					entityId(), space_->spaceId, update.position, update.rotation, update.velocity, update.flags
				);
			}
			else
			{
				WARN("CachedEntity::addMove(): No base entity for entityId %d!", entityId());
				return;
			}
		}

		auto visitor = [this, &update] (CachedEntity::WeakPtr witnessWeak)
		{
			CachedEntity::Ptr witness = witnessWeak.lock();
			SGW_ASSERT(witness->isAWitness());
			BaseEntity::Ptr entity = BaseEntityManager<BaseEntity>::instance().get(witness->entityId());
			// NOTE: Even though a cached entity exists with the entityID of the witness,
			// the real entity may be already deleted due to a disconnection or other event,
			// so we should check the existence of both the entity and its controller
			if (entity && entity->controller())
			{
				entity->controller()->moveEntity(
					this->entityId(), update.position, update.rotation, update.velocity, update.flags
				);
			}
		};
		space_->grid.visitWitnesses(shared_from_this(), visitor);

		// Update our position on the world grid
		space_->grid.move(shared_from_this(), position);
	}

	lastPosition_ = update.position;
}

WorldGridPosition CachedEntity::gridPosition() const
{
	return WorldGridPosition((int)lastPosition_.z, (int)lastPosition_.x);
}


/*
 * World grid callback called when a witnessed entity becomes invisible.
 *
 * @param entity    Entity that became invisible to us
 * @param destroyed Was the entity destroyed?
 */
void CachedEntity::onEntityInvisible(WeakPtr entity, bool destroyed)
{
	SGW_ASSERT(isAWitness());
	CachedEntity::Ptr ref = entity.lock();
#ifdef DEBUG_CACHE_UPDATES
	TRACE("onEntityInvisible: witness %d entity %d", entityId_, ref->entityId());
#endif
	if (entityId_ == ref->entityId())
	{
		return;
	}
	
	if (CacheOnClient)
	{
		if (destroyed)
		{
			// Delete cache information for this entity as we won't see it ever again
			auto it = knownEntities_->find(ref->entityId());
			if (it != knownEntities_->end())
				knownEntities_->erase(it);
		}
		else
		{
			auto & known = (*knownEntities_)[ref->entityId()];
			ref->updateRef(known);
		}
	}

	BaseEntity::Ptr self = BaseEntityManager<BaseEntity>::instance().get(entityId_);
	if (!self || !self->controller())
	{
		WARN("Called on entity %d without a controller!", entityId_);
		return;
	}
	
	self->controller()->leaveAoI(ref->entityId(), !CacheOnClient || destroyed);
}


/*
 * World grid callback called when a witnessed entity becomes visible.
 *
 * @param entity Entity that became visible to us
 */
void CachedEntity::onEntityVisible(WeakPtr entity)
{
	SGW_ASSERT(isAWitness());
	CachedEntity::Ptr ref = entity.lock();
	if (entityId_ == ref->entityId())
	{
		return;
	}

#ifdef DEBUG_CACHE_UPDATES
	TRACE("onEntityVisible: witness %d entity %d", entityId_, ref->entityId());
#endif
	BaseEntity::Ptr self = BaseEntityManager<BaseEntity>::instance().get(entityId_);
	if (!self || !self->controller())
	{
		WARN("Called on entity %d without a controller!", entityId_);
		return;
	}
	
	if (CacheOnClient)
	{
		auto it = knownEntities_->find(ref->entityId());
		if (it == knownEntities_->end())
		{
			// TODO: Pass a proper rotation value to createEntity()!
			Vec3 rotation;
			self->controller()->createEntity(ref->entityId(), ref->classId(), ref->lastPosition(), rotation);
		}

		// If we haven't seen this ID yet, it'll initialize the known entity data it with default (0) values
		auto & known = (*knownEntities_)[ref->entityId()];
		ref->sendDeltas(*this, known);

		if (it != knownEntities_->end())
		{
			// Don't recreate the entity, only make it visiblee as it already exists on the client
			self->controller()->enterAoI(ref->entityId());
		}
	}
	else
	{
		// TODO: Pass a proper rotation value to createEntity()!
		Vec3 rotation;
		self->controller()->createEntity(ref->entityId(), ref->classId(), ref->lastPosition(), rotation);

		// Avoid adding an entry to knownEntities_ if we don't cache on the client
		KnownVersion known;
		ref->sendDeltas(*this, known);
	}

	uint8_t requested = 0;
	if (ref->flags() & Mercury::ENTITY_HAS_DYNAMIC)
		requested |= Mercury::UPDATE_DYNAMIC;
	if (ref->flags() & Mercury::ENTITY_NOT_CACHED)
		requested |= Mercury::UPDATE_STATIC | Mercury::UPDATE_UNCACHED;

	if (requested != 0)
	{
		// Ask the CellApp to send data that we don't have locally
		space_->handler->sendRequestEntityUpdate(
			entityId_, ref->entityId(), requested
		);
	}
}


/*
 * Called when the cached entity is being deleted from the entity manager.
 */
void CachedEntity::onDelete()
{
	if (grid_)
	{
		leaveSpace();
	}
}

void CachedEntity::invalidateCacheStamps()
{
	for (unsigned i = 0; i < Mercury::MaxPropertySets; i++)
	{
		propertySets_[i].invalidate();
	}
}

void CachedEntity::invalidateCacheStamps(uint32_t propertySetId)
{
	SGW_ASSERT(propertySetId < Mercury::MaxPropertySets);
	propertySets_[propertySetId].invalidate();
}

void CachedEntity::beginCacheStamp(uint32_t propertySetId)
{
	SGW_ASSERT(writingPropertySet_ == InvalidPropertySetId);
	SGW_ASSERT(propertySetId < Mercury::MaxPropertySets);
	writingPropertySet_ = propertySetId;
	propertySets_[propertySetId].addStamp();
}

void CachedEntity::addCacheMessage(uint32_t propertySetId, uint8_t messageId, uint8_t flags, void * args, std::size_t argsLength)
{
	SGW_ASSERT(propertySetId < Mercury::MaxPropertySets);
	propertySets_[propertySetId].addMessage(messageId, flags, args, argsLength);
}

void CachedEntity::endCacheStamp()
{
	SGW_ASSERT(writingPropertySet_ != InvalidPropertySetId);
	auto const & set = propertySets_[writingPropertySet_];

	// This function will be called for each witness that can see us
	auto visitor = [this, &set] (WeakPtr entity)
	{
		Ptr witness = entity.lock();
		SGW_ASSERT(witness->isAWitness());
		uint32_t witnessId = witness->entityId();
#ifdef DEBUG_CACHE_UPDATES
		TRACE("Propagating: witness %d, entity %d, propset %d", witnessId, entityId(), writingPropertySet_);
#endif

		MessageWriter * writer = Service::instance().messageWriter(Service::ClientMailbox, witnessId);
		MessageWriter::DistributionPolicy policy;
		policy.flags = Mercury::DISTRIBUTE_RECIPIENT;
		policy.recipientId = witnessId;

		// Send the updates from the last cache stamp to each visitor
		// (this assumes that every entity in the witness list sees the latest version of this entity)
		for (auto const & msgIndex : set.stamps.rbegin()->messages)
		{
			auto const & msg = set.messages[msgIndex];
#ifdef DEBUG_CACHE_MESSAGES
			TRACE("Witness %d, entity %d, propset %d, ver %d: tx message 0x%02x",
				witnessId, entityId(), writingPropertySet_, ver, msg.messageId);
#endif
			writer->write(
				Service::ClientMailbox, policy, entityId(),
				msg.messageId, msg.args, msg.argsLength
			);
		}
	};
	grid_->visitWitnesses(shared_from_this(), visitor);

	writingPropertySet_ = InvalidPropertySetId;
}

/*
 * Notifies the witness entity about any cache updates that have occured since the
 * entity has last seen us. If no reference version is available, a full update is sent.
 *
 * @param witness    Entity witnessing us
 * @param refVersion Last cache version the witness has seen
 */
void CachedEntity::sendDeltas(CachedEntity & witness, KnownVersion const & refVersion) const
{
	if (refVersion.generationId != generationId_)
	{
		if (refVersion.generationId > generationId_)
		{
			WARN("Witness %d has seen too new generation %d of entity %d; current is %d",
				witness.entityId(), refVersion.generationId, entityId(), generationId_);
		}
		else
		{
#ifdef DEBUG_CACHE_UPDATES
			TRACE("Witness %d has seen generation %d of entity %d; current is %d",
				witness.entityId(), refVersion.generationId, entityId(), generationId_);
#endif
		}
		
		for (unsigned i = 0; i < Mercury::MaxPropertySets; i++)
		{
			sendDeltas(witness, i, 0);
		}
	}
	else
	{
		for (unsigned i = 0; i < Mercury::MaxPropertySets; i++)
		{
			sendDeltas(witness, i, refVersion.versions[i]);
		}
	}
}


/*
 * Notifies the witness entity about any cache updates that have occured since the
 * entity has last seen us. If no reference version is available, a full update is sent.
 *
 * @param witness       Entity witnessing us
 * @param propertySetId Property set to send
 * @param refVersion    Last cache version the witness has seen
 */
void CachedEntity::sendDeltas(CachedEntity & witness, uint32_t propertySetId, uint32_t refVersion) const
{
	auto const & set = propertySets_[propertySetId];
	uint32_t lastVersion = set.lastVersion();
#ifdef DEBUG_CACHE_UPDATES
	TRACE("Witness %d, entity %d, propset %d: known %d, latest is %d",
		witness.entityId(), entityId(), propertySetId, refVersion, lastVersion);
#endif

	// Work out the latest version the client really has
	if (refVersion > lastVersion)
	{
		// Client version > local version; this shouldn't happen!
		WARN("Requested delta of entity %d, propset %d with too new ref %d (latest is %d)",
			entityId(), propertySetId, refVersion, lastVersion);
		refVersion = set.firstStampId - 1;
	}
	else if (refVersion < set.firstStampId - 1)
	{
		// Client version is older than our last invalidated stamp,
		// reset its index to the nearest unused stamp
		refVersion = set.firstStampId - 1;
	}
	
	// TODO: This needs further optimization
	// (ie. MessageWriter shouldn't resolve the entity mailbox after each write)
	MessageWriter * writer = Service::instance().messageWriter(Service::ClientMailbox, witness.entityId());
	MessageWriter::DistributionPolicy policy;
	policy.flags = Mercury::DISTRIBUTE_RECIPIENT;
	policy.recipientId = witness.entityId();
	
	// Check each cache stamp for messages to send
	unsigned refIndex = refVersion + 1 - set.firstStampId;
	for (unsigned i = refIndex; i < set.stamps.size(); i++)
	{
		unsigned ver = set.firstStampId + i;
		for (auto const & msgIndex : set.stamps[i].messages)
		{
			auto const & msg = set.messages[msgIndex];
			// A (min, max) version pair determines the visibility of the message
			// The witness sees the message if min <= lastVer <= max and witnessVer < min
			if (msg.minVersion == ver && msg.maxVersion >= lastVersion)
			{
#ifdef DEBUG_CACHE_MESSAGES
				TRACE("Witness %d, entity %d, propset %d, ver %d: tx message 0x%02x",
					witness.entityId(), entityId(), propertySetId, ver, msg.messageId);
#endif
				writer->write(
					Service::ClientMailbox, policy, entityId(),
					msg.messageId, msg.args, msg.argsLength
				);
			}
		}
	}
}


/*
 * Updates the reference version so it contains the latest version numbers for this entity.
 * Should be called after receiving a delta or a propagated update.
 *
 * @param refVersion Cache version to update
 */
void CachedEntity::updateRef(KnownVersion & refVersion) const
{
	refVersion.generationId = generationId_;
	for (unsigned i = 0; i < Mercury::MaxPropertySets; i++)
	{
		refVersion.versions[i] = propertySets_[i].lastVersion();
	}
}


CachedEntity::Ptr CachedEntityManager::getEntity(uint32_t entityId) const
{
	auto it = entities_.find(entityId);
	if (it == entities_.end())
	{
		return CachedEntity::Ptr();
	}

	return it->second.entity;
}

CachedEntity::Ptr CachedEntityManager::addEntity(uint32_t entityId, uint8_t classId, uint8_t flags)
{
	auto it = entities_.find(entityId);
	if (it != entities_.end() && it->second.entity)
	{
		WARN("Failed to assign entity ID %d: already cached", entityId);
		return CachedEntity::Ptr();
	}

	if (it == entities_.end())
	{
		EntityInfo info;
		info.entity.reset(new CachedEntity(entityId, classId, 0, flags));
		info.generationId = 0;
		entities_.insert(std::make_pair(entityId, info));
		return info.entity;
	} else {
		it->second.generationId++;
		it->second.entity.reset(new CachedEntity(entityId, classId, it->second.generationId, flags));
		return it->second.entity;
	}
}

void CachedEntityManager::deleteEntity(uint32_t entityId)
{
	auto it = entities_.find(entityId);
	if (it == entities_.end() || !it->second.entity)
	{
		WARN("Failed to delete nonexistent entity ID %d", entityId);
		return;
	}

	it->second.entity->onDelete();
	entities_.erase(it);
}

void CachedEntityManager::deleteCellEntities(uint16_t cellId)
{
	for (auto it = entities_.begin(); it != entities_.end();)
	{
		if (Mercury::IsCellEntity(it->first, cellId) || it->second.entity->isOnCell(cellId))
		{
			auto delIt = it++;
			if (delIt->second.entity)
			{
				delIt->second.entity->onDelete();
			}
			entities_.erase(delIt);
		}
		else
		{
			++it;
		}
	}
}
