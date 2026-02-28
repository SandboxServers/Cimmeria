/* Copyright (c) 2012, STANISLAW ADASZEWSKI
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.
    * Neither the name of STANISLAW ADASZEWSKI nor the
      names of its contributors may be used to endorse or promote products
      derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL STANISLAW ADASZEWSKI BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE. */

#include "qnodeseditor.h"
#include "qneport.h"
#include "qneconnection.h"
#include "qnecomment.h"
#include "qneblock.h"

#include <QGraphicsScene>
#include <QEvent>
#include <QKeyEvent>
#include <QGraphicsSceneMouseEvent>
#include <QGraphicsView>
#include <QMessageBox>
#include <QScrollBar>


QNodesEditor::QNodesEditor(QObject *parent) :
    QObject(parent), nextId_(1), scene_(0), conn_(0), clipboard_(0),
    resizing_(false), draggingCanvas_(false), translating_(false), changed_(false)
{
}


QNodesEditor::~QNodesEditor()
{
    scene_->removeEventFilter(this);
    while (!items_.empty())
        delete items_[0];
}


void QNodesEditor::install(QGraphicsScene *s)
{
	s->installEventFilter(this);
    scene_ = s;
    QGraphicsRectItem * frame = new QGraphicsRectItem();
    frame->setRect(-4000, -4000, 8000, 8000);
    frame->setBrush(QColor(0xA9A9A9));
    frame->setPen(QPen(QColor(Qt::white)));
    frame->setZValue(-1000);
    scene_->addItem(frame);
}


QGraphicsScene * QNodesEditor::scene() const
{
    return scene_;
}


void QNodesEditor::registerItem(QNEItem * item)
{
    items_.append(item);
}


void QNodesEditor::unregisterItem(QNEItem * item)
{
    items_.remove(items_.indexOf(item));
}


void QNodesEditor::breakAllLinks(QNEBlock * block)
{
    QVector<QNEPort *> ports = block->ports();
    foreach (QNEPort * port, ports)
    {
        QVector<QNEConnection *> & conns = port->connections();
        while (!conns.empty())
        {
            removeItem(conns[0]);
            setDirty();
        }
    }
}


void QNodesEditor::removeItem(QNEItem * item)
{
    setDirty();
    if (item->type() == QNEBlock::Type)
        breakAllLinks(static_cast<QNEBlock *>(item));
    beforeRemoveItem(item);
    if (clipboard_ == item)
        clipboard_ = nullptr;
    if (item->isSelected())
    {
        selectionChanged(nullptr);
    }
    delete item;
}


QNEItem * QNodesEditor::copy()
{
    QList<QGraphicsItem *> items = scene()->selectedItems();
    if (items.length() > 1)
    {
        QMessageBox::warning(nullptr, "Warning", "Copying multiple nodes is not supported yet!");
        return nullptr;
    }

    if (!items.empty() && items[0]->type() > QGraphicsItem::UserType && items[0]->type() != QNEConnection::Type)
    {
        clipboard_ = static_cast<QNEConnection *>(items[0]);
    }
    else
        clipboard_ = nullptr;
    return clipboard_;
}


QNEItem * QNodesEditor::paste()
{
    if (!clipboard_)
        return nullptr;

    QNEItem * item = clipboard_->clone();
    item->setId(nextId_++);
    item->setPos(item->pos() + QPointF(50, 50));
    return item;
}

QNEItem * QNodesEditor::clipboard() const
{
    return clipboard_;
}

bool QNodesEditor::hasChanged() const
{
    return changed_;
}

void QNodesEditor::setDirty()
{
    changed_ = true;
    emit dirty(changed_);
}

void QNodesEditor::changesFlushed()
{
    changed_ = false;
    emit dirty(changed_);
}


QNEItem * QNodesEditor::itemAt(const QPointF &pos)
{
    QList<QGraphicsItem *> items = scene_->items(QRectF(pos - QPointF(2,2), QSize(2,2)));

    foreach(QGraphicsItem * item, items)
    {
		if (item->type() > QGraphicsItem::UserType)
        {
            return static_cast<QNEItem *>(item);
        }
    }

	return 0;
}

