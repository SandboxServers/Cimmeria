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

#ifndef QNECONNECTION_H
#define QNECONNECTION_H

#include "qneitem.h"

class QNEPort;
class QNodesEditor;
class QXmlStreamWriter;
class QDomElement;

class QNEConnection : public QNEItem
{
public:
	enum { Type = QGraphicsItem::UserType + 2 };

    QNEConnection(QNodesEditor * editor, QGraphicsScene *scene = 0);
	~QNEConnection();

    void setOutputPos(const QPointF & p);
    void setInputPos(const QPointF & p);
    void setOutputPort(QNEPort * p);
    void setInputPort(QNEPort * p);
    void setColor(QColor const & color);
	void updatePosFromPorts();
	void updatePath();
    QNEPort * outputPort() const;
    QNEPort * inputPort() const;

    virtual void save(QXmlStreamWriter * writer);
    virtual bool load(QDomElement * element);
    virtual QNEConnection * clone();

    void paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget);
    virtual QRectF boundingRect() const;
    virtual QPainterPath shape() const;
	int type() const { return Type; }

private:
    virtual void updateItemLayout();

    QPointF outputPos_;
    QPointF inputPos_;
    QNEPort * outputPort_;
    QNEPort * inputPort_;
    QPainterPath path_;
    QPainterPath boundingPath_;
    QColor color_;
};

#endif // QNECONNECTION_H
