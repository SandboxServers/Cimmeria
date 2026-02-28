#include "scriptnodeseditor.h"
#include "qneblock.h"
#include "qneconnection.h"
#include "qnecomment.h"

#include <QMenu>
#include <QtXml/QDomDocument>
#include <QtXml/QDomElement>
#include <QMessageBox>
#include <QXmlStreamWriter>
#include <QFile>
#include <QDebug>
#include <QBuffer>
#include <QToolTip>
#include <QHelpEvent>

class ToolTipMenu : public QMenu
{
public:
    ToolTipMenu(QWidget * parent)
        : QMenu(parent)
    {}

    bool event(QEvent* e)
    {
        if (e->type() == QEvent::ToolTip)
        {
            QHelpEvent * helpEvent = dynamic_cast<QHelpEvent*>(e);
            QAction * action = actionAt(helpEvent->pos());
            if (action && action->toolTip().length() > 0 && action->toolTip() != action->text())
            {
                QToolTip::showText(helpEvent->globalPos(), action->toolTip(), this);
                return true;
            }
        }
        else if (e->type() == QEvent::Paint && QToolTip::isVisible())
        {
            QToolTip::hideText();
        }

        return QMenu::event(e);
    }
};

ScriptNodesEditor::ScriptNodesEditor(ScriptDefinitions * definitions, QObject *parent) :
    QNodesEditor(parent), definitions_(definitions), type_("Mission")
{
}

void ScriptNodesEditor::hideUnusedPorts(QNEBlock * block)
{
    QVector<QNEPort *> ports = block->ports();
    foreach (QNEPort * port, ports)
    {
        if (!port->isConnected() && !port->hasAllFlags(QNEPort::AlwaysShow))
        {
            port->setPortFlag(QNEPort::Hidden);
        }
    }
    block->layoutChanged();
    block->update();
}

void ScriptNodesEditor::showAllPorts(QNEBlock * block)
{
    QVector<QNEPort *> ports = block->ports();
    foreach (QNEPort * port, ports)
    {
        port->clearPortFlag(QNEPort::Hidden);
    }
    block->layoutChanged();
    block->update();
}

bool ScriptNodesEditor::save(QString const & path)
{
    qDebug("Saving script data to %s ...", qPrintable(path));
    QBuffer buffer;
    buffer.open(QBuffer::ReadWrite);
    QXmlStreamWriter writer(&buffer);
    writer.writeStartDocument();
    writer.writeCharacters("\r\n");
    writer.writeStartElement("Script");
    writer.writeAttribute("Version", scriptVersion());
    writer.writeAttribute("DatasetVersion", definitions_->datasetVersion());
    writer.writeAttribute("Module", module_);
    writer.writeAttribute("Type", type_);
    writer.writeAttribute("NextId", QString::number(nextId_));
    writer.writeCharacters("\r\n");

    qDebug("Saving objects ...");
    foreach (QNEItem * item, items_)
    {
        item->save(&writer);
    }

    writer.writeEndElement();
    writer.writeEndDocument();

    QString tempPath = path + ".tmp";
    QFile file(tempPath);
    if (!file.open(QFile::WriteOnly))
    {
        QMessageBox::warning(nullptr, "Save Failed", "Failed to open file '" + path + "' for writing");
        return false;
    }
    file.write(buffer.buffer());
    file.close();
    bool hasOld = QFile(path).exists();
    if (hasOld)
    {
        if (!QFile::rename(path, path + ".old"))
        {
            QMessageBox::warning(nullptr, "Save Failed", "Failed to rename old file '" + path + "'");
            return false;
        }
    }
    if (!QFile::rename(tempPath, path))
    {
        QMessageBox::warning(nullptr, "Save Failed", "Failed to rename file '" + tempPath + "'");
        return false;
    }
    if (hasOld)
        QFile(path + ".old").remove();
    qDebug("Save completed.");
    changesFlushed();
    return true;
}

