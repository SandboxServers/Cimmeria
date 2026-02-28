#include <stdafx.hpp>

#include <boost/python/exec.hpp>
#include <boost/python/extract.hpp>
#include <boost/python/import.hpp>

#include <entity/py_client.hpp>
#include <entity/pyutil.hpp>
#include <common/service.hpp>

py_client * py_client::create(Mercury::TcpServer<py_client> & server, uint32_t connection_id)
{
	return new py_client();
}

py_client::py_client()
	: Mercury::UnifiedConnection(0), state_(PY_CLIENT_AUTH_WAIT)
{
}

py_client::~py_client()
{
}

void py_client::startup()
{
	SGW_ASSERT(!isConnected());
}

void py_client::onRequestAuthenticate(Reader & msg)
{
	std::string password;
	msg.readString(password);

	Writer writer(*this);
	writer.beginMessage(PYS_ACK_AUTHENTICATE);

	if (password == Service::instance().getConfigParameter("py_console_password"))
	{
		TRACE("PyWebClient login OK");
		writer << (uint32_t)0;
		state_ = PY_CLIENT_OK;
	}
	else
	{
		WARN("PyWebClient login failed");
		writer << (uint32_t)1;
	}

	writer.endMessage();
}

void py_client::onRequestEvaluate(Reader & msg)
{
	if (state_ != PY_CLIENT_OK)
	{
		WARN("PyWebClient execute requested without authentication");
		close();
		return;
	}

	std::string req;
	msg.readString(req);
	TRACE("PyWebClient evaluate: %s", req.c_str());

	PyGilGuard guard;
	try
	{
		bp::object main = bp::import("__main__");
		bp::object global(main.attr("__dict__"));
		bp::object result = bp::eval(req.c_str(), global, global);
		std::string tn = result.ptr()->ob_type->tp_name;
		
		Writer writer(*this);
		writer.beginMessage(PYS_ACK_EVALUATE);

		if (tn == "NoneType")
		{
			writer << (uint8_t)PY_EXEC_NONE;
		}
		else if (tn == "bool")
		{
			writer << (uint8_t)PY_EXEC_INTEGER << (int32_t)(bp::extract<bool>(result) ? 1 : 0);
		}
		else if (tn == "int")
		{
			writer << (uint8_t)PY_EXEC_INTEGER << (int32_t)bp::extract<int32_t>(result);
		}
		else if (tn == "float")
		{
			float res = bp::extract<float>(result);
			writer << (uint8_t)PY_EXEC_FLOAT << res;
		}
		else if (tn == "str")
		{
			std::string res = bp::extract<std::string>(result);
			writer << (uint8_t)PY_EXEC_STRING;
			writer.writeString(res);
		}
		else
		{
			WARN("Unable to serialize object of type <%s>", tn.c_str());
			writer << (uint8_t)PY_EXEC_NONE;
		}

		writer.endMessage();
	}
	catch (bp::error_already_set &)
	{
		PyUtil_ShowErr("PyConsole client exception");
		
		Writer writer(*this);
		writer.beginMessage(PYS_ACK_EVALUATE);
		writer << (uint8_t)PY_EXEC_EXCEPTION;
		writer.writeString(std::string("Python syntax/runtime error"));
		writer.endMessage();
	}
	catch (std::exception & e)
	{
		WARN("%s", e.what());
	}	
}

void py_client::onRequestExecute(Reader & msg)
{
	if (state_ != PY_CLIENT_OK)
	{
		WARN("PyWebClient execute requested without authentication");
		close();
		return;
	}

	std::string req;
	msg.readString(req);
	TRACE("PyWebClient execute: %s", req.c_str());

	PyGilGuard guard;
	try
	{
		bp::object main = bp::import("__main__");
		bp::object global(main.attr("__dict__"));
		bp::exec(req.c_str(), global, global);
		
		Writer writer(*this);
		writer.beginMessage(PYS_ACK_EXECUTE);
		writer << (uint8_t)PY_EXEC_NONE;
		writer.endMessage();
	}
	catch (bp::error_already_set &)
	{
		PyUtil_ShowErr("PyConsole client exception");
		
		Writer writer(*this);
		writer.beginMessage(PYS_ACK_EXECUTE);
		writer << (uint8_t)PY_EXEC_EXCEPTION;
		writer.writeString(std::string("Python syntax/runtime error"));
		writer.endMessage();
	}
	catch (std::exception & e)
	{
		WARN("%s", e.what());
	}
}

void py_client::onMessageReceived(Reader & msg)
{
	switch (msg.messageId())
	{
		case PYS_REQ_AUTHENTICATE:
			onRequestAuthenticate(msg);
			break;

		case PYS_REQ_EXECUTE:
			onRequestExecute(msg);
			break;

		case PYS_REQ_EVALUATE:
			onRequestEvaluate(msg);
			break;

		default:
			WARN("Unknown PyWebClient message received: %04x", msg.messageId());
	}
}

void py_client::onConnected(const boost::system::error_code & errcode)
{
	state_ = PY_CLIENT_AUTH_WAIT;
}

void py_client::onDisconnected(const boost::system::error_code & errcode)
{
}

void py_client::unregisterConnection()
{
}

