#include "ChainEditorWidget.h"
#include "ChainItemDialogs.h"
#include "databaseworker.h"
#include "editorconfiguration.h"

#include <QTreeWidget>
#include <QTableWidget>
#include <QHeaderView>
#include <QLineEdit>
#include <QSpinBox>
#include <QCheckBox>
#include <QComboBox>
#include <QPushButton>
#include <QSplitter>
#include <QGroupBox>
#include <QLabel>
#include <QHBoxLayout>
#include <QVBoxLayout>
#include <QFormLayout>
#include <QMessageBox>
#include <QScrollArea>
#include <QWidget>

// ---------------------------------------------------------------------------
// Construction helpers
// ---------------------------------------------------------------------------

ChainEditorWidget::ChainEditorWidget(DatabaseWorker * dbWorker,
                                     EditorConfiguration * config,
                                     QWidget *parent)
    : QDockWidget("Content Chain Editor", parent)
    , chainTree_(0)
    , descEdit_(0)
    , scopeTypeCombo_(0)
    , scopeIdSpin_(0)
    , enabledCheck_(0)
    , prioritySpin_(0)
    , triggerTable_(0)
    , conditionTable_(0)
    , actionTable_(0)
    , saveButton_(0)
    , reloadButton_(0)
    , pendingLoads_(4)
    , savingNewChain_(false)
    , dbWorker_(dbWorker)
    , config_(config)
{
    setObjectName("ChainEditorWidget");
    buildUi();
    fireLoadRequests();
}

// ---------------------------------------------------------------------------
// UI construction
// ---------------------------------------------------------------------------

void ChainEditorWidget::buildUi()
{
    // Outer container
    QWidget * outer = new QWidget(this);
    QHBoxLayout * outerLayout = new QHBoxLayout(outer);
    outerLayout->setContentsMargins(4, 4, 4, 4);

    // Splitter: tree on left, detail on right
    QSplitter * splitter = new QSplitter(Qt::Horizontal, outer);

    // --- Left: tree ---
    QWidget * leftPanel = new QWidget(splitter);
    QVBoxLayout * leftLayout = new QVBoxLayout(leftPanel);
    leftLayout->setContentsMargins(0, 0, 0, 0);

    QPushButton * newChainBtn = new QPushButton("New Chain", leftPanel);
    QObject::connect(newChainBtn, SIGNAL(clicked()), this, SLOT(onNewChain()));
    leftLayout->addWidget(newChainBtn);

    chainTree_ = new QTreeWidget(leftPanel);
    chainTree_->setHeaderLabel("Chains");
    chainTree_->setRootIsDecorated(true);
    leftLayout->addWidget(chainTree_);
    leftPanel->setLayout(leftLayout);

    QObject::connect(chainTree_, SIGNAL(itemSelectionChanged()),
                     this,       SLOT(onChainSelected()));

    // --- Right: detail panel + bottom buttons ---
    QWidget * rightPanel = new QWidget(splitter);
    QVBoxLayout * rightLayout = new QVBoxLayout(rightPanel);
    rightLayout->setContentsMargins(0, 0, 0, 0);

    QScrollArea * scroll = new QScrollArea(rightPanel);
    scroll->setWidgetResizable(true);
    scroll->setWidget(buildDetailPanel());
    rightLayout->addWidget(scroll, 1);

    // Bottom buttons
    QHBoxLayout * btnRow = new QHBoxLayout();
    saveButton_   = new QPushButton("Save to DB",        rightPanel);
    reloadButton_ = new QPushButton("Reload on Server",  rightPanel);
    btnRow->addWidget(saveButton_);
    btnRow->addWidget(reloadButton_);
    btnRow->addStretch();
    rightLayout->addLayout(btnRow);
    rightPanel->setLayout(rightLayout);

    QObject::connect(saveButton_,   SIGNAL(clicked()), this, SLOT(onSaveToDB()));
    QObject::connect(reloadButton_, SIGNAL(clicked()), this, SLOT(onReloadOnServer()));

    splitter->addWidget(leftPanel);
    splitter->addWidget(rightPanel);
    splitter->setStretchFactor(0, 1);
    splitter->setStretchFactor(1, 3);

    outerLayout->addWidget(splitter);
    outer->setLayout(outerLayout);
    setWidget(outer);
}

QWidget * ChainEditorWidget::buildDetailPanel()
{
    QWidget * panel = new QWidget();
    QVBoxLayout * vl = new QVBoxLayout(panel);
    vl->setContentsMargins(4, 4, 4, 4);

    // Metadata group
    QGroupBox * metaGroup = new QGroupBox("Chain Metadata", panel);
    QFormLayout * form = new QFormLayout(metaGroup);

    descEdit_ = new QLineEdit(metaGroup);
    form->addRow("Description", descEdit_);

    scopeTypeCombo_ = new QComboBox(metaGroup);
    scopeTypeCombo_->addItem("mission");
    scopeTypeCombo_->addItem("space");
    scopeTypeCombo_->addItem("effect");
    scopeTypeCombo_->addItem("global");
    form->addRow("Scope Type", scopeTypeCombo_);

    scopeIdSpin_ = new QSpinBox(metaGroup);
    scopeIdSpin_->setRange(0, 2147483647);
    scopeIdSpin_->setSpecialValueText("(null)");
    form->addRow("Scope Id (0=null)", scopeIdSpin_);

    enabledCheck_ = new QCheckBox(metaGroup);
    enabledCheck_->setChecked(true);
    form->addRow("Enabled", enabledCheck_);

    prioritySpin_ = new QSpinBox(metaGroup);
    prioritySpin_->setRange(0, 9999);
    form->addRow("Priority", prioritySpin_);

    metaGroup->setLayout(form);
    vl->addWidget(metaGroup);

    vl->addWidget(buildTriggerGroup());
    vl->addWidget(buildConditionGroup());
    vl->addWidget(buildActionGroup());

    vl->addStretch();
    panel->setLayout(vl);
    return panel;
}

