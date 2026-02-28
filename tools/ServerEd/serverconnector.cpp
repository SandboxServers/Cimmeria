#include "serverconnector.h"
#include <QMessageBox>

ServerConnector::ServerConnector(QObject *parent) :
    QObject(parent), sending_(false), connecting_(false), ready_(false), currentRequest_(nullptr)
{
    connection_ = new QTcpSocket(this);
    QObject::connect(connection_, SIGNAL(readyRead()), this, SLOT(readyRead()));
    QObject::connect(connection_, SIGNAL(bytesWritten(qint64)), this, SLOT(bytesWritten(qint64)));
    QObject::connect(connection_, SIGNAL(connected()), this, SLOT(connected()));
    QObject::connect(connection_, SIGNAL(error(QAbstractSocket::SocketError)), this, SLOT(error(QAbstractSocket::SocketError)));
}


void ServerConnector::setServerAddress(QString host, qint16 port)
{
    host_ = host;
    port_ = port;
}


void ServerConnector::setPassword(QString password)
{
    password_ = password;
}

void ServerConnector::submitRequest(AsyncRequest * request)
{
    requests_.push_back(request);
    if (!ready_)
    {
       openConnection();
    }
    else if (currentRequest_ == nullptr)
        startNextRequest();
}

void ServerConnector::completedRequest()
{
    Q_ASSERT(currentRequest_ != nullptr);
    delete currentRequest_;
    currentRequest_ = nullptr;
    startNextRequest();
}


void ServerConnector::openConnection()
{
    if (connecting_)
        return;

    connecting_ = true;
    connection_->connectToHost(host_, port_);
}

void ServerConnector::startNextRequest()
{
    Q_ASSERT(currentRequest_ == nullptr);
    if (!requests_.empty())
    {
        currentRequest_ = *requests_.begin();
        requests_.pop_front();
        currentRequest_->execute(this);
    }
}

void ServerConnector::connectionFailed(QString message)
{
    connecting_ = false;
    ready_ = false;
    sending_ = false;
    sendBuf_.clear();
    receiveBuf_.clear();
    delete currentRequest_;
    currentRequest_ = nullptr;
    while (!requests_.empty())
    {
        delete *requests_.begin();
        requests_.pop_front();
    }
    QMessageBox::warning(nullptr, "Network Error", message);
}

void ServerConnector::connected()
{
    qDebug("Connected to server");
    requestLogin(password_);
}


void ServerConnector::readyRead()
{
    receiveBuf_.append(connection_->readAll());

    while (receiveBuf_.length() >= sizeof(MessageHeader))
    {
        MessageHeader & header = *reinterpret_cast<MessageHeader *>(receiveBuf_.data());

        if (header.length < sizeof(MessageHeader))
            qFatal("Server sent too short message (%d bytes)", header.length);

        if (header.length > 0xffff)
            qFatal("Server sent too long message (%d bytes)", header.length);

        if (header.length <= receiveBuf_.length())
        {
            QByteArray body = receiveBuf_.mid(sizeof(MessageHeader), header.length - sizeof(MessageHeader));
            quint8 messageId = header.messageId;
            receiveBuf_.remove(0, header.length);
            handleMessage(messageId, body);
        }
    }
}


void ServerConnector::bytesWritten(qint64 bytes)
{
    Q_ASSERT(bytes <= sendBuf_.size());
    sendBuf_.remove(0, bytes);
    sending_ = false;

    if (sendBuf_.length() > 0)
    {
        connection_->write(sendBuf_);
        sending_ = true;
    }
}


void ServerConnector::error(QAbstractSocket::SocketError code)
{
    QString message;
    switch (code)
    {
    case QAbstractSocket::ConnectionRefusedError: message = "Connection refused"; break;
    case QAbstractSocket::RemoteHostClosedError: message = "Remote host closed connection"; break;
    case QAbstractSocket::HostNotFoundError: message = "Host not found"; break;
    case QAbstractSocket::SocketTimeoutError: message = "Operation timed out"; break;
    case QAbstractSocket::NetworkError: message = "General network error"; break;
    default: message = QString("Unknown network error: %0").arg((int)code); break;
    }
    connectionFailed("Network error while communicating with the CellApp: " + message);
    connection_->close();
}


void ServerConnector::requestLogin(QString password)
{
    QByteArray body;
    quint32 len = password.length();
    body.append((char *)&len, 4);
    body.append(password.toUtf8());
    sendMessage(0x01, body);
}


