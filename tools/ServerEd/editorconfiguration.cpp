#include "editorconfiguration.h"
#include <QFile>
#include <QDomDocument>
#include <QDomElement>
#include <QMessageBox>
#include <QXmlStreamWriter>

const unsigned int PropertyNum = 12;
const EditorConfiguration::PropertyDefinition PropertyDefinitions[PropertyNum] = {
    {"ServerPath", QVariant::String, ""},
    {"ServerHost", QVariant::String, ""},
    {"ServerPassword", QVariant::String, ""},
    {"ServerPort", QVariant::Int, 8990},
    {"DebugScripts", QVariant::Bool, false},
    {"InlineScripts", QVariant::Bool, true},
    {"PlayerName", QVariant::String, ""},
    {"DbHost", QVariant::String, ""},
    {"DbPort", QVariant::Int, 5432},
    {"DbName", QVariant::String, ""},
    {"DbUser", QVariant::String, ""},
    {"DbPassword", QVariant::String, ""},
};

EditorConfiguration::EditorConfiguration()
{
}

void EditorConfiguration::load(QString const & path)
{
    QFile file(path);
    QDomDocument doc;

    // Failed to load configuration file
    // This is not an error condition as the config file won't exist on first launch
    if (file.open(QFile::ReadOnly))
    {
        if (!doc.setContent(file.readAll()))
        {
            QMessageBox::warning(nullptr, "Configuration Load Failed", "Failed to parse configuration file '" + path + "'");
            return;
        }
    }

    QDomElement root = doc.firstChildElement("Configuration");
    for (unsigned int i = 0; i < PropertyNum; i++)
    {
        PropertyDefinition const & prop = PropertyDefinitions[i];
        QVariant value = root.attribute(prop.name, prop.defaultValue.toString());
        value.convert(prop.type);
        properties_.insert(prop.name, value);
    }
}

void EditorConfiguration::save(QString const & path)
{
    QFile file(path);
    if (!file.open(QFile::WriteOnly))
    {
        QMessageBox::warning(nullptr, "Configuration Save Failed", "Failed to open configuration file '" + path + "' for writing");
        return;
    }

    QXmlStreamWriter writer(&file);
    writer.writeStartDocument();
    writer.writeStartElement("Configuration");
    for (unsigned int i = 0; i < PropertyNum; i++)
    {
        PropertyDefinition const & prop = PropertyDefinitions[i];
        writer.writeAttribute(prop.name, properties_[prop.name].toString());
    }
    writer.writeEndElement();
    writer.writeEndDocument();
}

QVariant EditorConfiguration::get(QString const & name)
{
    Q_ASSERT(properties_.contains(name));
    return properties_[name];
}

void EditorConfiguration::set(QString const & name, QVariant value)
{
    Q_ASSERT(properties_.contains(name));
    Q_ASSERT(properties_[name].type() == value.type());
    properties_[name] = value;
}