QGroupBox * ChainEditorWidget::buildTriggerGroup()
{
    QGroupBox * grp = new QGroupBox("Triggers");
    QVBoxLayout * vl = new QVBoxLayout(grp);

    triggerTable_ = new QTableWidget(0, 6, grp);
    QStringList headers;
    headers << "Id" << "Event Type" << "Event Key" << "Scope" << "Once" << "Sort";
    triggerTable_->setHorizontalHeaderLabels(headers);
    triggerTable_->horizontalHeader()->setStretchLastSection(true);
    triggerTable_->setSelectionBehavior(QAbstractItemView::SelectRows);
    triggerTable_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    vl->addWidget(triggerTable_);

    QHBoxLayout * btns = new QHBoxLayout();
    QPushButton * addBtn  = new QPushButton("Add",    grp);
    QPushButton * editBtn = new QPushButton("Edit",   grp);
    QPushButton * delBtn  = new QPushButton("Delete", grp);
    btns->addWidget(addBtn);
    btns->addWidget(editBtn);
    btns->addWidget(delBtn);
    btns->addStretch();
    vl->addLayout(btns);

    QObject::connect(addBtn,  SIGNAL(clicked()), this, SLOT(onAddTrigger()));
    QObject::connect(editBtn, SIGNAL(clicked()), this, SLOT(onEditTrigger()));
    QObject::connect(delBtn,  SIGNAL(clicked()), this, SLOT(onDeleteTrigger()));

    grp->setLayout(vl);
    return grp;
}

QGroupBox * ChainEditorWidget::buildConditionGroup()
{
    QGroupBox * grp = new QGroupBox("Conditions");
    QVBoxLayout * vl = new QVBoxLayout(grp);

    conditionTable_ = new QTableWidget(0, 7, grp);
    QStringList headers;
    headers << "Id" << "Condition Type" << "Target Id" << "Target Key" << "Op" << "Value" << "Sort";
    conditionTable_->setHorizontalHeaderLabels(headers);
    conditionTable_->horizontalHeader()->setStretchLastSection(true);
    conditionTable_->setSelectionBehavior(QAbstractItemView::SelectRows);
    conditionTable_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    vl->addWidget(conditionTable_);

    QHBoxLayout * btns = new QHBoxLayout();
    QPushButton * addBtn  = new QPushButton("Add",    grp);
    QPushButton * editBtn = new QPushButton("Edit",   grp);
    QPushButton * delBtn  = new QPushButton("Delete", grp);
    btns->addWidget(addBtn);
    btns->addWidget(editBtn);
    btns->addWidget(delBtn);
    btns->addStretch();
    vl->addLayout(btns);

    QObject::connect(addBtn,  SIGNAL(clicked()), this, SLOT(onAddCondition()));
    QObject::connect(editBtn, SIGNAL(clicked()), this, SLOT(onEditCondition()));
    QObject::connect(delBtn,  SIGNAL(clicked()), this, SLOT(onDeleteCondition()));

    grp->setLayout(vl);
    return grp;
}

QGroupBox * ChainEditorWidget::buildActionGroup()
{
    QGroupBox * grp = new QGroupBox("Actions");
    QVBoxLayout * vl = new QVBoxLayout(grp);

    actionTable_ = new QTableWidget(0, 7, grp);
    QStringList headers;
    headers << "Id" << "Action Type" << "Target Id" << "Target Key" << "Params" << "Delay ms" << "Sort";
    actionTable_->setHorizontalHeaderLabels(headers);
    actionTable_->horizontalHeader()->setStretchLastSection(true);
    actionTable_->setSelectionBehavior(QAbstractItemView::SelectRows);
    actionTable_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    vl->addWidget(actionTable_);

    QHBoxLayout * btns = new QHBoxLayout();
    QPushButton * addBtn  = new QPushButton("Add",    grp);
    QPushButton * editBtn = new QPushButton("Edit",   grp);
    QPushButton * delBtn  = new QPushButton("Delete", grp);
    btns->addWidget(addBtn);
    btns->addWidget(editBtn);
    btns->addWidget(delBtn);
    btns->addStretch();
    vl->addLayout(btns);

    QObject::connect(addBtn,  SIGNAL(clicked()), this, SLOT(onAddAction()));
    QObject::connect(editBtn, SIGNAL(clicked()), this, SLOT(onEditAction()));
    QObject::connect(delBtn,  SIGNAL(clicked()), this, SLOT(onDeleteAction()));

    grp->setLayout(vl);
    return grp;
}

// ---------------------------------------------------------------------------
// DB load
// ---------------------------------------------------------------------------

