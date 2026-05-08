#include "objectdatabase.h"

ObjectDatabase::ObjectDatabase(ScriptDefinitions *defs, DatabaseWorker *worker)
    : QObject(0), defs_(defs), worker_(worker)
{
    connect(this, SIGNAL(submitDbRequest(DatabaseRequest*)), worker, SLOT(submit(DatabaseRequest*)));
}


void ObjectDatabase::lookup(QString const & ref, QString const & fragment)
{
    auto resIt = lookupResults_.find(QPair<QString, QString>(ref, fragment));
    if (resIt != lookupResults_.end())
    {
        qDebug("Returning cached name for %s (%s)", qPrintable(ref), qPrintable(fragment));
        emit nameLookupCompleted(ref, fragment, &resIt.value());
        return;
    }

    if (pendingNameLookups_.contains(QPair<QString, QString>(ref, fragment)))
    {
        qDebug("Name lookup already pending for %s (%s)", qPrintable(ref), qPrintable(fragment));
        return;
    }

    qDebug("Requesting name lookup for %s (%s)", qPrintable(ref), qPrintable(fragment));
    ScriptDefinitions::DatabaseRef * dbRef = defs_->findDatabaseRef(ref);
    Q_ASSERT(dbRef);
    QString query = QString(dbRef->nameLookupQuery).replace("{VALUE}", fragment);
    DatabaseRequest * req = new DatabaseRequest(query);
    QObject::connect(req, SIGNAL(completed()), this, SLOT(requestCompleted()));
    QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(requestFailed(QString)));
    req->moveToThread(worker_->thread());
    pendingNameLookups_.push_back(QPair<QString, QString>(ref, fragment));
    requestToNameLookup_.insert(req, QPair<QString, QString>(ref, fragment));
    emit submitDbRequest(req);
}


void ObjectDatabase::lookup(QString const & ref, int id)
{
    auto resIt = objectNames_.find(QPair<QString, int>(ref, id));
    if (resIt != objectNames_.end())
    {
        qDebug("Returning cached name for ID %s (%d)", qPrintable(ref), id);
        emit objectLookupCompleted(ref, id, resIt.value());
        return;
    }

    if (pendingLookups_.contains(QPair<QString, int>(ref, id)))
    {
        qDebug("ID lookup already pending for %s (%d)", qPrintable(ref), id);
        return;
    }

    qDebug("Requesting ID lookup for %s (%d)", qPrintable(ref), id);
    ScriptDefinitions::DatabaseRef * dbRef = defs_->findDatabaseRef(ref);
    Q_ASSERT(dbRef);
    QString query = QString(dbRef->idLookupQuery).replace("{VALUE}", QString::number(id));
    DatabaseRequest * req = new DatabaseRequest(query);
    QObject::connect(req, SIGNAL(completed()), this, SLOT(requestCompleted()));
    QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(requestFailed(QString)));
    req->moveToThread(worker_->thread());
    pendingLookups_.push_back(QPair<QString, int>(ref, id));
    requestToLookup_.insert(req, QPair<QString, int>(ref, id));
    emit submitDbRequest(req);
}


void ObjectDatabase::requestCompleted()
{
    auto req = static_cast<DatabaseRequest *>(sender());
    auto objIt = requestToLookup_.find(req);
    if (objIt != requestToLookup_.end())
    {
        QString name;
        if (req->rows().empty())
        {
            name = "(Invalid ID)";
        }
        else if (req->columns().size() != 1)
        {
            name = "(Invalid result set)";
        }
        else
            name = req->rows()[0].value(0);

        qDebug("ID lookup for %s (%d) returned: %s", qPrintable(objIt.value().first), objIt.value().second, qPrintable(name));
        objectNames_.insert(objIt.value(), name);
        emit objectLookupCompleted(objIt.value().first, objIt.value().second, name);
        requestToLookup_.erase(objIt);
    }
    else
    {
        auto nameIt = requestToNameLookup_.find(req);
        if (nameIt != requestToNameLookup_.end())
        {
            SearchResult results;
            results.columns = req->columns();
            results.rows = req->rows();

            qDebug("Name lookup for %s (%s) returned %d results", qPrintable(nameIt.value().first), qPrintable(nameIt.value().second), results.rows.size());
            lookupResults_.insert(nameIt.value(), results);
            emit nameLookupCompleted(nameIt.value().first, nameIt.value().second, &results);
            requestToNameLookup_.erase(nameIt);
        }
        else
            qFatal("Received completion callback for invalid database request!");
    }
}


void ObjectDatabase::requestFailed(QString reason)
{
    auto req = static_cast<DatabaseRequest *>(sender());
    auto objIt = requestToLookup_.find(req);
    if (objIt != requestToLookup_.end())
    {
        emit objectLookupFailed(objIt.value().first, objIt.value().second, reason);
        requestToLookup_.erase(objIt);
    }
    else
    {
        auto nameIt = requestToNameLookup_.find(req);
        if (nameIt != requestToNameLookup_.end())
        {
            emit nameLookupFailed(nameIt.value().first, nameIt.value().second, reason);
            requestToNameLookup_.erase(nameIt);
        }
        else
            qFatal("Received failure callback for invalid database request!");
    }
}