bool ScriptNodesEditor::load(QString const & path)
{
    changesFlushed();

    qDebug("Loading script data from %s ...", qPrintable(path));
    QFile file(path);
    if (!file.open(QFile::ReadOnly))
    {
        QMessageBox::warning(nullptr, "Load Failed", "Failed to open file '" + path + "' for reading");
        return false;
    }

    QDomDocument doc;
    if (!doc.setContent(file.readAll()))
    {
        QMessageBox::warning(nullptr, "Load Failed", "Failed to parse XML file '" + path + "'");
        return false;
    }

    qDebug("Loading nodes ...");
    QDomElement root = doc.firstChildElement("Script");
    if (root.isNull())
    {
        QMessageBox::warning(nullptr, "Load Failed", "Script file has no root node!");
        return false;
    }

    QString version = root.attribute("Version", "Unknown");
    if (version != scriptVersion())
        qWarning("Script file version (%s) differs from editor version (%s)", qPrintable(version), qPrintable(scriptVersion()));

    QString dataset = root.attribute("DatasetVersion", "Unknown");
    if (dataset != definitions_->datasetVersion())
        qWarning("Script file was created with a different version of the dataset (%s) than the editor (%s)",
                 qPrintable(dataset), qPrintable(definitions_->datasetVersion()));

    module_ = root.attribute("Module");
    if (module_ == "")
        qWarning("Script has no Python module name set.");

    type_ = root.attribute("Type");
    if (type_ == "")
    {
        type_ = "Mission";
        qWarning("Script contains no type information. Set type to '%s'.", qPrintable(type_));
    }

    nextId_ = root.attribute("NextId").toInt();
    bool hasNames = false;
    for (QDomElement node = root.firstChildElement("Node"); !node.isNull(); node = node.nextSiblingElement("Node"))
    {
        ScriptDefinitions::NodeTemplate * nodeTpl;
        QString ref = node.attribute("Ref");
        if (ref.length() == 0)
        {
            QString name = node.attribute("Name");
            nodeTpl = definitions_->findNode(name);
            if (nodeTpl)
                hasNames = true;
        }
        else
            nodeTpl = definitions_->findNodeByRef(ref);

        if (nodeTpl)
        {
            if (!nodeTpl->scriptTypes.empty() && !nodeTpl->scriptTypes.contains(type_))
            {
                qWarning("Node '%s' is not allowed in scripts of type '%s'", qPrintable(ref), qPrintable(type_));
            }
            else
            {
                QNEBlock * block = createBlockFromTemplate(nodeTpl);
                block->load(&node);
            }
        }
        else
            qWarning("Unknown node type %s (%s); not importing node", qPrintable(ref), qPrintable(node.attribute("Name")));
    }

    if (hasNames)
    {
        qWarning("Script has legacy nodes that are referenced by name. The script will be upgraded to node refs when the script is saved.");
        setDirty();
    }

    qDebug("Loading connections ...");
    for (QDomElement conn = root.firstChildElement("Connection"); !conn.isNull(); conn = conn.nextSiblingElement("Connection"))
    {
        QNEConnection * c = new QNEConnection(this, scene());
        if (!c->load(&conn))
            delete c;
    }

    qDebug("Loading comments ...");
    for (QDomElement comm = root.firstChildElement("Comment"); !comm.isNull(); comm = comm.nextSiblingElement("Comment"))
    {
        QNEComment * comment = new QNEComment(this, scene());
        comment->load(&comm);
    }

    qDebug("Load completed.");
    return true;
}

QString ScriptNodesEditor::getPortType(QNEPort * port)
{
    Q_ASSERT(port->getTemplate());
    return port->getTemplate()->type;
}

QVariant::Type ScriptNodesEditor::getPropertyType(QNEBlock * block, QString const & property)
{
    ScriptDefinitions::PropertyTemplate const * prop = block->getTemplate()->findProperty(property);
    if (prop)
        return prop->variantType;
    else
        return QVariant::Invalid;
}

QString ScriptNodesEditor::scriptVersion() const
{
    return "1.4";
}

QVector<QNEItem *> ScriptNodesEditor::items() const
{
    return items_;
}

ScriptDefinitions * ScriptNodesEditor::definitions() const
{
    return definitions_;
}

QString ScriptNodesEditor::getScriptModule() const
{
    return module_;
}

void ScriptNodesEditor::setScriptModule(QString module)
{
    module_ = module;
}

QString ScriptNodesEditor::getScriptType() const
{
    return type_;
}

void ScriptNodesEditor::setScriptType(QString type)
{
    Q_ASSERT(type == "Level" || type == "Mission");
    type_ = type;
}

