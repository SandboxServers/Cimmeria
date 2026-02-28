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

#ifndef QNEPORT_H
#define QNEPORT_H

#include <QGraphicsPathItem>
#include "scriptdefinitions.h"

class QNEBlock;
class QNEConnection;

class QNEPort : public QGraphicsPathItem
{
public:
    typedef ScriptDefinitions::PortTemplate Template;

    enum { Type = QGraphicsItem::UserType + 1 };
    enum PortFlag
    {
        Hidden = 0x01,
        AlwaysShow = 0x02,
        Output = 0x10000,
        Trigger = 0x20000,
        Required = 0x40000,
        PersistentFlags = 0xffff
    };
    Q_DECLARE_FLAGS(PortFlags, PortFlag)

    const static int MaxCategoryId = 1;

    enum Shape
    {
        Circle,
        Rectangle,
        TriangleRight,
        TriangleLeft
    };

    struct ShapeInfo
    {
        QColor color;
        QColor brushColor;
        QColor connectionColor;
        Shape shape;
    };

    struct TypeOverride
    {
        QString type;
        ShapeInfo shape;
    };

    QNEPort(QNEBlock * parent, QGraphicsScene *scene = 0);
	~QNEPort();

    PortFlags portFlags() const;
    PortFlags persistentFlags() const;
    bool hasAllFlags(PortFlags flags) const;
    void setPortFlag(PortFlags flags);
    void clearPortFlag(PortFlags flags);
    void setPersistentFlags(PortFlags flags);
    void setPortFlags(PortFlags flags);

    Template const * getTemplate() const;
    void setTemplate(Template const * ptr);

    QNEBlock* block() const;

    const QString& portName() const { return name_; }
	void setName(const QString &n);

    const QString& portType() const { return type_; }
    void setPortType(const QString &n);

    bool isOutput() const;

    int portCategory() const { return portCategory_; }
    void setPortCategory(int);

    void disconnectAll();
    bool isConnected();
    bool isConnected(QNEPort *);

    void paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget);
    QVector<QNEConnection*> & connections();
    int type() const { return Type; }
    int pointWidth();
    QColor getConnectionColor() const;

protected:
	QVariant itemChange(GraphicsItemChange change, const QVariant &value);

private:
    const static int Radius = 5;
    const static int Margin = 2;

    void flagsUpdated();
    void updateShape();

    bool shouldUpdateShape_;
    QGraphicsTextItem * label_;
    QVector<QNEConnection*> connections_;
    PortFlags portFlags_;
    QNEBlock * block_;
    QString name_, type_;
    int portCategory_;
    Template const * template_;
};

#endif // QNEPORT_H
