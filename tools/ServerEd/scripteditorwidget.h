#ifndef SCRIPTEDITORWIDGET_H
#define SCRIPTEDITORWIDGET_H

#include <QDockWidget>
#include <QGraphicsView>
#include <QGraphicsScene>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QMainWindow>
#include "scriptnodeseditor.h"
#include "editorconfiguration.h"
#include "scriptgraphicsview.h"
#include "objectdatabase.h"
#include "propertybrowser/qtpropertymanager.h"
#include "propertybrowser/qttreepropertybrowser.h"

class QtVariantPropertyManager;
class QtProperty;
class QNEBlock;
class QNEComment;

namespace Ui {
class ScriptEditorWidget;
}

class ScriptEditorWidget : public QDockWidget
{
    Q_OBJECT
    
public:
    explicit ScriptEditorWidget(EditorConfiguration * config, ScriptDefinitions * definitions, QWidget *parent = 0);
    ~ScriptEditorWidget();

    bool resetScript();
    bool saveScript(bool onlyIfDirty = false);
    bool saveScript(QString const & path);
    bool loadScript(QString const & path);

    bool compileScript();
    bool compileScript(QString const & path);

    QString const & scriptPath() const;
    ScriptNodesEditor * editor() const;

    bool eventFilter(QObject * object, QEvent * event);

signals:
    void closed(ScriptEditorWidget * widget);
    void focused(ScriptEditorWidget * widget);
    void lookupName(QString const & ref, QString const & fragment);
    void lookupId(QString const & ref, int id);
    void searchResults(QStringList ids, QStringList names);

public slots:
    void onSave();
    void onSaveAs();
    void onCompileScript();
    void onCompileScriptAs();
    void onCompileAndReload();
    void onReloadOnServer();
    void onSettings();

    void idLookupCompleted(QString ref, int id, QString name);
    void idLookupFailed(QString ref, int id, QString reason);
    void nameLookupCompleted(QString ref, QString name, ObjectDatabase::SearchResult * results);
    void nameLookupFailed(QString ref, QString name, QString reason);

    void lookupWidgetClosed();
    void searchFor(QString text);
    void selectItem(int id);

private slots:
    void blockSelected(QNEBlock * block, ScriptDefinitions::NodeTemplate const * tpl);
    void commentSelected(QNEComment * comment);
    void propertyChanged(QtProperty *property, const QVariant &val);
    void dbPropertyChanged(QtProperty *property, const int &val);
    void enumPropertyChanged(QtProperty *property, const qlonglong & val);
    void flagPropertyChanged(QtProperty *property, const qlonglong & val);
    void dbBrowseClicked();
    void onDeleteNode();
    void onDirty(bool dirty);

    void dbNameRequested(QtProperty *property);
    void dbPropertyChanging();
    
private:
    class ServerConnector * serverConnector();
    void setCurrentPath(QString const & path);
    virtual void closeEvent(QCloseEvent * event);
    virtual bool event(QEvent * e);

    QMainWindow * contents_;
    QVBoxLayout * verticalLayout_;
    ScriptGraphicsView * view_;
    QtTreePropertyBrowser * propertyBrowser_;

    class EditorConfiguration * config_;
    ScriptDefinitions * definitions_;
    class ServerConnector * server_;

    QGraphicsScene * scene_;
    ScriptNodesEditor * editor_;
    QtVariantPropertyManager * properties_;
    QtDatabaseIntPropertyManager * dbProperties_;
    class QtDatabaseSpinBoxFactory * dbFactory_;
    QtEnumPropertyManager * enumProperties_;
    QtFlagPropertyManager * flagProperties_;
    class ScriptDatabaseLookupWidget * lookupWidget_;
    QString lookupRef_;
    bool initializingProperty_;
    QNEBlock * block_;
    QNEComment * comment_;
    ScriptDefinitions::NodeTemplate const * template_;
    QString currentFile_, currentDisplayFile_;
    bool dirty_;

    class DatabaseRequest * idLookup_;
};

#endif // SCRIPTEDITORWIDGET_H
