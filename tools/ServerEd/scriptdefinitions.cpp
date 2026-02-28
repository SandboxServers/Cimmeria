#define QT_FORCE_ASSERTS

#include "scriptdefinitions.h"
#include <QFile>
#include <QRegExp>
#include <QtXml/QDomDocument>
#include <QtXml/QDomElement>

ScriptDefinitions::ScriptDefinitions(QString nodesPath, QString enumsPath)
{
    loadEnumerations(enumsPath);
    loadNodes(nodesPath);
}

QString ScriptDefinitions::datasetVersion() const
{
    return datasetVersion_;
}

QMap<QString, ScriptDefinitions::NodeTemplate> const & ScriptDefinitions::nodes() const
{
    return nodes_;
}

ScriptDefinitions::NodeTemplate * ScriptDefinitions::findNode(QString node)
{
    auto it = nodes_.find(node);
    if (it != nodes_.end())
        return &it.value();
    else
        return nullptr;
}

ScriptDefinitions::NodeTemplate * ScriptDefinitions::findNodeByRef(QString ref)
{
    auto it = nodesByRef_.find(ref);
    if (it != nodesByRef_.end())
        return it.value();
    else
        return nullptr;
}

ScriptDefinitions::DatabaseRef * ScriptDefinitions::findDatabaseRef(QString ref)
{
    auto it = databaseRefs_.find(ref);
    if (it != databaseRefs_.end())
        return &it.value();
    else
        return nullptr;
}

QMap<QString, ScriptDefinitions::Enumeration> const & ScriptDefinitions::enums() const
{
    return enumerations_;
}

ScriptDefinitions::Enumeration * ScriptDefinitions::findEnumeration(QString name)
{
    QMap<QString, Enumeration>::iterator it = enumerations_.find(name);
    if (it != enumerations_.end())
        return &it.value();
    else
        return nullptr;
}

QString ScriptDefinitions::trimScript(QString script)
{
    return script.replace(QRegExp("\\n([ \\t]*[\\r\\n]+)"), "\n").
            replace(QRegExp("([\\r\\n]+[ \\t]*)$"), "").
            replace(QRegExp("^([ \\t]*[\\r\\n]+)"), "").
            replace(QRegExp("^[ \\t]+$"), "");
}

