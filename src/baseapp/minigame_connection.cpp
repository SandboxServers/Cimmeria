#include <stdafx.hpp>
#include <baseapp/minigame_connection.hpp>
#include <baseapp/base_service.hpp>
#include <boost/lexical_cast.hpp>

#undef DEBUG_NET_MINIGAME

MinigameRequestManager::QueueEntry * MinigameRequestManager::queue(uint32_t entityId, const std::string & gameName, uint32_t difficulty,
	uint32_t techCompetency, uint32_t seed, uint32_t abilitiesMask, uint32_t intelligence, uint32_t playerLevel, CompletionHandler handler)
{
	QueueEntry ent;
	ent.entityId = entityId;
	ent.gameName = gameName;
	ent.difficulty = difficulty;
	ent.techCompetency = techCompetency;
	ent.seed = seed;
	ent.abilitiesMask = abilitiesMask;
	ent.intelligence = intelligence;
	ent.playerLevel = playerLevel;
	ent.handler = handler;
	ent.canceled = false;
	ent.session = nullptr;
	ent.key.resize(64);
	const char * hextable = "0123456789ABCDEF";
	for (unsigned int i = 0; i < 64; i++)
	{
		ent.key[i] = hextable[rand() % 16];
	}

	std::lock_guard<std::recursive_mutex> guard(lock_);
	auto it = queue_.find(entityId);
	if (it == queue_.end())
	{
		queue_.insert(std::make_pair(entityId, ent));
		return &queue_[entityId];
	}
	else
	{
		WARN("Entity %d already has an active minigame session!", entityId);
		return nullptr;
	}
}

MinigameRequestManager::QueueEntry * MinigameRequestManager::find(uint32_t entityId, const std::string & password)
{
	std::lock_guard<std::recursive_mutex> guard(lock_);
	auto it = queue_.find(entityId);
	if (it == queue_.end())
		return nullptr;

	if (it->second.key != password)
		return nullptr;

	return &it->second;
}

bool MinigameRequestManager::cancel(uint32_t entityId)
{
	std::lock_guard<std::recursive_mutex> guard(lock_);
	auto it = queue_.find(entityId);
	if (it == queue_.end())
		return false;

	if (it->second.canceled)
		return false;

	if (it->second.session)
	{
		TRACE("Aborting conn session ...");
		it->second.canceled = true;
		it->second.session->minigame()->abort(Minigame::AbortPlayerCanceled);
	}
	else
		queue_.erase(it);

	return true;
}

void MinigameRequestManager::remove(uint32_t entityId)
{
	std::lock_guard<std::recursive_mutex> guard(lock_);
	auto it = queue_.find(entityId);
	if (it != queue_.end())
		queue_.erase(it);
	else
		FAULT("No such queued request: %d", entityId);
}

MinigameConnection * MinigameConnection::Create(Mercury::TcpServer<MinigameConnection> & server, uint32_t connectionId)
{
	return new MinigameConnection(connectionId);
}

MinigameConnection::MinigameConnection(uint32_t connectionId)
	: socket_(BaseService::instance().ioService()), connectionId_(connectionId), registered_(false),
	  disconnected_(true), messageReceived_(0), entityId_(0)
{
}

MinigameConnection::~MinigameConnection()
{
	close();
#ifdef DEBUG_NET_MINIGAME
	LOGF(LL_TRACE, "Destroyed");
#endif
}

void MinigameConnection::connected()
{
#ifdef DEBUG_NET_MINIGAME
	LOGF(LL_TRACE, "Connected");
#endif
	disconnected_ = false;
	registered_ = true;
	receiveMessage();
}

boost::asio::ip::tcp::socket & MinigameConnection::socket()
{
	return socket_;
}

uint32_t MinigameConnection::connectionId() const
{
	return connectionId_;
}

Minigame::Ptr MinigameConnection::minigame() const
{
	return minigame_;
}

void MinigameConnection::close(const boost::system::error_code & errcode)
{
	if (!disconnected_)
		disconnected();
	disconnected_ = true;

	if (socket_.is_open())
		socket_.close();

	if (registered_)
	{
		registered_ = false;
		// unregisterConnection();
	}
}

void MinigameConnection::close()
{
	close(boost::system::error_code());
}

bool MinigameConnection::isConnected() const
{
	return socket_.is_open();
}

void MinigameConnection::queueMessage(std::string const & msg)
{
	lock_.lock();
	if (sendQueue_.empty() && sendMessage_.empty())
	{
		sendOffset_ = 0;
		sendMessage_ = msg;
		sendMessage();
	}
	else
		sendQueue_.push(msg);
	lock_.unlock();
}

