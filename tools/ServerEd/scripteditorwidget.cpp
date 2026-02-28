#define QT_FORCE_ASSERTS

#include "scripteditorwidget.h"
#include "ui_scripteditorwidget.h"
#include "scriptnodeseditor.h"
#include "scriptcompiler.h"
#include "qneblock.h"
#include "qneport.h"
#include "qnecomment.h"
#include "propertybrowser/qtvariantproperty.h"
#include "propertybrowser/qteditorfactory.h"
#include "configurationdialog.h"
#include "serverconnector.h"
#include "databaseworker.h"
#include "scriptdatabaselookupwidget.h"

#include <QCloseEvent>
#include <QMessageBox>
#include <QScrollBar>
#include <QShortcut>
#include <QFileDialog>
#include <QMenuBar>

extern QStringList gLoadWarnings;

ScriptEditorWidget::ScriptEditorWidget(EditorConfiguration * config, ScriptDefinitions * definitions, QWidget *parent) :
    QDockWidget(parent),
    config_(config),
    definitions_(definitions),
    server_(nullptr),
    editor_(nullptr),
    lookupWidget_(nullptr),
    initializingProperty_(false),
    block_(nullptr),
    comment_(nullptr),
    template_(nullptr),
    dirty_(false)
{
    if (objectName().isEmpty())
        setObjectName(QStringLiteral("ScriptEditorWidget"));
    setMinimumSize(QSize(300, 400));
    setFloating(true);
    setFeatures(QDockWidget::DockWidgetClosable | QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable);
    setAllowedAreas(Qt::RightDockWidgetArea);

    contents_ = new QMainWindow(0);
    QWidget * central = new QWidget(contents_);

    QMenu * fileMenu = new QMenu("Script", contents_);
    QAction * save = fileMenu->addAction("Save (Ctrl+S)");
    QObject::connect(save, SIGNAL(triggered()), this, SLOT(onSave()));
    QAction * saveAs = fileMenu->addAction("Save As ... (Ctrl+Alt+S)");
    QObject::connect(saveAs, SIGNAL(triggered()), this, SLOT(onSaveAs()));
    QAction * close = fileMenu->addAction("Close");
    QObject::connect(close, SIGNAL(triggered()), this, SLOT(close()));
    contents_->menuBar()->addMenu(fileMenu);

    QMenu * toolsMenu = new QMenu("Compile", contents_);
    QAction * compile = toolsMenu->addAction("Compile (F5)");
    QObject::connect(compile, SIGNAL(triggered()), this, SLOT(onCompileScript()));
    QObject::connect(toolsMenu->addAction("Compile As ..."), SIGNAL(triggered()), this, SLOT(onCompileScriptAs()));
    QAction * compileReload = toolsMenu->addAction("Compile and reload (F6)");
    QObject::connect(compileReload, SIGNAL(triggered()), this, SLOT(onCompileAndReload()));
    QObject::connect(toolsMenu->addAction("Reload on server"), SIGNAL(triggered()), this, SLOT(onReloadOnServer()));
    QObject::connect(toolsMenu->addAction("Settings"), SIGNAL(triggered()), this, SLOT(onSettings()));
    contents_->menuBar()->addMenu(toolsMenu);

    verticalLayout_ = new QVBoxLayout(central);

    view_ = new ScriptGraphicsView(0);
    view_->setObjectName(QStringLiteral("view"));
    QSizePolicy sizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    sizePolicy.setHorizontalStretch(0);
    sizePolicy.setVerticalStretch(0);
    sizePolicy.setHeightForWidth(view_->sizePolicy().hasHeightForWidth());
    view_->setSizePolicy(sizePolicy);
    view_->setMinimumSize(QSize(100, 100));
    verticalLayout_->addWidget(view_);

    propertyBrowser_ = new QtTreePropertyBrowser(0);
    propertyBrowser_->setMinimumSize(QSize(100, 60));
    propertyBrowser_->setMaximumSize(QSize(16777215, 200));
    verticalLayout_->addWidget(propertyBrowser_);

    contents_->setCentralWidget(central);
    contents_->setParent(this);
    setWidget(contents_);
    QMetaObject::connectSlotsByName(this);

    // TODO: Add multiple object selection feature?
    view_->setDragMode(QGraphicsView::NoDrag);
    // ui->graphicsView->setRenderHint(QPainter::Antialiasing);
    view_->setViewportUpdateMode(QGraphicsView::BoundingRectViewportUpdate);
    scene_ = new QGraphicsScene(this);
    scene_->setBackgroundBrush(QColor(0xC0C0C0));
    view_->setScene(scene_);

    properties_ = new QtVariantPropertyManager();
    dbProperties_ = new QtDatabaseIntPropertyManager();
    enumProperties_ = new QtEnumPropertyManager();
    flagProperties_ = new QtFlagPropertyManager();
    propertyBrowser_->setFactoryForManager(properties_, new QtVariantEditorFactory());
    dbFactory_ = new QtDatabaseSpinBoxFactory();
    propertyBrowser_->setFactoryForManager((QtIntPropertyManager *)dbProperties_, dbFactory_);
    propertyBrowser_->setFactoryForManager(enumProperties_, new QtEnumEditorFactory());
    propertyBrowser_->setFactoryForManager(flagProperties_->subBoolPropertyManager(), new QtCheckBoxFactory());
    QObject::connect(properties_, SIGNAL(valueChanged(QtProperty*,QVariant)),
                     this, SLOT(propertyChanged(QtProperty*,QVariant)));
    QObject::connect(dbProperties_, SIGNAL(valueChanged(QtProperty*,int)),
                     this, SLOT(dbPropertyChanged(QtProperty*,int)));
    QObject::connect(dbFactory_, SIGNAL(valueUpdated()),
                     this, SLOT(dbPropertyChanging()));
    QObject::connect(dbProperties_, SIGNAL(dbNameRequested(QtProperty*)),
                     this, SLOT(dbNameRequested(QtProperty*)));
    QObject::connect(dbFactory_, SIGNAL(onBrowseClicked()),
                     this, SLOT(dbBrowseClicked()));
    QObject::connect(enumProperties_, SIGNAL(valueChanged(QtProperty*,qlonglong)),
                     this, SLOT(enumPropertyChanged(QtProperty*,qlonglong)));
    QObject::connect(flagProperties_, SIGNAL(valueChanged(QtProperty*,qlonglong)),
                     this, SLOT(flagPropertyChanged(QtProperty*,qlonglong)));

    installEventFilter(this);
    installEventFilter(contents_);
    installEventFilter(view_);
    installEventFilter(properties_);

    resetScript();
}

