#ifndef SCRIPTNODESEDITOR_H
#define SCRIPTNODESEDITOR_H

#include "qnodeseditor.h"
#include "scriptdefinitions.h"
#include <QVariant>
#include <QVector>

class QMenu;

class ScriptNodesEditor : public QNodesEditor
{
    Q_OBJECT
public:
    explicit ScriptNodesEditor(ScriptDefinitions * definitions, QObject *parent = 0);

    void hideUnusedPorts(QNEBlock * block);
    void showAllPorts(QNEBlock * block);

    bool save(QString const & path);
    bool load(QString const & path);

    virtual QString getPortType(QNEPort * port);
    virtual QVariant::Type getPropertyType(QNEBlock * block, QString const & property);

    QString scriptVersion() const;
    QVector<QNEItem *> items() const;
    ScriptDefinitions * definitions() const;

    QString getScriptModule() const;
    void setScriptModule(QString module);

    QString getScriptType() const;
    void setScriptType(QString type);

signals:
    void blockSelected(QNEBlock * block, ScriptDefinitions::NodeTemplate const * tpl);
    void commentSelected(QNEComment * comment);
    
public slots:


private:
    virtual bool beforeConnect(QNEPort * outPort, QNEPort * inPort);
    virtual void showContextMenu(QNEItem * item, QPoint const & position, QPointF const & scenePosition);
    virtual void selectionChanged(QNEItem * item);

    void showContextMenu(QNEBlock * block, QPoint const & position, QPointF const & scenePosition);
    void showContextMenu(QNEComment * comment, QPoint const & position, QPointF const & scenePosition);
    void showContextMenu(QNEConnection * connection, QPoint const & position, QPointF const & scenePosition);
    void showContextMenu(QNEPort * port, QPoint const & position, QPointF const & scenePosition);
    void showContextMenu(QPoint const & position, QPointF const & scenePosition);
    void createNewNodeMenu(QMenu * parent, QString const & name, QString const & type);
    QNEBlock * createBlockFromTemplate(ScriptDefinitions::NodeTemplate const * tpl);

    ScriptDefinitions * definitions_;
    QString module_, type_;
};

#endif // SCRIPTNODESEDITOR_H
