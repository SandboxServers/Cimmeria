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

#include "qneconnection.h"
#include "qneport.h"
#include "qneblock.h"
#include "qnodeseditor.h"

#include <QBrush>
#include <QPen>
#include <QGraphicsScene>
#include <QPainter>
#include <QXmlStreamWriter>
#include <QDomElement>

QNEConnection::QNEConnection(QNodesEditor * editor, QGraphicsScene * scene) :
    QNEItem(editor, scene), outputPort_(0), inputPort_(0), color_(Qt::black)
{
    setFlag(QGraphicsItem::ItemIsSelectable);
    setZValue(-1);
}

QNEConnection::~QNEConnection()
{
    if (outputPort_)
        outputPort_->connections().remove(outputPort_->connections().indexOf(this));
    if (inputPort_)
        inputPort_->connections().remove(inputPort_->connections().indexOf(this));
}

void QNEConnection::save(QXmlStreamWriter * writer)
{
    writer->writeCharacters("\t");
    writer->writeStartElement("Connection");
    writer->writeAttribute("OutNode", QString::number(outputPort_->block()->id()));
    writer->writeAttribute("OutPort", outputPort_->portName());
    writer->writeAttribute("InNode", QString::number(inputPort_->block()->id()));
    writer->writeAttribute("InPort", inputPort_->portName());
    writer->writeEndElement();
    writer->writeCharacters("\r\n");
}

bool QNEConnection::load(QDomElement * element)
{
    int outNode = element->attribute("OutNode").toInt();
    int inNode = element->attribute("InNode").toInt();
    QString outPortName = element->attribute("OutPort");
    QString inPortName = element->attribute("InPort");

    QNEBlock * outBlock = editor()->findBlock(outNode);
    if (!outBlock)
    {
        qWarning("Output node (%d) of connection does not exist", outNode);
        return false;
    }

    QNEBlock * inBlock = editor()->findBlock(inNode);
    if (!inBlock)
    {
        qWarning("Input node (%d) of connection does not exist", inNode);
        return false;
    }

    QNEPort * outPort = outBlock->getPort(outPortName);
    if (!outPort)
    {
        qWarning("Output node (%s) of connection has no port '%s'",
                 qPrintable(outBlock->name()), qPrintable(outPortName));
        return false;
    }

    QNEPort * inPort = inBlock->getPort(inPortName);
    if (!inPort)
    {
        qWarning("Input node (%s) of connection has no port '%s'",
                 qPrintable(inBlock->name()), qPrintable(inPortName));
        return false;
    }

    QString outType = editor()->getPortType(outPort);
    QString inType = editor()->getPortType(inPort);
    if (inType != outType)
    {
        qWarning("Connection type mismatch: '%s::%s' has type %s, but '%s::%s' has type %s",
                 qPrintable(outBlock->name()), qPrintable(outPortName), qPrintable(outType),
                 qPrintable(inBlock->name()), qPrintable(inPortName), qPrintable(inType));
        return false;
    }

    if (!outPort->isOutput())
    {
        qWarning("Connection error: '%s::%s' is not an output port",
                 qPrintable(outBlock->name()), qPrintable(outPortName));
        return false;
    }

    if (inPort->isOutput())
    {
        qWarning("Connection error: '%s::%s' is not an input port",
                 qPrintable(inBlock->name()), qPrintable(inPortName));
        return false;
    }

    setColor(outPort->getConnectionColor());
    setOutputPos(outPort->pos() + outPort->parentItem()->pos());
    setOutputPort(outPort);
    setInputPos(inPort->pos() + inPort->parentItem()->pos());
    setInputPort(inPort);
    updatePath();
    return true;
}

QNEConnection * QNEConnection::clone()
{
    qFatal("Connections cannot be cloned!");
    return nullptr;
}

void QNEConnection::setOutputPos(const QPointF & p)
{
    outputPos_ = p;
    layoutChanged();
    updateLayout();
}

