#ifndef CHAINMODEL_H
#define CHAINMODEL_H

#include <QString>
#include <QVector>

// ---------------------------------------------------------------------------
// Plain data structures for the content-chain editor.
// No Qt model class is used; data is held directly in these structs and
// displayed via QTreeWidget / QTableWidget in ChainEditorWidget.
// ---------------------------------------------------------------------------

struct ChainTrigger
{
    int    triggerId;    // 0 = not yet in DB
    int    chainId;
    QString eventType;
    QString eventKey;    // empty string maps to NULL in DB
    QString scope;       // "player" | "space" | "global"
    bool   once;
    int    sortOrder;

    ChainTrigger()
        : triggerId(0)
        , chainId(0)
        , once(false)
        , sortOrder(0)
    {}
};

struct ChainCondition
{
    int    conditionId;  // 0 = not yet in DB
    int    chainId;
    QString conditionType;
    int    targetId;     // -1 = NULL
    QString targetKey;   // empty = NULL
    QString op;          // eq | neq | gt | lt | gte | lte
    QString value;       // empty = NULL
    int    sortOrder;

    ChainCondition()
        : conditionId(0)
        , chainId(0)
        , targetId(-1)
        , sortOrder(0)
    {}
};

struct ChainAction
{
    int    actionId;     // 0 = not yet in DB
    int    chainId;
    QString actionType;
    int    targetId;     // -1 = NULL
    QString targetKey;   // empty = NULL
    QString params;      // JSON string, default '{}'
    int    delayMs;
    int    sortOrder;

    ChainAction()
        : actionId(0)
        , chainId(0)
        , targetId(-1)
        , delayMs(0)
        , sortOrder(0)
    {}
};

struct ContentChain
{
    int    chainId;      // 0 = not yet in DB
    QString description;
    QString scopeType;   // "mission" | "space" | "effect" | "global"
    int    scopeId;      // -1 = NULL
    bool   enabled;
    int    priority;

    QVector<ChainTrigger>   triggers;
    QVector<ChainCondition> conditions;
    QVector<ChainAction>    actions;

    ContentChain()
        : chainId(0)
        , scopeId(-1)
        , enabled(true)
        , priority(0)
    {}
};

#endif // CHAINMODEL_H