void MinigameConnection::queueExtensionMessage(std::string const & message)
{
	std::string msg = 
		std::string() + 
		"<msg t='xt'>"
			"<body action='xtRes' r='-1'>"
				"<![CDATA[<dataObj>"
					+ message +
				"</dataObj>]]>"
			"</body>"
		"</msg>";
	queueMessage(msg);
}


void MinigameConnection::closeMinigame()
{
	queueExtensionMessage(
		std::string() + 
		"<var n='_cmd' t='s'>onPlayerLeaveGame</var>"
		"<var n='PlayerId' t='s'>" +userId_ +"</var>"
	);

	queueExtensionMessage("<var n='_cmd' t='s'>onGameEnd</var>");

	std::string msgRoomDel = 
		"<msg t='sys'>"
			"<body action='roomDel'>"
				"<rm id='1001' />"
			"</body>"
		"</msg>";
	queueMessage(msgRoomDel);
	minigameClosed();
}

void MinigameConnection::disconnected()
{
#ifdef DEBUG_NET_MINIGAME
	LOGF(LL_TRACE, "Connection closed");
#endif
	minigameClosed();
	BaseService::base_instance().minigameServer()->unregister_connection(shared_this());
}

void MinigameConnection::minigameClosed()
{
	LOGF(LL_TRACE, "Minigame closed");
	if (minigame_)
		minigame_->connectionClosed();
	minigame_.reset();
	if (entityId_ != 0)
	{
		BaseService::base_instance().minigameManager().remove(entityId_);
		entityId_ = 0;
	}
}

bool MinigameConnection::handleMessage(std::string const & message)
{
#ifdef DEBUG_NET_MINIGAME
	LOGF(LL_TRACE, "Received minigame message:\r\n%s", message.c_str());
#endif

	tinyxml2::XMLDocument doc;
	doc.Parse(message.c_str());
	if (doc.Error())
	{
		LOGF(LL_WARNING, "XML Parsing Error: %d", doc.ErrorID());
		return false;
	}

	tinyxml2::XMLElement * root = doc.FirstChildElement("msg");
	if (!root)
	{
		LOGF(LL_WARNING, "Couldn't find <msg> tag in message.");
		return false;
	}

	if ((std::string)root->Value() != "msg")
	{
		LOGF(LL_WARNING, "Illegal tag in message: %s.", root->Value());
		return false;
	}

	const char * msgType = root->Attribute("t");
	if (!msgType)
	{
		LOGF(LL_WARNING, "Request has no message type");
		return false;
	}

	tinyxml2::XMLElement * body = root->FirstChildElement("body");
	if (!body)
	{
		LOGF(LL_WARNING, "Couldn't find <body> tag in message.");
		return false;
	}
	
	int bodyR;
	const char * action_c = body->Attribute("action");
	if (!action_c || body->QueryIntAttribute("r", &bodyR) != tinyxml2::XML_SUCCESS)
	{
		LOGF(LL_WARNING, "Missing or bad attributes set for body tag.");
		return false;
	}
	
	std::string action = action_c;

	if (action == "verChk")
	{
		return handleVersionCheck(*body);
	}
	else if (action == "login")
	{
		return handleLogin(*body);
	}
	else if (action == "xtReq")
	{
		return handleExtensionRequest(*body);
	}
	else
	{
		LOGF(LL_WARNING, "Invalid action in message: %s.", action.c_str());
		return false;
	}
}

bool MinigameConnection::handleVersionCheck(tinyxml2::XMLElement & body)
{
	if (minigame_)
	{
		LOGF(LL_WARNING, "Duplicate version check request");
		return false;
	}

	int versionNumber;
	tinyxml2::XMLElement * version = body.FirstChildElement("ver");
	if (!version || version->QueryIntAttribute("v", &versionNumber) != tinyxml2::XML_SUCCESS)
	{
		LOGF(LL_WARNING, "Couldn't find version information in message.");
		return false;
	}

	std::string crossDomPolicy = 
		std::string() +
		"<cross-domain-policy><allow-access-from domain='*' to-ports='" + 
		Service::instance().getConfigParameter("minigame_external_port") + 
		"' /></cross-domain-policy>";
	queueMessage(crossDomPolicy);

	/*
	 * API version check successful: apiOK
	 * Check failed (incompatible): apiKO
	 *
	 * We need to send an apiOK message here even if the version doesn't match as 
	 * the client doesn't handle the apiKO message properly and leaves the minigame 
	 * window open forever.
	 */
	std::string apiOk = "<msg t='sys'><body action='apiOK' r='0'></body></msg>";
	queueMessage(apiOk);
	apiVersion_ = versionNumber;
	return true;
}

