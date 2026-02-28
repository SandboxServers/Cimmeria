#ifndef SERVERCONNECTOR_H
#define SERVERCONNECTOR_H

#include <QObject>
#include <QtNetwork/QTcpSocket>

class ServerConnector : public QObject
{
    Q_OBJECT
public:
    enum MessageId
    {
        MsgReqAuthenticate = 0x01,
        MsgAckAuthenticate = 0x02,
        MsgReqEvaluate = 0x03,
        MsgAckEvaluate = 0x04,
        MsgReqExecute = 0x05,
        MsgAckExecute = 0x06
    };

    enum ObjectType
    {
        ObjNone = 0x00,
        ObjException = 0x01,
        ObjInteger = 0x02,
        ObjFloat = 0x03,
        ObjString = 0x04
    };

#pragma pack(push, 1)
    struct MessageHeader
    {
        quint32 length;
        quint8 messageId;
    };
#pragma pack(pop)

    class AsyncRequest
    {
    public:
        virtual void execute(ServerConnector * connector) = 0;
        virtual void onExecAck(bool failed, QString exceptionMsg) = 0;
        virtual void onEvalAck(bool failed, QVariant result) = 0;
    };

    explicit ServerConnector(QObject *parent = 0);

    void setServerAddress(QString host, qint16 port);
    void setPassword(QString password);

    void submitRequest(AsyncRequest * request);
    void completedRequest();

    void requestExec(QString expr);
    void requestEval(QString expr);
    
signals:

    
private slots:
    void connected();
    void readyRead();
    void bytesWritten(qint64 bytes);
    void error(QAbstractSocket::SocketError code);
    

private:
    void openConnection();
    void startNextRequest();
    void connectionFailed(QString message);

    void requestLogin(QString password);
    void sendMessage(quint8 opcode, QByteArray message);

    void handleMessage(quint8 opcode, QByteArray message);
    void handleLoginAck(QByteArray message);
    void handleEvalAck(QByteArray message);
    void handleExecAck(QByteArray message);

    QTcpSocket * connection_;
    QString host_, password_;
    qint16 port_;

    QByteArray receiveBuf_, sendBuf_;
    bool sending_;
    bool connecting_, ready_;

    QVector<AsyncRequest *> requests_;
    AsyncRequest * currentRequest_;
};

class ReloadScriptRequest : public ServerConnector::AsyncRequest
{
public:
    ReloadScriptRequest(QString playerName, QString scriptModule, QString type);

    virtual void execute(ServerConnector * connector);
    virtual void onExecAck(bool failed, QString exceptionMsg);
    virtual void onEvalAck(bool failed, QVariant result);
private:
    QString playerName_, scriptModule_, type_;
    ServerConnector * connector_;
};

#endif // SERVERCONNECTOR_H
