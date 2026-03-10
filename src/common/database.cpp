#include <stdafx.hpp>
#include <common/service.hpp>
#include <common/database.hpp>
#include <soci/postgresql/soci-postgresql.h>

Database::Database()
	: work_(boost::asio::make_work_guard(service_)),
	  thread_([this] { service_.run(); }),
	  session_(nullptr)
{
}

Database::~Database()
{
	service_.stop();
	thread_.join();
}

void Database::initialize()
{
	sendKeepalive();
}

void Database::fetchRow(DbExecFunction execf, DbRowFunction rowf, DbErrorFunction failf)
{
	DbRequest req;
	req.executor_function = execf;
	req.row_function = rowf;
	req.failure_function = failf;
	queueLock_.lock();
	queue_.push(req);
	queueLock_.unlock();

	boost::asio::post(service_, [this] { performRequest(); });
}

void Database::synchronousPerform(std::string const & query)
{
	try
	{
		session().once << query;
	}
	catch (soci::soci_error & e)
	{
		FAULT("Database error: %s", e.what());
		delete session_;
		session_ = nullptr;
	}
}

void Database::asyncPerform(std::string const & query)
{
	boost::asio::post(service_, [this, query] { doQuery(query); });
}

void Database::synchronousQuery(std::string const & query, bp::list & list)
{
	try
	{
		soci::rowset<soci::row> rs = session().prepare << query;

		for (auto iter = rs.begin(); iter != rs.end(); ++iter)
		{
			soci::row const & r = *iter;
			// An empty row indicates that the query doesn't have a resultset
			// (e.g. INSERT, UPDATE, ...)
			if (r.size() == 0)
				return;
			bp::dict d;

			for (uint32_t i = 0; i < r.size(); i++)
			{
				std::string const & col_name = r.get_properties(i).get_name();
				if (r.get_indicator(i) == soci::i_null)
				{
					d[col_name] = bp::object();
				}
				else
				{
					switch (r.get_properties(i).get_data_type())
					{
						case soci::dt_string:
						case soci::dt_date:
							d[col_name] = r.get<std::string>(i);
							break;

						case soci::dt_integer:
							d[col_name] = r.get<int>(i);
							break;

						case soci::dt_double:
							d[col_name] = r.get<double>(i);
							break;

						case soci::dt_long_long:
							d[col_name] = r.get<long long>(i);
							break;

						case soci::dt_unsigned_long_long:
							d[col_name] = r.get<unsigned long>(i);
							break;

						default:
							throw std::runtime_error("Unknown soci column type!");
					}
				}
			}

			list.append(d);
		}
	}
	catch (soci::soci_error & e)
	{
		FAULT("Database error: %s", e.what());
		delete session_;
		session_ = nullptr;
	}
}

soci::session & Database::session()
{
	if (session_ == nullptr)
		session_ = new soci::session(soci::postgresql, Service::instance().getConfigParameter("db_connection_string"));

	return *session_;
}

void Database::performRequest()
{
	SGW_ASSERT(!queue_.empty());

	queueLock_.lock();
	DbRequest req = queue_.front();
	queue_.pop();
	queueLock_.unlock();

	try
	{
		if (session_ == nullptr)
			session_ = new soci::session(soci::postgresql, Service::instance().getConfigParameter("db_connection_string"));

		soci::row r;
		req.executor_function(*session_);
		if (session_->got_data())
			req.row_function();
		else
			req.failure_function(DATABASE_NO_SUCH_ROW);
	}
	catch (soci::soci_error & e)
	{
		FAULT("Database error: %s", e.what());
		delete session_;
		session_ = nullptr;
		req.failure_function(DATABASE_QUERY_FAILED);
	}
}


void Database::doQuery(std::string const query)
{
	try
	{
		session().once << query;
	}
	catch (soci::soci_error & e)
	{
		FAULT("Database error: %s", e.what());
		delete session_;
		session_ = nullptr;
	}
}


void Database::sendKeepalive()
{
	asyncPerform("select 1");
	uint64_t now = Service::instance().microTime();
	Service::instance().addTimer([this](auto &, auto, auto, auto *) { sendKeepalive(); },
		now + KeepaliveInterval);
}
