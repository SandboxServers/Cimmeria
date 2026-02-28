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

#include "qneport.h"
#include "qneconnection.h"
#include "qneblock.h"
#include "qnodeseditor.h"

#include <QGraphicsScene>
#include <QFontMetrics>
#include <QPen>
#include <QPainter>

// The table contains two entries for each category (one for input and one for output ports)
const QNEPort::ShapeInfo QNEPortCategoryShapes[(QNEPort::MaxCategoryId + 1) * 2] = {
    {Qt::black, Qt::black, Qt::black, QNEPort::TriangleRight}, // Cat 0, input ports
    {Qt::black, Qt::black, Qt::black, QNEPort::Circle}, // Cat 0, output ports
    {Qt::darkBlue, Qt::darkBlue, Qt::darkBlue, QNEPort::TriangleRight}, // Cat 1, input ports
    {Qt::darkBlue, Qt::darkBlue, Qt::darkBlue, QNEPort::Circle}  // Cat 1, output ports
};

const int QNEPortTypeOverrideCount = 6;
const QNEPort::TypeOverride QNEPortTypeOverrides[QNEPortTypeOverrideCount] = {
    {"Integer", Qt::darkCyan, Qt::darkCyan, Qt::darkCyan, QNEPort::Circle},
    {"Boolean", Qt::darkRed, Qt::darkRed, Qt::darkRed, QNEPort::Circle},
    {"String", Qt::darkGreen, Qt::darkGreen, Qt::darkGreen, QNEPort::Circle},
    {"Float", Qt::darkBlue, Qt::darkBlue, Qt::darkBlue, QNEPort::Circle},
    {"Entity", Qt::darkMagenta, Qt::darkMagenta, Qt::darkMagenta, QNEPort::Circle},
    {"Vector3", 0x00B26200, 0x00B26200, 0x00B26200, QNEPort::Circle}
};

QNEPort::QNEPort(QNEBlock * parent, QGraphicsScene * scene) :
    QGraphicsPathItem(parent), block_(parent)
{
    label_ = new QGraphicsTextItem(this);
    setFlag(QGraphicsItem::ItemSendsScenePositionChanges);

    portCategory_ = 0;
    template_ = 0;
    shouldUpdateShape_ = true;

    if (scene)
        scene->addItem(this);
}

QNEPort::~QNEPort()
{
    foreach(QNEConnection *conn, connections_)
    {
		delete conn;
    }
}

QNEPort::PortFlags QNEPort::portFlags() const
{
    return portFlags_;
}

QNEPort::PortFlags QNEPort::persistentFlags() const
{
    return (portFlags_ & PersistentFlags);
}

bool QNEPort::hasAllFlags(PortFlags flags) const
{
    return (portFlags_ & flags) == flags;
}

void QNEPort::setPortFlag(PortFlags flags)
{
    portFlags_ |= flags;
    flagsUpdated();
}

void QNEPort::clearPortFlag(PortFlags flags)
{
    portFlags_ &= ~flags;
    flagsUpdated();
}

void QNEPort::setPersistentFlags(PortFlags flags)
{
    portFlags_ = (portFlags_ & ~PersistentFlags) | (flags & PersistentFlags);
    flagsUpdated();
}

void QNEPort::setPortFlags(PortFlags flags)
{
    portFlags_ = flags;
    flagsUpdated();
}

QNEPort::Template const * QNEPort::getTemplate() const
{
    return template_;
}

void QNEPort::setTemplate(Template const * ptr)
{
    template_ = ptr;
}

QNEBlock* QNEPort::block() const
{
    return block_;
}

void QNEPort::setName(const QString &n)
{
    name_ = n;
    label_->setPlainText(n);
}

void QNEPort::setPortType(const QString &n)
{
    type_ = n;
    shouldUpdateShape_ = true;
}

bool QNEPort::isOutput() const
{
    return hasAllFlags(Output);
}

void QNEPort::flagsUpdated()
{
    setVisible(!hasAllFlags(Hidden));

	QFontMetrics fm(scene()->font());
    QRect r = fm.boundingRect(name_);

    if (isOutput())
        label_->setPos(-Radius - Margin - label_->boundingRect().width(), -label_->boundingRect().height() / 2);
	else
        label_->setPos(Radius + Margin, -label_->boundingRect().height() / 2);
    shouldUpdateShape_ = true;
}

void QNEPort::setPortCategory(int category)
{
    Q_ASSERT(category >= 0 && category <= MaxCategoryId);
    portCategory_ = category;
    shouldUpdateShape_ = true;
}

void QNEPort::disconnectAll()
{
    while (!connections_.empty())
        block_->editor()->removeItem(connections_[0]);
}


bool QNEPort::isConnected()
{
    return !connections_.empty();
}


bool QNEPort::isConnected(QNEPort * other)
{
    foreach (QNEConnection * conn, connections_)
    {
        if (conn->outputPort() == other || conn->inputPort() == other)
            return true;
    }

    return false;
}


void QNEPort::paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget)
{
    updateShape();
    painter->setRenderHints(QPainter::Antialiasing, true);
    QGraphicsPathItem::paint(painter, option, widget);
    painter->setRenderHints(QPainter::Antialiasing, false);
}

QVector<QNEConnection *> & QNEPort::connections()
{
    return connections_;
}

int QNEPort::pointWidth()
{
    return Radius;
}

QColor QNEPort::getConnectionColor() const
{
    for (int i = 0; i < QNEPortTypeOverrideCount; i++)
    {
        if (QNEPortTypeOverrides[i].type == type_)
            return QNEPortTypeOverrides[i].shape.connectionColor;
    }
    return QNEPortCategoryShapes[portCategory_ * 2 + (isOutput() ? 1 : 0)].connectionColor;
}

QVariant QNEPort::itemChange(GraphicsItemChange change, const QVariant &value)
{
	if (change == ItemScenePositionHasChanged)
	{
        foreach(QNEConnection * conn, connections_)
		{
			conn->updatePosFromPorts();
			conn->updatePath();
		}
    }
    return QGraphicsItem::itemChange(change, value);
}

void QNEPort::updateShape()
{
    if (!shouldUpdateShape_)
        return;
    shouldUpdateShape_ = false;

    ShapeInfo const * shape = &QNEPortCategoryShapes[portCategory_ * 2 + (isOutput() ? 1 : 0)];
    Shape shp = shape->shape;
    for (int i = 0; i < QNEPortTypeOverrideCount; i++)
    {
        if (QNEPortTypeOverrides[i].type == type_)
            shape = &QNEPortTypeOverrides[i].shape;
    }

    setPen(shape->color);
    setBrush(shape->brushColor);

    QPainterPath p;
    switch (shp)
    {
    case Circle:
        p.addEllipse(-Radius, -Radius, 2 * Radius, 2 * Radius);
        break;

    case Rectangle:
        p.addRect(-Radius, -Radius, 2 * Radius, 2 * Radius);
        break;

    case TriangleRight:
    {
        QVector<QPoint> points;
        points.append(QPoint(-Radius, -Radius));
        points.append(QPoint(Radius, 0));
        points.append(QPoint(-Radius, Radius));
        p.addPolygon(QPolygon(points));
        break;
    }

    case TriangleLeft:
    {
        QVector<QPoint> points;
        points.append(QPoint(Radius, -Radius));
        points.append(QPoint(-Radius, 0));
        points.append(QPoint(Radius, Radius));
        p.addPolygon(QPolygon(points));
        break;
    }

    default:
        Q_ASSERT("Illegal shape format!");
    }
    setPath(p);
}