void ScriptDefinitions::loadNodes(QString path)
{
    QFile nodesCfg(path);
    if (!nodesCfg.open(QFile::ReadOnly))
        qFatal("Failed to open node definition file '%s' for reading", qPrintable(path));

    QDomDocument doc;
    QString errMsg;
    int errLine, errCol;
    if (!doc.setContent(nodesCfg.readAll(), false, &errMsg, &errLine, &errCol))
        qFatal("Failed to parse '%s': %s (line %d, column %d)", qPrintable(path), qPrintable(errMsg), errLine, errCol);

    QDomElement root = doc.firstChildElement("Nodes");
    datasetVersion_ = root.attribute("Version");

    for (QDomElement node = root.firstChildElement("DatabaseRef"); !node.isNull(); node = node.nextSiblingElement("DatabaseRef"))
    {
        DatabaseRef ref;
        ref.ref = node.attribute("Ref");
        ref.nameLookupQuery = node.firstChildElement("FindByName").text();
        ref.idLookupQuery = node.firstChildElement("FindById").text();

        if (databaseRefs_.find(ref.ref) != databaseRefs_.end())
            qFatal("Duplicate database ref '%s' in definition file", qPrintable(ref.ref));

        databaseRefs_[ref.ref] = ref;
    }

    for (QDomElement node = root.firstChildElement("Node"); !node.isNull(); node = node.nextSiblingElement("Node"))
    {
        NodeTemplate tpl;
        tpl.ref = node.attribute("Ref");
        tpl.name = node.attribute("Name");
        if (tpl.ref.length() == 0)
            qFatal("Template node '%s' has no Ref attribute!", qPrintable(tpl.name));
        tpl.formatName = node.firstChildElement("Name").text().trimmed();
        tpl.description = node.firstChildElement("Description").text().trimmed();
        tpl.type = node.attribute("Type");
        tpl.category = node.attribute("Category");
        tpl.staticInitScript = trimScript(node.firstChildElement("StaticInitScript").text());
        tpl.staticTeardownScript = trimScript(node.firstChildElement("StaticTeardownScript").text());
        tpl.preInitScript = trimScript(node.firstChildElement("PreInitScript").text());
        tpl.initScript = trimScript(node.firstChildElement("InitScript").text());
        tpl.teardownScript = trimScript(node.firstChildElement("TeardownScript").text());
        tpl.persistScript = trimScript(node.firstChildElement("PersistScript").text());
        tpl.restoreScript = trimScript(node.firstChildElement("RestoreScript").text());
        for (QDomElement port = node.firstChildElement("Port"); !port.isNull(); port = port.nextSiblingElement("Port"))
        {
            PortTemplate p;
            p.name = port.attribute("Name");
            p.type = port.attribute("Type");
            p.description = node.firstChildElement("Description").text().trimmed();
            p.triggerScript = trimScript(port.text());
            p.defaultHide = (port.attribute("DefaultHide", "true") == "true");
            p.required = (port.attribute("Required", "true") == "true");
            p.output = (port.attribute("Direction", "out") == "out");
            p.trigger = (p.type == "Trigger");
            if (!p.trigger)
                PropertyTemplate::toVariantType(p.type, this);
            if (tpl.findPort(p.name) != nullptr)
                qFatal("Duplicate port name '%s' in node '%s'", qPrintable(p.name), qPrintable(tpl.name));
            tpl.ports.append(p);
        }

        for (QDomElement method = node.firstChildElement("Method"); !method.isNull(); method = method.nextSiblingElement("Method"))
        {
            MethodTemplate m;
            m.name = method.attribute("Name");
            m.body = method.text().trimmed();
            tpl.methods.append(m);
        }


        for (QDomElement type = node.firstChildElement("ScriptType"); !type.isNull(); type = type.nextSiblingElement("ScriptType"))
        {
            tpl.scriptTypes.append(type.text().trimmed());
        }

        // Add common properties to each node type
        tpl.properties.append(PropertyTemplate("Enabled", "Boolean", "true"));
        tpl.properties.append(PropertyTemplate("Debug", "Boolean", "false"));
        tpl.properties.append(PropertyTemplate("Comment", "String", ""));

        for (QDomElement prop = node.firstChildElement("Property"); !prop.isNull(); prop = prop.nextSiblingElement("Property"))
        {
            PropertyTemplate p(prop.attribute("Name"), prop.attribute("Type"), prop.attribute("DefaultValue"), this);
            p.databaseName = prop.attribute("DatabaseRef");
            if (p.databaseName.length())
            {
                p.databaseRef = findDatabaseRef(p.databaseName);
                if (!p.databaseRef)
                    qFatal("Node '%s' referenced invalid database ref '%s'", qPrintable(tpl.name), qPrintable(p.databaseName));
            }
            if (tpl.findProperty(p.name) != nullptr)
                qFatal("Duplicate property name '%s' in node '%s'", qPrintable(p.name), qPrintable(tpl.name));
            tpl.properties.append(p);
        }

        for (QDomElement import = node.firstChildElement("Import"); !import.isNull(); import = import.nextSiblingElement("Import"))
        {
            tpl.imports.append(import.text().trimmed());
        }

        if (nodes_.find(tpl.name) != nodes_.end())
            qFatal("Duplicate node type in definition file '%s'", qPrintable(tpl.name));

        nodes_[tpl.name] = tpl;
        nodesByRef_[tpl.ref] = &nodes_[tpl.name];
    }
}


void ScriptDefinitions::loadEnumerations(QString path)
{
    QFile enumsCfg(path);
    if (!enumsCfg.open(QFile::ReadOnly))
        qFatal("Failed to open enumerations file '%s' for reading", qPrintable(path));

    QDomDocument doc;
    QString errMsg;
    int errLine, errCol;
    if (!doc.setContent(enumsCfg.readAll(), false, &errMsg, &errLine, &errCol))
        qFatal("Failed to parse '%s': %s (line %d, column %d)", qPrintable(path), qPrintable(errMsg), errLine, errCol);

    QDomElement root = doc.firstChildElement("root");
    for (QDomElement node = root.firstChildElement(); !node.isNull(); node = node.nextSiblingElement())
    {
        Enumeration e;
        e.name = node.tagName();
        e.flags = (node.attribute("IsBitfield", "false") == "true");
        QDomElement tokens = node.firstChildElement("Tokens");
        for (QDomElement token = tokens.firstChildElement("Token"); !token.isNull(); token = token.nextSiblingElement("Token"))
        {
            QString name = token.firstChildElement("Name").text().trimmed();
            QString value = token.firstChildElement("Value").text().trimmed();
            e.values.insert(name, value.toULongLong());
        }

        if (enumerations_.find(e.name) != enumerations_.end())
            qFatal("Duplicate enumeration '%s' in definition file", qPrintable(e.name));

        enumerations_[e.name] = e;
    }
}


