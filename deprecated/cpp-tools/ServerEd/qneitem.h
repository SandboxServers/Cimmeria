#ifndef QNEITEM_H
#define QNEITEM_H

#include <QGraphicsItem>

class QNodesEditor;
class QGraphicsScene;
class QXmlStreamWriter;
class QDomElement;

class QNEItem : public QGraphicsItem
{
public:
    typedef void * UserPtr;

    QNEItem(QNodesEditor * editor, QGraphicsScene * scene = 0);
    virtual ~QNEItem();

    unsigned int id() const;
    void setId(unsigned int id);
    QString const & comment() const;
    void setComment(QString const & comment);
    QNodesEditor * editor() const;

    virtual void save(QXmlStreamWriter * writer) = 0;
    virtual bool load(QDomElement * element) = 0;
    virtual void layoutChanged();
    virtual QNEItem * clone() = 0;

protected:
    void updateLayout();

private:
    bool needsLayout_;
    virtual void updateItemLayout() = 0;

    QNodesEditor * editor_;
    unsigned int id_;
    QString comment_;
};

#endif // QNEITEM_H