ScriptEditorWidget::~ScriptEditorWidget()
{
    if (lookupWidget_)
        lookupWidget_->deleteLater();
    delete contents_;
    delete enumProperties_;
    delete flagProperties_;
    delete dbProperties_;
    delete properties_;
    delete editor_;
    delete scene_;
}

bool ScriptEditorWidget::resetScript()
{
    if (editor_)
    {
        delete editor_;
    }

    editor_ = new ScriptNodesEditor(definitions_);
    editor_->install(scene_);
    QObject::connect(editor_, SIGNAL(blockSelected(QNEBlock*,ScriptDefinitions::NodeTemplate const *)),
                     this, SLOT(blockSelected(QNEBlock*,ScriptDefinitions::NodeTemplate const *)));
    QObject::connect(editor_, SIGNAL(commentSelected(QNEComment*)),
                     this, SLOT(commentSelected(QNEComment*)));
    QObject::connect(editor_, SIGNAL(dirty(bool)), this, SLOT(onDirty(bool)));
    view_->setup();
    view_->setScaleStep(4);
    view_->horizontalScrollBar()->setValue(0);
    view_->verticalScrollBar()->setValue(0);
    setCurrentPath("");
    return true;
}

bool ScriptEditorWidget::saveScript(bool onlyIfDirty)
{
    if (!onlyIfDirty || dirty_)
    {
        if (onlyIfDirty && dirty_)
        {
            int button = QMessageBox::question(this, "Save changes", "Do you want to save your changes in script '" + currentDisplayFile_ + "'?",
                                               QMessageBox::Yes, QMessageBox::No, QMessageBox::Cancel);
            if (button == QMessageBox::No)
            {
                // The user doesn't want to save the script, so mark it as
                // non-dirty to prevent further dialogs during autosave/close
                onDirty(false);
                return true;
            }
            else if (button == QMessageBox::Cancel)
                return false;
        }

        if (currentFile_.length() == 0)
        {
            QString dataPath = QDir(config_->get("ServerPath").toString() + "/data/scripts").absolutePath();
            QString path = QFileDialog::getSaveFileName(this, "Save Script " + currentDisplayFile_, dataPath,
                "Script files (*.script);;All files (*.*)");
            if (path.length() == 0)
                return false;
            setCurrentPath(path);
        }
        if (!saveScript(currentFile_))
            return false;
    }

    return true;
}

