#pragma once

#include <mercury/unified_connection.hpp>
#include <mercury/sgw/entity_message_handler.hpp>
#include <cellapp/entity/cell_entity.hpp>
#include <common/service.hpp>

class BaseAppClient : public Mercury::UnifiedConnection
{
	public:
		// Sleep time after an operation failed (connection/registration failed)
		const static uint32_t ConnectionRecoveryTimeout = 30000;
		typedef boost::shared_ptr<BaseAppClient> Ptr;

		BaseAppClient();
		~BaseAppClient();

		
		/**
		 * Returns whether the client is ready to receive requests.
		 */
		inline bool isReady() const
		{
			return registered_;
		}

		void startup();

		/**
		 * Updates space availability information about a space on the BaseApp.
		 *
		 * @param spaceId   Space to update
		 * @param cellId    Cell instance ID running this space (0xffffffff = space is destroyed)
		 * @param worldName World name of this space
		 */
		void sendSpaceData(uint32_t spaceId, uint32_t cellId, std::string const & worldName);

		/**
		 * Notifies the BaseApp that the cell part of an entity was created.
		 *
		 * @param entityId Created entity ID
		 * @param spaceId  Space instance ID (0xffffffff = entity create failed)
		 */
		void sendEntityCreateAck(uint32_t entityId, uint32_t spaceId);

		/**
		 * Notifies the BaseApp that the cell part of an entity was destroyed.
		 *
		 * @param entityId Destroyed entity ID
		 */
		void sendEntityDeleteAck(uint32_t entityId);

		/**
		 * Notifies the BaseApp that the cell part of an entity was created.
		 *
		 * @param entityId  Player entity ID
		 * @param spaceId   ID of space the player entered
		 * @param position  Position of player
		 * @param rotation  Rotation of player
		 */
		void sendCellPlayerCreated(uint32_t entityId, uint32_t spaceId,
			Vec3 const & position, Vec3 const & rotation);

		/**
		 * Notifies the BaseApp about current game time on the cell.
		 *
		 * @param time Current game time
		 */
		void sendGameTick(uint64_t time);

		/**
		 * Notifies a base entity that another entity entered its AoI (area of interest).
		 *
		 * @param spaceId   Space to update
		 * @param entity    Entering entity
		 * @param witnessId Entity ID of witness
		 */
		void sendEnteredAoI(uint32_t spaceId, CellEntity * entity, uint32_t witnessId);

		/**
		 * Notifies a base entity that another entity left its AoI (area of interest).
		 *
		 * @param spaceId      Space to update
		 * @param entityId     Entity leaving the AoI
		 * @param deleteEntity Was the entity deleted or just made invisible?
		 */
		void sendLeftAoI(uint32_t spaceId, uint32_t entityId, bool deleteEntity);

		/**
		 * Notifies clients that an entity has moved / is moving.
		 *
		 * @param spaceId      Space to update
		 * @param entity       Entering entity
		 * @param deleteEntity Was the entity deleted or just made invisible?
		 */
		void sendEntityMove(CellEntity * entity, bool forced = false);

		/**
		 * Creates a backup of the cell state of the entity and sends it to the BaseApp.
		 *
		 * @param entity Entity to back up
		 */
		void sendEntityBackup(CellEntity * entity);

		/**
		 * Notifies the BaseApp that the restore of a cell entity was successful.
		 *
		 * @param entityId Entity that was restored
		 */
		void sendEntityRestoreAck(uint32_t entityId);

		/**
		 * Requests the BaseApp to start moving an entity to a different space.
		 *
		 * @param entityId  Entity to move
		 * @param spaceId   Destination space ID
		 * @param worldName Destination world name
		 * @param position  Destination position
		 * @param rotation  Destination orientation
		 */
		void sendSwitchSpace(uint32_t entityId, uint32_t spaceId, std::string const & worldName, 
			Vec3 const & position, Vec3 const & rotation);

		/**
		 * Calls an RPC method on a base entity.
		 *
		 * @param entityId  Entity to call this RPC method on
		 * @param messageId Message type to send
		 * @param method    RPC arguments serializer
		 * @param args      Arguments to send
		 */
		void sendBaseAppMessage(uint32_t entityId, uint8_t messageId, PyMethod & method, bp::object args);

		/**
		 * Calls an RPC method on a client entity.
		 *
		 * @param entityId     Entity to call this RPC method on
		 * @param messageId    Message type to send
		 * @param distribution Message distribution policy
		 * @param method       RPC arguments serializer
		 * @param args         Arguments to send
		 */
		void sendClientMessage(uint32_t entityId, uint8_t messageId, 
			MessageWriter::DistributionPolicy distribution, PyMethod & method, bp::object args);

