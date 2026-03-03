#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include "scriptnodeseditor.h"

class ScriptEditorWidget;
class ChainEditorWidget;

namespace Ui {
class MainWindow;
}

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = 0);
    ~MainWindow();

    void newScript();
    bool openScript(QString const & path);
    ScriptEditorWidget * createEditor();

private slots:
    void onFileOpenRequested(QString path);
    void onScriptEditorClosed(ScriptEditorWidget * widget);
    void onDbError(QString error);
    void onReloadChainEngine();

    void on_actionNew_Mission_triggered();
    void on_actionLoad_Mission_triggered();
    void on_actionQuit_triggered();
    void on_actionBulk_Compile_Scripts_triggered();

private:
    virtual void closeEvent(QCloseEvent * event);
    void collectScripts(QString const & path, QStringList &scriptPaths);

    Ui::MainWindow *ui;
    QVector<ScriptEditorWidget *> scripts_;
    class EditorConfiguration * config_;
    ScriptDefinitions * definitions_;
    class QThread * dbThread_;
    class DatabaseWorker * dbWorker_;
    class ObjectDatabase * objectDb_;
    ChainEditorWidget * chainEditor_;
};

#endif // MAINWINDOW_H