void ChainEditorWidget::fireLoadRequests()
{
    // chains
    {
        DatabaseRequest * req = new DatabaseRequest(
            "SELECT chain_id, description, scope_type, scope_id, enabled, priority"
            " FROM resources.content_chains"
            " ORDER BY scope_type, priority, chain_id");
        QObject::connect(req, SIGNAL(completed()), this, SLOT(onChainsLoaded()));
        QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(onChainsLoadFailed(QString)));
        req->moveToThread(dbWorker_->thread());
        emit submitDbRequest(req);
    }
    // triggers
    {
        DatabaseRequest * req = new DatabaseRequest(
            "SELECT trigger_id, chain_id, event_type, event_key, scope, once, sort_order"
            " FROM resources.content_triggers"
            " ORDER BY chain_id, sort_order, trigger_id");
        QObject::connect(req, SIGNAL(completed()), this, SLOT(onTriggersLoaded()));
        QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(onTriggersLoadFailed(QString)));
        req->moveToThread(dbWorker_->thread());
        emit submitDbRequest(req);
    }
    // conditions
    {
        DatabaseRequest * req = new DatabaseRequest(
            "SELECT condition_id, chain_id, condition_type, target_id, target_key,"
            "       operator, value, sort_order"
            " FROM resources.content_conditions"
            " ORDER BY chain_id, sort_order, condition_id");
        QObject::connect(req, SIGNAL(completed()), this, SLOT(onConditionsLoaded()));
        QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(onConditionsLoadFailed(QString)));
        req->moveToThread(dbWorker_->thread());
        emit submitDbRequest(req);
    }
    // actions
    {
        DatabaseRequest * req = new DatabaseRequest(
            "SELECT action_id, chain_id, action_type, target_id, target_key,"
            "       params, delay_ms, sort_order"
            " FROM resources.content_actions"
            " ORDER BY chain_id, sort_order, action_id");
        QObject::connect(req, SIGNAL(completed()), this, SLOT(onActionsLoaded()));
        QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(onActionsLoadFailed(QString)));
        req->moveToThread(dbWorker_->thread());
        emit submitDbRequest(req);
    }
}

void ChainEditorWidget::onChainsLoaded()
{
    DatabaseRequest * req = static_cast<DatabaseRequest *>(sender());
    chains_.clear();
    Q_FOREACH (QStringList const & row, req->rows())
    {
        ContentChain c;
        c.chainId    = row[0].toInt();
        c.description = row[1];
        c.scopeType  = row[2];
        c.scopeId    = row[3].isEmpty() ? -1 : row[3].toInt();
        c.enabled    = (row[4] == "t" || row[4] == "true" || row[4] == "1");
        c.priority   = row[5].toInt();
        chains_.append(c);
    }
    req->deleteLater();
    checkLoadDone();
}

void ChainEditorWidget::onChainsLoadFailed(QString message)
{
    qDebug("ChainEditorWidget: failed to load chains: %s", qPrintable(message));
    sender()->deleteLater();
    pendingLoads_--;
}

void ChainEditorWidget::onTriggersLoaded()
{
    DatabaseRequest * req = static_cast<DatabaseRequest *>(sender());
    Q_FOREACH (QStringList const & row, req->rows())
    {
        int chainId = row[1].toInt();
        for (int i = 0; i < chains_.size(); ++i)
        {
            if (chains_[i].chainId == chainId)
            {
                ChainTrigger t;
                t.triggerId = row[0].toInt();
                t.chainId   = chainId;
                t.eventType = row[2];
                t.eventKey  = row[3];
                t.scope     = row[4];
                t.once      = (row[5] == "t" || row[5] == "true" || row[5] == "1");
                t.sortOrder = row[6].toInt();
                chains_[i].triggers.append(t);
                break;
            }
        }
    }
    req->deleteLater();
    checkLoadDone();
}

void ChainEditorWidget::onTriggersLoadFailed(QString message)
{
    qDebug("ChainEditorWidget: failed to load triggers: %s", qPrintable(message));
    sender()->deleteLater();
    pendingLoads_--;
}

void ChainEditorWidget::onConditionsLoaded()
{
    DatabaseRequest * req = static_cast<DatabaseRequest *>(sender());
    Q_FOREACH (QStringList const & row, req->rows())
    {
        int chainId = row[1].toInt();
        for (int i = 0; i < chains_.size(); ++i)
        {
            if (chains_[i].chainId == chainId)
            {
                ChainCondition c;
                c.conditionId   = row[0].toInt();
                c.chainId       = chainId;
                c.conditionType = row[2];
                c.targetId      = row[3].isEmpty() ? -1 : row[3].toInt();
                c.targetKey     = row[4];
                c.op            = row[5];
                c.value         = row[6];
                c.sortOrder     = row[7].toInt();
                chains_[i].conditions.append(c);
                break;
            }
        }
    }
    req->deleteLater();
    checkLoadDone();
}

void ChainEditorWidget::onConditionsLoadFailed(QString message)
{
    qDebug("ChainEditorWidget: failed to load conditions: %s", qPrintable(message));
    sender()->deleteLater();
    pendingLoads_--;
}

