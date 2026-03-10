#include <stdafx.hpp>
#include <boost/lexical_cast.hpp>
#include <boost/tokenizer.hpp>
#include <random>
#include <common/service.hpp>
#include <mercury/tcp_server.hpp>
#include <authentication/logon_connection.hpp>
#include <authentication/service_main.hpp>

const char * LogonFailureMessages[LogonQueue::FailureMax + 1] = 
{
	"(No error)",
	"The specified account name is invalid.",
	"The specified password has is invalid.",
	"The specified service does not exist.",
	"The user name or password is invalid.",
	"Your account is disabled.",
	"Access is denied.",
	"No shards are available to the authentication server.",
	"No such shard.",
	"The requested shard is offline.",
	"A request to the database server failed.",
	"BaseApp timed out while waiting for a response to an outstanding logon request.",
	"Lost connection to BaseApp while waiting for a response to an outstanding logon request.",
	"Internal error.",
	"The BaseApp rejected your logon request.",
	"Your logon session has expired. Please log in again.",
	"No CellApps are available to the BaseApp.",
	"Protocol version mismatch; your client version is not supported by the server."
};

LogonConnection::LogonConnection(uint32_t connectionId)
	: socket_(Service::instance().ioService()), connectionId_(connectionId), requestPending_(false)
{
}

LogonConnection::~LogonConnection()
{
	TRACE("LogonConnection closed");
}

LogonConnection * LogonConnection::construct(Mercury::TcpServer<LogonConnection> & server, uint32_t connectionId)
{
	return new LogonConnection(connectionId);
}

void LogonConnection::connected()
{
	// TRACE("Connected");
	beginReceivingRequest();
}

boost::asio::ip::tcp::socket & LogonConnection::socket()
{
	return socket_;
}

uint32_t LogonConnection::connectionId() const
{
	return connectionId_;
}

void LogonConnection::close(const boost::system::error_code & errcode)
{
	if (socket_.is_open())
	{
		onDisconnected(errcode);
		socket_.close();
	}
	AuthenticationService::as_instance().loginServer().unregister_connection(shared_from_this());
}

void LogonConnection::sendReply(const std::string & body)
{
	requestPending_ = false;

	if (!response_.empty())
	{
		LOGF(LL_WARNING, "A logon response is already being sent!");
		return;
	}

	lock_.lock();
	response_ = 
		"HTTP/1.1 200 OK\r\n"
		"Content-Type: text/xml\r\n"
		"Content-Length: ";
	response_ += boost::lexical_cast<std::string>(body.length()) +"\r\n";
	response_ += std::string("Set-Cookie: SID=") + requestSession_ +"\r\n\r\n";
	response_ += body;
	sendResponse(0);
	lock_.unlock();
}

void LogonConnection::sendErrorReply()
{
	requestPending_ = false;

	if (!response_.empty())
	{
		LOGF(LL_WARNING, "A logon response is already being sent!");
		return;
	}

	lock_.lock();
	response_ = 
		"HTTP/1.1 400 Bad Request\r\n"
		"Content-Type: text/html\r\n"
		"Content-Length: 11\r\n\r\n"
		"Bad Request";
	sendResponse(0);
	lock_.unlock();
}


void LogonConnection::beginReceivingRequest()
{
	receiveOffset_ = 0;
	receiveLength_ = MaxRequestLength;
	headerLength_ = 0;
	receivingHeader_ = true;
	requestUri_ = "";
	receive();
}

void LogonConnection::receive()
{
	assert(receiveOffset_ < MaxRequestLength);

	auto self = shared_from_this();
	socket_.async_receive(
		boost::asio::buffer(&request_[receiveOffset_], MaxRequestLength - receiveOffset_),
		[self](const boost::system::error_code & err, size_t len) {
			self->onReceived(err, len);
		}
	);
}

size_t LogonConnection::headerEndOffset()
{
	for (int i = 0; i < ((receiveOffset_ > 4) ? (receiveOffset_ - 3) : 0); i++)
	{
		if (request_[i] == '\r' && request_[i + 1] == '\n' && request_[i + 2] == '\r' && request_[i + 3] == '\n')
			return i + 4;
	}

	return 0;
}

