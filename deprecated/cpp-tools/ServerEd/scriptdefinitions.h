#ifndef SCRIPTDEFINITIONS_H
#define SCRIPTDEFINITIONS_H

#include <QString>
#include <QVariant>
#include <QVector>
#include <QMap>

class ScriptDefinitions
{
public:
    struct Enumeration;

    struct DatabaseRef
    {
        QString ref;
        QString nameLookupQuery;
        QString idLookupQuery;
    };

    struct PortTemplate
    {
        QString name;
        QString type;
        QString triggerScript;
        QString description;
        bool defaultHide;
        bool required;
        bool output;
        bool trigger;
    };

    struct PropertyTemplate
    {
        enum Type
        {
            PlainProperty,
            EnumProperty,
            FlagsProperty
        };

        PropertyTemplate();
        PropertyTemplate(QString const & name, QString const & type, QString const & value, ScriptDefinitions * defs = 0);
        static QVariant::Type toVariantType(QString name, ScriptDefinitions * defs = 0);
        QVariant toVariant(QString value) const;
        QString toString(QVariant value) const;
        QString toScriptString(QVariant value) const;

        QString name;
        QString type;
        Type editorType;
        QVariant::Type variantType;
        QString defaultValue;
        QVariant variantDefaultValue;
        QString databaseName;
        DatabaseRef * databaseRef;
        struct Enumeration * enumeration;
    };

    struct MethodTemplate
    {
        QString name, body;
    };

    struct NodeTemplate
    {
        QString ref, name, formatName, description;
        QString type;
        QString category;
        QString staticInitScript, staticTeardownScript;
        QString preInitScript, initScript, teardownScript, persistScript, restoreScript;
        QVector<PortTemplate> ports;
        QVector<PropertyTemplate> properties;
        QVector<MethodTemplate> methods;
        QVector<QString> imports;
        QVector<QString> scriptTypes;

        PortTemplate const * findPort(QString port) const;
        PropertyTemplate const * findProperty(QString property) const;
    };

    struct Enumeration
    {
        QString name;
        bool flags;
        QMap<QString, quint64> values;
    };

    ScriptDefinitions(QString nodesPath, QString enumsPath);

    QString datasetVersion() const;

    QMap<QString, NodeTemplate> const & nodes() const;
    NodeTemplate * findNode(QString node);
    NodeTemplate * findNodeByRef(QString ref);
    DatabaseRef * findDatabaseRef(QString ref);

    QMap<QString, Enumeration> const & enums() const;
    Enumeration * findEnumeration(QString name);

    QString trimScript(QString script);

private:
    QMap<QString, NodeTemplate> nodes_;
    QMap<QString, NodeTemplate *> nodesByRef_;
    QMap<QString, Enumeration> enumerations_;
    QMap<QString, DatabaseRef> databaseRefs_;
    QString datasetVersion_;

    void loadNodes(QString path);
    void loadEnumerations(QString path);
};

#endif // SCRIPTDEFINITIONS_H