bool ScriptEditorWidget::saveScript(QString const & path)
{
    if (editor_->save(path))
    {
        if (currentFile_ == "")
            setCurrentPath(path);
        return true;
    }
    else
        return false;
}

bool ScriptEditorWidget::loadScript(QString const & path)
{
    gLoadWarnings.clear();
    bool loaded = editor_->load(path);
    if (loaded)
    {
        setCurrentPath(path);
        view_->centerOnScene();
    }

    if (!gLoadWarnings.empty())
    {
        QString warnMsg = "The following warnings were raised while loading the script:\r\n\r\n";
        foreach (QString const & s, gLoadWarnings)
        {
            warnMsg += s + "\r\n";
        }
        QMessageBox::warning(this, "Load Warnings", warnMsg);
    }
    return loaded;
}

QString const & ScriptEditorWidget::scriptPath() const
{
    return currentFile_;
}

ScriptNodesEditor * ScriptEditorWidget::editor() const
{
    return editor_;
}


bool ScriptEditorWidget::eventFilter(QObject * object, QEvent * event)
{
    if (event->type() == QEvent::KeyPress)
    {
        QKeyEvent * key = static_cast<QKeyEvent *>(event);
        Qt::KeyboardModifiers modifiers = key->modifiers();

        if (key->key() == Qt::Key_S && modifiers & Qt::ControlModifier)
        {
            if (modifiers & Qt::AltModifier)
                onSaveAs();
            else
                onSave();
        }
        if (key->key() == Qt::Key_W && modifiers & Qt::ControlModifier)
            close();
        else if (key->key() == Qt::Key_F5)
            onCompileScript();
        else if (key->key() == Qt::Key_F6)
            onCompileAndReload();
        else if (key->key() == Qt::Key_Delete)
            onDeleteNode();
    }

    return false;
}

void ScriptEditorWidget::blockSelected(QNEBlock * block, ScriptDefinitions::NodeTemplate const * tpl)
{
    dbProperties_->flushNames();
    propertyBrowser_->clear();
    block_ = block;
    template_ = tpl;
    comment_ = nullptr;
    if (block)
    {
        foreach (ScriptDefinitions::PropertyTemplate const & prop, tpl->properties)
        {
            if (prop.variantType != QVariant::Invalid)
            {
                switch (prop.editorType)
                {
                case ScriptDefinitions::PropertyTemplate::PlainProperty:
                {
                    QVariant value = block->getProperty(prop.name);
                    if (!value.isValid())
                        value = prop.variantDefaultValue;

                    if (prop.databaseRef)
                    {
                        QtProperty * property = dbProperties_->addProperty(prop.name);
                        initializingProperty_ = true;
                        dbProperties_->setValue(property, value.toInt());
                        initializingProperty_ = false;
                        propertyBrowser_->addProperty(property);
                    }
                    else
                    {
                        QtVariantProperty * property = properties_->addProperty(prop.variantType, prop.name);
                        initializingProperty_ = true;
                        property->setValue(value);
                        initializingProperty_ = false;
                        propertyBrowser_->addProperty(property);
                    }
                    break;
                }

                case ScriptDefinitions::PropertyTemplate::EnumProperty:
                {
                    ScriptDefinitions::Enumeration * e = definitions_->findEnumeration(prop.type);
                    QStringList names = e->values.keys();
                    QtProperty * property = enumProperties_->addProperty(prop.name);
                    qulonglong value = block->getProperty(prop.name).toULongLong();
                    int index = e->values.values().indexOf(value);
                    initializingProperty_ = true;
                    enumProperties_->setEnumNames(property, names);
                    enumProperties_->setValue(property, index);
                    initializingProperty_ = false;
                    propertyBrowser_->addProperty(property);
                    break;
                }

                case ScriptDefinitions::PropertyTemplate::FlagsProperty:
                {
                    ScriptDefinitions::Enumeration * e = definitions_->findEnumeration(prop.type);
                    QStringList names = e->values.keys();
                    QtProperty * property = flagProperties_->addProperty(prop.name);
                    qulonglong value = block->getProperty(prop.name).toULongLong();

                    qlonglong indices = 0;
                    qulonglong mask = 1;
                    for (int i = 0; i < 32; i++)
                    {
                        if (value & mask)
                        {
                            int index = e->values.values().indexOf(mask);
                            if (index != -1)
                                indices |= ((qulonglong)1 << index);
                        }
                        mask <<= 1;
                    }
                    initializingProperty_ = true;
                    flagProperties_->setFlagNames(property, names);
                    flagProperties_->setValue(property, indices);
                    initializingProperty_ = false;
                    propertyBrowser_->addProperty(property);
                    break;
                }

                default:
                    qFatal("Illegal property template editor type");
                }
            }
        }
    }
}

