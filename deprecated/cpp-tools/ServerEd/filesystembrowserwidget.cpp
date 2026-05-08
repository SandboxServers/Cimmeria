#include "filesystembrowserwidget.h"
#include "ui_filesystembrowserwidget.h"

FilesystemBrowserWidget::FilesystemBrowserWidget(QWidget *parent) :
QDockWidget(parent),
ui(new Ui::FilesystemBrowserWidget)
{
    ui->setupUi(this);

    filesystem_ = new QFileSystemModel(this);
    filesystem_->setRootPath("../data/scripts/");
    ui->projectBrowser->setModel(filesystem_);
    ui->projectBrowser->setRootIndex(filesystem_->index("../data/scripts"));
    ui->projectBrowser->setColumnHidden(1, true);
    ui->projectBrowser->setColumnHidden(2, true);
    ui->projectBrowser->setColumnHidden(3, true);
}

FilesystemBrowserWidget::~FilesystemBrowserWidget()
{
    delete ui;
}

void FilesystemBrowserWidget::on_projectBrowser_doubleClicked(const QModelIndex &index)
{
    if (!filesystem_->isDir(index))
        emit fileOpenRequested(filesystem_->filePath(index));
}
