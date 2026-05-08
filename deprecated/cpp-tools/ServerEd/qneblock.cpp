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

#include "qneblock.h"
#include "qneconnection.h"
#include "qnodeseditor.h"

#include <QPen>
#include <QGraphicsScene>
#include <QFontMetrics>
#include <QPainter>
#include <QStyleOptionGraphicsItem>
#include <QXmlStreamWriter>
#include <QDomElement>

const QColor QNEBlockCategoryBrushes[QNEPort::MaxCategoryId + 1] = {
    QColor(0x00808080),
    QColor(0x00666666)
};

QNEBlock::QNEBlock(QNodesEditor * editor, QGraphicsScene *scene) : QNEItem(editor, scene),
    width_(MarginWidth), height_(HeaderHeight + BodyPaddingTop), headerTextLeft_(0),
    boundingWidth_(MarginWidth), template_(nullptr)
{
    setFlag(QGraphicsItem::ItemIsSelectable);
}

QNEBlock::~QNEBlock()
{
}

QString const & QNEBlock::name() const
{
    return name_;
}

void QNEBlock::setName(QString const & name)
{
    if (name_ == displayName_)
    {
        displayName_ = name;
        layoutChanged();
    }
    name_ = name;
}

QNEBlock::Template const * QNEBlock::getTemplate() const
{
    return template_;
}

void QNEBlock::setTemplate(Template const * ptr)
{
    template_ = ptr;
}

QString const & QNEBlock::blockType() const
{
    return type_;
}

void QNEBlock::setBlockType(QString const & type)
{
    type_ = type;
}

QVariant QNEBlock::getProperty(QString const & name)
{
    return properties_[name];
}

void QNEBlock::setProperty(QString const & name, QVariant const & value)
{
    properties_[name] = value;
    if (type_ == "Variable" && (name == "Variable Name" || name == "Value"))
    {
        QString varName = getProperty("Variable Name").value<QString>();
        QString varValue = getProperty("Value").value<QString>();

        if (varName.length() > 0)
            displayName_ = varName;
        else
            displayName_ = name_;

        if (varValue.length() > 0)
            displayName_ += " (" + varValue +")";
        layoutChanged();
        updateLayout();
    }
    else if (name == "Comment")
    {
        layoutChanged();
        updateLayout();
    }

    updateFormattedName();
}

bool QNEBlock::isNodeEnabled()
{
    return getProperty("Enabled").toBool();
}

bool QNEBlock::isDebuggingEnabled()
{
    return getProperty("Debug").toBool();
}


QVector<QNEPort *> QNEBlock::ports()
{
    QVector<QNEPort *> res;
    foreach(QGraphicsItem *port_, childItems())
    {
        if (port_->type() == QNEPort::Type)
            res.append((QNEPort *)port_);
    }
    return res;
}

QNEPort * QNEBlock::getPort(QString const & name) const
{
    QMap<QString, QNEPort *>::const_iterator iter = ports_.find(name);
    if (iter == ports_.end())
        return nullptr;
    else
        return iter.value();
}

QNEPort * QNEBlock::addPort(QString const & name)
{
    Q_ASSERT(ports_.find(name) == ports_.end());
    QNEPort * port = new QNEPort(this);
    port->setName(name);
    ports_.insert(name, port);
    layoutChanged();
	return port;
}

QNEPort * QNEBlock::findPort(QString const & name, unsigned int flags, unsigned int excludeFlags) const
{
    foreach (QNEPort * port, ports_)
    {
        if (port->portName() == name && port->hasAllFlags((QNEPort::PortFlags)flags) && !((unsigned int)port->portFlags() & excludeFlags))
        {
            return port;
        }
    }

    return nullptr;
}

void QNEBlock::save(QXmlStreamWriter * writer)
{
    writer->writeCharacters("\t");
    writer->writeStartElement("Node");
    writer->writeAttribute("Id", QString::number(id()));
    writer->writeAttribute("Ref", template_->ref);
    writer->writeAttribute("X", QString::number(pos().x()));
    writer->writeAttribute("Y", QString::number(pos().y()));

    QMap<QString, QVariant>::Iterator iter;
    for (iter = properties_.begin(); iter != properties_.end(); ++iter)
    {
        writer->writeCharacters("\r\n\t\t");
        writer->writeStartElement("Property");
        writer->writeAttribute("Name", iter.key());
        writer->writeAttribute("Value", iter.value().value<QString>());
        writer->writeEndElement();
    }

    QMap<QString, QNEPort *>::Iterator piter;
    for (piter = ports_.begin(); piter != ports_.end(); ++piter)
    {
        writer->writeCharacters("\r\n\t\t");
        writer->writeStartElement("Port");
        writer->writeAttribute("Name", piter.key());
        writer->writeAttribute("Flags", QString::number(piter.value()->persistentFlags()));
        writer->writeEndElement();
    }

    writer->writeCharacters("\r\n\t");
    writer->writeEndElement();
    writer->writeCharacters("\r\n");
}