void ScriptEditorWidget::commentSelected(QNEComment * comment)
{
    propertyBrowser_->clear();
    block_ = nullptr;
    comment_ = comment;

    QtVariantProperty * property = properties_->addProperty(QVariant::String, "Comment");
    property->setValue(comment_->comment());
    propertyBrowser_->addProperty(property);

    property = properties_->addProperty(QVariant::Int, "Width");
    property->setValue((int)comment_->boxSize().width());
    propertyBrowser_->addProperty(property);

    property = properties_->addProperty(QVariant::Int, "Height");
    property->setValue((int)comment_->boxSize().height());
    propertyBrowser_->addProperty(property);

    property = properties_->addProperty(QVariant::Int, "Color Scheme");
    property->setValue(comment_->colorScheme());
    propertyBrowser_->addProperty(property);
}

void ScriptEditorWidget::propertyChanged(QtProperty *property, const QVariant &val)
{
    if (block_)
    {
        if (!initializingProperty_)
        {
            block_->setProperty(property->propertyName(), val);
            editor_->setDirty();
        }
    }
    else if (comment_)
    {
        QString name = property->propertyName();
        if (name == "Comment")
        {
            comment_->setComment(val.value<QString>());
            editor_->setDirty();
        }
        else if (name == "Width")
        {
            comment_->setBoxSize(QSizeF(val.value<int>(), comment_->boxSize().height()));
            editor_->setDirty();
        }
        else if (name == "Height")
        {
            comment_->setBoxSize(QSizeF(comment_->boxSize().width(), val.value<int>()));
            editor_->setDirty();
        }
        else if (name == "Color Scheme")
        {
            comment_->setColorScheme(val.value<int>());
            editor_->setDirty();
        }
    }
}

void ScriptEditorWidget::dbPropertyChanged(QtProperty *property, const int &val)
{
    if (block_ && !initializingProperty_)
    {
        block_->setProperty(property->propertyName(), val);
        editor_->setDirty();
    }
}

void ScriptEditorWidget::enumPropertyChanged(QtProperty *property, const qlonglong & val)
{
    if (initializingProperty_)
        return;

    ScriptDefinitions::PropertyTemplate const * prop = template_->findProperty(property->propertyName());
    ScriptDefinitions::Enumeration * e = definitions_->findEnumeration(prop->type);
    block_->setProperty(property->propertyName(), e->values.values()[val]);
    editor_->setDirty();
}

void ScriptEditorWidget::flagPropertyChanged(QtProperty *property, const qlonglong & val)
{
    if (initializingProperty_)
        return;

    ScriptDefinitions::PropertyTemplate const * prop = template_->findProperty(property->propertyName());
    ScriptDefinitions::Enumeration * e = definitions_->findEnumeration(prop->type);

    qlonglong value = 0;
    qlonglong mask = 1;
    for (int i = 0; i < 32; i++)
    {
        if (val & mask)
        {
            value |= e->values.values()[i];
        }
        mask <<= 1;
    }
    block_->setProperty(property->propertyName(), value);
    editor_->setDirty();
}

