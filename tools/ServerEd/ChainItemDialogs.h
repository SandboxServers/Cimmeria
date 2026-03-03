#ifndef CHAINITEMDIALOGS_H
#define CHAINITEMDIALOGS_H

#include <QDialog>
#include "ChainModel.h"

class QComboBox;
class QLineEdit;
class QSpinBox;
class QCheckBox;
class QDialogButtonBox;

// ---------------------------------------------------------------------------
// TriggerDialog
// Add/edit a single ChainTrigger row.
// ---------------------------------------------------------------------------
class TriggerDialog : public QDialog
{
    Q_OBJECT
public:
    explicit TriggerDialog(QWidget *parent = 0);

    void   populate(ChainTrigger const & t);
    ChainTrigger value() const;

private:
    QComboBox * eventTypeCombo_;
    QLineEdit * eventKeyEdit_;
    QComboBox * scopeCombo_;
    QCheckBox * onceCheck_;
    QSpinBox  * sortOrderSpin_;
    QDialogButtonBox * buttons_;
};

// ---------------------------------------------------------------------------
// ConditionDialog
// Add/edit a single ChainCondition row.
// ---------------------------------------------------------------------------
class ConditionDialog : public QDialog
{
    Q_OBJECT
public:
    explicit ConditionDialog(QWidget *parent = 0);

    void   populate(ChainCondition const & c);
    ChainCondition value() const;

private:
    QComboBox * conditionTypeCombo_;
    QSpinBox  * targetIdSpin_;
    QLineEdit * targetKeyEdit_;
    QComboBox * operatorCombo_;
    QLineEdit * valueEdit_;
    QSpinBox  * sortOrderSpin_;
    QDialogButtonBox * buttons_;
};

// ---------------------------------------------------------------------------
// ActionDialog
// Add/edit a single ChainAction row.
// ---------------------------------------------------------------------------
class ActionDialog : public QDialog
{
    Q_OBJECT
public:
    explicit ActionDialog(QWidget *parent = 0);

    void   populate(ChainAction const & a);
    ChainAction value() const;

private:
    QComboBox * actionTypeCombo_;
    QSpinBox  * targetIdSpin_;
    QLineEdit * targetKeyEdit_;
    QLineEdit * paramsEdit_;
    QSpinBox  * delayMsSpin_;
    QSpinBox  * sortOrderSpin_;
    QDialogButtonBox * buttons_;
};

#endif // CHAINITEMDIALOGS_H