bool MinigameConnection::handleLogin(tinyxml2::XMLElement & body)
{
	if (minigame_)
	{
		LOGF(LL_WARNING, "Duplicate login request");
		return false;
	}

	/*
	 * We couldn't refuse clients with bad API versions in handleVersionCheck(),
	 * but we can do it now by sending a loginFailed message.
	 */
	if (apiVersion_ != ApiVersion)
	{
		queueExtensionMessage("<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>");
		LOGF(LL_WARNING, "Client sent invalid version number: %d", apiVersion_);
		return false;
	}

	tinyxml2::XMLElement * login = body.FirstChildElement("login");
	if (!login)
	{
		LOGF(LL_WARNING, "Missing <login> tag");
		return false;
	}
	
	const char * zone = login->Attribute("z");
	if (!zone)
	{
		LOGF(LL_WARNING, "Missing 'zone' property in login message");
		return false;
	}

	tinyxml2::XMLElement * nick = login->FirstChildElement("nick");
	tinyxml2::XMLElement * password = login->FirstChildElement("pword");
	if (!nick || !password)
	{
		LOGF(LL_WARNING, "Missing <nick> or <password> tag in login message");
		return false;
	}

	userId_ = nick->GetText();
	std::string passwd = password->GetText();

	MinigameRequestManager::QueueEntry * entry = BaseService::base_instance().minigameManager().find(atol(userId_.c_str()), passwd);
	if (!entry)
	{
		WARN("Minigame login failed for entity ID %s, Key %s (maybe it timed out?)", userId_.c_str(), passwd.c_str());
		queueExtensionMessage("<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>");
		return false;
	}

	if (entry->gameName != zone)
	{
		WARN("Minigame game name (%s) differs from queue entry (%s)", entry->gameName.c_str(), zone);
		queueExtensionMessage("<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>");
		return false;
	}

	LOGF(LL_TRACE, "Minigame login: %s, %s", userId_.c_str(), passwd.c_str());
	entityId_ = entry->entityId;

	minigame_ = Minigame::createMinigame(*entry, *this);
	if (!minigame_)
	{
		WARN("Failed to create a minigame of type '%s'", zone);
		queueExtensionMessage("<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>");
		entry->handler(nullptr, MinigameNotStarted);
		return false;
	}
	entry->session = this;

	std::string roomListMsg = 
		std::string() + 
		"<msg t='sys'>"
			"<body action='rmList' r='0'>"
				"<rm id='1001' priv='0' temp='0' game='1' ucnt='1' maxu='1' scnt='0' maxs='100'>"
					"<n><![CDATA[" + minigame_->getMinigameName() + "-10001]]></n>"
				"</rm>"
			"</body>"
		"</msg>";
	queueMessage(roomListMsg);

	queueExtensionMessage("<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginSucceeded</var>");
	
	std::string joinMsg =
		std::string() + 
		"<msg t='sys'>"
			"<body action='joinOK' r='1001'>"
				"<pid id='1' />"
				"<vars>"
					"<var n='abilityBitfield' t='n'><![CDATA[" + boost::lexical_cast<std::string>(entry->abilitiesMask) + "]]></var>"
					"<var n='seed' t='n'><![CDATA[" + boost::lexical_cast<std::string>(entry->seed) + "]]></var>"
					"<var n='difficulty' t='n'><![CDATA[" + boost::lexical_cast<std::string>(entry->difficulty) + "]]></var>"
					"<var n='techcomp' t='n'><![CDATA[" + boost::lexical_cast<std::string>(entry->techCompetency) + "]]></var>"
					"<var n='pclevel' t='n'><![CDATA[" + boost::lexical_cast<std::string>(entry->playerLevel) + "]]></var>"
					"<var n='intelligence' t='n'><![CDATA[" + boost::lexical_cast<std::string>(entry->intelligence) + "]]></var>"
					"<var n='instcc' t='n'><![CDATA[-1]]></var>"
					"<var n='CA0' t='n'><![CDATA[0]]></var>"
					"<var n='CA1' t='n'><![CDATA[0]]></var>"
					"<var n='CA2' t='n'><![CDATA[0]]></var>"
					"<var n='CA3' t='n'><![CDATA[0]]></var>"
					"<var n='CA4' t='n'><![CDATA[0]]></var>"
				"</vars>"
				"<uLs r='1001'>"
					"<u i='999' m='0' s='0' p='1'>"
						"<n><![CDATA[" + userId_ + "]]></n>"
						"<vars></vars>"
					"</u>"
				"</uLs>"
			"</body>"
		"</msg>";
	queueMessage(joinMsg);

	/*
	 * Update user number in rooms
	 * r: Current room ID
	 * u: User count
	 * s: Spectator count (?)
	 */
	std::string userCountMsg = 
		"<msg t='sys'>"
			"<body action='uCount' r='1001' u='1' s='0'>"
			"</body>"
		"</msg>";
	queueMessage(userCountMsg);

	minigame_->start();

	queueExtensionMessage(
		std::string() + 
		"<var n='_cmd' t='s'>onPlayerJoinGame</var>"
		"<var n='PlayerId' t='s'>" + userId_ + "</var>"
	);

	/*
	 * Send and extension message (xt) requesting the client to start the game
	 */
	queueExtensionMessage("<var n='_cmd' t='s'>onGameBegin</var>");
	return true;
}

