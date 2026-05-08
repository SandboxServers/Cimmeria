#include "scriptdatabaselookupwidget.h"
#include "ui_scriptdatabaselookupwidget.h"
#include <QKeyEvent>

ScriptDatabaseLookupWidget::ScriptDatabaseLookupWidget(QWidget *parent) :
QWidget(parent),
ui(new Ui::ScriptDatabaseLookupWidget),
waitingForResults_(false),
changed_(false)
{
    ui->setupUi(this);
    ui->matches->setColumnWidth(0, 75);
    ui->matches->setColumnWidth(1, 270);
    ui->matches->verticalHeader()->setDefaultSectionSize(17);
    setWindowFlags(0);
    setWindowFlags((Qt::WindowFlags)(Qt::CustomizeWindowHint));
}

ScriptDatabaseLookupWidget::~ScriptDatabaseLookupWidget()
{
    delete ui;
}

void ScriptDatabaseLookupWidget::searchResults(QStringList ids, QStringList names)
{
    waitingForResults_ = false;

    if (changed_)
    {
        changed_ = false;
        waitingForResults_ = true;
        emit searchFor(ui->nameFragment->text());
    }

    while (ui->matches->rowCount() > 0)
        ui->matches->removeRow(0);
    Q_ASSERT(ids.size() == names.size());
    for (int i = 0; i < ids.size(); i++)
    {
        int rows = ui->matches->rowCount();
        ui->matches->insertRow(rows);
        QTableWidgetItem * id = new QTableWidgetItem(ids[i]);
        ui->matches->setItem(rows, 0, id);
        QTableWidgetItem * name = new QTableWidgetItem(names[i]);
        ui->matches->setItem(rows, 1, name);
    }
}

void ScriptDatabaseLookupWidget::focusOutEvent(QFocusEvent * event)
{
    QWidget::focusOutEvent(event);
    hide();
    emit closed();
}

void ScriptDatabaseLookupWidget::keyPressEvent(QKeyEvent * event)
{
    QWidget::keyPressEvent(event);
    if (event->key() == Qt::Key_Escape)
    {
        hide();
        emit closed();
    }
    else if (event->key() == Qt::Key_Return && ui->nameFragment->hasFocus())
    {
        if (ui->matches->rowCount() > 0)
            on_matches_doubleClicked(ui->matches->model()->index(0, 0));
    }
    else if (event->key() == Qt::Key_Return && ui->matches->hasFocus())
    {
        if (!ui->matches->selectedItems().empty())
            on_matches_doubleClicked(ui->matches->model()->index(ui->matches->selectedItems()[0]->row(), 0));
    }
    else if (event->key() == Qt::Key_Down && ui->nameFragment->hasFocus())
    {
        ui->matches->setFocus();
        ui->matches->selectRow(0);
    }
    else if (event->key() == Qt::Key_Down && ui->matches->hasFocus())
    {
        if (ui->matches->selectedItems().empty())
            ui->matches->selectRow(0);
        else if (ui->matches->rowCount() > ui->matches->selectedItems()[0]->row())
            ui->matches->selectRow(ui->matches->selectedItems()[0]->row() + 1);
    }
    else if (event->key() == Qt::Key_Up && ui->matches->hasFocus())
    {
        if (ui->matches->selectedItems().empty())
            ui->matches->selectRow(0);
        else if (ui->matches->selectedItems()[0]->row() > 0)
            ui->matches->selectRow(ui->matches->selectedItems()[0]->row() - 1);
        else
            ui->nameFragment->setFocus();
    }
}

void ScriptDatabaseLookupWidget::on_nameFragment_textChanged(const QString & text)
{
    if (!waitingForResults_)
    {
        waitingForResults_ = true;
        emit searchFor(ui->nameFragment->text());
    }
    else
        changed_ = true;
}

void ScriptDatabaseLookupWidget::on_matches_doubleClicked(const QModelIndex &index)
{
    if (index.isValid())
        emit selectItem(ui->matches->model()->data(ui->matches->model()->index(index.row(), 0)).toInt());
}