void LogonConnection::onReceived(const boost::system::error_code & errcode, size_t receivedLength)
{
	if (errcode != boost::system::errc::success)
	{
		if (errcode != boost::asio::error::eof || receiveOffset_ != 0)
			LOGF(LL_WARNING, "Receive failed: %s", errcode.message().c_str());
		close(errcode);
		return;
	}
	
	receiveOffset_ += receivedLength;
	SGW_ASSERT(receiveOffset_ <= request_.size());
	if (receiveOffset_ == request_.size() || receiveOffset_ > receiveLength_)
	{
		LOGF(LL_WARNING, "Received message too long (%d bytes)", receiveOffset_);
		close(errcode);
		return;
	}

	if (requestPending_)
	{
		LOGF(LL_WARNING, "Received data on socket when a request was already in progress");
		close(errcode);
		return;
	}

	if (receivingHeader_)
	{
		size_t end = headerEndOffset();
		if (end != 0)
		{
			std::string header;
			header.resize(end);
			memcpy(const_cast<char *>(header.data()), &request_[0], end);
			processHeader(header);
		}
	}

	if (!receivingHeader_ && receiveOffset_ == receiveLength_)
	{
		std::string body;
		body.resize(receiveLength_ - headerLength_);
		memcpy(const_cast<char *>(body.data()), &request_[headerLength_], receiveLength_ - headerLength_);
		processBody(body);
		beginReceivingRequest();
	}
	else
	{
		receive();
	}
}

void LogonConnection::processHeader(std::string const & header)
{
	try
	{
		typedef boost::tokenizer<boost::char_separator<char> > tokenizer;

		int bodyLength = -1;
		bool first = true;
		boost::char_separator<char> sep("\r\n");
		tokenizer tok(header, sep);
		for (auto iter = tok.begin(); iter != tok.end(); ++iter)
		{
			if (first)
			{
				// Parse the first HTTP header
				// "POST /url HTTP/1.1"
				boost::char_separator<char> arg_sep(" ");
				tokenizer arg_tok(*iter, arg_sep);
				auto arg_iter = arg_tok.begin();
				if (*arg_iter != "POST")
				{
					std::stringstream errmsg;
					errmsg << "Expected HTTP method POST, got " << *arg_iter;
					throw std::runtime_error(errmsg.str().c_str());
				}

				++arg_iter;
				if (arg_iter == arg_tok.end())
				{
					throw std::runtime_error("Missing URI from HTTP request header");
				}

				requestUri_ = *arg_iter;
				first = false;
			}
			else
			{
				// Parse subsequent HTTP headers
				// "Header-Name: Value"
				boost::char_separator<char> param_sep(": ");
				tokenizer param_tok(*iter, param_sep);
				auto param_iter = param_tok.begin();
				if (*param_iter == "Content-Length")
				{
					++param_iter;
					if (param_iter != param_tok.end())
					{
						bodyLength = boost::lexical_cast<int>(*param_iter);
					}
				}
				if (*param_iter == "Cookie")
				{
					++param_iter;
					if (param_iter != param_tok.end() && param_iter->length() > 4 && param_iter->substr(0, 4) == "SID=")
					{
						requestSession_ = param_iter->substr(4);
					}
				}
			}
		}

		if (requestUri_ == "")
		{
			throw std::runtime_error("HTTP request contains no URI");
		}

		if (bodyLength < 1)
		{
			std::stringstream errmsg;
			errmsg << "Illegal Content-Length in POST request: " << bodyLength;
			throw std::runtime_error(errmsg.str().c_str());
		}

		if (header.size() + bodyLength >= MaxRequestLength)
		{
			std::stringstream errmsg;
			errmsg << "Received too long POST request; client tried to send " << (header.size() + receiveLength_) << " bytes, maximum is " << MaxRequestLength;
			throw std::runtime_error(errmsg.str().c_str());
		}
		
		receiveLength_ = header.size() + bodyLength;
		receivingHeader_ = false;
		headerLength_ = header.size();
		// receiveBody(receiveOffset_ - end);
	}
	catch (std::runtime_error & e)
	{
		LOGF(LL_WARNING, "Failed to process header: %s", e.what());
		sendErrorReply();
		close(boost::system::error_code());
	}
}


void LogonConnection::processBody(std::string const & body)
{
	requestPending_ = true;

	if (requestUri_ == "/SGWLogin/UserAuth")
	{
		onUserAuthentication(body);
	}
	else if (requestUri_ == "/SGWLogin/ServerSelection")
	{
		onServerSelection(body);
	}
	else
	{
		WARN("Unknown logon URI: %s", requestUri_.c_str());
		sendErrorReply();
	}

	requestSession_ = "";
}

void LogonConnection::onDisconnected(const boost::system::error_code & errcode)
{
	// TRACE("Disconnected: %s", errcode.message().c_str());
}

