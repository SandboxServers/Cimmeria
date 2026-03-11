#pragma once

#include <entity/mailbox.hpp>
#include <entity/entity.hpp>
#include <baseapp/entity/base_entity_manager.hpp>
#include <common/vec3.hpp>
#include <memory>

class Controller
{
public:
	virtual void sendResource(uint32_t categoryId, uint32_t elementId) = 0;
	virtual void createCellPlayer(uint32_t entityId, uint32_t spaceId,
		float x, float y, float z, float yaw, float pitch, float roll) = 0;
	virtual void switchEntity(class BaseEntity * entity) = 0;
	virtual void forwardMessage(uint32_t entityId, uint8_t messageId, bp::object args) = 0;
	virtual void forwardMessage(uint32_t entityId, uint8_t messageId, void * buffer, std::size_t size) = 0;
	virtual void disconnectEntity(bool killConnection) = 0;
	virtual void gameTick(uint64_t time) = 0;
	virtual void createEntity(uint32_t entityId, uint8_t classId, Vec3 & position, Vec3 & rotation) = 0;
	virtual void leaveAoI(uint32_t entityId, bool deleteEntity) = 0;
	virtual void enterAoI(uint32_t entityId) = 0;
	virtual void moveEntity(uint32_t entityId, Vec3 const & position, Vec3 const & rotation, Vec3 const & velocity, uint8_t flags) = 0;
	virtual void forcedPosition(uint32_t entityId, uint32_t spaceId, Vec3 const & position, Vec3 const & rotation, Vec3 const & velocity, uint8_t flags) = 0;
};

class BaseEntity : public Entity, public std::enable_shared_from_this<BaseEntity>
{
public:
	typedef std::shared_ptr<BaseEntity> Ptr;
	typedef std::weak_ptr<BaseEntity> WeakPtr;

	static void registerClass();

	BaseEntity();
	~BaseEntity();


	/*
	 * Loads Cell entity backup information from the stream
	 *
	 * @param stream Load backup from this stream
	 */
	template <typename _T>
	void unpackBackup(_T & stream)
	{
		delete [] backup_;
		delete [] pythonBackup_;

		stream >> backupSize_;
		backup_ = new uint8_t[backupSize_];
		stream.read(backup_, backupSize_);

		stream >> pythonBackupSize_;
		pythonBackup_ = new uint8_t[pythonBackupSize_];
		stream.read(pythonBackup_, pythonBackupSize_);
	}


	/*
	 * Writes Cell entity backup information to the stream.
	 *
	 * @param stream Save backup to this stream
	 */
	template <typename _T>
	void packBackup(_T & stream)
	{
		SGW_ASSERT(hasBackup() && "Cell part of the entity is not backed up");
		// stream << backupSize_;
		stream.write(backup_, backupSize_);
		stream << pythonBackupSize_;
		stream.write(pythonBackup_, pythonBackupSize_);
	}

	bool hasBackup();

	void setupClient(Mercury::BaseChannel::Ptr channel);
	void attachedToController(Controller * controller);
	void detachedFromController();
	void destroyCellPart(bool removeLocally);

	void cellEntityCreated(uint32_t spaceId);
	void cellEntityCreateFailed();
	void cellEntityDestroyed();
	void cellEntityConnected();
	void cellEntityDisconnected();
	void createCellPlayer(uint32_t spaceId, float x, float y, float z, 
		float yaw, float pitch, float roll);

	void cookedDataError(uint32_t categoryId, uint32_t elementId);

	Mailbox::Ptr client() const;
	Mailbox::Ptr cell() const;
	Controller * controller() const;
	uint32_t spaceId() const;

	void disconnect(bool killConnection);

private:
	bp::object pyClient() const;
	bp::object pyCell() const;

	void updateSpace(uint32_t spaceId);
	virtual void removeFromManager();

	Mailbox::Ptr client_;
	Mailbox::Ptr cell_;
	// Mercury channel of the attached client
	Mercury::BaseChannel::Ptr clientChannel_;
	// Controller attached to this entity
	Controller * controller_;

	uint32_t spaceId_;
	// Last known location of this entity
	// Since position is tracked by the CellApp, this is only an approximation 
	// and periodically is updated by the CellApp via movement messages
	Vec3 lastPosition_;

	uint8_t * backup_;
	uint32_t backupSize_;
	uint8_t * pythonBackup_;
	uint32_t pythonBackupSize_;
};
