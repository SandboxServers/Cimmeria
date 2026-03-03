#-------------------------------------------------
#
# Project created by QtCreator 2013-02-19T20:42:18
#
#-------------------------------------------------

QT       += core gui xml network sql

greaterThan(QT_MAJOR_VERSION, 4): QT += widgets

TARGET = ServerEd
TEMPLATE = app

win32:RC_FILE = icon.rc

SOURCES += main.cpp\
        mainwindow.cpp \
    qnodeseditor.cpp \
    qneport.cpp \
    qneconnection.cpp \
    qneblock.cpp \
    scriptnodeseditor.cpp \
    propertybrowser/qtvariantproperty.cpp \
    propertybrowser/qttreepropertybrowser.cpp \
    propertybrowser/qtpropertymanager.cpp \
    propertybrowser/qtpropertybrowserutils.cpp \
    propertybrowser/qtpropertybrowser.cpp \
    propertybrowser/qtgroupboxpropertybrowser.cpp \
    propertybrowser/qteditorfactory.cpp \
    propertybrowser/qtbuttonpropertybrowser.cpp \
    qnecomment.cpp \
    qneitem.cpp \
    scriptgraphicsview.cpp \
    scriptdefinitions.cpp \
    scriptcompiler.cpp \
    serverconnector.cpp \
    configurationdialog.cpp \
    editorconfiguration.cpp \
    scripteditorwidget.cpp \
    filesystembrowserwidget.cpp \
    databaseworker.cpp \
    objectdatabase.cpp \
    scriptdatabaselookupwidget.cpp \
    ChainItemDialogs.cpp \
    ChainEditorWidget.cpp

HEADERS  += mainwindow.h \
    qnodeseditor.h \
    qneport.h \
    qneconnection.h \
    qneblock.h \
    scriptnodeseditor.h \
    propertybrowser/qtvariantproperty.h \
    propertybrowser/qttreepropertybrowser.h \
    propertybrowser/qtpropertymanager.h \
    propertybrowser/qtpropertybrowserutils_p.h \
    propertybrowser/qtpropertybrowser.h \
    propertybrowser/qtgroupboxpropertybrowser.h \
    propertybrowser/qteditorfactory.h \
    propertybrowser/qtbuttonpropertybrowser.h \
    qnecomment.h \
    qneitem.h \
    scriptgraphicsview.h \
    scriptdefinitions.h \
    scriptcompiler.h \
    serverconnector.h \
    configurationdialog.h \
    editorconfiguration.h \
    scripteditorwidget.h \
    filesystembrowserwidget.h \
    databaseworker.h \
    objectdatabase.h \
    scriptdatabaselookupwidget.h \
    ChainModel.h \
    ChainItemDialogs.h \
    ChainEditorWidget.h

FORMS    += mainwindow.ui \
    configurationdialog.ui \
    scripteditorwidget.ui \
    filesystembrowserwidget.ui \
    scriptdatabaselookupwidget.ui

RESOURCES += \
    images.qrc

OTHER_FILES += \
    icon.rc