void ChainEditorWidget::onActionsLoaded()
{
    DatabaseRequest * req = static_cast<DatabaseRequest *>(sender());
    Q_FOREACH (QStringList const & row, req->rows())
    {
        int chainId = row[1].toInt();
        for (int i = 0; i < chains_.size(); ++i)
        {
            if (chains_[i].chainId == chainId)
            {
                ChainAction a;
                a.actionId  = row[0].toInt();
                a.chainId   = chainId;
                a.actionType = row[2];
                a.targetId  = row[3].isEmpty() ? -1 : row[3].toInt();
                a.targetKey = row[4];
                a.params    = row[5];
                a.delayMs   = row[6].toInt();
                a.sortOrder = row[7].toInt();
                chains_[i].actions.append(a);
                break;
            }
        }
    }
    req->deleteLater();
    checkLoadDone();
}

void ChainEditorWidget::onActionsLoadFailed(QString message)
{
    qDebug("ChainEditorWidget: failed to load actions: %s", qPrintable(message));
    sender()->deleteLater();
    pendingLoads_--;
}

void ChainEditorWidget::checkLoadDone()
{
    --pendingLoads_;
    if (pendingLoads_ <= 0)
    {
        pendingLoads_ = 0;
        populateTree();
    }
}

// ---------------------------------------------------------------------------
// Tree population
// ---------------------------------------------------------------------------

void ChainEditorWidget::populateTree()
{
    chainTree_->clear();

    Q_FOREACH (ContentChain const & c, chains_)
    {
        QTreeWidgetItem * grp = groupItemForScopeType(c.scopeType);
        QTreeWidgetItem * item = new QTreeWidgetItem(grp);
        item->setText(0, c.description.isEmpty()
                         ? QString("[chain %1]").arg(c.chainId)
                         : c.description);
        item->setData(0, Qt::UserRole, c.chainId);
    }

    chainTree_->expandAll();
}

QTreeWidgetItem * ChainEditorWidget::groupItemForScopeType(QString const & scopeType)
{
    for (int i = 0; i < chainTree_->topLevelItemCount(); ++i)
    {
        if (chainTree_->topLevelItem(i)->text(0) == scopeType)
            return chainTree_->topLevelItem(i);
    }
    QTreeWidgetItem * grp = new QTreeWidgetItem(chainTree_);
    grp->setText(0, scopeType);
    QFont f = grp->font(0);
    f.setBold(true);
    grp->setFont(0, f);
    grp->setData(0, Qt::UserRole, QVariant());  // group items have no chain id
    return grp;
}

// ---------------------------------------------------------------------------
// Tree selection
// ---------------------------------------------------------------------------

void ChainEditorWidget::onChainSelected()
{
    QList<QTreeWidgetItem *> sel = chainTree_->selectedItems();
    if (sel.isEmpty())
    {
        clearDetail();
        return;
    }
    QTreeWidgetItem * item = sel.first();
    QVariant ud = item->data(0, Qt::UserRole);
    if (!ud.isValid() || ud.isNull())
    {
        clearDetail();
        return;
    }
    int chainId = ud.toInt();
    for (int i = 0; i < chains_.size(); ++i)
    {
        if (chains_[i].chainId == chainId)
        {
            showChain(chains_[i]);
            return;
        }
    }
    clearDetail();
}

void ChainEditorWidget::showChain(ContentChain const & chain)
{
    descEdit_->setText(chain.description);

    int idx = scopeTypeCombo_->findText(chain.scopeType);
    if (idx >= 0)
        scopeTypeCombo_->setCurrentIndex(idx);

    scopeIdSpin_->setValue(chain.scopeId < 0 ? 0 : chain.scopeId);
    enabledCheck_->setChecked(chain.enabled);
    prioritySpin_->setValue(chain.priority);

    populateTriggerTable(chain);
    populateConditionTable(chain);
    populateActionTable(chain);
}

void ChainEditorWidget::clearDetail()
{
    descEdit_->clear();
    scopeTypeCombo_->setCurrentIndex(0);
    scopeIdSpin_->setValue(0);
    enabledCheck_->setChecked(true);
    prioritySpin_->setValue(0);
    triggerTable_->setRowCount(0);
    conditionTable_->setRowCount(0);
    actionTable_->setRowCount(0);
}

// ---------------------------------------------------------------------------
// Table population helpers
// ---------------------------------------------------------------------------

void ChainEditorWidget::populateTriggerTable(ContentChain const & chain)
{
    triggerTable_->setRowCount(0);
    int row = 0;
    Q_FOREACH (ChainTrigger const & t, chain.triggers)
    {
        triggerTable_->insertRow(row);
        triggerTable_->setItem(row, 0, new QTableWidgetItem(QString::number(t.triggerId)));
        triggerTable_->setItem(row, 1, new QTableWidgetItem(t.eventType));
        triggerTable_->setItem(row, 2, new QTableWidgetItem(t.eventKey));
        triggerTable_->setItem(row, 3, new QTableWidgetItem(t.scope));
        triggerTable_->setItem(row, 4, new QTableWidgetItem(t.once ? "yes" : "no"));
        triggerTable_->setItem(row, 5, new QTableWidgetItem(QString::number(t.sortOrder)));
        ++row;
    }
}

