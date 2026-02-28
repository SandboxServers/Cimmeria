#include "scriptgraphicsview.h"
#include "qneitem.h"
#include <QWheelEvent>

const unsigned int MaxScaleSteps = 7;
const float ScaleSteps[MaxScaleSteps] = {
    0.2f,
    0.4f,
    0.6f,
    0.8f,
    1.0f,
    1.5f,
    2.0f
};

ScaledPixmapItem::ScaledPixmapItem(const QPixmap & pixmap, QGraphicsItem * parentItem)
    : QGraphicsPixmapItem(pixmap, parentItem)
{
    setCacheMode(NoCache);
    setSize(pixmap.height());
}

ScaledPixmapItem::~ScaledPixmapItem()
{
}

float ScaledPixmapItem::setSize(float height)
{
    height_ = height;
    float ar = pixmap().width() / (float)pixmap().height();
    width_ = height_ / ar;
    return width_;
}

QRectF ScaledPixmapItem::boundingRect() const
{
    return QRectF(offset(), offset() + QPointF(width_, height_));
}

void ScaledPixmapItem::paint(QPainter *painter, const QStyleOptionGraphicsItem*, QWidget*)
 {
    // Scale QGraphicsPixmapItem to wanted 'size' and keep the aspect ratio using boundingRect()
    painter->drawPixmap(boundingRect().toRect(), pixmap());
}

ScriptGraphicsView::ScriptGraphicsView(QWidget * parent) :
    QGraphicsView(parent), bgItem_(nullptr), scaleStep_(4)
{
    background_.load(":/resources/ScriptBackground.png");
}

QRectF ScriptGraphicsView::getSceneBounds()
{
    if (items().empty())
    {
        return QRectF();
    }

    QRectF bounds(items().at(0)->boundingRect());
    for (auto item : items())
    {
        // Don't add non-script items (eg. scene frame) to the scene bounds
        if (dynamic_cast<QNEItem *>(item) != nullptr)
            bounds = bounds.united(item->boundingRect());
    }

    return bounds;
}

void ScriptGraphicsView::centerOnScene()
{
    QRectF bounds = getSceneBounds();
    // We allow a ~20% threshold when fitting the scene
    QSizeF size = bounds.size() * 0.8;


    // Check each zoom level until we find one where our scene fits the graphics view
    int step = 4;
    for (; step >= 0; --step)
    {
        QSizeF viewportSize(
            (float)width() / ScaleSteps[step],
            (float)height() / ScaleSteps[step]
        );

        if (viewportSize.width() >= size.width() && viewportSize.height() >= size.height())
        {
            break;
        }
    }

    setScaleStep(step);
    centerOn(bounds.center().x(), bounds.center().y());
}

void ScriptGraphicsView::setScaleStep(int step)
{
    if (step < 0)
        step = 0;
    if (step >= MaxScaleSteps)
        step = MaxScaleSteps - 1;
    scaleStep_ = step;

    updateTransformation();
}

void ScriptGraphicsView::setup()
{
    bgItem_ = new ScaledPixmapItem(background_);
    bgItem_->setZValue(-100);
    updateBackground();
    scene()->addItem(bgItem_);
}

void ScriptGraphicsView::scrollContentsBy(int dx, int dy)
{
    QGraphicsView::scrollContentsBy(dx, dy);
    updateBackground();
}


void ScriptGraphicsView::resizeEvent(QResizeEvent * event)
{
    QGraphicsView::resizeEvent(event);
    updateBackground();
}

void ScriptGraphicsView::wheelEvent(QWheelEvent * event)
{
    if (event->delta() > 0)
    {
        setScaleStep(scaleStep_ + 1);
    }
    else
    {
        setScaleStep(scaleStep_ - 1);
    }
}

void ScriptGraphicsView::updateBackground()
{
    int viewportWidth = viewport()->width();
    int viewportHeight = viewport()->height();
    QPointF size = mapToScene(QPoint(viewportWidth, viewportHeight)) - mapToScene(QPoint(0,0));
    float w = bgItem_->setSize(size.y());
    float viewWidth = viewportWidth - (w / size.x() * viewportWidth);

    bgItem_->setOffset(mapToScene(QPoint(viewWidth, 0)));
}

void ScriptGraphicsView::updateTransformation()
{
    setTransformationAnchor(QGraphicsView::AnchorUnderMouse);
    QTransform trans;
    trans.scale(ScaleSteps[scaleStep_], ScaleSteps[scaleStep_]);
    setTransform(trans);
}