void ServerConnector::requestExec(QString expr)
{
    QByteArray body;
    quint32 len = expr.length();
    body.append((char *)&len, 4);
    body.append(expr.toUtf8());
    sendMessage(0x05, body);
}


void ServerConnector::requestEval(QString expr)
{
    QByteArray body;
    quint32 len = expr.length();
    body.append((char *)&len, 4);
    body.append(expr.toUtf8());
    sendMessage(0x03, body);
}


void ServerConnector::sendMessage(quint8 opcode, QByteArray message)
{
    MessageHeader header;
    header.length = message.length() + sizeof(header);
    header.messageId = opcode;
    sendBuf_.append((char *)&header, sizeof(header));
    sendBuf_.append(message);

    if (!sending_)
    {
        connection_->write(sendBuf_);
        sending_ = true;
    }
}


void ServerConnector::handleMessage(quint8 opcode, QByteArray message)
{
    switch (opcode)
    {
    case 0x02: handleLoginAck(message); break;
    case 0x04: handleEvalAck(message); break;
    case 0x06: handleExecAck(message); break;
    default: qFatal("Illegal message received: 0x%02x", opcode);
    }
}


void ServerConnector::handleLoginAck(QByteArray message)
{
    quint32 status = *reinterpret_cast<quint32 *>(message.data());
    if (status != 0)
    {
        QString errmsg = QString("Failed to authenticate with the CellApp: error code %0").arg(status);
        connectionFailed(errmsg);
        return;
    }

    connecting_ = false;
    ready_ = true;
    startNextRequest();
}


void ServerConnector::handleEvalAck(QByteArray message)
{
    quint8 type = *reinterpret_cast<quint8 *>(message.data());
    qDebug("handleEvalAck: code %d", type);
    Q_ASSERT(currentRequest_ != nullptr);

    QVariant arg;
    switch (type)
    {
    case ObjNone:
        break;

    case ObjException:
    case ObjString:
    {
        quint32 length = *reinterpret_cast<quint32 *>(message.data() + 1);
        QByteArray str(message.data() + 5, length);
        arg = QString::fromUtf8(str);
        break;
    }

    case ObjInteger:
        arg = *reinterpret_cast<qint32 *>(message.data() + 1);
        break;

    case ObjFloat:
        arg = *reinterpret_cast<float *>(message.data() + 1);
        break;

    default:
        qFatal("Illegal response object type: %d", type);
    }

    currentRequest_->onEvalAck(type == ObjException, arg);
}


void ServerConnector::handleExecAck(QByteArray message)
{
    quint8 type = *reinterpret_cast<quint8 *>(message.data());
    qDebug("handleExecAck: code %d", type);
    Q_ASSERT(currentRequest_ != nullptr);
    Q_ASSERT(type == ObjNone || type == ObjException);
    if (type == ObjException)
    {
        quint32 length = *reinterpret_cast<quint32 *>(message.data() + 1);
        QByteArray exc(message.data() + 5, length);
        currentRequest_->onExecAck(true, QString::fromUtf8(exc));
    }
    else
    {
        currentRequest_->onExecAck(false, "");
    }
}



ReloadScriptRequest::ReloadScriptRequest(QString playerName, QString scriptModule, QString type)
    : playerName_(playerName), scriptModule_(scriptModule), type_(type)
{
}

void ReloadScriptRequest::execute(ServerConnector * connector)
{
    connector_ = connector;
    connector_->requestExec("import cell.SGWPlayer");
}

void ReloadScriptRequest::onExecAck(bool failed, QString exceptionMsg)
{
    if (!failed)
    {
        connector_->requestEval("cell.SGWPlayer.reloadScript('" + playerName_ + "', '" + scriptModule_ + "', '" + type_ + "')");
    }
    else
    {
        QMessageBox::warning(
            nullptr, "Reload Failed",
            QString("Module import failed: %0").arg(exceptionMsg)
        );
        connector_->completedRequest();
    }
}

void ReloadScriptRequest::onEvalAck(bool failed, QVariant result)
{
    if (failed)
    {
        QMessageBox::warning(
            nullptr, "Reload Failed",
            QString("Reload failed: %0").arg(result.toString())
        );
    }
    else if (!result.toBool())
    {
        QMessageBox::warning(
            nullptr, "Reload Failed",
            "Reload failed: Script is not loaded on the server"
        );
    }
    connector_->completedRequest();
}

