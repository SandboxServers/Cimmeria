#ifndef FILESYSTEMBROWSERWIDGET_H
#define FILESYSTEMBROWSERWIDGET_H

#include <QDockWidget>
#include <QFileSystemModel>

namespace Ui {
class FilesystemBrowserWidget;
}

class FilesystemBrowserWidget : public QDockWidget
{
    Q_OBJECT
    
public:
    explicit FilesystemBrowserWidget(QWidget *parent = 0);
    ~FilesystemBrowserWidget();

signals:
    void fileOpenRequested(QString path);
    
private slots:
    void on_projectBrowser_doubleClicked(const QModelIndex &index);

private:
    Ui::FilesystemBrowserWidget *ui;
    QFileSystemModel * filesystem_;
};

#endif // FILESYSTEMBROWSERWIDGET_H
