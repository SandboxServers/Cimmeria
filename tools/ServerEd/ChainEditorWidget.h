#ifndef CHAINEDITORWIDGET_H
#define CHAINEDITORWIDGET_H

#include <QDockWidget>
#include <QVector>
#include <QString>

#include "ChainModel.h"

class QTreeWidget;
class QTreeWidgetItem;
class QTableWidget;
class QLineEdit;
class QSpinBox;
class QCheckBox;
class QComboBox;
class QPushButton;
class QSplitter;
class QGroupBox;
class QLabel;

class DatabaseWorker;
class DatabaseRequest;
class EditorConfiguration;

// ---------------------------------------------------------------------------
// ChainEditorWidget
//
// QDockWidget that provides a full CRUD UI for the content_chains family of
// tables (content_chains, content_triggers, content_conditions,
// content_actions).
//
// Layout:
//   Left  : QTreeWidget  — chains grouped by scope_type
//   Right : metadata fields + three QTableWidgets (Triggers / Conditions /
//           Actions), each with Add / Edit / Delete buttons
//   Bottom: "Save to DB" and "Reload on Server" buttons
// ---------------------------------------------------------------------------
class ChainEditorWidget : public QDockWidget
{
    Q_OBJECT
public:
    explicit ChainEditorWidget(DatabaseWorker * dbWorker,
                               EditorConfiguration * config,
                               QWidget *parent = 0);

signals:
    // Forwarded to DatabaseWorker::submit() by mainwindow
    void submitDbRequest(DatabaseRequest * request);

    // Emitted when "Reload on Server" is clicked; mainwindow calls
    // serverConnector->requestExec("engine.reload()")
    void reloadOnServer();

private slots:
    // DB load completion slots
    void onChainsLoaded();
    void onChainsLoadFailed(QString message);
    void onTriggersLoaded();
    void onTriggersLoadFailed(QString message);
    void onConditionsLoaded();
    void onConditionsLoadFailed(QString message);
    void onActionsLoaded();
    void onActionsLoadFailed(QString message);

    // Tree selection
    void onChainSelected();

    // New-chain button
    void onNewChain();

    // Trigger table buttons
    void onAddTrigger();
    void onEditTrigger();
    void onDeleteTrigger();

    // Condition table buttons
    void onAddCondition();
    void onEditCondition();
    void onDeleteCondition();

    // Action table buttons
    void onAddAction();
    void onEditAction();
    void onDeleteAction();

    // Bottom buttons
    void onSaveToDB();
    void onReloadOnServer();

    // Save completion slots
    void onSaveChainCompleted();
    void onSaveChainFailed(QString message);
    void onInsertSubRowsCompleted();
    void onInsertSubRowsFailed(QString message);

private:
    // UI construction helpers
    void buildUi();
    QWidget * buildDetailPanel();
    QGroupBox * buildTriggerGroup();
    QGroupBox * buildConditionGroup();
    QGroupBox * buildActionGroup();

    // Data management
    void fireLoadRequests();
    void checkLoadDone();
    void populateTree();
    QTreeWidgetItem * groupItemForScopeType(QString const & scopeType);

    // Chain detail panel population
    void showChain(ContentChain const & chain);
    void clearDetail();

    // Table population helpers
    void populateTriggerTable(ContentChain const & chain);
    void populateConditionTable(ContentChain const & chain);
    void populateActionTable(ContentChain const & chain);

    // Collect current detail-panel values back into a ContentChain
    ContentChain collectCurrentChain() const;

    // SQL helpers
    QString escapeSql(QString const & s) const;
    QString buildSaveBlock(ContentChain const & chain) const;
    QString buildInsertSubRowsSql(int chainId, ContentChain const & chain) const;

    // Returns index into chains_ for the currently selected chain, or -1
    int selectedChainIndex() const;

    // --- UI widgets ---
    QTreeWidget  * chainTree_;

    // Metadata fields (right panel top)
    QLineEdit    * descEdit_;
    QComboBox    * scopeTypeCombo_;
    QSpinBox     * scopeIdSpin_;
    QCheckBox    * enabledCheck_;
    QSpinBox     * prioritySpin_;

    // Sub-row tables
    QTableWidget * triggerTable_;
    QTableWidget * conditionTable_;
    QTableWidget * actionTable_;

    // Bottom buttons
    QPushButton  * saveButton_;
    QPushButton  * reloadButton_;

    // --- Data ---
    QVector<ContentChain> chains_;
    int pendingLoads_;

    // Pending new-chain insert: chainId returned by INSERT...RETURNING
    ContentChain pendingNewChain_;
    bool         savingNewChain_;

    // Back-pointer to the worker for re-submitting save requests
    DatabaseWorker      * dbWorker_;
    EditorConfiguration * config_;
};

#endif // CHAINEDITORWIDGET_H
