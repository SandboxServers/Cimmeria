#ifndef QNECOMMENT_H
#define QNECOMMENT_H

#include "qneitem.h"

#include <QGraphicsItem>
#include <QSize>

class QNodesEditor;

class QNEComment : public QNEItem
{
public:
    struct ColorScheme
    {
        QColor rect, brush;
    };

    enum { Type = QGraphicsItem::UserType + 4 };

    static const int ResizeHandleSize = 15;

    QNEComment(QNodesEditor * editor, QGraphicsScene *scene = 0);
    ~QNEComment();

    QSizeF const & boxSize() const;
    void setBoxSize(QSizeF const & size);
    unsigned int colorScheme() const;
    void setColorScheme(unsigned int scheme);

    virtual void save(QXmlStreamWriter * writer);
    virtual bool load(QDomElement * element);
    virtual QNEComment * clone();

    virtual void paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget);
    virtual QRectF boundingRect() const;
    QRectF commentArea() const;
    QSizeF textSize() const;
    int type() const { return Type; }

private:
    void updateItemLayout();

    QNodesEditor * editor_;
    QSizeF size_;
    int width_, height_;
    unsigned int colorScheme_;
};

#endif // QNECOMMENT_H