bool ScriptNodesEditor::beforeConnect(QNEPort * outPort, QNEPort * inPort)
{
    if (outPort->getTemplate()->type != inPort->getTemplate()->type)
        return false;

    return true;
}

void ScriptNodesEditor::showContextMenu(QPoint const & position, QPointF const & scenePosition)
{
    QMenu menu;
    createNewNodeMenu(&menu, "New Action", "Action");
    createNewNodeMenu(&menu, "New Condition", "Condition");
    createNewNodeMenu(&menu, "New Variable", "Variable");
    createNewNodeMenu(&menu, "New Event", "Event");
    menu.addAction("New Comment")->setData(QVariant((QChar)1));
    QAction * pasteAction = menu.addAction("Paste");
    pasteAction->setData(QVariant((QChar)2));
    pasteAction->setEnabled(clipboard() != nullptr);
    QAction * action = menu.exec(position);
    if (action)
    {
        QVariant type = action->data();
        if (type.type() == QVariant::Char)
        {
            QChar option = type.value<QChar>();
            if (option == 1)
            {
                QNEComment * comment = new QNEComment(this, scene());
                comment->setPos(scenePosition);
                comment->setComment("Comment");
                comment->setBoxSize(QSizeF(150, 150));
                setDirty();
            }
            else if (option == 2)
            {
                QNEItem * item = paste();
                if (item)
                {
                    item->setPos(scenePosition);
                    setDirty();
                }
            }
        }
        else if (type.type() == QVariant::ULongLong)
        {
            ScriptDefinitions::NodeTemplate * tpl = (ScriptDefinitions::NodeTemplate *)type.value<quint64>();
            QNEBlock * block = createBlockFromTemplate(tpl);
            block->setId(nextId_++);
            block->setPos(scenePosition);
            setDirty();
        }
    }
}


void ScriptNodesEditor::showContextMenu(QNEItem * item, QPoint const & position, QPointF const & scenePosition)
{
    if (!item)
        showContextMenu(position, scenePosition);
    else if (item->type() == QNEBlock::Type)
        showContextMenu(static_cast<QNEBlock *>(item), position, scenePosition);
    else if (item->type() == QNEConnection::Type)
        showContextMenu(static_cast<QNEConnection *>(item), position, scenePosition);
    else if (item->type() == QNEComment::Type)
        showContextMenu(static_cast<QNEComment *>(item), position, scenePosition);
    else if (item->type() == QNEPort::Type)
        showContextMenu(reinterpret_cast<QNEPort *>(item), position, scenePosition);
}


void ScriptNodesEditor::showContextMenu(QNEBlock * block, QPoint const & position, QPointF const & scenePosition)
{
    QMenu menu;
    menu.addAction("Remove " + block->blockType())->setData(QVariant((QChar)1));
    menu.addAction("Break all links")->setData(QVariant((QChar)2));
    menu.addAction("Hide unused ports")->setData(QVariant((QChar)3));
    menu.addAction("Show all ports")->setData(QVariant((QChar)4));
    menu.addAction("Copy " + block->blockType())->setData(QVariant((QChar)5));
    QAction * action = menu.exec(position);
    if (action)
    {
        QVariant type = action->data();
        if (type.type() == QVariant::Char)
        {
            if (type.value<QChar>() == 1)
                removeItem(block);
            else if (type.value<QChar>() == 2)
                breakAllLinks(block);
            else if (type.value<QChar>() == 3)
                hideUnusedPorts(block);
            else if (type.value<QChar>() == 4)
                showAllPorts(block);
            else if (type.value<QChar>() == 5)
                copy();
        }
    }
}


void ScriptNodesEditor::showContextMenu(QNEComment * comment, QPoint const & position, QPointF const & scenePosition)
{
    QPointF curPos = scenePosition - comment->pos();
    if (comment->commentArea().contains(curPos))
    {
        QMenu menu;
        menu.addAction("Remove Comment")->setData(QVariant((QChar)1));
        menu.addAction("Copy Comment")->setData(QVariant((QChar)2));
        QAction * action = menu.exec(position);
        if (action)
        {
            QVariant type = action->data();
            if (type.type() == QVariant::Char)
            {
                if (type.value<QChar>() == 1)
                    removeItem(comment);
                else if (type.value<QChar>() == 2)
                    copy();
            }
        }
    }
    else
        showContextMenu(position, scenePosition);
}


