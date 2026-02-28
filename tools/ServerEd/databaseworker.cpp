#include "databaseworker.h"
#include "editorconfiguration.h"

#include <QtSql/QSqlQuery>
#include <QtSql/QSqlRecord>

DatabaseWorkerThread::DatabaseWorkerThread(QObject *parent)
    : QThread(parent)
{
}

void DatabaseWorkerThread::run()
{
    sleep(1);
    emit workerStarted();
    exec();
}


DatabaseRequest::DatabaseRequest(QString query)
    : query_(query)
{
}

void DatabaseRequest::execute(QSqlDatabase & database)
{
    QSqlQuery query;
    if (query.exec(query_))
    {
        for (int r = 0; r < query.size(); r++)
        {
            query.next();
            QSqlRecord record = query.record();
            if (columns_.empty())
            {
                for (int i = 0; i < record.count(); i++)
                    columns_.push_back(record.fieldName(i));
            }

            QStringList data;
            for (int i = 0; i < record.count(); i++)
                data.push_back(record.value(i).toString());
            rows_.push_back(data);
        }
        emit completed();
    }
    else
        emit failed(query.lastError().driverText() + "\r\n" + query.lastError().databaseText());
}

QStringList const & DatabaseRequest::columns() const
{
    return columns_;
}

QVector<QStringList> const & DatabaseRequest::rows() const
{
    return rows_;
}


DatabaseWorker::DatabaseWorker(EditorConfiguration * config, QObject *parent) :
    config_(config), QObject(parent)
{
}

void DatabaseWorker::startup()
{
    connection_ = QSqlDatabase::addDatabase("QPSQL");
    connection_.setHostName(config_->get("DbHost").toString());
    connection_.setPort(config_->get("DbPort").toInt());
    connection_.setDatabaseName(config_->get("DbName").toString());
    connection_.setUserName(config_->get("DbUser").toString());
    connection_.setPassword(config_->get("DbPassword").toString());
    bool connected = connection_.open();
    if (!connected)
        emit error(qPrintable(connection_.lastError().driverText() + "\r\n" + connection_.lastError().databaseText()));
}

void DatabaseWorker::submit(DatabaseRequest * request)
{
    Q_ASSERT(request->thread() == QThread::currentThread());
    request->execute(connection_);
}
