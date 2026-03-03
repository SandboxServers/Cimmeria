#include "ChainItemDialogs.h"

#include <QComboBox>
#include <QLineEdit>
#include <QSpinBox>
#include <QCheckBox>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QVBoxLayout>
#include <QString>

// ---------------------------------------------------------------------------
// TriggerDialog
// ---------------------------------------------------------------------------

TriggerDialog::TriggerDialog(QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle("Trigger");

    eventTypeCombo_ = new QComboBox(this);
    eventTypeCombo_->setEditable(true);
    eventTypeCombo_->addItem("on_enter");
    eventTypeCombo_->addItem("on_exit");
    eventTypeCombo_->addItem("on_interact");
    eventTypeCombo_->addItem("on_death");
    eventTypeCombo_->addItem("on_mission_complete");
    eventTypeCombo_->addItem("on_mission_step");
    eventTypeCombo_->addItem("on_timer");
    eventTypeCombo_->addItem("on_ability_used");
    eventTypeCombo_->addItem("on_effect_applied");
    eventTypeCombo_->addItem("on_spawn");
    eventTypeCombo_->addItem("on_despawn");
    eventTypeCombo_->addItem("on_chat");
    eventTypeCombo_->addItem("on_loot");
    eventTypeCombo_->addItem("on_item_acquired");

    eventKeyEdit_ = new QLineEdit(this);
    eventKeyEdit_->setPlaceholderText("(optional)");

    scopeCombo_ = new QComboBox(this);
    scopeCombo_->addItem("player");
    scopeCombo_->addItem("space");
    scopeCombo_->addItem("global");

    onceCheck_ = new QCheckBox(this);

    sortOrderSpin_ = new QSpinBox(this);
    sortOrderSpin_->setRange(0, 9999);

    buttons_ = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, Qt::Horizontal, this);
    QObject::connect(buttons_, SIGNAL(accepted()), this, SLOT(accept()));
    QObject::connect(buttons_, SIGNAL(rejected()), this, SLOT(reject()));

    QFormLayout * form = new QFormLayout();
    form->addRow("Event Type", eventTypeCombo_);
    form->addRow("Event Key", eventKeyEdit_);
    form->addRow("Scope", scopeCombo_);
    form->addRow("Once", onceCheck_);
    form->addRow("Sort Order", sortOrderSpin_);

    QVBoxLayout * layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(buttons_);
    setLayout(layout);
}

void TriggerDialog::populate(ChainTrigger const & t)
{
    int idx = eventTypeCombo_->findText(t.eventType);
    if (idx >= 0)
        eventTypeCombo_->setCurrentIndex(idx);
    else
        eventTypeCombo_->setEditText(t.eventType);

    eventKeyEdit_->setText(t.eventKey);

    idx = scopeCombo_->findText(t.scope);
    if (idx >= 0)
        scopeCombo_->setCurrentIndex(idx);

    onceCheck_->setChecked(t.once);
    sortOrderSpin_->setValue(t.sortOrder);
}

ChainTrigger TriggerDialog::value() const
{
    ChainTrigger t;
    t.eventType  = eventTypeCombo_->currentText().trimmed();
    t.eventKey   = eventKeyEdit_->text().trimmed();
    t.scope      = scopeCombo_->currentText();
    t.once       = onceCheck_->isChecked();
    t.sortOrder  = sortOrderSpin_->value();
    return t;
}

// ---------------------------------------------------------------------------
// ConditionDialog
// ---------------------------------------------------------------------------

ConditionDialog::ConditionDialog(QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle("Condition");

    conditionTypeCombo_ = new QComboBox(this);
    conditionTypeCombo_->setEditable(true);
    conditionTypeCombo_->addItem("has_item");
    conditionTypeCombo_->addItem("has_flag");
    conditionTypeCombo_->addItem("mission_state");
    conditionTypeCombo_->addItem("effect_active");
    conditionTypeCombo_->addItem("stat_value");
    conditionTypeCombo_->addItem("level");
    conditionTypeCombo_->addItem("archetype");
    conditionTypeCombo_->addItem("faction");
    conditionTypeCombo_->addItem("in_region");
    conditionTypeCombo_->addItem("npc_alive");

    targetIdSpin_ = new QSpinBox(this);
    targetIdSpin_->setRange(0, 2147483647);
    targetIdSpin_->setSpecialValueText("(null)");

    targetKeyEdit_ = new QLineEdit(this);
    targetKeyEdit_->setPlaceholderText("(optional)");

    operatorCombo_ = new QComboBox(this);
    operatorCombo_->addItem("eq");
    operatorCombo_->addItem("neq");
    operatorCombo_->addItem("gt");
    operatorCombo_->addItem("lt");
    operatorCombo_->addItem("gte");
    operatorCombo_->addItem("lte");

    valueEdit_ = new QLineEdit(this);
    valueEdit_->setPlaceholderText("(optional)");

    sortOrderSpin_ = new QSpinBox(this);
    sortOrderSpin_->setRange(0, 9999);

    buttons_ = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, Qt::Horizontal, this);
    QObject::connect(buttons_, SIGNAL(accepted()), this, SLOT(accept()));
    QObject::connect(buttons_, SIGNAL(rejected()), this, SLOT(reject()));

    QFormLayout * form = new QFormLayout();
    form->addRow("Condition Type", conditionTypeCombo_);
    form->addRow("Target Id (0=null)", targetIdSpin_);
    form->addRow("Target Key", targetKeyEdit_);
    form->addRow("Operator", operatorCombo_);
    form->addRow("Value", valueEdit_);
    form->addRow("Sort Order", sortOrderSpin_);

    QVBoxLayout * layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(buttons_);
    setLayout(layout);
}