void ChainEditorWidget::populateConditionTable(ContentChain const & chain)
{
    conditionTable_->setRowCount(0);
    int row = 0;
    Q_FOREACH (ChainCondition const & c, chain.conditions)
    {
        conditionTable_->insertRow(row);
        conditionTable_->setItem(row, 0, new QTableWidgetItem(QString::number(c.conditionId)));
        conditionTable_->setItem(row, 1, new QTableWidgetItem(c.conditionType));
        conditionTable_->setItem(row, 2, new QTableWidgetItem(c.targetId < 0
                                                               ? "(null)"
                                                               : QString::number(c.targetId)));
        conditionTable_->setItem(row, 3, new QTableWidgetItem(c.targetKey));
        conditionTable_->setItem(row, 4, new QTableWidgetItem(c.op));
        conditionTable_->setItem(row, 5, new QTableWidgetItem(c.value));
        conditionTable_->setItem(row, 6, new QTableWidgetItem(QString::number(c.sortOrder)));
        ++row;
    }
}

void ChainEditorWidget::populateActionTable(ContentChain const & chain)
{
    actionTable_->setRowCount(0);
    int row = 0;
    Q_FOREACH (ChainAction const & a, chain.actions)
    {
        actionTable_->insertRow(row);
        actionTable_->setItem(row, 0, new QTableWidgetItem(QString::number(a.actionId)));
        actionTable_->setItem(row, 1, new QTableWidgetItem(a.actionType));
        actionTable_->setItem(row, 2, new QTableWidgetItem(a.targetId < 0
                                                            ? "(null)"
                                                            : QString::number(a.targetId)));
        actionTable_->setItem(row, 3, new QTableWidgetItem(a.targetKey));
        actionTable_->setItem(row, 4, new QTableWidgetItem(a.params));
        actionTable_->setItem(row, 5, new QTableWidgetItem(QString::number(a.delayMs)));
        actionTable_->setItem(row, 6, new QTableWidgetItem(QString::number(a.sortOrder)));
        ++row;
    }
}

// ---------------------------------------------------------------------------
// New chain
// ---------------------------------------------------------------------------

void ChainEditorWidget::onNewChain()
{
    ContentChain c;
    c.chainId    = 0;
    c.description = "New Chain";
    c.scopeType  = "global";
    c.scopeId    = -1;
    c.enabled    = true;
    c.priority   = 0;
    chains_.append(c);
    populateTree();
    showChain(chains_.last());
}

// ---------------------------------------------------------------------------
// Trigger CRUD slots
// ---------------------------------------------------------------------------

int ChainEditorWidget::selectedChainIndex() const
{
    QList<QTreeWidgetItem *> sel = chainTree_->selectedItems();
    if (sel.isEmpty())
        return -1;
    QVariant ud = sel.first()->data(0, Qt::UserRole);
    if (!ud.isValid() || ud.isNull())
        return -1;
    int chainId = ud.toInt();
    for (int i = 0; i < chains_.size(); ++i)
    {
        if (chains_[i].chainId == chainId)
            return i;
    }
    return -1;
}

void ChainEditorWidget::onAddTrigger()
{
    int idx = selectedChainIndex();
    if (idx < 0)
    {
        QMessageBox::warning(this, "No Chain Selected", "Please select a chain first.");
        return;
    }
    TriggerDialog dlg(this);
    if (dlg.exec() == QDialog::Accepted)
    {
        ChainTrigger t = dlg.value();
        t.chainId = chains_[idx].chainId;
        chains_[idx].triggers.append(t);
        populateTriggerTable(chains_[idx]);
    }
}

void ChainEditorWidget::onEditTrigger()
{
    int idx = selectedChainIndex();
    if (idx < 0) return;
    int row = triggerTable_->currentRow();
    if (row < 0 || row >= chains_[idx].triggers.size()) return;

    TriggerDialog dlg(this);
    dlg.populate(chains_[idx].triggers[row]);
    if (dlg.exec() == QDialog::Accepted)
    {
        ChainTrigger t = dlg.value();
        t.triggerId = chains_[idx].triggers[row].triggerId;
        t.chainId   = chains_[idx].triggers[row].chainId;
        chains_[idx].triggers[row] = t;
        populateTriggerTable(chains_[idx]);
    }
}

void ChainEditorWidget::onDeleteTrigger()
{
    int idx = selectedChainIndex();
    if (idx < 0) return;
    int row = triggerTable_->currentRow();
    if (row < 0 || row >= chains_[idx].triggers.size()) return;

    chains_[idx].triggers.remove(row);
    populateTriggerTable(chains_[idx]);
}

// ---------------------------------------------------------------------------
// Condition CRUD slots
// ---------------------------------------------------------------------------

void ChainEditorWidget::onAddCondition()
{
    int idx = selectedChainIndex();
    if (idx < 0)
    {
        QMessageBox::warning(this, "No Chain Selected", "Please select a chain first.");
        return;
    }
    ConditionDialog dlg(this);
    if (dlg.exec() == QDialog::Accepted)
    {
        ChainCondition c = dlg.value();
        c.chainId = chains_[idx].chainId;
        chains_[idx].conditions.append(c);
        populateConditionTable(chains_[idx]);
    }
}

