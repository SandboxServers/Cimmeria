#include <stdafx.hpp>
#include <authentication/logon_queue.hpp>
#include <authentication/service_main.hpp>
#include <soci/soci.h>
#include <soci/postgresql/soci-postgresql.h>

LogonQueue::LogonQueue()
{
}

LogonQueue::~LogonQueue()
{
}

void LogonQueue::queueRequest(Request const & request)
{
	QueueItem * item = new QueueItem();
	item->request = request;

	std::lock_guard<std::mutex> lock(lock_);
	queue_.push(QueueItem::Ptr(item));

	if (queue_.size() == 1)
		processQueueItem();
}

void LogonQueue::doLogonQuery(soci::session & s, QueueItem::Ptr req)
{
	s << "SELECT account_id, upper(password) as password, accesslevel, enabled FROM account WHERE account_name = :accname",
		soci::use(req->request.userName), soci::into(req->accountId), soci::into(req->dbPassword), 
		soci::into(req->accessLevel), soci::into(req->enabled);
}

void LogonQueue::processQueueItem()
{
	assert(!queue_.empty());

	QueueItem::Ptr item = queue_.front();
	if (item->request.serviceName != "SGW_BETA")
	{
		item->request.errorCb(InvalidService);
	}
	else if (item->request.password.length() != 40 || 
		item->request.password.find_first_not_of("0123456789ABCDEF") != std::string::npos)
	{
		item->request.errorCb(MalformedPassword);
	}
	else if (item->request.userName.length() < 3 || 
		item->request.userName.length() > 20 || 
		item->request.userName.find_first_not_of("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-_") != std::string::npos)
	{
		item->request.errorCb(MalformedUserId);
	}
	else
	{
		std::string acct_name = item->request.userName;
		// TRACE("Fetching data for user %s ...", req->user_name.c_str());

		AuthenticationService::as_instance().dbMgr().fetchRow(
			[item](soci::session & s) { doLogonQuery(s, item); },
			[item]()
				{
					if (item->request.password != item->dbPassword)
					{
						DEBUG1("Logon failed: Password mismatch for account '%s'", item->request.userName.c_str());
						item->request.errorCb(BadUserPassword);
					}
					else if (item->enabled != "t")
					{
						DEBUG1("Logon failed: Account '%s' is not enabled", item->request.userName.c_str());
						item->request.errorCb(AccountDisabled);
					}
					else
					{
						INFO("Logon successful for account '%s'", item->request.userName.c_str());
						item->request.successCb(item->accountId, std::ref(item->request.userName), item->accessLevel);
					}
				},
				[item](DbErrorType err_type)
				{
					if (err_type == DATABASE_NO_SUCH_ROW)
					{
						DEBUG1("Logon failed: No such account id: '%s'", item->request.userName.c_str());
						item->request.errorCb(BadUserPassword);
					}
					else
					{
						WARN("Logon failed: Database error");
						item->request.errorCb(DbRequestFailed);
					}
				});
	}

	queue_.pop();
}