ScriptDefinitions::PropertyTemplate::PropertyTemplate()
    : databaseRef(nullptr)
{
    variantType = QVariant::Invalid;
}

ScriptDefinitions::PropertyTemplate::PropertyTemplate(QString const & name, QString const & type, QString const & value, ScriptDefinitions * defs)
    : databaseRef(nullptr)
{
    this->name = name;
    this->type = type;
    this->defaultValue = value;
    this->variantType = toVariantType(type, defs);
    this->variantDefaultValue = this->toVariant(value);

    if (this->variantType == QVariant::ULongLong && defs)
    {
        enumeration = defs->findEnumeration(type);
        this->editorType = enumeration->flags ? FlagsProperty : EnumProperty;
    }
    else
    {
        this->editorType = PlainProperty;
    }
}

QVariant::Type ScriptDefinitions::PropertyTemplate::toVariantType(QString name, ScriptDefinitions * defs)
{
    QVariant::Type type;
    if (name == "Boolean")
        type = QVariant::Bool;
    else if (name == "Integer")
        type = QVariant::Int;
    else if (name == "Float")
        type = QVariant::Double;
    else if (name == "String")
        type = QVariant::String;
    else if (name == "Vector3")
        type = QVariant::String;
    else if (name == "Entity")
        // TODO: What is the proper type for this?
        type = QVariant::String;
    else
    {
        Enumeration * e = defs ? defs->findEnumeration(name) : 0;
        if (e)
        {
            type = QVariant::ULongLong;
        }
        else
        {
            type = QVariant::Invalid;
            qFatal("Unknown property type '%s'", qPrintable(name));
        }
    }
    return type;
}

QVariant ScriptDefinitions::PropertyTemplate::toVariant(QString value) const
{
    QVariant var(value);
    if (var.canConvert(variantType))
    {
        if (!var.convert(variantType))
            qWarning("Failed to convert value '%s' to variant type %d",
                     qPrintable(value), variantType);
        return var;
    }
    else
    {
        qWarning("Variant type %d is not convertable from string!", variantType);
        return QVariant();
    }
}

QString ScriptDefinitions::PropertyTemplate::toString(QVariant value) const
{
    if (value.canConvert(variantType))
    {
        QVariant var(value);
        if (!var.convert(variantType))
            qWarning("Failed to convert variant type %d to string",
                     value.type());
        return var.value<QString>();
    }
    else
    {
        qWarning("Variant type %d is not convertable to string!", value.type());
        return "";
    }
}

QString ScriptDefinitions::PropertyTemplate::toScriptString(QVariant value) const
{
    if (type == "Entity")
        return "None";

    if (variantType == QVariant::Bool)
        return value.toBool() ? "True" : "False";

    if (variantType == QVariant::String)
        return "'" + value.toString().replace(QRegExp("([\"\\\\])"), "\\\\1") + "'";

    if (value.canConvert(variantType))
    {
        QVariant var(value);
        if (!var.convert(variantType))
            qWarning("Failed to convert variant type %d to string",
                     value.type());
        return var.value<QString>();
    }
    else
    {
        qWarning("Variant type %d is not convertable to string!", value.type());
        return "";
    }
}

ScriptDefinitions::PortTemplate const * ScriptDefinitions::NodeTemplate::findPort(QString port) const
{
    for (int i = 0; i < ports.size(); i++)
    {
        if (ports[i].name == port)
            return &ports[i];
    }

    return nullptr;
}

ScriptDefinitions::PropertyTemplate const * ScriptDefinitions::NodeTemplate::findProperty(QString property) const
{
    for (int i = 0; i < properties.size(); i++)
    {
        if (properties[i].name == property)
            return &properties[i];
    }

    return nullptr;
}
