#include <stdafx.hpp>
#include <baseapp/minigame.hpp>
#include <baseapp/minigame_connection.hpp>
#include <boost/lexical_cast.hpp>
#include <boost/python/long.hpp>
#include <boost/python/str.hpp>
#include <boost/python/list.hpp>
#include <boost/python/dict.hpp>
#include <boost/python/import.hpp>
#include <boost/python/extract.hpp>
#include <boost/python/scope.hpp>
#include <boost/python/class.hpp>
#include <entity/pyutil.hpp>

#undef DEBUG_NET_MINIGAME

Minigame::Ptr Minigame::createMinigame(MinigameRequestManager::QueueEntry const & request, MinigameConnection & connection)
{
	PyGilGuard guard;
	try
	{
		std::string modName = std::string("base.minigame.") + request.gameName;
		bp::object obj = bp::import(modName.c_str())
			.attr(request.gameName.c_str())();
		PythonMinigame::Ptr ptr = bp::extract<PythonMinigame::Ptr>(obj);
		ptr->setObject(obj);
		ptr->setup(request, connection);
		return ptr;
	}
	catch (bp::error_already_set &)
	{
		FAULT("Exception while creating minigame '%s':", request.gameName.c_str());
		PyUtil_ShowErr();
		return Ptr();
	}
}


void PythonMinigame::registerClass()
{
	bp::object mod = bp::import(bp::str("Atrea"));
	bp::scope active_scope(mod);
	bp::class_<PythonMinigame, PythonMinigame::Ptr> cls =
		bp::class_<PythonMinigame, PythonMinigame::Ptr>("Minigame");
	
	cls.def_readonly("gameName", &PythonMinigame::gameName_);
	cls.def_readonly("key", &PythonMinigame::key_);
	cls.def_readonly("entityId", &PythonMinigame::entityId_);
	cls.def_readonly("difficulty", &PythonMinigame::difficulty_);
	cls.def_readonly("techCompetency", &PythonMinigame::techCompetency_);
	cls.def_readonly("seed", &PythonMinigame::seed_);
	cls.def_readonly("abilitiesMask", &PythonMinigame::abilitiesMask_);
	cls.def_readonly("intelligence", &PythonMinigame::intelligence_);
	cls.def_readonly("playerLevel", &PythonMinigame::playerLevel_);
	cls.def("abort", &PythonMinigame::abort);
	cls.def("send", &PythonMinigame::sendExtensionMessage);
	cls.def("victory", &PythonMinigame::victory);
}


PythonMinigame::PythonMinigame()
	: connection_(nullptr), running_(false)
{
}

PythonMinigame::~PythonMinigame()
{
	if (running_)
		handler_(this, MinigameCanceled);
}


void PythonMinigame::setup(MinigameRequestManager::QueueEntry const & request, MinigameConnection & connection)
{
	handler_ = request.handler;
	connection_ = &connection;
	gameName_ = request.gameName;
	key_ = request.key;
	entityId_ = request.entityId;
	difficulty_ = request.difficulty;
	techCompetency_ = request.techCompetency;
	seed_ = request.seed;
	abilitiesMask_ = request.abilitiesMask;
	intelligence_ = request.intelligence;
	playerLevel_ = request.playerLevel;
}

void PythonMinigame::setObject(bp::object & o)
{
	object_ = o;
}


void PythonMinigame::sendExtensionMessage(bp::object args)
{
	if (!connection_)
		return;

	PyUtil_AssertGIL();
	std::stringstream msg;
	try
	{
		packDict(msg, args);
		connection_->queueExtensionMessage(msg.str());
#ifdef DEBUG_NET_MINIGAME
		TRACE("Sending XT message:\r\n", msg.str().c_str());
#endif
	}
	catch (bp::error_already_set &)
	{
		FAULT("Exception while packing extension message:");
		PyUtil_ShowErr();
	}
}


void PythonMinigame::victory()
{
	if (running_)
	{
		TRACE("%s: victory!", gameName_.c_str());
		running_ = false;
		if (connection_)
			connection_->closeMinigame();
		handler_(this, MinigameVictory);
	}
}


void PythonMinigame::start()
{
	running_ = true;
	PyGilGuard guard;
	try
	{
		object_.attr("started")();
	}
	catch (bp::error_already_set &)
	{
		FAULT("Exception while calling Minigame::started:");
		PyUtil_ShowErr();
	}
}


void PythonMinigame::connectionClosed()
{
	connection_ = nullptr;
	if (running_)
		abort(AbortConnectionClosed);
}


void PythonMinigame::abort(AbortReason /*reason*/)
{
	if (!running_)
	{
		FAULT("Minigame is not running!");
		return;
	}

	TRACE("Aborting minigame");
	
	{
		PyGilGuard guard;
		try
		{
			object_.attr("aborted")();
		}
		catch (bp::error_already_set &)
		{
			FAULT("Exception while calling Minigame::aborted:");
			PyUtil_ShowErr();
		}
	}
	
	running_ = false;
	if (connection_)
		connection_->closeMinigame();
	handler_(this, MinigameCanceled);
}