void ConditionDialog::populate(ChainCondition const & c)
{
    int idx = conditionTypeCombo_->findText(c.conditionType);
    if (idx >= 0)
        conditionTypeCombo_->setCurrentIndex(idx);
    else
        conditionTypeCombo_->setEditText(c.conditionType);

    // targetId: -1 means NULL; spin special value 0 is used for null display
    targetIdSpin_->setValue(c.targetId < 0 ? 0 : c.targetId);

    targetKeyEdit_->setText(c.targetKey);

    idx = operatorCombo_->findText(c.op);
    if (idx >= 0)
        operatorCombo_->setCurrentIndex(idx);

    valueEdit_->setText(c.value);
    sortOrderSpin_->setValue(c.sortOrder);
}

ChainCondition ConditionDialog::value() const
{
    ChainCondition c;
    c.conditionType = conditionTypeCombo_->currentText().trimmed();
    // 0 in the spin maps back to -1 (NULL sentinel)
    int rawId = targetIdSpin_->value();
    c.targetId  = (rawId == 0) ? -1 : rawId;
    c.targetKey = targetKeyEdit_->text().trimmed();
    c.op        = operatorCombo_->currentText();
    c.value     = valueEdit_->text().trimmed();
    c.sortOrder = sortOrderSpin_->value();
    return c;
}

// ---------------------------------------------------------------------------
// ActionDialog
// ---------------------------------------------------------------------------

ActionDialog::ActionDialog(QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle("Action");

    actionTypeCombo_ = new QComboBox(this);
    actionTypeCombo_->setEditable(true);
    actionTypeCombo_->addItem("give_item");
    actionTypeCombo_->addItem("take_item");
    actionTypeCombo_->addItem("give_xp");
    actionTypeCombo_->addItem("set_flag");
    actionTypeCombo_->addItem("clear_flag");
    actionTypeCombo_->addItem("advance_mission");
    actionTypeCombo_->addItem("complete_mission");
    actionTypeCombo_->addItem("fail_mission");
    actionTypeCombo_->addItem("play_dialog");
    actionTypeCombo_->addItem("apply_effect");
    actionTypeCombo_->addItem("remove_effect");
    actionTypeCombo_->addItem("spawn_entity");
    actionTypeCombo_->addItem("despawn_entity");
    actionTypeCombo_->addItem("teleport");
    actionTypeCombo_->addItem("send_chat");
    actionTypeCombo_->addItem("run_script");
    actionTypeCombo_->addItem("enable_chain");
    actionTypeCombo_->addItem("disable_chain");

    targetIdSpin_ = new QSpinBox(this);
    targetIdSpin_->setRange(0, 2147483647);
    targetIdSpin_->setSpecialValueText("(null)");

    targetKeyEdit_ = new QLineEdit(this);
    targetKeyEdit_->setPlaceholderText("(optional)");

    paramsEdit_ = new QLineEdit(this);
    paramsEdit_->setPlaceholderText("{}");

    delayMsSpin_ = new QSpinBox(this);
    delayMsSpin_->setRange(0, 600000);
    delayMsSpin_->setSuffix(" ms");

    sortOrderSpin_ = new QSpinBox(this);
    sortOrderSpin_->setRange(0, 9999);

    buttons_ = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, Qt::Horizontal, this);
    QObject::connect(buttons_, SIGNAL(accepted()), this, SLOT(accept()));
    QObject::connect(buttons_, SIGNAL(rejected()), this, SLOT(reject()));

    QFormLayout * form = new QFormLayout();
    form->addRow("Action Type", actionTypeCombo_);
    form->addRow("Target Id (0=null)", targetIdSpin_);
    form->addRow("Target Key", targetKeyEdit_);
    form->addRow("Params (JSON)", paramsEdit_);
    form->addRow("Delay", delayMsSpin_);
    form->addRow("Sort Order", sortOrderSpin_);

    QVBoxLayout * layout = new QVBoxLayout(this);
    layout->addLayout(form);
    layout->addWidget(buttons_);
    setLayout(layout);
}

void ActionDialog::populate(ChainAction const & a)
{
    int idx = actionTypeCombo_->findText(a.actionType);
    if (idx >= 0)
        actionTypeCombo_->setCurrentIndex(idx);
    else
        actionTypeCombo_->setEditText(a.actionType);

    int rawId = a.targetId < 0 ? 0 : a.targetId;
    targetIdSpin_->setValue(rawId);

    targetKeyEdit_->setText(a.targetKey);
    paramsEdit_->setText(a.params.isEmpty() ? "{}" : a.params);
    delayMsSpin_->setValue(a.delayMs);
    sortOrderSpin_->setValue(a.sortOrder);
}

ChainAction ActionDialog::value() const
{
    ChainAction a;
    a.actionType = actionTypeCombo_->currentText().trimmed();
    int rawId    = targetIdSpin_->value();
    a.targetId   = (rawId == 0) ? -1 : rawId;
    a.targetKey  = targetKeyEdit_->text().trimmed();
    a.params     = paramsEdit_->text().trimmed();
    if (a.params.isEmpty())
        a.params = "{}";
    a.delayMs   = delayMsSpin_->value();
    a.sortOrder = sortOrderSpin_->value();
    return a;
}