void ScriptEditorWidget::dbBrowseClicked()
{
    QtProperty * property = propertyBrowser_->currentItem()->property();
    Q_ASSERT(property && block_);
    ScriptDefinitions::PropertyTemplate const * tpl = block_->getTemplate()->findProperty(property->propertyName());
    Q_ASSERT(tpl);
    if (tpl->databaseRef)
    {
        if (lookupWidget_)
            lookupWidgetClosed();
        lookupRef_ = tpl->databaseName;
        lookupWidget_ = new ScriptDatabaseLookupWidget();
        connect(lookupWidget_, SIGNAL(searchFor(QString)), this, SLOT(searchFor(QString)));
        connect(lookupWidget_, SIGNAL(selectItem(int)), this, SLOT(selectItem(int)));
        connect(lookupWidget_, SIGNAL(closed()), this, SLOT(lookupWidgetClosed()));
        connect(this, SIGNAL(searchResults(QStringList,QStringList)), lookupWidget_, SLOT(searchResults(QStringList,QStringList)));
        lookupWidget_->show();
    }
}

void ScriptEditorWidget::onDeleteNode()
{
    foreach (QGraphicsItem * item, scene_->selectedItems())
    {
        QNEItem * editorItem = dynamic_cast<QNEItem *>(item);
        if (editorItem)
            editor_->removeItem(editorItem);
    }
}

void ScriptEditorWidget::onDirty(bool dirty)
{
    if (dirty != dirty_)
    {
        if (dirty)
            setWindowTitle(currentDisplayFile_ + " *");
        else
            setWindowTitle(currentDisplayFile_);
        dirty_ = dirty;
    }
}

void ScriptEditorWidget::dbNameRequested(QtProperty * property)
{
    Q_ASSERT(property && block_);
    ScriptDefinitions::PropertyTemplate const * tpl = block_->getTemplate()->findProperty(property->propertyName());
    Q_ASSERT(tpl && tpl->databaseRef);
    emit lookupId(tpl->databaseName, dbProperties_->value(property));
}

void ScriptEditorWidget::dbPropertyChanging()
{
    if (propertyBrowser_->currentItem()->property())
        dbNameRequested(propertyBrowser_->currentItem()->property());
}

void ScriptEditorWidget::idLookupCompleted(QString ref, int id, QString name)
{
    if (!block_)
        return;

    foreach (QtProperty * prop, dbProperties_->properties())
    {
        ScriptDefinitions::PropertyTemplate const * tpl = block_->getTemplate()->findProperty(prop->propertyName());
        if (tpl && tpl->databaseName == ref)
        {
            dbProperties_->setDbName(prop->propertyName(), name, prop);
            dbFactory_->dbNameUpdated(prop, name);
        }
    }
}

void ScriptEditorWidget::idLookupFailed(QString ref, int id, QString reason)
{
    qDebug("LookupFailed: %s", qPrintable(reason));
    idLookupCompleted(ref, id, "(Database Error)");
}

void ScriptEditorWidget::nameLookupCompleted(QString ref, QString name, ObjectDatabase::SearchResult * results)
{
    qDebug("NameLookupCompleted");
    if (lookupWidget_ && ref == lookupRef_)
    {
        QStringList ids, names;
        foreach (QStringList row, results->rows)
        {
            ids.push_back(row[0]);
            names.push_back(row[1]);
        }

        emit searchResults(ids, names);
    }
}

void ScriptEditorWidget::nameLookupFailed(QString ref, QString name, QString reason)
{
    qDebug("NameLookupFailed: %s", qPrintable(reason));
    QStringList ids, names;
    ids.push_back("");
    ids.push_back("");
    names.push_back("(Lookup failed!)");
    names.push_back(reason);
    emit searchResults(ids, names);
}

void ScriptEditorWidget::lookupWidgetClosed()
{
    Q_ASSERT(lookupWidget_);
    lookupWidget_->deleteLater();
    lookupWidget_ = nullptr;
}

void ScriptEditorWidget::searchFor(QString text)
{
    emit lookupName(lookupRef_, text);
}

void ScriptEditorWidget::selectItem(int id)
{
    lookupWidgetClosed();

    QtProperty * property = propertyBrowser_->currentItem()->property();
    Q_ASSERT(property && block_);
    ScriptDefinitions::PropertyTemplate const * tpl = block_->getTemplate()->findProperty(property->propertyName());
    Q_ASSERT(tpl && tpl->databaseRef && tpl->databaseName == lookupRef_);
    dbProperties_->setValue(property, id);
    dbNameRequested(property);
}


void ScriptEditorWidget::onSave()
{
    saveScript();
}

void ScriptEditorWidget::onSaveAs()
{
    QString path = QFileDialog::getSaveFileName(this, "Save Script " + currentDisplayFile_, QString(),
        "Script files (*.script);;All files (*.*)");
    if (path.length() == 0)
        return;
    setCurrentPath(path);
    saveScript(currentFile_);
}