void QNEConnection::setInputPos(const QPointF & p)
{
    inputPos_ = p;
    layoutChanged();
    updateLayout();
}

void QNEConnection::setOutputPort(QNEPort * p)
{
    if (outputPort_)
        outputPort_->connections().remove(outputPort_->connections().indexOf(this));

    outputPort_ = p;
    outputPort_->connections().append(this);
    layoutChanged();
}

void QNEConnection::setInputPort(QNEPort *p)
{
    if (inputPort_)
        inputPort_->connections().remove(inputPort_->connections().indexOf(this));

    inputPort_ = p;
    inputPort_->connections().append(this);
    layoutChanged();
}

void QNEConnection::setColor(QColor const & color)
{
    color_ = color;
    layoutChanged();
}

void QNEConnection::updatePosFromPorts()
{
    outputPos_ = outputPort_->scenePos();
    inputPos_ = inputPort_->scenePos();
    layoutChanged();
    updateLayout();
}

void QNEConnection::updatePath()
{
    qreal dx = inputPos_.x() - outputPos_.x();
    qreal dy = inputPos_.y() - outputPos_.y();

    QPainterPath p;
    p.moveTo(outputPos_);
    p.cubicTo(
        QPointF(outputPos_.x() + dx * 0.25, outputPos_.y() + dy * 0.1),
        QPointF(outputPos_.x() + dx * 0.75, outputPos_.y() + dy * 0.9),
        inputPos_
    );
    path_ = p;

    QPainterPath bp;
    bp.moveTo(outputPos_ - QPointF(1.0f, 1.0f));
    bp.cubicTo(
        QPointF(outputPos_.x() + dx * 0.25, outputPos_.y() + dy * 0.1) - QPointF(1.0f, 1.0f),
        QPointF(outputPos_.x() + dx * 0.75, outputPos_.y() + dy * 0.9) - QPointF(1.0f, 1.0f),
        inputPos_ - QPointF(1.0f, 1.0f)
    );
    bp.moveTo(outputPos_);
    bp.cubicTo(
        QPointF(outputPos_.x() + dx * 0.25, outputPos_.y() + dy * 0.1),
        QPointF(outputPos_.x() + dx * 0.75, outputPos_.y() + dy * 0.9),
        inputPos_
    );
    bp.moveTo(outputPos_ + QPointF(1.0f, 1.0f));
    bp.cubicTo(
        QPointF(outputPos_.x() + dx * 0.25, outputPos_.y() + dy * 0.1) + QPointF(1.0f, 1.0f),
        QPointF(outputPos_.x() + dx * 0.75, outputPos_.y() + dy * 0.9) + QPointF(1.0f, 1.0f),
        inputPos_ + QPointF(1.0f, 1.0f)
    );
    boundingPath_ = bp;
}

QNEPort* QNEConnection::outputPort() const
{
    return outputPort_;
}

QNEPort* QNEConnection::inputPort() const
{
    return inputPort_;
}

void QNEConnection::paint(QPainter * painter, const QStyleOptionGraphicsItem *, QWidget *)
{
    painter->setRenderHint(QPainter::Antialiasing, true);
    if (isSelected() ||
        (inputPort_ && inputPort_->block()->isSelected()) ||
        (inputPort_ && outputPort_->block()->isSelected()))
        painter->setPen(QPen(Qt::yellow, 2));
    else
        painter->setPen(QPen(color_, 2));
    painter->setBrush(Qt::NoBrush);
    painter->drawPath(path_);
    painter->setRenderHint(QPainter::Antialiasing, false);
}

QRectF QNEConnection::boundingRect() const
{
    return boundingPath_.boundingRect();
}

QPainterPath QNEConnection::shape() const
{
    return boundingPath_;
}

void QNEConnection::updateItemLayout()
{
    prepareGeometryChange();
    updatePath();
    update();
}


