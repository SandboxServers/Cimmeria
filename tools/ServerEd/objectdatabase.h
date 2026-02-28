#ifndef OBJECTDATABASE_H
#define OBJECTDATABASE_H

#include <QObject>
#include <QMap>
#include <QPair>
#include "databaseworker.h"
#include "scriptdefinitions.h"

class ObjectDatabase : public QObject
{
    Q_OBJECT
public:
    struct SearchResult
    {
        QStringList columns;
        QVector<QStringList> rows;
    };

    ObjectDatabase(ScriptDefinitions * defs, DatabaseWorker * worker);

public slots:
    void lookup(QString const & ref, QString const & fragment);
    void lookup(QString const & ref, int id);

signals:
    void submitDbRequest(DatabaseRequest * request);
    void objectLookupCompleted(QString ref, int id, QString name);
    void objectLookupFailed(QString ref, int id, QString reason);
    void nameLookupCompleted(QString ref, QString fragment, ObjectDatabase::SearchResult * results);
    void nameLookupFailed(QString ref, QString fragment, QString reason);

private slots:
    void requestCompleted();
    void requestFailed(QString reason);

private:
    DatabaseWorker * worker_;
    ScriptDefinitions * defs_;

    QMap<DatabaseRequest *, QPair<QString, int> > requestToLookup_;
    QMap<DatabaseRequest *, QPair<QString, QString> > requestToNameLookup_;
    QVector<QPair<QString, int> > pendingLookups_;
    QVector<QPair<QString, QString> > pendingNameLookups_;
    QMap<QPair<QString, int>, QString> objectNames_;
    QMap<QPair<QString, QString>, SearchResult> lookupResults_;
};

#endif // OBJECTDATABASE_H
