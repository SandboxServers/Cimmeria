#ifndef CONFIGURATIONDIALOG_H
#define CONFIGURATIONDIALOG_H

#include <QDialog>

namespace Ui {
class ConfigurationDialog;
}

class ConfigurationDialog : public QDialog
{
    Q_OBJECT
    
public:
    explicit ConfigurationDialog(QWidget *parent = 0);
    ~ConfigurationDialog();

    void load(class ScriptNodesEditor * editor);
    void load(class EditorConfiguration * config);
    void save(class ScriptNodesEditor * editor);
    void save(class EditorConfiguration * config);
    
private:
    Ui::ConfigurationDialog *ui;
};

#endif // CONFIGURATIONDIALOG_H