bool PythonMinigame::onMinigameMessage(std::string const & cmd, tinyxml2::XMLElement const & message)
{
	if (!running_)
	{
		WARN("Minigame is not running!");
		return false;
	}
	
	PyGilGuard guard;
	try
	{
#ifdef DEBUG_NET_MINIGAME
		TRACE("Processing XT message:\r\n", cmd.c_str());
#endif
		bp::object args = unpackArray(message);
		if (args.is_none())
			return false;

		bp::object handled = object_.attr("message")(cmd, args);
		return (bool)handled;
	}
	catch (bp::error_already_set &)
	{
		FAULT("Exception while calling Minigame::message:");
		PyUtil_ShowErr();
		return false;
	}
}

std::string const & PythonMinigame::getMinigameName() const
{
	return gameName_;
}

bool PythonMinigame::running() const
{
	return running_;
}

bp::object PythonMinigame::unpackArray(tinyxml2::XMLElement const & element)
{
	bp::dict items;
	for (tinyxml2::XMLElement const * var = element.FirstChildElement("var"); var; var = var->NextSiblingElement("var"))
	{
		if (!unpackVariable(items, *var))
			return bp::object();
	}

	return items;
}

bool PythonMinigame::unpackVariable(bp::object & container, tinyxml2::XMLElement const & element)
{
	const char * name = element.Attribute("n");
	const char * type = element.Attribute("t");
	const char * value = element.GetText();
	if (!name || !type || !value || type[1] != 0)
	{
		WARN("Illegal variable data in minigame request.");
		return false;
	}

	switch (type[0])
	{
	case 's':
		container[name] = value;
		break;

	case 'n':
		container[name] = atol(value);
		break;

	case 'b':
		container[name] = (atol(value) == 1);
		break;

	case 'a':
		{
			bp::object items = unpackArray(element);
			if (items.is_none())
				return false;
			container[name] = items;
			break;
		}

	default:
		WARN("Illegal variable type '%c' in minigame request.", type[0]);
		return false;
	}

	return true;
}


bool PythonMinigame::packDict(std::stringstream & stream, bp::object & var)
{
	bp::extract<bp::dict> dict(var);
	if (!dict.check())
	{
		WARN("Python variable is not a dict (got %s)", var.ptr()->ob_type->tp_name);
		return false;
	}

	bp::list items = (bp::list)dict().items();
	bp::ssize_t len = bp::len(items);
	for (bp::ssize_t i = 0; i < len; i++)
	{
		std::string key = bp::extract<std::string>(items[i][0])();
		bp::object var = items[i][1];
		if (!packVariable(stream, key, var))
			return false;
	}

	return true;
}


bool PythonMinigame::packList(std::stringstream & stream, bp::object & var)
{
	bp::extract<bp::list> list(var);
	if (!list.check())
	{
		WARN("Python variable is not a list (got %s)", var.ptr()->ob_type->tp_name);
		return false;
	}

	bp::list items = list();
	bp::ssize_t len = bp::len(items);
	for (bp::ssize_t i = 0; i < len; i++)
	{
		bp::object var = items[i];
		if (!packVariable(stream, boost::lexical_cast<std::string>(i), var))
			return false;
	}

	return true;
}


bool PythonMinigame::packVariable(std::stringstream & stream, std::string const & name, bp::object & var)
{
	bp::extract<std::string> str(var);
	if (str.check())
	{
		stream << "<var n='" << name << "' t='s'>";
		stream << str();
		stream << "</var>";
		return true;
	}
	
	bp::extract<int64_t> i(var);
	if (i.check())
	{
		stream << "<var n='" << name << "' t='n'>";
		stream << i();
		stream << "</var>";
		return true;
	}
	
	bp::extract<float> f(var);
	if (f.check())
	{
		stream << "<var n='" << name << "' t='n'>";
		stream << f();
		stream << "</var>";
		return true;
	}
	
	bp::extract<bool> b(var);
	if (b.check())
	{
		stream << "<var n='" << name << "' t='b'>";
		stream << (i() ? "true" : "false");
		stream << "</var>";
		return true;
	}
	
	bp::extract<bp::dict> dict(var);
	if (dict.check())
	{
		stream << "<obj o='" << name << "' t='a'>";
		packDict(stream, var);
		stream << "</obj>";
		return true;
	}
	
	bp::extract<bp::list> list(var);
	if (list.check())
	{
		stream << "<obj o='" << name << "' t='a'>";
		packList(stream, var);
		stream << "</obj>";
		return true;
	}
	
	WARN("Python variable type unsupported (got %s)", var.ptr()->ob_type->tp_name);
	return false;
}