void ChainEditorWidget::onEditCondition()
{
    int idx = selectedChainIndex();
    if (idx < 0) return;
    int row = conditionTable_->currentRow();
    if (row < 0 || row >= chains_[idx].conditions.size()) return;

    ConditionDialog dlg(this);
    dlg.populate(chains_[idx].conditions[row]);
    if (dlg.exec() == QDialog::Accepted)
    {
        ChainCondition c = dlg.value();
        c.conditionId = chains_[idx].conditions[row].conditionId;
        c.chainId     = chains_[idx].conditions[row].chainId;
        chains_[idx].conditions[row] = c;
        populateConditionTable(chains_[idx]);
    }
}

void ChainEditorWidget::onDeleteCondition()
{
    int idx = selectedChainIndex();
    if (idx < 0) return;
    int row = conditionTable_->currentRow();
    if (row < 0 || row >= chains_[idx].conditions.size()) return;

    chains_[idx].conditions.remove(row);
    populateConditionTable(chains_[idx]);
}

// ---------------------------------------------------------------------------
// Action CRUD slots
// ---------------------------------------------------------------------------

void ChainEditorWidget::onAddAction()
{
    int idx = selectedChainIndex();
    if (idx < 0)
    {
        QMessageBox::warning(this, "No Chain Selected", "Please select a chain first.");
        return;
    }
    ActionDialog dlg(this);
    if (dlg.exec() == QDialog::Accepted)
    {
        ChainAction a = dlg.value();
        a.chainId = chains_[idx].chainId;
        chains_[idx].actions.append(a);
        populateActionTable(chains_[idx]);
    }
}

void ChainEditorWidget::onEditAction()
{
    int idx = selectedChainIndex();
    if (idx < 0) return;
    int row = actionTable_->currentRow();
    if (row < 0 || row >= chains_[idx].actions.size()) return;

    ActionDialog dlg(this);
    dlg.populate(chains_[idx].actions[row]);
    if (dlg.exec() == QDialog::Accepted)
    {
        ChainAction a = dlg.value();
        a.actionId = chains_[idx].actions[row].actionId;
        a.chainId  = chains_[idx].actions[row].chainId;
        chains_[idx].actions[row] = a;
        populateActionTable(chains_[idx]);
    }
}

void ChainEditorWidget::onDeleteAction()
{
    int idx = selectedChainIndex();
    if (idx < 0) return;
    int row = actionTable_->currentRow();
    if (row < 0 || row >= chains_[idx].actions.size()) return;

    chains_[idx].actions.remove(row);
    populateActionTable(chains_[idx]);
}

// ---------------------------------------------------------------------------
// Collect current detail panel values into the in-memory chain
// ---------------------------------------------------------------------------

ContentChain ChainEditorWidget::collectCurrentChain() const
{
    ContentChain c;
    // We'll fill chainId from the selected tree item in onSaveToDB
    c.description = descEdit_->text().trimmed();
    c.scopeType   = scopeTypeCombo_->currentText();
    int rawScope  = scopeIdSpin_->value();
    c.scopeId     = (rawScope == 0) ? -1 : rawScope;
    c.enabled     = enabledCheck_->isChecked();
    c.priority    = prioritySpin_->value();

    // Sub-rows come from the in-memory chain (they are edited in-place)
    // So we return the struct without triggers/conditions/actions here;
    // onSaveToDB fills those from chains_[idx].
    return c;
}

// ---------------------------------------------------------------------------
// SQL helpers
// ---------------------------------------------------------------------------

QString ChainEditorWidget::escapeSql(QString const & s) const
{
    QString out = s;
    out.replace("'", "''");
    return out;
}