void ScriptNodesEditor::showContextMenu(QNEConnection * connection, QPoint const & position, QPointF const & scenePosition)
{
    QMenu menu;
    menu.addAction("Remove Connection")->setData(QVariant((QChar)1));
    QAction * action = menu.exec(position);
    if (action)
    {
        QVariant type = action->data();
        if (type.type() == QVariant::Char)
        {
            if (type.value<QChar>() == 1)
                removeItem(connection);
        }
    }
}


void ScriptNodesEditor::showContextMenu(QNEPort * port, QPoint const & position, QPointF const & scenePosition)
{
    QMenu menu;
    menu.addAction("Break all port links")->setData(QVariant((QChar)1));
    QAction * action = menu.exec(position);
    if (action)
    {
        QVariant type = action->data();
        if (type.type() == QVariant::Char)
        {
            if (type.value<QChar>() == 1)
            {
                port->disconnectAll();
            }
        }
    }
}


void ScriptNodesEditor::selectionChanged(QNEItem * item)
{
    if (item)
    {
        if (item->type() == QNEBlock::Type)
        {
            QNEBlock * block = static_cast<QNEBlock *>(item);
            emit blockSelected(block, block->getTemplate());
        }
        else if (item->type() == QNEComment::Type)
        {
            QNEComment * comment = static_cast<QNEComment *>(item);
            emit commentSelected(comment);
        }
        else
            emit blockSelected(nullptr, nullptr);
    }
    else
        emit blockSelected(nullptr, nullptr);
}

void ScriptNodesEditor::createNewNodeMenu(QMenu * parent, QString const & name, QString const & type)
{
    QMenu * menu = new ToolTipMenu(parent);
    menu->setTitle(name);
    parent->addMenu(menu);

    QMap<QString, QMenu *> nodeMenus;
    QMap<QString, ScriptDefinitions::NodeTemplate>::ConstIterator iter =
            definitions_->nodes().begin();
    for (; iter != definitions_->nodes().end(); ++iter)
    {
        ScriptDefinitions::NodeTemplate const & tpl = iter.value();
        if (tpl.type == type && (tpl.scriptTypes.empty() || tpl.scriptTypes.contains(type_)))
        {
            QAction * action;
            if (tpl.category.length() == 0)
            {
                action = menu->addAction(tpl.name);
            }
            else if (nodeMenus.find(tpl.category) == nodeMenus.end())
            {
                QMenu * item = new ToolTipMenu(menu);
                item->setTitle(tpl.category);
                menu->addMenu(item);
                nodeMenus.insert(tpl.category, item);
                action = item->addAction(tpl.name);
            }
            else
            {
                action = nodeMenus.find(tpl.category).value()->addAction(tpl.name);
            }
            action->setData((quint64)&tpl);
            action->setToolTip(tpl.description);
            action->setStatusTip(tpl.description);
        }
    }
}

QNEBlock * ScriptNodesEditor::createBlockFromTemplate(ScriptDefinitions::NodeTemplate const * tpl)
{
    QNEBlock * block = new QNEBlock(this, scene());
    block->setName(tpl->name);
    block->setBlockType(tpl->type);
    block->setTemplate(tpl);
    block->setToolTip(tpl->description);

    foreach (ScriptDefinitions::PortTemplate const & port, tpl->ports)
    {
        QNEPort::PortFlags flags;
        if (port.required)
            flags |= QNEPort::AlwaysShow;
        else if (port.defaultHide)
            flags |= QNEPort::Hidden;
        if (port.output)
            flags |= QNEPort::Output;
        if (port.trigger)
            flags |= QNEPort::Trigger;
        if (port.required)
            flags |= QNEPort::Required;
        QNEPort * p = block->addPort(port.name);
        p->setPortType(port.type);
        p->setPortFlags(flags);
        p->setTemplate(&port);
        p->setToolTip(port.description);
    }

    foreach (ScriptDefinitions::PropertyTemplate const & prop, tpl->properties)
    {
        block->setProperty(prop.name, prop.variantDefaultValue);
    }

    return block;
}

