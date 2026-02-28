#include "mainwindow.h"
#include <QApplication>
#include <QMessageBox>
#include <QStringList>

QStringList gLoadWarnings;

void msgHandler(QtMsgType type, const QMessageLogContext &, const QString & msg)
{
    if (type == QtWarningMsg)
    {
        gLoadWarnings.append(msg);
        // QMessageBox::warning(nullptr, "Warning", msg);
    }
    else if (type == QtFatalMsg || type == QtCriticalMsg)
    {
        QMessageBox::critical(nullptr, "Error", msg);
        abort();
    }
}

int main(int argc, char *argv[])
{
    QApplication a(argc, argv);
#ifndef QT_DEBUG
    qInstallMessageHandler(msgHandler);
#endif
    MainWindow w;
    w.show();
    
    return a.exec();
}
