#include "qneitem.h"
#include "qnodeseditor.h"

#include <QGraphicsScene>

QNEItem::QNEItem(QNodesEditor * editor, QGraphicsScene * scene) : QGraphicsItem(),
    needsLayout_(false), editor_(editor), id_(0)
{
    if (editor_)
        editor_->registerItem(this);
    if (scene)
        scene->addItem(this);
    layoutChanged();
}

QNEItem::~QNEItem()
{
    if (editor_)
        editor_->unregisterItem(this);
}

unsigned int QNEItem::id() const
{
    return id_;
}

void QNEItem::setId(unsigned int id)
{
    id_ = id;
}

QString const & QNEItem::comment() const
{
    return comment_;
}

void QNEItem::setComment(QString const & comment)
{
    comment_ = comment;
    layoutChanged();
    updateLayout();
}

QNodesEditor * QNEItem::editor() const
{
    return editor_;
}

void QNEItem::layoutChanged()
{
    needsLayout_ = true;
}

void QNEItem::updateLayout()
{
    if (needsLayout_)
    {
        needsLayout_ = false;
        updateItemLayout();
    }
}

