#ifndef SCRIPTGRAPHICSVIEW_H
#define SCRIPTGRAPHICSVIEW_H

#include <QGraphicsView>
#include <QPixmap>
#include <QGraphicsPixmapItem>

class QWheelEvent;

class ScaledPixmapItem: public QGraphicsPixmapItem
{
public:
    ScaledPixmapItem(const QPixmap &pixmap, QGraphicsItem *parentItem = 0);
    ~ScaledPixmapItem();

public:
    QRectF boundingRect() const;
    void paint(QPainter *painter, const QStyleOptionGraphicsItem *option, QWidget *widget = 0);
    float setSize(float height);

private:
    float width_, height_;
};

class ScriptGraphicsView : public QGraphicsView
{
    Q_OBJECT
public:
    explicit ScriptGraphicsView(QWidget *parent = 0);

    QRectF getSceneBounds();
    void centerOnScene();
    void setScaleStep(int step);
    void setup();
    
private:
    virtual void scrollContentsBy(int dx, int dy);
    virtual void resizeEvent(QResizeEvent * event);
    void wheelEvent(QWheelEvent * event);
    void updateBackground();
    void updateTransformation();

    QPixmap background_;
    ScaledPixmapItem * bgItem_;
    int scaleStep_;
};

#endif // SCRIPTGRAPHICSVIEW_H