bool MinigameConnection::handleExtensionRequest(tinyxml2::XMLElement & body)
{
	tinyxml2::XMLDocument request;
	request.Parse(body.GetText());

	if (request.Error())
	{
		LOGF(LL_WARNING, "XML Parsing Error: %d", request.ErrorID());
		return false;
	}

	tinyxml2::XMLElement * root = request.FirstChildElement("dataObj");
	if (!root)
	{
		LOGF(LL_WARNING, "Missing <dataObj> tag in extension request.");
		return false;
	}

	std::string cmd;
	for (tinyxml2::XMLElement * var = root->FirstChildElement("var"); var; var = var->NextSiblingElement("var"))
	{
		const char * varName = var->Attribute("n");
		if (varName && std::string(varName) == "cmd")
		{
			cmd = var->GetText();
			break;
		}
	}

	tinyxml2::XMLElement * param = root->FirstChildElement("obj");
	if (!param)
	{
		LOGF(LL_WARNING, "Minigame request doesn't have its mandatory object argument.");
		return false;
	}
	
	if (minigame_)
	{
		/*
		 * We need a copy of the minigame pointer here as the minigame_ ref
		 * might be reset when the session is finished and the minigame object
		 * would be destroyed in the middle of the onMinigameMessage() call
		 */
		Minigame::Ptr ref = minigame_;
		return ref->onMinigameMessage(cmd, *param);
	}
	else
	{
		LOGF(LL_WARNING, "Minigame request received with no minigame instance.");
		return false;
	}
}

void MinigameConnection::receiveMessage()
{
	auto self = shared_this();
	socket_.async_receive(
		boost::asio::buffer(&message_[messageReceived_], MaxMessageLength - messageReceived_),
		[self](const boost::system::error_code & err, size_t len) {
			self->receivedMessage(err, len);
		});
}

void MinigameConnection::receivedMessage(const boost::system::error_code & errcode, size_t receivedLength)
{
	if (errcode == boost::system::errc::success)
	{
		messageReceived_ += receivedLength;

		for (;;)
		{
			size_t length = strnlen(reinterpret_cast<const char *>(&message_[0]), messageReceived_);
			if (length != messageReceived_)
			{
				std::string msg = reinterpret_cast<const char *>(&message_[0]);
				memmove(&message_[0], &message_[length + 1], messageReceived_ - length - 1);
				messageReceived_ -= length + 1;
				if (!handleMessage(msg))
				{
					WARN("Minigame server: Failed to handle message");
					if (minigame_)
						minigame_->abort(Minigame::AbortIllegalAction);
					close();
				}
			}
			else
			{
				if (messageReceived_ == MaxMessageLength)
				{
					WARN("Minigame server: Received too long message (%d bytes)", MaxMessageLength);
					if (minigame_)
						minigame_->abort(Minigame::AbortIllegalAction);
					close();
				}
				break;
			}
		}

		if (isConnected())
			receiveMessage();
	}
	else
	{
		WARN("Minigame server: receive failed: %s", errcode.message().c_str());
		close(errcode);
	}
}


void MinigameConnection::sendMessage()
{
	SGW_ASSERT(sendOffset_ <= sendMessage_.length());
#ifdef DEBUG_NET_MINIGAME
	LOGF(LL_TRACE, "Sending msg (%d bytes sent, %d total)", sendOffset_, sendMessage_.length() + 1);
#endif
	auto self = shared_this();
	socket_.async_send(
		boost::asio::buffer(&sendMessage_[sendOffset_], sendMessage_.length() + 1 - sendOffset_),
		[self](const boost::system::error_code & err, size_t len) {
			self->messageSent(err, len);
		});
}


void MinigameConnection::messageSent(const boost::system::error_code & errcode, size_t sentLength)
{
	if (errcode == boost::system::errc::success)
	{
		sendOffset_ += sentLength;
		if (sendOffset_ < sendMessage_.length() + 1)
			sendMessage();
		else
		{
			SGW_ASSERT(sendOffset_ == sendMessage_.length() + 1);
#ifdef DEBUG_NET_MINIGAME
			LOGF(LL_TRACE, "Sent message");
#endif

			lock_.lock();
			sendMessage_.clear();
			sendOffset_ = 0;
			if (!sendQueue_.empty())
			{
				sendMessage_ = sendQueue_.front();
				sendQueue_.pop();
				sendMessage();
			}
			lock_.unlock();
		}
	}
	else
	{
		LOGF(LL_WARNING, "Send failed: %s", errcode.message().c_str());
		close(errcode);
	}
}