QString ChainEditorWidget::buildSaveBlock(ContentChain const & chain) const
{
    // We use a PL/pgSQL anonymous block so the entire save (update chain +
    // delete + re-insert sub-rows) is atomic.
    QString sql;
    sql  = "DO $$ BEGIN\n";

    // --- Update the chain header ---
    QString scopeIdSql = (chain.scopeId < 0)
                         ? "NULL"
                         : QString::number(chain.scopeId);
    sql += QString(
        "UPDATE resources.content_chains"
        " SET description='%1', scope_type='%2', scope_id=%3,"
        "     enabled=%4, priority=%5"
        " WHERE chain_id=%6;\n")
        .arg(escapeSql(chain.description))
        .arg(escapeSql(chain.scopeType))
        .arg(scopeIdSql)
        .arg(chain.enabled ? "true" : "false")
        .arg(chain.priority)
        .arg(chain.chainId);

    // --- Delete existing sub-rows ---
    sql += QString("DELETE FROM resources.content_triggers   WHERE chain_id=%1;\n").arg(chain.chainId);
    sql += QString("DELETE FROM resources.content_conditions WHERE chain_id=%1;\n").arg(chain.chainId);
    sql += QString("DELETE FROM resources.content_actions    WHERE chain_id=%1;\n").arg(chain.chainId);

    // --- Re-insert triggers ---
    Q_FOREACH (ChainTrigger const & t, chain.triggers)
    {
        QString eventKeySql = t.eventKey.isEmpty()
                              ? "NULL"
                              : QString("'%1'").arg(escapeSql(t.eventKey));
        sql += QString(
            "INSERT INTO resources.content_triggers"
            " (chain_id, event_type, event_key, scope, once, sort_order)"
            " VALUES (%1,'%2',%3,'%4',%5,%6);\n")
            .arg(chain.chainId)
            .arg(escapeSql(t.eventType))
            .arg(eventKeySql)
            .arg(escapeSql(t.scope))
            .arg(t.once ? "true" : "false")
            .arg(t.sortOrder);
    }

    // --- Re-insert conditions ---
    Q_FOREACH (ChainCondition const & c, chain.conditions)
    {
        QString targetIdSql  = (c.targetId < 0)     ? "NULL" : QString::number(c.targetId);
        QString targetKeySql = c.targetKey.isEmpty() ? "NULL" : QString("'%1'").arg(escapeSql(c.targetKey));
        QString valueSql     = c.value.isEmpty()     ? "NULL" : QString("'%1'").arg(escapeSql(c.value));
        sql += QString(
            "INSERT INTO resources.content_conditions"
            " (chain_id, condition_type, target_id, target_key, operator, value, sort_order)"
            " VALUES (%1,'%2',%3,%4,'%5',%6,%7);\n")
            .arg(chain.chainId)
            .arg(escapeSql(c.conditionType))
            .arg(targetIdSql)
            .arg(targetKeySql)
            .arg(escapeSql(c.op))
            .arg(valueSql)
            .arg(c.sortOrder);
    }

    // --- Re-insert actions ---
    Q_FOREACH (ChainAction const & a, chain.actions)
    {
        QString targetIdSql  = (a.targetId < 0)     ? "NULL" : QString::number(a.targetId);
        QString targetKeySql = a.targetKey.isEmpty() ? "NULL" : QString("'%1'").arg(escapeSql(a.targetKey));
        QString paramsSql    = a.params.isEmpty()    ? "'{}'" : QString("'%1'").arg(escapeSql(a.params));
        sql += QString(
            "INSERT INTO resources.content_actions"
            " (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)"
            " VALUES (%1,'%2',%3,%4,%5::jsonb,%6,%7);\n")
            .arg(chain.chainId)
            .arg(escapeSql(a.actionType))
            .arg(targetIdSql)
            .arg(targetKeySql)
            .arg(paramsSql)
            .arg(a.delayMs)
            .arg(a.sortOrder);
    }

    sql += "END; $$;";
    return sql;
}

QString ChainEditorWidget::buildInsertSubRowsSql(int chainId, ContentChain const & chain) const
{
    // Used for new chains after INSERT...RETURNING gives us the real chain_id.
    // Wrapped in a DO block so all inserts execute atomically in one
    // QSqlQuery::exec() call (Qt only executes the first statement otherwise).
    QString body;

    Q_FOREACH (ChainTrigger const & t, chain.triggers)
    {
        QString eventKeySql = t.eventKey.isEmpty()
                              ? "NULL"
                              : QString("'%1'").arg(escapeSql(t.eventKey));
        body += QString(
            "INSERT INTO resources.content_triggers"
            " (chain_id, event_type, event_key, scope, once, sort_order)"
            " VALUES (%1,'%2',%3,'%4',%5,%6);\n")
            .arg(chainId)
            .arg(escapeSql(t.eventType))
            .arg(eventKeySql)
            .arg(escapeSql(t.scope))
            .arg(t.once ? "true" : "false")
            .arg(t.sortOrder);
    }

    Q_FOREACH (ChainCondition const & c, chain.conditions)
    {
        QString targetIdSql  = (c.targetId < 0)     ? "NULL" : QString::number(c.targetId);
        QString targetKeySql = c.targetKey.isEmpty() ? "NULL" : QString("'%1'").arg(escapeSql(c.targetKey));
        QString valueSql     = c.value.isEmpty()     ? "NULL" : QString("'%1'").arg(escapeSql(c.value));
        body += QString(
            "INSERT INTO resources.content_conditions"
            " (chain_id, condition_type, target_id, target_key, operator, value, sort_order)"
            " VALUES (%1,'%2',%3,%4,'%5',%6,%7);\n")
            .arg(chainId)
            .arg(escapeSql(c.conditionType))
            .arg(targetIdSql)
            .arg(targetKeySql)
            .arg(escapeSql(c.op))
            .arg(valueSql)
            .arg(c.sortOrder);
    }

    Q_FOREACH (ChainAction const & a, chain.actions)
    {
        QString targetIdSql  = (a.targetId < 0)     ? "NULL" : QString::number(a.targetId);
        QString targetKeySql = a.targetKey.isEmpty() ? "NULL" : QString("'%1'").arg(escapeSql(a.targetKey));
        QString paramsSql    = a.params.isEmpty()    ? "'{}'" : QString("'%1'").arg(escapeSql(a.params));
        body += QString(
            "INSERT INTO resources.content_actions"
            " (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)"
            " VALUES (%1,'%2',%3,%4,%5::jsonb,%6,%7);\n")
            .arg(chainId)
            .arg(escapeSql(a.actionType))
            .arg(targetIdSql)
            .arg(targetKeySql)
            .arg(paramsSql)
            .arg(a.delayMs)
            .arg(a.sortOrder);
    }

    if (body.isEmpty())
        return QString();
    return QString("DO $$ BEGIN\n") + body + QString("END; $$;");
}

// ---------------------------------------------------------------------------
// Save to DB
// ---------------------------------------------------------------------------

