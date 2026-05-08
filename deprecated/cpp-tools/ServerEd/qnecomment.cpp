#include "qnecomment.h"
#include "qnodeseditor.h"

#include <QGraphicsScene>
#include <QFontMetrics>
#include <QPainter>
#include <QXmlStreamWriter>
#include <QtXml/QDomElement>

const int QNECommentSchemeCount = 7;
const QNEComment::ColorScheme QNECommentSchemes[QNECommentSchemeCount] = {
    {Qt::darkRed, Qt::darkRed},
    {Qt::darkGreen, Qt::darkGreen},
    {Qt::darkBlue, Qt::darkBlue},
    {Qt::darkCyan, Qt::darkCyan},
    {Qt::darkMagenta, Qt::darkMagenta},
    {Qt::darkYellow, Qt::darkYellow},
    {Qt::darkGray, Qt::darkGray}
};


QNEComment::QNEComment(QNodesEditor * editor, QGraphicsScene * scene) :
    QNEItem(editor, scene)
{
    setFlag(QGraphicsItem::ItemIsSelectable);
    setZValue(-2);
    colorScheme_ = 0;
}


QNEComment::~QNEComment()
{
}


QSizeF const & QNEComment::boxSize() const
{
    return size_;
}


void QNEComment::setBoxSize(QSizeF const & size)
{
    size_ = size;
    layoutChanged();
    updateLayout();
}


unsigned int QNEComment::colorScheme() const
{
    return colorScheme_;
}


void QNEComment::setColorScheme(unsigned int scheme)
{
    colorScheme_ = scheme;
    if (colorScheme_ >= QNECommentSchemeCount)
        colorScheme_ = 0;
    layoutChanged();
    updateLayout();
}


void QNEComment::save(QXmlStreamWriter * writer)
{
    writer->writeCharacters("\t");
    writer->writeStartElement("Comment");
    writer->writeAttribute("Id", QString::number(id()));
    writer->writeAttribute("Text", comment());
    writer->writeAttribute("X", QString::number(pos().x()));
    writer->writeAttribute("Y", QString::number(pos().y()));
    writer->writeAttribute("Width", QString::number(boxSize().width()));
    writer->writeAttribute("Height", QString::number(boxSize().height()));
    writer->writeAttribute("Color", QString::number(colorScheme()));
    writer->writeEndElement();
    writer->writeCharacters("\r\n");
}


bool QNEComment::load(QDomElement * element)
{
    setId(element->attribute("Id").toUInt());
    setComment(element->attribute("Text"));
    setPos(element->attribute("X").toFloat(), element->attribute("Y").toFloat());
    setBoxSize(QSizeF(element->attribute("Width").toFloat(), element->attribute("Height").toFloat()));
    setColorScheme(element->attribute("Color").toUInt());
    return true;
}


QNEComment * QNEComment::clone()
{
    QNEComment * copy = new QNEComment(editor(), scene());
    copy->setPos(pos());
    copy->setComment(comment());
    copy->setBoxSize(boxSize());
    copy->setColorScheme(colorScheme());
    return copy;
}


void QNEComment::paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget)
{
    ColorScheme const & scheme = QNECommentSchemes[colorScheme_];

    QColor brush(scheme.brush);
    painter->setBrush(QColor(brush.red(), brush.green(), brush.blue(), 0x30));
    if (isSelected())
        painter->setPen(QPen(Qt::yellow, 2));
    else
        painter->setPen(QPen(scheme.rect, 2));
    painter->drawRect(0, 0, width_, height_);

    painter->setBrush(Qt::black);
    painter->setPen(QPen(Qt::black));
    QPolygon poly;
    poly.append(QPoint(width_ - ResizeHandleSize, height_));
    poly.append(QPoint(width_, height_));
    poly.append(QPoint(width_, height_ - ResizeHandleSize));
    painter->drawPolygon(poly);

    painter->setPen(QPen(Qt::white));
    QFont commentFont(scene()->font());
    commentFont.setPointSizeF(commentFont.pointSizeF() + 0.5f);
    commentFont.setBold(true);
    painter->setFont(commentFont);
    painter->drawText(5, 15, comment());
}

QRectF QNEComment::boundingRect() const
{
    return QRectF(0, 0, width_, height_);
}

QRectF QNEComment::commentArea() const
{
    QSizeF size = textSize();
    return QRectF(0, 0, 10 + size.width(), 10 + size.height());
}

QSizeF QNEComment::textSize() const
{
    QFont commentFont(scene()->font());
    commentFont.setPointSizeF(commentFont.pointSizeF() + 0.5f);
    commentFont.setBold(true);
    QFontMetrics fm(commentFont);
    return QSizeF(fm.width(comment()), fm.height());
}

void QNEComment::updateItemLayout()
{
    prepareGeometryChange();
    width_ = size_.width();
    height_ = size_.height();

    QSizeF size = textSize();
    if (height_ < size.height() + 30)
        height_ = size.height() + 30;
    if (width_ < size.width() + 30)
        width_ = size.width() + 30;
}

