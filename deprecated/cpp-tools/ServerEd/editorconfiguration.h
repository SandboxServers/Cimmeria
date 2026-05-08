#ifndef EDITORCONFIGURATION_H
#define EDITORCONFIGURATION_H

#include <QString>
#include <QVariant>

class EditorConfiguration
{
public:
    struct PropertyDefinition
    {
        QString name;
        QVariant::Type type;
        QVariant defaultValue;
    };

    EditorConfiguration();

    void load(QString const & path);
    void save(QString const & path);

    QVariant get(QString const & name);
    void set(QString const & name, QVariant value);

private:
    QMap<QString, QVariant> properties_;
};

#endif // EDITORCONFIGURATION_H
