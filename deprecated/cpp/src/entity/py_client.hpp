#pragma once

#include <mercury/unified_connection.hpp>
#include <mercury/tcp_server.hpp>

enum py_client_cmd
{
	PYS_REQ_AUTHENTICATE = 0x01,
	PYS_ACK_AUTHENTICATE = 0x02,
	PYS_REQ_EVALUATE = 0x03,
	PYS_ACK_EVALUATE = 0x04,
	PYS_REQ_EXECUTE = 0x05,
	PYS_ACK_EXECUTE = 0x06
};

enum py_client_state
{
	PY_CLIENT_AUTH_WAIT,
	PY_CLIENT_OK
};

enum py_exec_response
{
	PY_EXEC_NONE = 0,
	PY_EXEC_EXCEPTION = 1,
	PY_EXEC_INTEGER = 2,
	PY_EXEC_FLOAT = 3,
	PY_EXEC_STRING = 4
};

class py_client : public Mercury::UnifiedConnection
{
	public:
		typedef boost::shared_ptr<py_client> Ptr;

		static py_client * create(Mercury::TcpServer<py_client> & server, uint32_t connection_id);

		py_client();
		~py_client();

		void startup();

	protected:
		py_client_state state_;

		// Overridden handlers of unified connection
		virtual void onMessageReceived(Reader & msg);
		virtual void onConnected(const boost::system::error_code & errcode);
		virtual void onDisconnected(const boost::system::error_code & errcode);
		virtual void unregisterConnection();

	private:
		void onRequestAuthenticate(Reader & msg);
		void onRequestExecute(Reader & msg);
		void onRequestEvaluate(Reader & msg);
};