bool QNodesEditor::eventFilter(QObject *o, QEvent *e)
{
	QGraphicsSceneMouseEvent *me = (QGraphicsSceneMouseEvent*) e;

    switch ((int)e->type())
	{
        case QEvent::KeyPress:
        {
            QKeyEvent * event = static_cast<QKeyEvent *>(e);
            if (event->modifiers() == Qt::ControlModifier)
            {
                if (event->key() == Qt::Key_C)
                {
                    copy();
                }

                if (event->key() == Qt::Key_V)
                {
                    paste();
                }
            }
            break;
        }

        case QEvent::GraphicsSceneMousePress:
        {
            switch ((int) me->button())
            {
                case Qt::LeftButton:
                {
                    QNEItem * item = itemAt(me->scenePos());
                    resizing_ = false;
                    if (!(me->modifiers() & Qt::ControlModifier))
                    {
                        draggingCanvas_ = true;
                    }

                    if (item)
                    {
                        selectionChanged(item);
                        // The widget only selects items when releasing the mouse, so we need to
                        // click twice to drag the item by default.
                        // This "hack" fixes this behavior
                        item->setSelected(true);

                        if (!draggingCanvas_ && item->type() == QNEComment::Type)
                        {
                            QNEComment * comment = static_cast<QNEComment *>(item);
                            QPointF bmax = comment->boundingRect().bottomRight() + comment->pos();
                            QPointF cursor = me->scenePos();
                            QPointF dist = (bmax - cursor);
                            float length = dist.x() + dist.y();

                            if ((me->modifiers() & Qt::ControlModifier) && length >= 0 && length < QNEComment::ResizeHandleSize * 1.5f)
                            {
                                resizing_ = true;
                                setDirty();
                                return true;
                            }
                        }

                        if (!draggingCanvas_ && item->type() == QNEPort::Type)
                        {
                            // FIXME FIXME FIXME
                            // Fix type conversion hack
                            QNEPort * port = reinterpret_cast<QNEPort *>(item);
                            conn_ = new QNEConnection(this, scene_);
                            conn_->setColor(port->getConnectionColor());
                            conn_->setOutputPort(port);
                            conn_->setOutputPos(item->scenePos());
                            conn_->setInputPos(me->scenePos());
                            conn_->updatePath();
                            return true;
                        }
                    }
                    else
                    {
                        selectionChanged(nullptr);
                    }

                    break;
                }

                case Qt::RightButton:
                {
                    QNEItem * item = itemAt(me->scenePos());
                    showContextMenu(item, me->screenPos(), me->scenePos());
                    break;
                }
            }
            break;
        }

        case QEvent::GraphicsSceneMouseMove:
        {
            if (draggingCanvas_)
            {
                if (!translating_)
                {
                    translating_ = true;
                    QPointF dist = me->lastScenePos() - me->scenePos();
                    QGraphicsView * view = scene()->views()[0];
                    view->horizontalScrollBar()->setValue(view->horizontalScrollBar()->value() + dist.x());
                    view->verticalScrollBar()->setValue(view->verticalScrollBar()->value() + dist.y());
                    translating_ = false;
                }
            }
            else if (me->buttons() & Qt::LeftButton)
            {
                QList<QGraphicsItem *> selection = scene()->selectedItems();
                QNEItem * selected = nullptr;
                if (selection.length() == 1 && selection[0]->type() > QGraphicsItem::UserType)
                    selected = static_cast<QNEItem *>(selection[0]);

                if (conn_)
                {
                    conn_->setInputPos(me->scenePos());
                    conn_->updatePath();
                    return true;
                }
                else if (selected && resizing_)
                {
                    QNEComment * comment = static_cast<QNEComment *>(selected);
                    QPointF size = me->scenePos() - comment->pos();
                    comment->setBoxSize(QSizeF(std::max(30.0, size.x()), std::max(30.0, size.y())));
                }
                else if (me->modifiers() & Qt::ControlModifier)
                {
                    QPointF dist = me->lastScenePos() - me->scenePos();
                    for (auto item : selection)
                    {
                        item->setPos(item->pos() - dist);
                    }
                }
            }
            break;
        }

        case QEvent::GraphicsSceneMouseRelease:
        {
            if (conn_ && me->button() == Qt::LeftButton)
            {
                QGraphicsItem *item = itemAt(me->scenePos());
                if (item && item->type() == QNEPort::Type)
                {
                    QNEPort * srcPort, * dstPort;
                    if (conn_->outputPort()->isOutput())
                    {
                        srcPort = conn_->outputPort();
                        dstPort = (QNEPort*) item;
                    }
                    else
                    {
                        dstPort = conn_->outputPort();
                        srcPort = (QNEPort*) item;
                    }

                    if (srcPort->block() != dstPort->block() && srcPort->isOutput() != dstPort->isOutput() &&
                            !srcPort->isConnected(dstPort) && srcPort->portCategory() == dstPort->portCategory() &&
                            beforeConnect(srcPort, dstPort))
                    {
                        conn_->setColor(srcPort->getConnectionColor());
                        conn_->setOutputPos(srcPort->scenePos());
                        conn_->setOutputPort(srcPort);
                        conn_->setInputPos(dstPort->scenePos());
                        conn_->setInputPort(dstPort);
                        conn_->updatePath();
                        setDirty();
                        conn_ = 0;
                        return true;
                    }
                }

                delete conn_;
                conn_ = 0;
                return true;
            }

            resizing_ = false;
            draggingCanvas_ = false;
            break;
        }
	}

	return QObject::eventFilter(o, e);
}

bool QNodesEditor::hasBlocks() const
{
    return !items_.empty();
}

QNEBlock * QNodesEditor::findBlock(int nodeId) const
{
    foreach (QNEItem * item, items_)
    {
        if (item->type() == QNEBlock::Type && item->id() == nodeId)
            return static_cast<QNEBlock *>(item);
    }

    return nullptr;
}


bool QNodesEditor::beforeConnect(QNEPort * outPort, QNEPort * inPort)
{
    Q_UNUSED(outPort)
    Q_UNUSED(inPort)
    return true;
}


void QNodesEditor::beforeRemoveItem(QNEItem * item)
{
    Q_UNUSED(item)
}


void QNodesEditor::showContextMenu(QNEItem * item, QPoint const & position, QPointF const & scenePosition)
{
    Q_UNUSED(item)
    Q_UNUSED(position)
    Q_UNUSED(scenePosition)
}


void QNodesEditor::selectionChanged(QNEItem * item)
{
    Q_UNUSED(item)
}
