#ifndef SCRIPTDATABASELOOKUPWIDGET_H
#define SCRIPTDATABASELOOKUPWIDGET_H

#include <QWidget>

namespace Ui {
class ScriptDatabaseLookupWidget;
}

class ScriptDatabaseLookupWidget : public QWidget
{
    Q_OBJECT
    
public:
    explicit ScriptDatabaseLookupWidget(QWidget *parent = 0);
    ~ScriptDatabaseLookupWidget();

signals:
    void searchFor(QString text);
    void selectItem(int id);
    void closed();

public slots:
    void searchResults(QStringList ids, QStringList names);

protected:
    virtual void focusOutEvent(QFocusEvent * event);
    virtual void keyPressEvent(QKeyEvent * event);
    
private slots:
    void on_nameFragment_textChanged(const QString &arg1);

    void on_matches_doubleClicked(const QModelIndex &index);

private:
    Ui::ScriptDatabaseLookupWidget *ui;
    bool waitingForResults_, changed_;
};

#endif // SCRIPTDATABASELOOKUPWIDGET_H
