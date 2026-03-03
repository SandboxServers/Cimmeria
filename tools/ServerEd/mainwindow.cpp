#include "mainwindow.h"
#include "ui_mainwindow.h"
#include "editorconfiguration.h"
#include "scripteditorwidget.h"
#include "filesystembrowserwidget.h"
#include "scriptcompiler.h"
#include "databaseworker.h"
#include "objectdatabase.h"
#include "chaineditorwidget.h"
#include "serverconnector.h"

#include <QCloseEvent>
#include <QMessageBox>
#include <QFileDialog>
#include <QStringList>
#include <QThread>

extern QStringList gLoadWarnings;

MainWindow::MainWindow(QWidget *parent) :
    QMainWindow(parent),
    ui(new Ui::MainWindow),
    chainEditor_(0)
{
    setTabPosition(Qt::RightDockWidgetArea, QTabWidget::North);
    setTabPosition(Qt::LeftDockWidgetArea, QTabWidget::North);
    ui->setupUi(this);

    ui->actionNew_Mission->setShortcut(QKeySequence("Ctrl+N"));
    ui->actionLoad_Mission->setShortcut(QKeySequence("Ctrl+L"));
    ui->actionQuit->setShortcut(QKeySequence("Ctrl+Q"));

    config_ = new EditorConfiguration();
    config_->load("Configuration.xml");

    dbThread_ = new DatabaseWorkerThread;
    dbWorker_ = new DatabaseWorker(config_);
    dbWorker_->moveToThread(dbThread_);
    connect(dbWorker_, SIGNAL(error(QString)), this, SLOT(onDbError(QString)));
    connect(dbThread_, SIGNAL(workerStarted()), dbWorker_, SLOT(startup()));
    connect(dbWorker_, SIGNAL(finished()), dbThread_, SLOT(quit()));
    connect(dbWorker_, SIGNAL(finished()), dbWorker_, SLOT(deleteLater()));
    connect(dbThread_, SIGNAL(finished()), dbThread_, SLOT(deleteLater()));
    dbThread_->start();

    definitions_ = new ScriptDefinitions("../entities/editor/nodes.xml", "../entities/editor/enumerations.xml");
    objectDb_ = new ObjectDatabase(definitions_, dbWorker_);

    FilesystemBrowserWidget * fs = new FilesystemBrowserWidget();
    QObject::connect(fs, SIGNAL(fileOpenRequested(QString)), this, SLOT(onFileOpenRequested(QString)));
    addDockWidget(Qt::LeftDockWidgetArea, fs);
    newScript();

    chainEditor_ = new ChainEditorWidget(dbWorker_, config_);
    QObject::connect(chainEditor_, SIGNAL(submitDbRequest(DatabaseRequest*)),
                     dbWorker_,   SLOT(submit(DatabaseRequest*)));
    QObject::connect(chainEditor_, SIGNAL(reloadOnServer()),
                     this,         SLOT(onReloadChainEngine()));
    addDockWidget(Qt::RightDockWidgetArea, chainEditor_);
    QList<ScriptEditorWidget *> existingEditors = findChildren<ScriptEditorWidget *>();
    if (!existingEditors.isEmpty())
        tabifyDockWidget(existingEditors.first(), chainEditor_);
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::newScript()
{
    createEditor();
}

bool MainWindow::openScript(QString const & path)
{
    QString absolutePath = QDir(path).absolutePath();
    foreach (ScriptEditorWidget * editor, scripts_)
    {
        if (editor->scriptPath() == absolutePath)
        {
            editor->raise();
            return true;
        }
    }

    ScriptEditorWidget * editor = createEditor();
    if (!editor->loadScript(path))
    {
        editor->deleteLater();
        return false;
    }
    else
        return true;
}


ScriptEditorWidget * MainWindow::createEditor()
{
    ScriptEditorWidget * editor = new ScriptEditorWidget(config_, definitions_);
    QObject::connect(editor, SIGNAL(closed(ScriptEditorWidget*)), this, SLOT(onScriptEditorClosed(ScriptEditorWidget*)));
    QObject::connect(editor, SIGNAL(lookupId(QString,int)), objectDb_, SLOT(lookup(QString,int)));
    QObject::connect(editor, SIGNAL(lookupName(QString,QString)), objectDb_, SLOT(lookup(QString,QString)));
    QObject::connect(objectDb_, SIGNAL(nameLookupCompleted(QString,QString,ObjectDatabase::SearchResult *)), editor, SLOT(nameLookupCompleted(QString,QString,ObjectDatabase::SearchResult *)));
    QObject::connect(objectDb_, SIGNAL(nameLookupFailed(QString,QString,QString)), editor, SLOT(nameLookupFailed(QString,QString,QString)));
    QObject::connect(objectDb_, SIGNAL(objectLookupCompleted(QString,int,QString)), editor, SLOT(idLookupCompleted(QString,int,QString)));
    QObject::connect(objectDb_, SIGNAL(objectLookupFailed(QString,int,QString)), editor, SLOT(idLookupFailed(QString,int,QString)));
    addDockWidget(Qt::RightDockWidgetArea, editor);
    QList<ScriptEditorWidget *> dockWidgets = findChildren<ScriptEditorWidget *>();
    foreach (ScriptEditorWidget * widget, dockWidgets)
    {
        if (editor != widget)
            tabifyDockWidget(widget, editor);
    }
    // Hack to work around raise() not working in tabbed dock areas
    qApp->processEvents();
    editor->raise();
    // Hack to work around dock widgets not being redrawn if a draw a requested after raise()
    qApp->processEvents();
    scripts_.append(editor);
    return editor;
}

void MainWindow::onFileOpenRequested(QString path)
{
    openScript(path);
}

void MainWindow::onScriptEditorClosed(ScriptEditorWidget * widget)
{
    int index = scripts_.indexOf(widget);
    if (index != -1)
        scripts_.remove(index);
}

void MainWindow::onDbError(QString error)
{
    QMessageBox::warning(this, "Database Error", error);
}

void MainWindow::on_actionNew_Mission_triggered()
{
    newScript();
}

void MainWindow::on_actionLoad_Mission_triggered()
{
    QString path = QFileDialog::getOpenFileName(this, "Load Script", "data/scripts/",
        "Script files (*.script);;All files (*.*)");
    if (path.length() == 0)
        return;
    openScript(path);
}

void MainWindow::on_actionQuit_triggered()
{
    close();
}

void MainWindow::on_actionBulk_Compile_Scripts_triggered()
{
    if (config_->get("ServerPath").toString() == "")
    {
        QMessageBox::warning(this, "Bulk Compile Failed", "Cannot compile scripts: server root path is not set.");
        return;
    }

    QFileDialog dlg(this, "Select directory to bulk compile");
    dlg.setFileMode(QFileDialog::Directory);
    dlg.setOption(QFileDialog::ShowDirsOnly);
    if (dlg.exec() && dlg.selectedFiles().length() == 1)
    {
        QStringList scripts;
        collectScripts(dlg.selectedFiles()[0], scripts);
        if (scripts.length() == 0)
        {
            QMessageBox::warning(this, "Bulk Compile", "Directory contains no script files.");
            return;
        }

        QString s("About to compile %1 scripts.\r\nProceed?");
        if (QMessageBox::question(this, "Bulk Compile", s.arg(scripts.length()), QMessageBox::Yes, QMessageBox::No) == QMessageBox::Yes)
        {
            setCursor(Qt::WaitCursor);
            QApplication::processEvents();
            foreach (QString const & script, scripts)
            {
                qDebug("CompileScripts: Compiling script %s", qPrintable(script));
                QGraphicsScene * scene = new QGraphicsScene();
                ScriptNodesEditor * editor = new ScriptNodesEditor(definitions_);
                editor->install(scene);
                if (editor->load(script))
                {
                    if (editor->getScriptModule() == "")
                    {
                        QMessageBox::warning(this, "Compilation Failed", "Cannot compile script: script module name is not set.");
                        continue;
                    }

                    QString subdir = "";
                    if (editor->getScriptType() == "Level")
                        subdir = "spaces/";
                    else if (editor->getScriptType() == "Mission")
                        subdir = "missions/";
                    else if (editor->getScriptType() == "Effect")
                        subdir = "effects/";
                    else
                        Q_ASSERT(false && "Invalid script type!");

                    QString path = config_->get("ServerPath").toString() + "/python/cell/" + subdir + editor->getScriptModule().replace(".", "/") + ".py";
                    QDir dir(QFileInfo(path).dir());
                    if (!dir.exists())
                        dir.mkdir(dir.path());

                    gLoadWarnings.clear();
                    ScriptOptimizer opt(*editor);
                    opt.compile(path);
                    if (!gLoadWarnings.empty())
                    {
                        QString warnMsg = "The following warnings were raised while compiling script " + script + ":\r\n\r\n";
                        foreach (QString const & s, gLoadWarnings)
                        {
                            warnMsg += s + "\r\n";
                        }
                        QMessageBox::warning(this, "Compilation Warnings", warnMsg);
                    }
                } else
                    QMessageBox::warning(this, "Load Failed", "Cannot compile script: failed to load script " + script);
                delete editor;
                delete scene;
            }
            setCursor(Qt::ArrowCursor);
            QMessageBox::information(this, "Bulk Compile", "Compilation completed.");
        }
    }
}

void MainWindow::collectScripts(QString const & path, QStringList & scriptPaths)
{
    qDebug("CollectScripts: Recursing into %s", qPrintable(path));
    QDir scripts(path);
    foreach (QFileInfo const & info, scripts.entryInfoList())
    {
        if (info.fileName() != "." && info.fileName() != "..")
        {
            if (info.isDir())
                collectScripts(info.filePath(), scriptPaths);
            else if (info.isFile() && info.suffix() == "script")
                scriptPaths.append(info.filePath());
        }
    }
}

void MainWindow::onReloadChainEngine()
{
    // Send a Python exec to the running game server asking it to reload
    // the content chain engine.  ServerConnector handles the TCP connection
    // and authentication internally.
    ServerConnector * conn = new ServerConnector(this);
    conn->setServerAddress(config_->get("ServerHost").toString(),
                           static_cast<qint16>(config_->get("PythonConsolePort").toInt()));
    conn->setPassword(config_->get("PythonConsolePassword").toString());
    conn->requestExec("engine.reload()");
    qDebug("MainWindow: sent engine.reload() to server");
}

void MainWindow::closeEvent(QCloseEvent * event)
{
    event->ignore();
    while (!scripts_.empty())
    {
        ScriptEditorWidget * editor = scripts_[0];
        if (editor->saveScript(true))
        {
            scripts_.remove(0);
            editor->close();
            editor->deleteLater();
        }
        else
            break;
    }

    if (scripts_.empty())
    {
        event->accept();
    }
}
