#pragma once

#include <soci/soci.h>

enum DbErrorType
{
	DATABASE_NO_SUCH_ROW = 1,
	DATABASE_QUERY_FAILED = 2
};

typedef std::function<void (soci::session & session)> DbExecFunction;
typedef std::function<void ()> DbRowFunction;
typedef std::function<void (DbErrorType type)> DbErrorFunction;

struct DbRequest
{
	DbExecFunction executor_function;
	DbRowFunction row_function;
	DbErrorFunction failure_function;
};

class Database
{
	public:
		// How often should we send a keepalive request to the DB server (in milliseconds)
		static const unsigned int KeepaliveInterval = 60000;

		Database();
		~Database();

		void initialize();
		soci::session & session();

		void fetchRow(DbExecFunction execf, DbRowFunction rowf, DbErrorFunction failf);
		void synchronousQuery(std::string const & query, bp::list & list);
		void synchronousPerform(std::string const & query);
		void asyncPerform(std::string const & query);

	private:
		boost::asio::io_service service_;
		boost::asio::io_service::work work_;
		boost::thread thread_;
		soci::session * session_;
		std::queue<DbRequest> queue_;
		std::mutex queueLock_;
		
		void performRequest();

		/*
		 * Executes an SQL query on the worker thread and discards the results.
		 *
		 * @param query SQL query to execute
		 */
		void doQuery(std::string const query);

		/*
		 * Sends a keepalive request to the database to make sure that the
		 * connection isn't closed when there is no DB activity
		 */
		void sendKeepalive();
};