bool QNEBlock::load(QDomElement * element)
{
    setId(element->attribute("Id").toUInt());
    setPos(element->attribute("X").toFloat(), element->attribute("Y").toFloat());
    for (QDomElement port = element->firstChildElement("Port"); !port.isNull(); port = port.nextSiblingElement("Port"))
    {
        QString portName = port.attribute("Name");
        QNEPort * p = getPort(portName);
        if (!p)
        {
            qWarning("Port '%s' of node '%s' does not exist",
                     qPrintable(portName), qPrintable(name()));
            continue;
        }

        p->setPersistentFlags((QNEPort::PortFlags)port.attribute("Flags").toUInt());
    }

    for (QDomElement prop = element->firstChildElement("Property"); !prop.isNull(); prop = prop.nextSiblingElement("Property"))
    {
        QString propName = prop.attribute("Name");
        QVariant::Type type = editor()->getPropertyType(this, propName);
        if (type == QVariant::Invalid)
        {
            qWarning("Property '%s' of node '%s' does not exist or has invalid type",
                     qPrintable(propName), qPrintable(name()));
            continue;
        }

        QVariant var(prop.attribute("Value"));
        if (!var.convert(type))
        {
            qWarning("Failed to convert variant property '%s' of node '%s' (malformed value?)",
                     qPrintable(propName), qPrintable(name()));
        }
        else
            setProperty(propName, var);
    }

    return true;
}

void QNEBlock::paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget)
{
	Q_UNUSED(option)
	Q_UNUSED(widget)

    updateLayout();

    int y = 0;
    QString comment = getProperty("Comment").value<QString>();
    if (comment.length() > 0)
    {
        painter->setPen(QPen(QColor(0xF0, 0xF0, 0xF0)));
        painter->drawText(CommentMarginLeft, y + 8, comment);
        y += CommentHeight;
    }

    if (displayName_.length() > 0)
    {
        if (isSelected())
        {
            painter->setRenderHint(QPainter::Antialiasing, true);
            QPen pen(Qt::yellow);
            pen.setWidth(2);
            painter->setPen(pen);
        }
        else
        {
            QPen pen(Qt::white);
            pen.setWidth(1);
            painter->setPen(pen);
        }
        painter->setBrush(QColor(153, 76, 76));
        painter->drawRect(0, y, width_, y + HeaderHeight - HeaderMargin);
        painter->setRenderHint(QPainter::Antialiasing, false);

        painter->setPen(QPen(QColor(0xF0, 0xF0, 0xF0)));
        QFont font = painter->font();
        font.setBold(true);
        painter->setFont(font);
        painter->drawText(headerTextLeft_, y + 13, displayName_);
        font.setBold(false);
        painter->setFont(font);

        y += HeaderHeight;
    }

    QPair<int, int> category;
    if (isSelected())
    {
        painter->setRenderHint(QPainter::Antialiasing, true);
        QPen pen(Qt::yellow);
        pen.setWidth(2);
        painter->setPen(pen);
    }
    else
    {
        QPen pen(Qt::white);
        pen.setWidth(1);
        painter->setPen(pen);
    }
    foreach (category, categoryHeights_)
    {
        painter->setBrush(QNEBlockCategoryBrushes[category.first]);
        painter->drawRect(0, y, width_, category.second - y);
        y = category.second;
    }
    painter->setRenderHint(QPainter::Antialiasing, false);
}

QNEBlock * QNEBlock::clone()
{
    QNEBlock * b = new QNEBlock(editor(), scene());
    b->properties_ = properties_;
    b->setTemplate(getTemplate());
    b->setName(name_);
    b->setBlockType(type_);
    b->setPos(pos());

    foreach (QGraphicsItem *port_, childItems())
	{
		if (port_->type() == QNEPort::Type)
		{
            QNEPort * port = static_cast<QNEPort *>(port_);
            QNEPort * clone = b->addPort(port->portName());
            clone->setPortType(port->portType());
            clone->setPortFlags(port->portFlags());
            clone->setPortCategory(port->portCategory());
            clone->setTemplate(port->getTemplate());
		}
    }

	return b;
}

void QNEBlock::layoutChanged()
{
    QNEItem::layoutChanged();
    oldWidth_ = width_;
    oldHeight_ = height_;
}

QRectF QNEBlock::boundingRect() const
{
    return QRectF(0, 0, boundingWidth_, height_);
}

QVariant QNEBlock::itemChange(GraphicsItemChange change, const QVariant &value)
{
    if (change == QGraphicsItem::ItemSelectedChange)
    {
        foreach (QNEPort * port, ports_.values())
        {
            foreach (QNEConnection * conn, port->connections())
            {
                conn->update();
            }
        }
    }
    return QGraphicsItem::itemChange(change, value);
}