void ScriptEditorWidget::onCompileScript()
{
    compileScript();
}

void ScriptEditorWidget::onCompileScriptAs()
{
    QString path = QFileDialog::getSaveFileName(this, "Compile Script" + currentDisplayFile_, QString(),
        "Python script files (*.py);;All files (*.*)");
    if (path.length() == 0)
        return;

    compileScript(path);
}

void ScriptEditorWidget::onReloadOnServer()
{
    if (config_->get("ServerHost").toString() == "")
    {
        QMessageBox::warning(this, "Reload Failed", "Cannot reload script: server address is not set.");
        return;
    }

    if (config_->get("ServerPassword").toString() == "")
    {
        QMessageBox::warning(this, "Reload Failed", "Cannot reload script: server password is not set.");
        return;
    }

    ReloadScriptRequest * req = new ReloadScriptRequest(
        config_->get("PlayerName").toString(),
        editor_->getScriptModule(),
        editor_->getScriptType().toLower()
    );
    serverConnector()->submitRequest(req);
}

void ScriptEditorWidget::onCompileAndReload()
{
    if (compileScript())
        onReloadOnServer();
}

void ScriptEditorWidget::onSettings()
{
    ConfigurationDialog config;
    config.load(editor_);
    config.load(config_);
    if (config.exec())
    {
        config.save(editor_);
        config.save(config_);
        config_->save("Configuration.xml");
    }
}

ServerConnector * ScriptEditorWidget::serverConnector()
{
    if (!server_)
    {
        server_ = new ServerConnector();
        server_->setServerAddress(config_->get("ServerHost").toString(), config_->get("ServerPort").toInt());
        server_->setPassword(config_->get("ServerPassword").toString());
    }

    return server_;
}

bool ScriptEditorWidget::compileScript()
{
    if (config_->get("ServerPath").toString() == "")
    {
        QMessageBox::warning(this, "Compilation Failed", "Cannot compile script: server root path is not set.");
        return false;
    }

    if (editor_->getScriptModule() == "")
    {
        QMessageBox::warning(this, "Compilation Failed", "Cannot compile script: script module name is not set.");
        return false;
    }

    QString subdir = "";
    if (editor_->getScriptType() == "Level")
        subdir = "spaces/";
    else if (editor_->getScriptType() == "Mission")
        subdir = "missions/";
    else if (editor_->getScriptType() == "Effect")
        subdir = "effects/";
    else
        Q_ASSERT(false && "Invalid script type!");

    QString path = config_->get("ServerPath").toString() + "/python/cell/" + subdir + editor_->getScriptModule().replace(".", "/") + ".py";
    return compileScript(path);
}


bool ScriptEditorWidget::compileScript(QString const & path)
{
    gLoadWarnings.clear();
    ScriptOptimizer opt(*editor_);
    bool compiled = opt.compile(path);
    if (!gLoadWarnings.empty())
    {
        QString warnMsg = "The following warnings were raised while compiling the script:\r\n\r\n";
        foreach (QString const & s, gLoadWarnings)
        {
            warnMsg += s + "\r\n";
        }
        QMessageBox::warning(this, "Compilation Warnings", warnMsg);
    }
    return compiled;
}

void ScriptEditorWidget::setCurrentPath(QString const & path)
{
    if (path == "")
    {
        currentFile_ = path;
        currentDisplayFile_ = "New Script";
    }
    else
    {
        QString abs = QDir(path).absolutePath();
        QString scriptAbs = QDir("../data/scripts").absolutePath();
        currentFile_ = abs;
        if (scriptAbs == abs.left(scriptAbs.length()))
            currentDisplayFile_ = abs.mid(scriptAbs.length() + 1);
        else
            currentDisplayFile_ = abs;
    }
    setWindowTitle(currentDisplayFile_);
}

void ScriptEditorWidget::closeEvent(QCloseEvent * event)
{
    event->ignore();
    if (saveScript(true))
    {
        event->accept();
        emit closed(this);
        deleteLater();
    }
}


bool ScriptEditorWidget::event(QEvent * e)
{
    if (e->type() == QEvent::NonClientAreaMouseButtonDblClick)
        return true;

    return QDockWidget::event(e);
}