		/**
		 * Notifies the BaseApp that a local entity was created on the cell.
		 *
		 * @param entityId  ID of new entity
		 * @param spaceId   Space the entity is bound to
		 * @param classId   Entity type ID
		 * @param position  Position of the entity
		 * @param flags     Entity flags (Mercury.CellEntityFlags)
		 */
		void sendCellEntityCreated(uint32_t entityId, uint32_t spaceId, uint8_t classId, 
			Vec3 const & position, uint8_t flags);

		/**
		 * Notifies the BaseApp that a local entity was destroyed on the cell.
		 *
		 * @param entityId  ID of entity to delete
		 */
		void sendCellEntityDeleted(uint32_t entityId);

		/**
		 * Begin sending a cache stamp update to the BaseApp.
		 * Messages in the update must be added using sendClientMessage().
		 * To finish the update, call endCacheStamp().
		 * 
		 * @param entityId      ID of entity to update
		 * @param propertySetId ID of the property set to update (0-based index)
		 * @param invalidate    Invalidate currently cached updates in this set?
		 */
		void beginCacheStamp(uint32_t entityId, uint32_t propertySetId, bool invalidate);

		/**
		 * Finish sending a cache stamp update to the BaseApp.
		 */
		void endCacheStamp();

	protected:
		// Overridden handlers of unified connection

		/**
		 * Called when a message was received from the peer. The stream contains
		 * message contents excluding the header and message ID.
		 *
		 * @param msg Message stream reader
		 */
		virtual void onMessageReceived(Reader & msg);

		/**
		 * Callback function called when the connection request was completed.
		 *
		 * @param errcode Error code; connection was successful if code == errc::success
		 */
		virtual void onConnected(const boost::system::error_code & errcode);

		/**
		 * Callback function called when the connection to the server was closed.
		 *
		 * @param errcode Reason why the connection was lost
		 */
		virtual void onDisconnected(const boost::system::error_code & errcode);
		virtual void unregisterConnection();

	private:
		boost::asio::deadline_timer recoveryTimer_;
		bool registered_;
		uint32_t cacheEntityId_;
		Writer * cacheWriter_;

		void reconnectIn(uint32_t msecs);
		void reconnectTimerExpired(const boost::system::error_code & errcode);
		void reconnect();

		/**
		 * Called when an acknowledgement is received to the authentication request
		 */
		void onRegistrationAck(Reader & msg);

		/**
		 * Called when the BaseApp requested the cell to create a cell part for a base entity
		 */
		void onCreateEntityRequest(Reader & msg);

		/**
		 * Called when the BaseApp requested the cell to destroy the cell part of an entity
		 */
		void onDestroyEntityRequest(Reader & msg);

		/**
		 * Called when a client controller is attached to an entity
		 * (the player is taking control of that entity)
		 */
		void onConnectEntityRequest(Reader & msg);

		/**
		 * Called when a client controller is detached from an entity
		 * (the player is releasing control of that entity)
		 */
		void onDisconnectEntityRequest(Reader & msg);

		/**
		 * Called when the entity moved on the client side
		 */
		void onEntityMove(Reader & msg);

		/**
		 * Restores an entity from its BaseApp backup
		 */
		void onRestoreEntityRequest(Reader & msg);

		/**
		 * Called when an entity message was received from the BaseApp
		 */
		void onBaseAppMessage(Reader & msg);

		/**
		 * Called when an entity message was received from the client
		 */
		void onClientMessage(Reader & msg);

		/**
		 * Called when the CellApp needs to send property updates for an entity
		 */
		void onRequestEntityUpdate(Reader & msg);

		/**
		 * Sends a registration request to the BaseApp to register this cell
		 */
		void sendAuthenticationRequest();

		/**
		 * Returns a BaseAppClient shared ptr from enable_shared_from_this<>
		 */
		inline Ptr shared_this()
		{
			return boost::static_pointer_cast<BaseAppClient>(shared_from_this());
		}

		CellEntity::Ptr createEntity(uint32_t entityId, uint32_t dbid, uint32_t spaceId, 
			std::string const & className, std::string const & worldName,
			Vec3 const & position, Vec3 const & rotation);

		Space * findOrCreateSpace(uint32_t spaceId, std::string const & worldName, uint32_t creatorId);
};

