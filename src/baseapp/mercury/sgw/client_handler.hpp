#pragma once
#include <mercury/sgw/entity_message_handler.hpp>
#include <baseapp/entity/base_entity.hpp>

namespace Mercury
{
namespace SGW
{

class ClientHandler : public ::SGW::EntityMessageHandler, public Controller
{
	public:
		static void initialize();

		ClientHandler(uint32_t mailbox_id, std::string const & accountName, uint32_t accessLevel);
		virtual ~ClientHandler();
		
		// Handle normal messages (x < 0x80)
		virtual void onBaseMessage(Mercury::Message::Ptr msg) override;
		// Handle entity messages with entityId (0x80 <= x < 0xC0)
		virtual void onEntityMessage(uint8_t rpcMessageId, uint32_t entityId, Mercury::Message::Ptr msg) override;
		// Handle entity messages with entityId (0xC0 <= x)
		virtual void onBaseEntityMessage(uint8_t rpcMessageId, Mercury::Message::Ptr msg) override;

		virtual void onConnected() override;
		virtual void onDisconnected() override;
		virtual void tick(uint64_t time) override;
		virtual Message::Table const & sendMessages() override;
		virtual Message::Table const & receiveMessages() override;

		virtual void sendResource(uint32_t categoryId, uint32_t elementId) override;
		virtual void createCellPlayer(uint32_t entityId, uint32_t spaceId,
			float x, float y, float z, float rotX, float rotY, float rotZ) override;
		virtual void switchEntity(BaseEntity * entity) override;
		virtual void disconnectEntity(bool killConnection) override;
		virtual void gameTick(uint64_t time) override;
		virtual void createEntity(uint32_t entityId, uint8_t classId, Vec3 & position, Vec3 & rotation) override;
		virtual void enterAoI(uint32_t entityId) override;
		virtual void leaveAoI(uint32_t entityId, bool deleteEntity) override;
		virtual void moveEntity(uint32_t entityId, Vec3 const & position, Vec3 const & rotation, Vec3 const & velocity, uint8_t flags) override;
		virtual void forcedPosition(uint32_t entityId, uint32_t spaceId, Vec3 const & position, Vec3 const & rotation, Vec3 const & velocity, uint8_t flags) override;
		void sendBundle(Bundle::Ptr bundle);
		void flush();

		void enableEntities();

		virtual void forwardMessage(uint32_t entityId, uint8_t messageId, bp::object args) override;
		virtual void forwardMessage(uint32_t entityId, uint8_t messageId, void * buffer, std::size_t size) override;

	private:
		void packXYZ(Mercury::Bundle & bundle, Vec3 const & v);
		bool canSendResource();

		const static uint32_t ResourceTxQueueSize = 128;

		struct ResourceId
		{
			uint32_t categoryId, elementId;
		};

		// ID of next proxied resource sent to the client
		uint16_t proxyDataId_;
		BaseEntity::Ptr entity_;
		bool entitySystemEnabled_;
		bool connectionSetup_;
		// Account ID of connected client
		uint32_t accountId_;
		// Account name of connected client
		std::string accountName_;
		// Access level of account
		uint32_t accessLevel_;

		// List of resources that are queued because the client send buffer is full
		std::vector<ResourceId> queuedResources_;

		static bool unreliableTickSync_;
		static bool unreliableMovement_;
};

}
}
