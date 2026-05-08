#include "configurationdialog.h"
#include "ui_configurationdialog.h"
#include "scriptnodeseditor.h"
#include "editorconfiguration.h"

ConfigurationDialog::ConfigurationDialog(QWidget *parent) :
    QDialog(parent),
    ui(new Ui::ConfigurationDialog)
{
    ui->setupUi(this);
    ui->versionLabel->setText("Built on " __DATE__ " " __TIME__);
}

ConfigurationDialog::~ConfigurationDialog()
{
    delete ui;
}

void ConfigurationDialog::load(ScriptNodesEditor * editor)
{
    ui->scriptName->setText(editor->getScriptModule());
    if (editor->getScriptType() == "Level")
        ui->scriptType->setCurrentIndex(0);
    else if (editor->getScriptType() == "Mission")
        ui->scriptType->setCurrentIndex(1);
    else if (editor->getScriptType() == "Effect")
        ui->scriptType->setCurrentIndex(2);
    else
        Q_ASSERT(false && "Unknown script type!");
}

void ConfigurationDialog::load(EditorConfiguration * config)
{
    ui->serverRoot->setText(config->get("ServerPath").toString());
    ui->serverHost->setText(config->get("ServerHost").toString());
    ui->serverPassword->setText(config->get("ServerPassword").toString());
    ui->serverPort->setValue(config->get("ServerPort").toInt());
    ui->enableDebugMode->setChecked(config->get("DebugScripts").toBool());
    ui->enableInlining->setChecked(config->get("InlineScripts").toBool());
    ui->playerName->setText(config->get("PlayerName").toString());

    ui->dbHost->setText(config->get("DbHost").toString());
    ui->dbPort->setValue(config->get("DbPort").toInt());
    ui->dbName->setText(config->get("DbName").toString());
    ui->dbUser->setText(config->get("DbUser").toString());
    ui->dbPass->setText(config->get("DbPassword").toString());
}

void ConfigurationDialog::save(ScriptNodesEditor * editor)
{
    editor->setScriptModule(ui->scriptName->text());
    switch (ui->scriptType->currentIndex())
    {
    case 0: editor->setScriptType("Level"); break;
    case 1: editor->setScriptType("Mission"); break;
    case 2: editor->setScriptType("Effect"); break;
    default: Q_ASSERT(false && "Bad script selection index!");
    }
}

void ConfigurationDialog::save(EditorConfiguration * config)
{
    config->set("ServerPath", ui->serverRoot->text());
    config->set("ServerHost", ui->serverHost->text());
    config->set("ServerPassword", ui->serverPassword->text());
    config->set("ServerPort", ui->serverPort->value());
    config->set("DebugScripts", ui->enableDebugMode->isChecked());
    config->set("InlineScripts", ui->enableInlining->isChecked());
    config->set("PlayerName", ui->playerName->text());

    config->set("DbHost", ui->dbHost->text());
    config->set("DbPort", ui->dbPort->value());
    config->set("DbName", ui->dbName->text());
    config->set("DbUser", ui->dbUser->text());
    config->set("DbPassword", ui->dbPass->text());
}