void LogonConnection::onUserAuthentication(const std::string & request)
{
	// LOGF(DEBUG2, "Received logon request");
	tinyxml2::XMLDocument doc;
	doc.Parse(request.c_str());
	if (doc.Error())
	{
		WARN("Received malformed request: %d", doc.ErrorID());
		sendErrorReply();
		return;
	}

	tinyxml2::XMLElement * root = doc.FirstChildElement("sgwLogin:SGWLoginRequest");
	if (root == nullptr)
	{
		WARN("Invalid root element in logon request!");
		sendErrorReply();
		return;
	}

	const char * sku = root->Attribute("SKU");
	const char * accountName = root->Attribute("AccountName");
	const char * password = root->Attribute("Password");
	const char * protocolDigest = root->Attribute("ProtocolDigest");

	if (!sku || !accountName || !password || !protocolDigest)
	{
		WARN("One or more required data elements are missing!");
		sendErrorReply();
		return;
	}

	if (strlen(accountName) > 100)
	{
		WARN("Login account name is too long!");
		sendErrorReply();
		return;
	}

	std::string const & digest = AuthenticationService::instance().getConfigParameter("protocol_digest");
	if (protocolDigest != digest)
	{
		WARN("Client ProtocolHash mismatch; got '%s', expected '%s'", protocolDigest, digest.c_str());
		onLogonFailed(LogonQueue::VersionMismatch);
		return;
	}
	
	DEBUG2("%s: Authenticating account '%s'", socket_.remote_endpoint().address().to_string().c_str(), accountName);
	LogonQueue::Request req;
	req.serviceName = sku;
	req.userName = accountName;
	req.password = password;
	auto self = shared_from_this();
	req.successCb = [self](uint32_t id, std::string const & name, uint32_t level) {
		self->onLogonSuccessful(id, name, level);
	};
	req.errorCb = [self](LogonQueue::FailureCode code) {
		self->onLogonFailed(code);
	};
	AuthenticationService::as_instance().logonQueue().queueRequest(req);
}

void LogonConnection::onServerSelection(const std::string & request)
{
	// LOGF(DEBUG2, "Received server logon request");
	tinyxml2::XMLDocument doc;
	doc.Parse(request.c_str());
	if (doc.Error())
	{
		WARN("Received malformed request: %d", doc.ErrorID());
		sendErrorReply();
		return;
	}

	tinyxml2::XMLElement * root = doc.FirstChildElement("sgwLogin:SGWSelectServerRequest");
	if (root == nullptr)
	{
		WARN("Invalid root element in server logon request!");
		sendErrorReply();
		return;
	}

	const char * shard = root->Attribute("ServerSelection");

	if (!shard)
	{
		WARN("One or more required data elements are missing!");
		sendErrorReply();
		return;
	}

	AuthenticationService::LoginSession * session = AuthenticationService::as_instance().getSession(requestSession_);
	if (requestSession_ == "" || session == nullptr)
	{
		onShardEnterFailed(LogonQueue::SessionExpired);
		return;
	}

	FrontendConnection::Ptr frontend = AuthenticationService::as_instance().shard(shard);
	
	if (!frontend)
	{
		WARN("%s: Attempted to log on to nonexistent shard '%s'!", socket_.remote_endpoint().address().to_string().c_str(), shard);
		onShardEnterFailed(LogonQueue::NoSuchServer);
		return;
	}

	if (frontend->isProtected() && session->accessLevel < 2)
	{
		WARN("%s: Attempted to log on to shard '%s' without access!", socket_.remote_endpoint().address().to_string().c_str(), shard);
		onShardEnterFailed(LogonQueue::AccessDenied);
		return;
	}

	DEBUG2("%s: Logging on to shard '%s'", socket_.remote_endpoint().address().to_string().c_str(), shard);
	auto self = shared_from_this();
	frontend->logon(session->accountId, session->accountName, session->accessLevel,
		[self, frontend](FrontendConnection::ShardSessionData data) {
			self->onShardEnterSuccessful(data, frontend);
		},
		[self](LogonQueue::FailureCode code) {
			self->onShardEnterFailed(code);
		});
}

void LogonConnection::onShardEnterSuccessful(FrontendConnection::ShardSessionData & session, FrontendConnection::Ptr conn)
{
	sendReply(std::string("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>"
				"<ns3:SGWServerLocationResponse xmlns:ns3=\"http://www.stargateworlds.com/xml/sgwlogin\" xmlns:ns1=\"http://www.cheyenneme.com/xml/cmebase\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"sgwLogin http://www.stargateworlds.com/xml/sgwlogin\">"
					"<ServerLocation SessionKey=\"") + session.sessionKey +"\" Port=\"" +boost::lexical_cast<std::string>(session.port) +"\" IP=\"" +session.address +"\" BWMailBox=\"" +boost::lexical_cast<std::string>(session.mailboxId) +"\">"
						"<TICKET Ticket=\"" + session.ticketId +"\" />"
					"</ServerLocation>"
				"</ns3:SGWServerLocationResponse>");
}

