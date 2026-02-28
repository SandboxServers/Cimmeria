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

#ifndef QNEBLOCK_H
#define QNEBLOCK_H

#include "qneitem.h"
#include "qneport.h"
#include "scriptdefinitions.h"

class QNEBlock : public QNEItem
{
public:
	enum { Type = QGraphicsItem::UserType + 3 };
    typedef ScriptDefinitions::NodeTemplate Template;

    QNEBlock(QNodesEditor * editor, QGraphicsScene *scene = 0);
    ~QNEBlock();

    QString const & name() const;
    void setName(QString const & name);

    Template const * getTemplate() const;
    void setTemplate(Template const * ptr);

    QString const & blockType() const;
    void setBlockType(QString const & type);

    QVariant getProperty(QString const & name);
    void setProperty(QString const & name, QVariant const & value);
    bool isNodeEnabled();
    bool isDebuggingEnabled();

    QVector<QNEPort *> ports();
    QNEPort * getPort(QString const & name) const;
    QNEPort * addPort(QString const & name);
    QNEPort * findPort(QString const & name, unsigned int flags, unsigned int excludeFlags) const;

    virtual void save(QXmlStreamWriter * writer);
    virtual bool load(QDomElement * element);
    virtual void layoutChanged();
    virtual QNEBlock * clone();
    virtual void paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget);

    virtual QRectF boundingRect() const;

	int type() const { return Type; }

protected:
	QVariant itemChange(GraphicsItemChange change, const QVariant &value);

private:
    void updateItemLayout();
    void calculateWidthFromPorts(int category);
    void updatePortLayout(int categoryIndex);
    void updateFormattedName();

    // Margin added to sum of port widths
    const static int MarginWidth = 20;
    // Size of block comment container
    const static int CommentHeight = 16;
    // Size of block comment container
    const static int CommentMarginLeft = 5;
    // Size of block name container
    const static int HeaderHeight = 20;
    // Margin in block name container
    const static int HeaderMargin = 0;
    // Vertical padding after the header
    const static int BodyPaddingTop = 8;
    // Vertical padding after each section
    const static int BodyPadding = 8;
    // Vertical padding after each port
    const static int PortPadding = 3;
    // Port connection point width (this are will be redrawn after a geometry change)
    const static int PortConnectorWidth = 5;

    int width_, height_, boundingWidth_;
    int oldWidth_, oldHeight_;
    int headerTextLeft_;
    Template const * template_;
    QString name_, displayName_, type_;
    QVector<QPair<int, int> > categoryHeights_;
    QMap<QString, QVariant> properties_;
    QMap<QString, QNEPort *> ports_;
};

#endif // QNEBLOCK_H