void ChainEditorWidget::onSaveToDB()
{
    int idx = selectedChainIndex();
    if (idx < 0)
    {
        QMessageBox::warning(this, "No Chain Selected", "Please select a chain before saving.");
        return;
    }

    // Merge UI fields back into the in-memory chain
    ContentChain meta = collectCurrentChain();
    chains_[idx].description = meta.description;
    chains_[idx].scopeType   = meta.scopeType;
    chains_[idx].scopeId     = meta.scopeId;
    chains_[idx].enabled     = meta.enabled;
    chains_[idx].priority    = meta.priority;

    ContentChain & chain = chains_[idx];

    if (chain.chainId > 0)
    {
        // Existing chain: atomic DO block update
        QString sql = buildSaveBlock(chain);
        qDebug("ChainEditorWidget: saving existing chain %d", chain.chainId);

        DatabaseRequest * req = new DatabaseRequest(sql);
        QObject::connect(req, SIGNAL(completed()), this, SLOT(onSaveChainCompleted()));
        QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(onSaveChainFailed(QString)));
        req->moveToThread(dbWorker_->thread());
        emit submitDbRequest(req);
    }
    else
    {
        // New chain: INSERT...RETURNING to get the chain_id, then insert sub-rows
        savingNewChain_ = true;
        pendingNewChain_ = chain;

        QString scopeIdSql = (chain.scopeId < 0)
                             ? "NULL"
                             : QString::number(chain.scopeId);
        QString sql = QString(
            "INSERT INTO resources.content_chains"
            " (description, scope_type, scope_id, enabled, priority)"
            " VALUES ('%1','%2',%3,%4,%5)"
            " RETURNING chain_id")
            .arg(escapeSql(chain.description))
            .arg(escapeSql(chain.scopeType))
            .arg(scopeIdSql)
            .arg(chain.enabled ? "true" : "false")
            .arg(chain.priority);

        qDebug("ChainEditorWidget: inserting new chain");
        DatabaseRequest * req = new DatabaseRequest(sql);
        QObject::connect(req, SIGNAL(completed()), this, SLOT(onSaveChainCompleted()));
        QObject::connect(req, SIGNAL(failed(QString)), this, SLOT(onSaveChainFailed(QString)));
        req->moveToThread(dbWorker_->thread());
        emit submitDbRequest(req);
    }
}

void ChainEditorWidget::onSaveChainCompleted()
{
    DatabaseRequest * req = static_cast<DatabaseRequest *>(sender());

    if (savingNewChain_)
    {
        // Phase 1 complete: get the newly assigned chain_id
        int newChainId = 0;
        if (!req->rows().isEmpty() && !req->rows()[0].isEmpty())
            newChainId = req->rows()[0][0].toInt();
        req->deleteLater();

        if (newChainId <= 0)
        {
            QMessageBox::warning(this, "Save Error", "INSERT...RETURNING did not return a chain_id.");
            savingNewChain_ = false;
            return;
        }

        // Update the in-memory chain with the real id
        for (int i = 0; i < chains_.size(); ++i)
        {
            if (chains_[i].chainId == 0)
            {
                chains_[i].chainId = newChainId;
                pendingNewChain_.chainId = newChainId;
                break;
            }
        }

        // Phase 2: insert sub-rows
        QString subSql = buildInsertSubRowsSql(newChainId, pendingNewChain_);
        if (subSql.isEmpty())
        {
            // No sub-rows to insert; done
            savingNewChain_ = false;
            qDebug("ChainEditorWidget: new chain %d saved (no sub-rows)", newChainId);
            QMessageBox::information(this, "Saved", "Chain saved successfully.");
            populateTree();
            return;
        }

        DatabaseRequest * subReq = new DatabaseRequest(subSql);
        QObject::connect(subReq, SIGNAL(completed()), this, SLOT(onInsertSubRowsCompleted()));
        QObject::connect(subReq, SIGNAL(failed(QString)), this, SLOT(onInsertSubRowsFailed(QString)));
        subReq->moveToThread(dbWorker_->thread());
        emit submitDbRequest(subReq);
    }
    else
    {
        req->deleteLater();
        qDebug("ChainEditorWidget: chain saved");
        QMessageBox::information(this, "Saved", "Chain saved successfully.");
        populateTree();
    }
}

void ChainEditorWidget::onSaveChainFailed(QString message)
{
    sender()->deleteLater();
    savingNewChain_ = false;
    qDebug("ChainEditorWidget: save failed: %s", qPrintable(message));
    QMessageBox::warning(this, "Save Failed", "Failed to save chain:\n" + message);
}

void ChainEditorWidget::onInsertSubRowsCompleted()
{
    sender()->deleteLater();
    savingNewChain_ = false;
    qDebug("ChainEditorWidget: new chain sub-rows saved (chain_id=%d)", pendingNewChain_.chainId);
    QMessageBox::information(this, "Saved", "Chain saved successfully.");
    populateTree();
}

void ChainEditorWidget::onInsertSubRowsFailed(QString message)
{
    sender()->deleteLater();
    savingNewChain_ = false;
    qDebug("ChainEditorWidget: sub-row insert failed: %s", qPrintable(message));
    QMessageBox::warning(this, "Save Failed", "Chain header saved but sub-rows failed:\n" + message);
}

// ---------------------------------------------------------------------------
// Reload on server
// ---------------------------------------------------------------------------

void ChainEditorWidget::onReloadOnServer()
{
    emit reloadOnServer();
}