void LogonConnection::onShardEnterFailed(LogonQueue::FailureCode errorCode)
{
	assert(errorCode <= LogonQueue::FailureMax);
	sendReply(std::string("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>"
		"<ns3:SGWServerLocationResponse xmlns:ns1=\"http://www.cheyenneme.com/xml/cmebase\" xmlns:ns3=\"http://www.stargateworlds.com/xml/sgwlogin\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"sgwLogin http://www.stargateworlds.com/xml/sgwlogin\">"
			"<ServerSelectionError ns1:ErrorStr=\"") + LogonFailureMessages[static_cast<uint32_t>(errorCode)] +"\" ns1:ErrorNum=\"1\" />"
		"</ns3:SGWServerLocationResponse>");
}

void LogonConnection::onLogonSuccessful(uint32_t accountId, std::string const & accountName, uint32_t accessLevel)
{
	std::string charset = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ";
	requestSession_ = "";
	std::uniform_int_distribution<> charset_distribution(0, (int)charset.length() - 1);
	for (int i = 0; i < 40; i++)
		requestSession_.push_back(charset[charset_distribution(Service::instance().rng())]);

	const std::vector<FrontendConnection::Ptr> & shardList = AuthenticationService::as_instance().shards();
	if (shardList.empty())
	{
		onLogonFailed(LogonQueue::NoServersAvailable);
		return;
	}

	AuthenticationService::as_instance().registerSession(requestSession_, accountId, accountName, accessLevel);

	std::string shardsXml;
	for (auto iter = shardList.begin(); iter != shardList.end(); ++iter)
	{
		shardsXml += std::string("<Shard ServerName=\"") +(*iter)->shardName() +"\" Fullness=\"LOW\" Busy=\"LOW\" />";
	}

	sendReply(std::string("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>"
				"<ns2:SGWLoginResponse xmlns:ns2=\"http://www.stargateworlds.com/xml/sgwlogin\" xmlns:ns3=\"http://www.cheyenneme.com/xml/cmebase\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"sgwLogin http://www.stargateworlds.com/xml/sgwlogin\">"
					"<SGWLoginSuccess>"
						"<AccountInfo ExpireDate=\"0000-00-00T00:00:00.000Z\" AccountId=\""
							+ boost::lexical_cast<std::string>(accountId) + "\" />"
						"<SGWShardListResp>")
							+ shardsXml
						+"</SGWShardListResp>"
					"</SGWLoginSuccess>"
				"</ns2:SGWLoginResponse>");
}

void LogonConnection::onLogonFailed(LogonQueue::FailureCode errorCode)
{
	assert(errorCode <= LogonQueue::FailureMax);
	sendReply(std::string(
		"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>"
			"<ns2:SGWLoginResponse xmlns:ns2=\"http://www.stargateworlds.com/xml/sgwlogin\" xmlns:ns3=\"http://www.cheyenneme.com/xml/cmebase\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"sgwLogin http://www.stargateworlds.com/xml/sgwlogin\">"
				"<SGWLoginError ns3:ErrorStr=\"") +LogonFailureMessages[static_cast<uint32_t>(errorCode)] +"\" ns3:ErrorNum=\"1\" />"
			"</ns2:SGWLoginResponse>");
}

void LogonConnection::sendResponse(size_t sendOffset)
{
	sendOffset_ = sendOffset;
	assert(sendOffset_ < response_.length());
	auto self = shared_from_this();
	socket_.async_send(
		boost::asio::buffer(&response_.data()[sendOffset_], response_.length() - sendOffset_),
		[self](const boost::system::error_code & err, size_t len) {
			self->onResponseSent(err, len);
		});
}

void LogonConnection::onResponseSent(const boost::system::error_code & errcode, size_t sentLength)
{
	if (errcode == boost::system::errc::success)
	{
		sendOffset_ += sentLength;
		if (sendOffset_ < response_.length())
			sendResponse(sendOffset_);
		else
		{
			assert(sendOffset_ == response_.length());

			lock_.lock();
			response_.clear();
			lock_.unlock();
		}
	}
	else
	{
		LOGF(LL_WARNING, "Send failed: %s", errcode.message().c_str());
		close(errcode);
	}
}

