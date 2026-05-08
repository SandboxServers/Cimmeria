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

#ifndef QNODESEDITOR_H
#define QNODESEDITOR_H

#include <QObject>
#include <QVector>
#include <QVariant>

class QGraphicsScene;
class QNEConnection;
class QGraphicsItem;
class QPointF;
class QNEItem;
class QNEBlock;
class QNEPort;
class QNEComment;

class QNodesEditor : public QObject
{
	Q_OBJECT
public:
	explicit QNodesEditor(QObject *parent = 0);
    ~QNodesEditor();

	void install(QGraphicsScene *scene);
    bool eventFilter(QObject *, QEvent *);

    bool hasBlocks() const;
    QNEBlock * findBlock(int nodeId) const;

    void breakAllLinks(QNEBlock * block);
    void removeItem(QNEItem * item);

    QNEItem * copy();
    QNEItem * paste();
    QNEItem * clipboard() const;
    QGraphicsScene * scene() const;

    virtual QString getPortType(QNEPort * port) = 0;
    virtual QVariant::Type getPropertyType(QNEBlock * block, QString const & property) = 0;

    bool hasChanged() const;
    void setDirty();
    void changesFlushed();

signals:
    void dirty(bool dirty);

protected:
    friend class QNEItem;

    void registerItem(QNEItem * item);
    void unregisterItem(QNEItem * item);

    QVector<QNEItem *> items_;
    int nextId_;

private:
    QNEItem * itemAt(const QPointF&);

    virtual bool beforeConnect(QNEPort * outPort, QNEPort * inPort);
    virtual void beforeRemoveItem(QNEItem * item);
    virtual void showContextMenu(QNEItem * item, QPoint const & position, QPointF const & scenePosition);
    virtual void selectionChanged(QNEItem * item);

private:
    QGraphicsScene *scene_;
    QNEConnection *conn_;
    QNEItem * clipboard_;
    bool resizing_;
    bool draggingCanvas_;
    bool translating_;
    bool changed_;
};

#endif // QNODESEDITOR_H
