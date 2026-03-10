#pragma once

#include <mercury/unified_connection.hpp>
#include <mercury/tcp_server.hpp>
#include <baseapp/entity/base_entity.hpp>

namespace Mercury
{

class CellAppConnection : public Mercury::UnifiedConnection
{
	public:
		typedef std::shared_ptr<CellAppConnection> Ptr;

		static CellAppConnection * Create(Mercury::TcpServer<CellAppConnection> & server, uint32_t connectionId);

		CellAppConnection();
		~CellAppConnection();

		/**
		 * Returns the instance ID of the CellApp this link is connected to.
		 */
		uint32_t cellId() const;

		
		/**
		 * Returns whether the CellApp is ready to receive requests.
		 */
		inline bool isReady() const
		{
			return registered_;
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
		void sendCreateEntity(BaseEntity * entity, uint32_t spaceId, std::string const & worldName, Vec3 & position, Vec3 & rotation);

		/**
		 * Requests the CellApp to destroy the cell part of an entity
		 *
		 * @param entityId Entity ID to destroy
		 */
		void sendDestroyEntity(uint32_t entityId);
		
		/**
		 * Requests the CellApp to attach a client controller to an entity
		 *
		 * @param entityId Entity ID to attach
		 */
		void sendConnectEntity(uint32_t entityId);
		
		/**
		 * Requests the CellApp to detach a client controller from an entity
		 *
		 * @param entityId Entity ID to detach
		 */
		void sendDisconnectEntity(uint32_t entityId);

		/**
		 * Notifies the CellApp that the client has moved its controlled entity
		 *
		 * @param entityId Controlled entity ID
		 * @param position Current position of entity
		 * @param rotation Current rotation of entity
		 * @param velocity Current velocity of entity
		 * @param flags    Movement modifier flags (is jumping, running, etc.)
		 */
		void sendEntityMove(uint32_t entityId, Vec3 & position, Vec3 & rotation, Vec3 & velocity, uint8_t flags);

		/**
		 * Restores an entity from its BaseApp backup
		 *
		 * @param entity    Entity to restore
		 * @param spaceId   Space ID to restore to
		 * @param recovery  Is the restore because of a recovery or a space switch? 
		 * @param worldName World name to restore to (if space ID is not specified)
		 * @param position  Position of entity on space
		 * @param rotation  Rotation of entity on space
		 */
		void sendRestoreEntity(BaseEntity * entity, uint32_t spaceId, bool recovery,
			std::string const & worldName, Vec3 & position, Vec3 & rotation);

		/**
		 * Calls an internal RPC method on a cell entity.
		 *
		 * @param entityId  Entity to call this RPC method on
		 * @param messageId Message type to send
		 * @param method    RPC arguments serializer
		 * @param args      Tuple of arguments to send
		 */
		void sendInternalMessage(uint32_t entityId, uint8_t messageId, PyMethod * method, bp::object args);

		/**
		 * Calls an internal RPC method on a cell entity.
		 *
		 * @param entityId  Entity to call this RPC method on
		 * @param messageId Message type to send
		 * @param args      Serialized arguments to send
		 * @param argsSize  Size of arguments in bytes
		 */
		void sendInternalMessage(uint32_t entityId, uint8_t messageId, void * args, std::size_t argsSize);

		/**
		 * Calls an exposed RPC method on a cell entity.
		 *
		 * @param entityId  Entity to call this RPC method on
		 * @param messageId Message type to send
		 * @param method    RPC arguments serializer
		 * @param args      Tuple of arguments to send
		 */
		void sendExposedMessage(uint32_t entityId, uint8_t messageId, PyMethod * method, bp::object args);

		/**
		 * Calls an exposed RPC method on a cell entity.
		 *
		 * @param entityId  Entity to call this RPC method on
		 * @param messageId Message type to send
		 * @param args      Serialized arguments to send
		 * @param argsSize  Size of arguments in bytes
		 */
		void sendExposedMessage(uint32_t entityId, uint8_t messageId, void * args, std::size_t argsSize);

		/**
		 * Requests the CellApp to update the status of an entity on the client.
		 *
		 * @param witnessId Client requesting the update
		 * @param entityId  Entity to update
		 * @param flags     Requested update types
		 */
		void sendRequestEntityUpdate(uint32_t witnessId, uint32_t entityId, uint8_t flags);

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
		bool registered_;
		uint32_t cellId_;

		/**
		 * Notifies the BaseApp that the cell part of an entity was created.
		 */
		void onEntityCreateAck(Reader & msg);

		/**
		 * Notifies the BaseApp that the cell part of an entity was destroyed.
		 */
		void onEntityDeleteAck(Reader & msg);
		
		/**
		 * Updates space availability information about a space on the BaseApp.
		 */
		void onSpaceData(Reader & msg);

		/**
		 * Notifies the BaseApp that the cell part of a player was created.
		 */
		void onCellPlayerCreateAck(Reader & msg);

		/**
		 * Notifies the BaseApp that the cell part of a client controlled entity was disconnected.
		 */
		void onEntityDisconnectAck(Reader & msg);

		/**
		 * Called when the Cell updated its game timer.
		 */
		void onGameTick(Reader & msg);

		/**
		 * Notifies a base entity that another entity entered its AoI (area of interest).
		 * TODO: DEPRECATE
		 */
		void onEnteredAoI(Reader & msg);

		/**
		 * Notifies a base entity that another entity left its AoI (area of interest).
		 * TODO: DEPRECATE
		 */
		void onLeftAoI(Reader & msg);

		/**
		 * Notifies clients that a Cell entity has moved / is moving.
		 */
		void onEntityMove(Reader & msg);

		/**
		 * Creates a backup of the cell state of the entity and sends it to the BaseApp.
		 */
		void onEntityBackup(Reader & msg);

		/**
		 * Notifies the BaseApp that the restore of a cell entity was successful.
		 */
		void onEntityRestoreAck(Reader & msg);

		/**
		 * Requests the BaseApp to start moving an entity to a different space.
		 */
		void onSwitchSpace(Reader & msg);

		/**
		 * Calls an RPC method on a base entity.
		 */
		void onBaseAppMessage(Reader & msg);

		/**
		 * Calls an RPC method on a client entity.
		 */
		void onClientMessage(Reader & msg);

		/**
		 * Called when a cell-only entity was created on the CellApp
		 */
		void onCellEntityCreated(Reader & msg);
		
		/**
		 * Called when a cell-only entity was deleted on the CellApp
		 */
		void onCellEntityDeleted(Reader & msg);
		
		/**
		 * Called when the cell has updated the cached messages of an entity
		 */
		void onUpdateCacheStamp(Reader & msg);

		/**
		 * Called when an authentication/registration request was received from the CellApp
		 */
		void onAuthenticationRequest(Reader & msg);

		/**
		 * Sends an acknowledgement to the CellApps authentication request
		 */
		void sendRegistrationAck(uint32_t resultCode);

		/**
		 * Returns a BaseAppClient shared ptr from enable_shared_from_this<>
		 */
		inline Ptr shared_this()
		{
			return std::static_pointer_cast<CellAppConnection>(shared_from_this());
		}
};

}