void QNEBlock::updateItemLayout()
{
    prepareGeometryChange();
    height_ = BodyPaddingTop;
    width_ = MarginWidth;

    categoryHeights_.clear();
    for (int i = 0; i <= QNEPort::MaxCategoryId; i++)
    {
        calculateWidthFromPorts(i);
    }

    QFont headerFont(scene()->font());
    headerFont.setBold(true);
    QFontMetrics fm(headerFont);
    QString comment = getProperty("Comment").value<QString>();
    if (comment.length() > 0)
    {
        height_ += CommentHeight;
    }

    qreal headerWidth = fm.width(displayName_);
    if (displayName_.length() > 0)
    {
        if (width_ < headerWidth + MarginWidth)
            width_ = headerWidth + MarginWidth;
        height_ += HeaderHeight;
    }

    for (int i = 0; i < categoryHeights_.size(); i++)
    {
        updatePortLayout(i);
        height_ += BodyPadding;
    }

    headerTextLeft_ = (width_ / 2) - (headerWidth / 2);

    boundingWidth_ = width_;
    if (comment.length() > 0)
    {
        QFontMetrics fm(scene()->font());
        int width = CommentMarginLeft + fm.width(comment);
        boundingWidth_ = std::max(width, width_);
    }
}


void QNEBlock::updatePortLayout(int categoryIndex)
{
    QFontMetrics fm(scene()->font());
    int h = fm.height();

    int category = categoryHeights_[categoryIndex].first;

    int yIn = height_, yOut = height_;
    foreach (QGraphicsItem * port_, childItems())
    {
        QNEPort * port = (QNEPort *)port_;
        Q_ASSERT(port->hasAllFlags(QNEPort::Hidden) != port->isVisible());
        if (port->portCategory() != category || port->hasAllFlags(QNEPort::Hidden))
            continue;

        if (port->isOutput())
        {
            port->setPos(width_ + port->pointWidth() + 1, yOut);
            yOut += h + PortPadding;
        }
        else
        {
            port->setPos(-port->pointWidth() - 1, yIn);
            yIn += h + PortPadding;
        }
    }

    height_ = std::max(yIn, yOut);
    categoryHeights_[categoryIndex].second = height_;
}


void QNEBlock::calculateWidthFromPorts(int category)
{
    QFontMetrics fm(scene()->font());

    QVector<QPair<int, int> > widths;
    int inputIdx = 0, outputIdx = 0;
    foreach (QGraphicsItem *port_, childItems())
    {
        QNEPort * port = (QNEPort*) port_;
        Q_ASSERT(port->hasAllFlags(QNEPort::Hidden) != port->isVisible());
        if (port->portCategory() != category || port->hasAllFlags(QNEPort::Hidden))
            continue;

        int w = fm.width(port->portName());
        if (port->isOutput())
        {
            if (outputIdx >= widths.size())
                widths.append(QPair<int, int>(0, w));
            else
                widths[outputIdx].second = w;
            outputIdx++;
        }
        else
        {
            if (inputIdx >= widths.size())
                widths.append(QPair<int, int>(w, 0));
            else
                widths[inputIdx].first = w;
            inputIdx++;
        }
    }

    QPair<int, int> w;
    foreach (w, widths)
    {
        int pairW = w.first + w.second + MarginWidth;
        if (pairW > width_)
            width_ = pairW;
    }

    if (widths.size() > 0)
        categoryHeights_.append(QPair<int, int>(category, 0));
}


void QNEBlock::updateFormattedName()
{
    if (template_->formatName.length())
    {
        QRegExp expr("\\{([^{}]+)\\}");
        int pos, lastPos = -1;
        QString formatted = template_->formatName;
        while ((pos = expr.indexIn(formatted)) != -1 && pos >= lastPos)
        {
            QString name = expr.cap(1), replacement;
            ScriptDefinitions::PropertyTemplate const * def = template_->findProperty(name);
            if (def)
            {
                QVariant value = getProperty(name);
                if (value.isValid())
                {
                    if (def->editorType == ScriptDefinitions::PropertyTemplate::EnumProperty)
                    {
                        quint64 i = value.toULongLong();
                        for (auto it = def->enumeration->values.begin(); it != def->enumeration->values.end(); ++it)
                        {
                            if (it.value() == i)
                            {
                                replacement = it.key();
                                break;
                            }
                        }
                    }
                    else
                        replacement = def->toString(value);
                }
                else
                    replacement = "???";
            }
            else
                replacement = "[[" + name + "]]";
            formatted = formatted.replace(pos, name.length() + 2, replacement);
            lastPos = pos + replacement.length();
        }

        if (displayName_ != formatted)
        {
            displayName_ = formatted;
            updateItemLayout();
        }
    }
}
