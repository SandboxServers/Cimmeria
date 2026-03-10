#pragma once

#include <util/singleton.hpp>
#include <mercury/timer.hpp>
#include <common/database.hpp>
#include <boost/asio/steady_timer.hpp>
#include <random>
#include <soci/soci.h>

class MessageWriter;

class Service : public singleton<Service>
{
	public:
		enum EndpointType
		{
			BaseMailbox,
			BaseExposedMailbox,
			CellMailbox,
			CellExposedMailbox,
			ClientMailbox
		};

		typedef std::mt19937 RandomNumberGenerator;
		typedef Mercury::TimerMgr<uint64_t> TimerMgr;
		// How often should we tick the master timer (in milliseconds)
		static const unsigned int TickInterval = 10;

		Service();
		virtual ~Service();

		void waitForTermination();
		void start();
		void stop();

		inline boost::asio::io_context & ioService()
		{
			return service_;
		}

		inline const std::string & serviceName() const
		{
			return serviceName_;
		}

		inline uint16_t servicePort() const
		{
			return servicePort_;
		}

		inline RandomNumberGenerator & rng()
		{
			return rng_;
		}

		const std::string & getConfigParameter(const std::string & parameter) const;
		soci::session & db();
		Database & dbMgr();

		/*
		 * Registers a new timer. When the timer expires, the handler callback function is called.
		 *
		 * @param handler  Handler to call when the timer expires
		 * @param time     Expiration time in milliseconds (current time is Service::microTime())
		 * @param userData Optional user pointer passed to the handler function
		 */
		TimerMgr::TimerId addTimer(TimerMgr::TimerHandler handler, uint64_t time, void * userData = nullptr);

		/*
		 * Cancels an active timer by its ID.
		 * WARNING: Only use this method to cancel timers that are known to be active.
		 *
		 * @param timer Timer ID to cancel
		 */
		void cancelTimer(TimerMgr::TimerId timer);

		/*
		 * Returns the current server time in milliseconds
		 */
		uint64_t microTime() const;

		virtual MessageWriter * messageWriter(EndpointType endpoint, uint32_t entityId) = 0;

	protected:
		virtual void initialize() = 0;
		virtual void cleanup() = 0;
		virtual std::string internalServiceName() = 0;
		virtual uint16_t internalPort();

		std::string serviceName_;
		uint16_t servicePort_;

		// Configuration file version expected by the server
		std::string configVersion_;

		/*
		 * Exposes all configuration variables from the config .xml file via the Atrea.config module.
		 */
		void exposeConfiguration();

	private:
		boost::asio::io_context service_;
		boost::asio::executor_work_guard<boost::asio::io_context::executor_type> work_;
		std::map<std::string, std::string> configOptions_;
		Database dbMgr_;
		// Manages all application timers
		TimerMgr timers_;
		// ASIO timer that ticks all registered timers
		boost::asio::steady_timer timer_;
		// When was the last timer tick?
		uint64_t lastTick_;
		// Shared RNG
		RandomNumberGenerator rng_;

		static void exitHandler();

		void loadConfiguration();

		/*
		 * Called when the asio timer expired
		 */
		void tick(const boost::system::error_code & error);
};

enum DistributionFlags
{
	DISTRIBUTE_CLIENT = 0x01,
	DISTRIBUTE_LOD = 0x02,
	DISTRIBUTE_SPACE = 0x04,
	DISTRIBUTE_RECIPIENT = 0x08
};

class MessageWriter
{
public:
	struct DistributionPolicy
	{
		inline DistributionPolicy()
			: flags(0), recipientId(0), spaceId(0)
		{}

		// DistributionFlags
		uint8_t flags;
		uint32_t recipientId;
		uint32_t spaceId;
	};

	virtual ~MessageWriter();
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, bp::object args) = 0;
	virtual void write(Service::EndpointType endpoint, DistributionPolicy policy, uint32_t entityId, uint8_t messageId, void * args, size_t argsSize) = 0;
};
