#pragma once

#include <boost/shared_ptr.hpp>
#include <boost/thread/recursive_mutex.hpp>
#include <boost/python/object.hpp>
#include <random>

enum MinigameCompletionStatus
{
	// The minigame was canceled after being started
	MinigameCanceled = 0,
	// The player won the minigame
	MinigameVictory = 1,
	// The player lost the minigame
	MinigameDefeat = 2,
	// The minigame was canceled before being started
	MinigameNotStarted = 3
};

class MinigameRequestManager
{
public:
	typedef boost::function<void (class Minigame *, MinigameCompletionStatus)> CompletionHandler;

	struct QueueEntry
	{
		std::string key;
		uint32_t entityId;
		std::string gameName;
		uint32_t difficulty;
		uint32_t techCompetency;
		uint32_t seed;
		uint32_t abilitiesMask;
		uint32_t intelligence;
		uint32_t playerLevel;
		CompletionHandler handler;
		bool canceled;
		class MinigameConnection * session;
	};

	MinigameRequestManager::QueueEntry * queue(uint32_t entityId, const std::string & gameName, uint32_t difficulty, uint32_t techCompetency, 
		uint32_t seed, uint32_t abilitiesMask, uint32_t intelligence, uint32_t playerLevel, CompletionHandler handler);
	QueueEntry * find(uint32_t entityId, const std::string & password);
	bool cancel(uint32_t entityId);
	void remove(uint32_t entityId);

private:
	std::map<uint32_t, QueueEntry> queue_;
	boost::recursive_mutex lock_;
};

class MinigameMessageHandler
{
public:
	virtual inline ~MinigameMessageHandler() {}
	virtual bool onMinigameMessage(std::string const & cmd, tinyxml2::XMLElement const & message) = 0;
};

class Minigame : public MinigameMessageHandler
{
	public:
		typedef boost::shared_ptr<Minigame> Ptr;

		enum AbortReason
		{
			/*
			 *	The game was aborted because of a premature connection close
			 */
			AbortConnectionClosed = 0,
			/*
			 *	The player canceled the minigame (via the minigame server or CellApp RPC).
			 */
			AbortPlayerCanceled = 1,
			/*
			 *	The player tried to do something nasty
			 */
			AbortIllegalAction = 2
		};

		virtual inline ~Minigame() {}
		virtual void start() = 0;
		virtual void connectionClosed() = 0;
		virtual void abort(AbortReason reason) = 0;
		virtual std::string const & getMinigameName() const = 0;
		virtual bool running() const = 0;

		static Ptr createMinigame(MinigameRequestManager::QueueEntry const & request, class MinigameConnection & connection);
};

class PythonMinigame : public Minigame
{
public:
	typedef boost::shared_ptr<PythonMinigame> Ptr;

	static void registerClass();

	PythonMinigame();
	virtual ~PythonMinigame();
	
	void setup(MinigameRequestManager::QueueEntry const & request, class MinigameConnection & connection);
	void setObject(bp::object & o);
	void sendExtensionMessage(bp::object args);
	void victory();

	virtual void start();
	virtual void connectionClosed();
	virtual void abort(AbortReason reason);
	virtual bool onMinigameMessage(std::string const & cmd, tinyxml2::XMLElement const & message);
	virtual std::string const & getMinigameName() const;
	virtual bool running() const;

private:
	bp::object unpackArray(tinyxml2::XMLElement const & element);
	bool unpackVariable(bp::object & container, tinyxml2::XMLElement const & element);
	bool packList(std::stringstream & stream, bp::object & var);
	bool packDict(std::stringstream & stream, bp::object & var);
	bool packVariable(std::stringstream & stream, std::string const & name, bp::object & var);

	MinigameRequestManager::CompletionHandler handler_;
	MinigameConnection * connection_;
	bp::object object_;
	std::string gameName_;
	bool running_;
	
	std::string key_;
	uint32_t entityId_;
	uint32_t difficulty_;
	uint32_t techCompetency_;
	uint32_t seed_;
	uint32_t abilitiesMask_;
	uint32_t intelligence_;
	uint32_t playerLevel_;
};

