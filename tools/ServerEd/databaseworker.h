#ifndef DATABASEWORKER_H
#define DATABASEWORKER_H

#include <QObject>
#include <QThread>
#include <QVector>
#include <QStringList>
#include <QtSql/QSqlDatabase>
#include <QtSql/QSqlError>

class DatabaseWorkerThread : public QThread
{
    Q_OBJECT
public:
    explicit DatabaseWorkerThread(QObject *parent = 0);

signals:
    void workerStarted();

protected:
    virtual void run();
};


class DatabaseRequest : public QObject
{
    Q_OBJECT
public:
    explicit DatabaseRequest(QString query);

    void execute(QSqlDatabase & database);

    QStringList const & columns() const;
    QVector<QStringList> const & rows() const;

signals:
    void completed();
    void failed(QString message);

private:
    QString query_;
    QStringList columns_;
    QVector<QStringList> rows_;
};


class DatabaseWorker : public QObject
{
    Q_OBJECT
public:
    explicit DatabaseWorker(class EditorConfiguration * config, QObject *parent = 0);

public slots:
    void startup();
    void submit(DatabaseRequest * request);

signals:
    void finished();
    void error(QString err);

private:
    EditorConfiguration * config_;
    QSqlDatabase connection_;
};

#endif // DATABASEWORKER_H
