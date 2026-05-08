#define QT_FORCE_ASSERTS

#include "scriptcompiler.h"
#include "qneitem.h"
#include "qneblock.h"
#include "qneport.h"
#include "qneconnection.h"

#include <QFile>
#include <QDebug>
#include <QMessageBox>

/**
 * Creates a new script compilation unit.
 *
 * @param blockId ID of node editor block
 * @param name    Name of this block node
 * @param type    Type of this block node
 * @param flags   Static compilation flags
 * @param script  Script text to compile
 */
ScriptNode::ScriptNode(int blockId, QString name, Type type, unsigned int flags, QString script)
    : blockId_(blockId), name_(name), type_(type), flags_(flags), script_(script)
{
}


/**
 * Returns the node editor block ID of this node
 */
int ScriptNode::blockId() const
{
    return blockId_;
}


/**
 * Returns the name of this node
 */
QString ScriptNode::name() const
{
    return name_;
}


/**
 * Returns the script type of this node
 */
ScriptNode::Type ScriptNode::type() const
{
    return type_;
}


/**
 * Returns the uncompiled script code for this node
 */
QString ScriptNode::script() const
{
    return script_;
}


/**
 * Returns the list of connections where this node is the source
 */
QVector<class ScriptConnection *> const & ScriptNode::outboundLinks() const
{
    return outLinks_;
}


/**
 * Returns the list of connections where this node is the target
 */
QVector<class ScriptConnection *> const & ScriptNode::inboundLinks() const
{
    return inLinks_;
}


/**
 * Updates the compiled script of this node and sets compiled flag.
 */
void ScriptNode::setCompiledScript(QString script)
{
    compiledScript_ = script;
    flags_ |= Compiled;
}


/**
 * Returns the compiled script of this node
 */
QString ScriptNode::compiledScript() const
{
    return compiledScript_;
}


/**
 * Updates node flags
 *
 * @param flags Node flags
 */
void ScriptNode::setFlags(unsigned int flags)
{
    flags_ = flags;
}


/**
 * Returns node flags
 */
unsigned int ScriptNode::flags() const
{
    return flags_;
}


/**
 * Checks if the node is still referenced
 * (is updated after each optimization step by optimize())
 */
bool ScriptNode::isAlive() const
{
    return !(flags_ & Eliminate);
}


/**
 * Was the script compiled already?
 */
bool ScriptNode::isCompiled() const
{
    return (flags_ & Compiled);
}


/**
 * Should we generate a function or is the script completely inlined?
 */
bool ScriptNode::shouldEmitFunction() const
{
    return (flags_ & (Called | DontInline)) | !(flags_ & Inlined);
}


/**
 * Update optimization flags based on rules
 * Returns true if any optimizations were made
 */
bool ScriptNode::optimize()
{
    bool optimized = false;
    // Do an optimization step on eachoutbound  connection
    foreach (ScriptConnection * out, outLinks_)
    {
        if (!(out->flags() & ScriptConnection::Eliminate))
            optimized |= out->optimize();
    }

    // Try eliminating this node if it isn't referenced
    if (!(flags_ & (Force | Called)))
    {
        int connections = 0;
        foreach (ScriptConnection * out, outLinks_)
        {
            if (!(out->flags() & ScriptConnection::Eliminate))
                connections++;
        }
        foreach (ScriptConnection * out, inLinks_)
        {
            if (!(out->flags() & ScriptConnection::Eliminate))
                connections++;
        }

        if (connections == 0)
        {
            qDebug("Node: No alive links --> Eliminate");
            flags_ |= Eliminate;
            optimized = true;
        }
    }

    return optimized;
}


/**
 * Checks if there are any outbound connections with the specified flags.
 *
 * @param connectionFlags Required ScriptConnection flags (ALL must be set)
 * @param nodeFlags       Required ScriptNode flags (ALL must be set)
 *
 * @return True if a link was found; false otherwise
 */
bool ScriptNode::hasOutboundLinksAll(unsigned int connectionFlags, unsigned int nodeFlags) const
{
    foreach (ScriptConnection * link, outLinks_)
    {
        if (link->isAlive() && (link->flags() & connectionFlags) == connectionFlags &&
            (link->target()->flags() & nodeFlags) == nodeFlags)
            return true;
    }

    return false;
}


/**
 * Checks if there are any inbound connections with the specified flags.
 *
 * @param connectionFlags Required ScriptConnection flags (ALL must be set)
 * @param nodeFlags       Required ScriptNode flags (ALL must be set)
 *
 * @return True if a link was found; false otherwise
 */
bool ScriptNode::hasInboundLinksAll(unsigned int connectionFlags, unsigned int nodeFlags) const
{
    foreach (ScriptConnection * link, inLinks_)
    {
        if (link->isAlive() && (link->flags() & connectionFlags) == connectionFlags &&
            (link->target()->flags() & nodeFlags) == nodeFlags)
            return true;
    }

    return false;
}


/**
 * Checks if there are any outbound connections with the specified flags.
 *
 * @param connectionFlags Required ScriptConnection flags (at least one of them must be set)
 * @param nodeFlags       Required ScriptNode flags (at least one of them must be set)
 *
 * @return True if a link was found; false otherwise
 */
bool ScriptNode::hasOutboundLinksAny(unsigned int connectionFlags, unsigned int nodeFlags) const
{
    foreach (ScriptConnection * link, outLinks_)
    {
        if (link->isAlive() && (link->flags() & connectionFlags || !connectionFlags) &&
            (link->target()->flags() & nodeFlags || !nodeFlags))
            return true;
    }

    return false;
}


/**
 * Checks if there are any inbound connections with the specified flags.
 *
 * @param connectionFlags Required ScriptConnection flags (at least one of them must be set)
 * @param nodeFlags       Required ScriptNode flags (at least one of them must be set)
 *
 * @return True if a link was found; false otherwise
 */
bool ScriptNode::hasInboundLinksAny(unsigned int connectionFlags, unsigned int nodeFlags) const
{
    foreach (ScriptConnection * link, inLinks_)
    {
        if (link->isAlive() && (link->flags() & connectionFlags || !connectionFlags) &&
            (link->target()->flags() & nodeFlags || !nodeFlags))
            return true;
    }

    return false;
}


/**
 * Dump node properties to debug channel (qDebug)
 */
void ScriptNode::debugDump()
{
    QString type;
    if (type_ == InputPort)
        type = "InputPort";
    else if (type_ == OutputPort)
        type = "OutputPort";
    else if (type_ == Script)
        type = "Script";
    else
        type = "UNKNOWN";

    QString flags;
    if (flags_ & Force) flags += "Force ";
    if (flags_ & Eliminate) flags += "Eliminate ";
    if (flags_ & Trigger) flags += "Trigger ";
    if (flags_ & InitScript) flags += "InitScript ";
    if (flags_ & TeardownScript) flags += "TeardownScript ";
    if (flags_ & PersistScript) flags += "PersistScript ";
    if (flags_ & RestoreScript) flags += "RestoreScript ";
    if (flags_ & DontInline) flags += "DontInline ";
    if (flags_ & Called) flags += "Called ";
    if (flags_ & Compiled) flags += "Compiled ";
    if (flags_ & Inlined) flags += "Inlined ";
    if (flags_ & Required) flags += "Required ";
    if (flags_ & NamedMethod) flags += "NamedMethod ";
    if (flags_ & PreInitScript) flags += "PreInitScript ";
    if (flags_ & StaticInitScript) flags += "StaticInitScript ";
    if (flags_ & StaticTeardownScript) flags += "StaticTeardownScript ";

    qDebug("%s: (%d) %s", qPrintable(type), blockId_, qPrintable(name_));
    if (flags.length())
        qDebug("    Flags: %s", qPrintable(flags));
    if (!inLinks_.empty() || !outLinks_.empty())
        qDebug("    Links: %d in / %d out", inLinks_.size(), outLinks_.size());
}


/**
 * Registers an outgoing link from this node to another node.
 * This should only be called by ScriptConnection!
 *
 * @param link Link being registered
 */
void ScriptNode::addOutLink(ScriptConnection * link)
{
    // Debug check to make sure that we aren't linking twice to the same node
    Q_ASSERT(!outLinks_.contains(link));
    foreach (ScriptConnection * out, outLinks_)
    {
        Q_ASSERT(out->target() != link->target());
    }

    // Check for local loops (A -> B -> A and A -> A)
    Q_ASSERT(link->source() != link->target());
    Q_ASSERT(!inLinks_.contains(link));
    foreach (ScriptConnection * in, inLinks_)
    {
        Q_ASSERT(in->source() != link->target());
    }

    outLinks_.append(link);
}


/**
 * Unregisters an outgoing link from this node to another node.
 * This should only be called by ScriptConnection!
 *
 * @param link Link being unregistered
 */
void ScriptNode::removeOutLink(ScriptConnection * link)
{
    int index = outLinks_.indexOf(link);
    Q_ASSERT(index != -1);
    outLinks_.remove(index);
}


/**
 * Registers an incoming link to this node.
 * This should only be called by ScriptConnection!
 *
 * @param link Link being registered
 */
void ScriptNode::addInLink(ScriptConnection * link)
{
    // Debug check to make sure that we aren't linking twice to the same node
    Q_ASSERT(!inLinks_.contains(link));
    foreach (ScriptConnection * in, inLinks_)
    {
        Q_ASSERT(in->source() != link->source());
    }

    // Check for local loops (A -> B -> A and A -> A)
    Q_ASSERT(link->source() != link->target());
    Q_ASSERT(!outLinks_.contains(link));
    foreach (ScriptConnection * out, outLinks_)
    {
        Q_ASSERT(out->target() != link->source());
    }

    inLinks_.append(link);
}


/**
 * Unregisters an incoming link to this node.
 * This should only be called by ScriptConnection!
 *
 * @param link Link being unregistered
 */
void ScriptNode::removeInLink(ScriptConnection * link)
{
    int index = inLinks_.indexOf(link);
    Q_ASSERT(index != -1);
    inLinks_.remove(index);
}


/**
 * Creates a connection between two script nodes.
 * (This will automatically register the connection.)
 *
 * @param source Source node
 * @param target Target node
 */
ScriptConnection::ScriptConnection(ScriptNode * source, ScriptNode * target)
    : source_(source), target_(target)
{
    source_->addOutLink(this);
    target_->addInLink(this);
}


ScriptConnection::~ScriptConnection()
{
    source_->removeOutLink(this);
    target_->removeInLink(this);
}


/**
 * @return Returns the source node of the connection
 */
ScriptNode * ScriptConnection::source() const
{
    return source_;
}


/**
 * @return Returns the target node of the connection
 */
ScriptNode * ScriptConnection::target() const
{
    return target_;
}


/**
 * Update connection flags.
 *
 * @param flags See ScriptConnection::Flags enumeration
 */
void ScriptConnection::setFlags(unsigned int flags)
{
    flags_ = flags;
}


/**
 * Returns connection flags.
 *
 * @return Flags (see ScriptConnection::Flags enumeration)
 */
unsigned int ScriptConnection::flags() const
{
    return flags_;
}


/**
 * Checks if the connection is still referenced
 *(is updated after each optimization step by optimize())
 */
bool ScriptConnection::isAlive() const
{
    return !(flags_ & Eliminate);
}


/**
 * Update optimization flags by performing the following compilation optimization checks:
 *  - A --(T)--> B and B unconnected (= no unoptimized outbound links) and has no script: mark link and B as (O)ptimized out
 *  - A --(P)--> B and (B unconnected or B has no write/access links): mark link and B as (O)ptimized out
 *  - A --(W)--> B and B has no read/access links and B unconnected: mark link and B as (O)ptimized out
 *  - A --(C)--> B and A has no T/P links: mark link as (O)ptimized out
 *  - A --(R)--> B and B has no write/access/inbound connections: mark link as (O)ptimized out
 *
 * @return Returns true if any optimizations were made
 */
bool ScriptConnection::optimize()
{
    if (!isAlive())
        return false;

    // A --(T)--> B and B unconnected (= no unoptimized outbound links) and has no script: mark link and B as (O)ptimized out
    if (flags_ & Trigger && !target_->hasOutboundLinksAll() && target_->isCompiled() && target_->compiledScript().length() == 0)
    {
        target_->setFlags(target_->flags() | ScriptNode::Eliminate);
        flags_ |= Eliminate;
        qDebug("Connection %s --> %s: A --(T)--> B Eliminate", qPrintable(source_->name()), qPrintable(target_->name()));
        return true;
    }

    // - A --(P)--> B and (B unconnected or B has no write/access links): mark link and B as (O)ptimized out
    if (flags_ & Propagate && !target_->hasInboundLinksAny(Read | Access) && !target_->hasOutboundLinksAll())
    {
        target_->setFlags(target_->flags() | ScriptNode::Eliminate);
        flags_ |= Eliminate;
        qDebug("Connection %s --> %s: A --(P)--> B Eliminate", qPrintable(source_->name()), qPrintable(target_->name()));
        return true;
    }

    // - A --(W)--> B and B has no read/access links and B unconnected: mark link and B as (O)ptimized out
    if (flags_ & Write && !target_->hasOutboundLinksAll() && !target_->hasInboundLinksAny(Read | Access))
    {
        target_->setFlags(target_->flags() | ScriptNode::Eliminate);
        flags_ |= Eliminate;
        qDebug("Connection %s --> %s: A --(W)--> B Eliminate", qPrintable(source_->name()), qPrintable(target_->name()));
        return true;
    }

    // - A --(C)--> B and A has no T/P links: mark link as (O)ptimized out
    if (flags_ & Connected && !source_->hasInboundLinksAny(Trigger | Propagate))
    {
        flags_ |= Eliminate;
        qDebug("Connection %s --> %s: A --(C)--> B Eliminate", qPrintable(source_->name()), qPrintable(target_->name()));
        return true;
    }

    // - A --(R)--> B and B has no write/access/inbound connections: mark link and B as (O)ptimized out
    if (flags_ & Read && !target_->hasInboundLinksAny(Write | Access | Propagate))
    {
        // Sure about this?
        target_->setFlags(target_->flags() | ScriptNode::Eliminate);
        flags_ |= Eliminate;
        qDebug("Connection %s --> %s: A --(R)--> B Eliminate", qPrintable(source_->name()), qPrintable(target_->name()));
        return true;
    }

    // Source node if link is eliminated --> mark link as (O)ptimized out
    if (!source_->isAlive())
    {
        flags_ |= Eliminate;
        qDebug("Connection %s --> %s: Not Alive --> Eliminate", qPrintable(source_->name()), qPrintable(target_->name()));
        return true;
    }

    return false;
}


/**
 * Returns the specified port of this block.
 *
 * @param name Port to find
 *
 * @return Port (nullptr if not found)
 */
ScriptNode * ScriptOptimizer::Block::port(QString name) const
{
    auto it = nodes.find(name);
    if (it != nodes.end() && (it.value()->type() == ScriptNode::OutputPort || it.value()->type() == ScriptNode::InputPort))
        return it.value();
    else
        return nullptr;
}


/**
 * Returns the specified property of this block.
 *
 * @param name Property to find
 *
 * @return Property value (an invalid QVariant is returned not found)
 */
QVariant ScriptOptimizer::Block::property(QString name) const
{
    auto it = properties.find(name);
    if (it != properties.end())
        return it.value();
    else
        return QVariant();
}


/**
 * Creates a new script compiler and optimizer
 *
 * @param editor Node set editor
 */
ScriptOptimizer::ScriptOptimizer(ScriptNodesEditor & editor)
    : editor_(editor)
{
    addNodeSet(editor);
}


ScriptOptimizer::~ScriptOptimizer()
{
}


/**
 * Adds all blocks and connections to the optimizer node list from the editor.
 *
 * @param editor Node set editor
 */
void ScriptOptimizer::addNodeSet(ScriptNodesEditor & editor)
{
    qDebug("Adding blocks ...");
    foreach (QNEItem * item, editor.items())
    {
        QNEBlock * block = dynamic_cast<QNEBlock *>(item);
        if (block)
            addBlock(*block);
    }

    qDebug("Adding connections ...");
    foreach (QNEItem * item, editor.items())
    {
        QNEBlock * block = dynamic_cast<QNEBlock *>(item);
        if (block)
            addBlockConnections(*block);
    }
}


/**
 * Adds the specified node to the optimizer node list.
 *
 * @param node Node to add
 */
void ScriptOptimizer::addNode(ScriptNode * node)
{
    Q_ASSERT(blockMap_.contains(node->blockId()));
    Block * block = blockMap_[node->blockId()];

    Q_ASSERT(!block->nodes.contains(node->name()));
    block->nodes.insert(node->name(), node);
    nodes_.push_back(node);

}


/**
 * Adds all block properties, nodes and scripts (init/teardown/...) to the optimizer.
 *
 * @param block Block to add
 */
void ScriptOptimizer::addBlock(QNEBlock & block)
{
    Q_ASSERT(!blockMap_.contains(block.id()));
    Block * mapped = new Block();
    refBlocks_.push_back(&block);
    blockMap_[block.id()] = mapped;
    // TODO: Honor block.isNodeEnabled()
    mapped->id = block.id();
    mapped->name = block.name() + '_' + QString::number(block.id());
    mapped->debug = block.isDebuggingEnabled();
    foreach (ScriptDefinitions::PropertyTemplate const & property, block.getTemplate()->properties)
    {
        mapped->properties.insert(property.name, block.getProperty(property.name));
    }

    QNEBlock::Template const & tpl = *block.getTemplate();
    addBlockScript(block, "StaticInitScript", ScriptNode::StaticInitScript, tpl.staticInitScript);
    addBlockScript(block, "StaticTeardownScript", ScriptNode::StaticTeardownScript, tpl.staticTeardownScript);
    addBlockScript(block, "PreInitScript", ScriptNode::PreInitScript, tpl.preInitScript);
    addBlockScript(block, "InitScript", ScriptNode::InitScript, tpl.initScript);
    addBlockScript(block, "TeardownScript", ScriptNode::TeardownScript, tpl.teardownScript);
    addBlockScript(block, "PersistScript", ScriptNode::PersistScript, tpl.persistScript);
    addBlockScript(block, "RestoreScript", ScriptNode::RestoreScript, tpl.restoreScript);
    foreach (ScriptDefinitions::MethodTemplate const & method, tpl.methods)
    {
        addBlockScript(block, method.name, ScriptNode::NamedMethod | ScriptNode::DontInline, method.body);
    }

    foreach (QNEPort * port, block.ports())
    {
        unsigned int flags = 0;
        if (port->getTemplate()->trigger)
            flags |= ScriptNode::Trigger;
        if (port->getTemplate()->required)
            flags |= ScriptNode::Required;
        ScriptNode * node = new ScriptNode(
            block.id(), port->getTemplate()->name,
            port->isOutput() ? ScriptNode::OutputPort : ScriptNode::InputPort,
            flags,
            port->getTemplate()->triggerScript
        );
        addNode(node);
    }
}


/**
 * Adds a script node to the optimizer list.
 *
 * @param block  Owner of the script instance
 * @param name   Name of script node (for logging/debugging/display purposes)
 * @param flags  Special node flags (see ScriptNode::Flags)
 * @param script Script text
 */
void ScriptOptimizer::addBlockScript(QNEBlock & block, QString name, unsigned int flags, QString script)
{
    if (script.length())
    {
        ScriptNode * node = new ScriptNode(
            block.id(), name,
            ScriptNode::Script,
            ScriptNode::Force | flags,
            script
        );
        addNode(node);
    }
}


/**
 * Adds all block connections to the optimizer
 *
 * @param block Block whose connections should be added
 */
void ScriptOptimizer::addBlockConnections(QNEBlock & block)
{
    foreach (QNEPort * port, block.ports())
    {
        Q_ASSERT(blockMap_.contains(block.id()));
        Block * mapped = blockMap_[block.id()];
        Q_ASSERT(mapped->nodes.contains(port->getTemplate()->name));
        ScriptNode * source = mapped->nodes[port->getTemplate()->name];

        foreach (QNEConnection * conn, port->connections())
        {
            if (conn->outputPort() == port)
            {
                QNEPort * inPort = conn->inputPort();
                Q_ASSERT(port->isOutput() && !inPort->isOutput());
                Q_ASSERT(blockMap_.contains(inPort->block()->id()));
                Block * targetMapped = blockMap_[inPort->block()->id()];
                Q_ASSERT(targetMapped->nodes.contains(inPort->getTemplate()->name));
                ScriptNode * target = targetMapped->nodes[inPort->getTemplate()->name];
                ScriptConnection * connection = new ScriptConnection(source, target);
                connection->setFlags(ScriptConnection::Connected | ScriptConnection::Persistent);
                connections_.push_back(connection);
            }
        }
    }
}


/**
 * Add a new dependency between two script nodes.
 * If the connection already exists, its flags are updated.
 * If the connection doesn't exist, a new one is added.
 *
 * @param node       Dependent node
 * @param dependency Dependency
 * @param flags      Connection flags (see ScriptConnection::Flags)
 */
void ScriptOptimizer::addDependency(ScriptNode * node, ScriptNode * dependency, unsigned int flags)
{
    // Check if the nodes are already connected
    foreach (ScriptConnection * conn, node->outboundLinks())
    {
        if (conn->target() == dependency)
        {
            // Update the existing dependency with new flags
            if ((conn->flags() & flags) != flags)
            {
                qDebug("Setting %d (%d) dependency on link %s -> %s", flags, conn->flags(), qPrintable(node->name()), qPrintable(dependency->name()));
                conn->setFlags(conn->flags() | flags);
                if (conn->flags() & ScriptConnection::Eliminate)
                {
                    qDebug("Unsetting eliminate flag on dependent link!");
                    Q_ASSERT(false && "Unsetting eliminate flag on dependent link!");
                    conn->setFlags(conn->flags() & ~ScriptConnection::Eliminate);
                }
            }
            return;
        }
    }

    // Create a new dependency
    qDebug("Adding new %d dependency on link %s -> %s", flags, qPrintable(node->name()), qPrintable(dependency->name()));
    ScriptConnection * conn = new ScriptConnection(node, dependency);
    conn->setFlags(flags);
    connections_.push_back(conn);
}


/**
 * Optimize and compile the whole script
 *
 * @param path Output path
 * @return True if compilation is successful, false otherwise
 */
bool ScriptOptimizer::compile(QString const & path)
{
    qDebug("Compiling script to %s ...", qPrintable(path));
    QFile file(path);
    if (!file.open(QFile::WriteOnly))
    {
        QMessageBox::warning(nullptr, "Save Failed", "Failed to open file '" + path + "' for writing");
        return false;
    }

    if (!optimize())
        return false;

    QString parentClass;
    if (editor_.getScriptType() == "Effect")
        parentClass = "EffectScript";
    else
        parentClass = "Script";

    QTextStream stream(&file);
    stream << "# Automatically generated by Atrea Script Editor\r\n";
    stream << "# Script Version: " << editor_.scriptVersion() << "\r\n";
    stream << "# Dataset Version: " << editor_.definitions()->datasetVersion() << "\r\n";
    stream << "\r\n";
    stream << "from cell.Script import " + parentClass + "\r\n";

    qDebug("Collecting imports ...");
    QStringList imports;
    foreach (QNEBlock * it, refBlocks_)
    {
        QNEBlock::Template const * tpl = it->getTemplate();
        foreach (QString import, tpl->imports)
        {
            qDebug("CheckImport %s", qPrintable(import));
            if (!imports.contains(import))
            {
                imports.push_back(import);
                stream << import << "\r\n";
            }
        }
    }

    QString mod = editor_.getScriptModule();
    int sep = mod.lastIndexOf('.');
    if (sep != -1)
        mod = mod.mid(sep + 1);
    if (mod == "")
    {
        qWarning("Module name not set; saving script with default class name");
        mod = "UnnamedScriptClass";
    }

    stream << "\r\n";
    stream << "class " << mod << "(" + parentClass + "):\r\n";

    qDebug("Saving nodes ...");
    QString staticInitScript, staticTeardownScript, preInitScript, initScript, teardownScript, persistScript, restoreScript;
    foreach (ScriptNode * node, nodes_)
    {
        if (node->isAlive())
        {
            ScriptNodeCompiler comp(this, blockMap_[node->blockId()], node, ScriptNodeCompiler::DontCompile);
            if (node->flags() & (ScriptNode::InitScript | ScriptNode::PreInitScript))
            {
                if (node->compiledScript().length() > 0)
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                     if (node->flags() & ScriptNode::PreInitScript)
                         preInitScript += script + "\r\n";
                     else
                         initScript += script + "\r\n";
                }
            }
            else if (node->flags() & ScriptNode::StaticInitScript)
            {
                if (node->compiledScript().length() > 0)
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    staticInitScript += script + "\r\n";
                }
            }
            else if (node->flags() & ScriptNode::StaticTeardownScript)
            {
                if (node->compiledScript().length() > 0)
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    staticTeardownScript += script + "\r\n";
                }
            }
            else if (node->flags() & ScriptNode::TeardownScript)
            {
                if (node->compiledScript().length() > 0)
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    teardownScript += script + "\r\n";
                }
            }
            else if (node->flags() & ScriptNode::PersistScript)
            {
                if (node->compiledScript().length() > 0)
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    persistScript += script + "\r\n";
                }
            }
            else if (node->flags() & ScriptNode::RestoreScript)
            {
                if (node->compiledScript().length() > 0)
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    restoreScript += script + "\r\n";
                }
            }
            else if (node->flags() & ScriptNode::NamedMethod)
            {
                Q_ASSERT(node->isAlive());
                if (node->compiledScript().length() > 0)
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    stream << "\tdef " << node->name() << "(self):\r\n";
                    stream << script << "\r\n";
                    stream << "\r\n";
                }
            }
            else if ((node->type() == ScriptNode::InputPort || node->type() == ScriptNode::OutputPort) &&
                !(node->flags() & ScriptNode::Trigger))
            {
                if (node->flags() & ScriptNode::Required && (
                    (node->type() == ScriptNode::InputPort && !node->hasInboundLinksAll(ScriptConnection::Connected)) ||
                    (node->type() == ScriptNode::OutputPort && !node->hasOutboundLinksAll(ScriptConnection::Connected))))
                {
                    qWarning("Required port '%s' of node '%s' is not connected",
                        qPrintable(node->name()), qPrintable(blockMap_[node->blockId()]->name));
                }
                stream << "\t" << comp.compileName(node) << " = None\r\n";
                if (node->type() == ScriptNode::OutputPort && node->shouldEmitFunction())
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    stream << "\tdef " << comp.compileFunctionName(node) << "(self):\r\n";
                    stream << script << "\r\n";
                    stream << "\r\n";
                }
            }
            else if ((node->type() == ScriptNode::InputPort || node->type() == ScriptNode::OutputPort) &&
                    node->flags() & ScriptNode::Trigger)
            {
                if (node->shouldEmitFunction())
                {
                    QString script = comp.reindentScript(node->compiledScript(), 2).join("\r\n");
                    stream << "\tdef " << comp.compileFunctionName(node) << "(self):\r\n";
                    stream << script << "\r\n";
                    stream << "\r\n";
                }
            }
            else
                Q_ASSERT(false && "Unhandled node type!");
        }
        else
            qDebug("Eliminated node '%s'", qPrintable(node->name()));
    }

    if (editor_.getScriptType() == "Effect")
    {
        stream << "\tdef __init__(self, owner, effect, nvps):\r\n";
        stream << "\t\t" + parentClass + ".__init__(self, owner, effect, nvps)\r\n";
    }
    else
    {
        stream << "\tdef __init__(self, owner, storedVars):\r\n";
        stream << "\t\t" + parentClass + ".__init__(self, owner, storedVars)\r\n";
    }
    stream << preInitScript;
    stream << initScript;
    stream << "\r\n";

    if (staticInitScript.length())
    {
        stream << "\t@classmethod\r\n";
        stream << "\tdef staticInit(cls):\r\n";
        stream << staticInitScript;
        stream << "\r\n";
    }

    if (staticTeardownScript.length())
    {
        stream << "\t@classmethod\r\n";
        stream << "\tdef staticTeardown(cls):\r\n";
        stream << staticTeardownScript;
        stream << "\r\n";
    }

    if (teardownScript.length())
    {
        stream << "\tdef teardown(self):\r\n";
        stream << teardownScript;
        stream << "\r\n";
    }

    if (persistScript.length())
    {
        stream << "\tdef persist(self):\r\n";
        stream << persistScript;
        if (persistScript.length() < 4)
            stream << "\t\tpass\r\n";
        stream << "\r\n";
    }

    if (restoreScript.length())
    {
        stream << "\tdef restore(self):\r\n";
        stream << restoreScript;
        if (restoreScript.length() < 4)
            stream << "\t\tpass\r\n";
        stream << "\r\n";
    }

    if (staticInitScript.length())
    {
        stream << "\r\n" << mod << ".staticInit()\r\n";
    }

    qDebug("Save completed.");
    return true;
}


/**
 * Optimizes the script
 *  1) Transform nodes to script objects
 *  2) Do a full compilation step on all objects
 *  3) Collect all dependencies
 *  4) Compile objects that are not eliminated
 *  5) Exec optimization checks on all nodes
 *  6) Do 4..5 until no further optimizations can be made (iteration limit is currently 10)
 *
 * @return  True if optimization is successful, false otherwise
 */
bool ScriptOptimizer::optimize()
{
    qDebug("Optimizing script ...");
    // Remove non-persistent connections that were added during compilation
    for (int i = connections_.size() - 1; i >= 0; i--)
    {
        if (!(connections_[i]->flags() & ScriptConnection::Persistent))
        {
            delete connections_[i];
            connections_.remove(i);
        }
    }

    // Precompile all nodes to make sure that all dependencies are discovered
    qDebug(" --- BEGIN INITIAL COMPILATION --- ");
    foreach (ScriptNode * node, nodes_)
    {
        if (node->isAlive())
        {
            // qDebug("Running precompilation step on '%s' ...", qPrintable(node->name()));
            ScriptNodeCompiler comp(this, blockMap_[node->blockId()], node, ScriptNodeCompiler::AllowInlining);
            comp.compile();
            // qDebug("%s", qPrintable(node->compiledScript()));
        }
    }

    if (!findCompilationOrder())
        return false;


    // Clear all dynamic flags before optimization
    // (they'll be set during the first optimized compilation step)
    foreach (ScriptNode * node, nodes_)
    {
        node->setFlags(node->flags() & ~ScriptNode::DynamicFlags);
    }

    foreach (ScriptConnection * conn, connections_)
    {
        conn->setFlags(conn->flags() & ~ScriptConnection::DynamicFlags);
    }

    QVector<ScriptNode *> dependentOrder;
    dependentOrder.reserve(orderedNodes_.size());
    std::reverse_copy(orderedNodes_.begin(), orderedNodes_.end(), std::back_inserter(dependentOrder));

    bool optimizedOnce = false;
    int iters = 0;
    do
    {
        // Do a compilation step
        qDebug(" --- BEGIN OPTIMIZED COMPILATION --- ");
        foreach (ScriptNode * node, dependentOrder)
        {
            if (node->isAlive())
            {
                node->setFlags(node->flags() & ~(ScriptNode::Compiled | ScriptNode::Inlined));
                qDebug("Running compilation step on '%s' ...", qPrintable(node->name()));
                ScriptNodeCompiler comp(this, blockMap_[node->blockId()], node,
                    ScriptNodeCompiler::AllowInlining | ScriptNodeCompiler::OrderedCompile);
                comp.compile();
                // qDebug("%s", qPrintable(node->compiledScript()));
            }
        }

        // Update elimination flags for each node based on the compilation output
        qDebug(" --- BEGIN CONNECTION ELIMINATION --- ");
        optimizedOnce = false;
        bool optimized = false;
        do
        {
            optimized = false;
            foreach (ScriptNode * node, dependentOrder)
            {
                if (node->isAlive())
                    optimized |= node->optimize();
            }
            optimizedOnce |= optimized;
            iters++;
        }
        while (optimized && iters < MaxOptimizationIterations);
    } while (optimizedOnce && iters < MaxOptimizationIterations);

    if (iters == MaxOptimizationIterations)
    {
        qWarning("Maximal number of optimization iterations reached. Dependency graph too complex or there is a bug in the optimization logic.");
    }
    debugDump();

    return true;
}

/**
 * Dump optimizer settings, all nodes and connections to debug channel
 */
void ScriptOptimizer::debugDump()
{
    qDebug(" --- BEGIN SCRIPT OPTIMIZER DUMP ---");
    foreach (ScriptNode * node, nodes_)
    {
        node->debugDump();
    }
}


/**
 * Determines the order in which nodes must be compiled
 *
 * @return True if nodes could be ordered, False otherwise
 */
bool ScriptOptimizer::findCompilationOrder()
{
    QSet<ScriptNode *> processed;
    unsigned int lastProcesssed = 0;
    do
    {
        qDebug("Processing dependency graph (%d / %d)", orderedNodes_.size(), nodes_.size());
        foreach (ScriptNode * node, nodes_)
        {
            if (processed.contains(node))
                continue;

            bool ok = true;
            foreach (ScriptConnection * conn, node->inboundLinks())
            {
                if (!processed.contains(conn->source()))
                {
                    ok = false;
                    break;
                }
            }

            if (!ok)
                continue;

            processed.insert(node);
            orderedNodes_.push_back(node);
        }

        if (lastProcesssed == orderedNodes_.size() && !nodes_.empty())
        {
            qWarning("Precompilation step could not be executed because a circular dependency was detected between nodes.");
            findCycles();
            return false;
        }

        lastProcesssed = orderedNodes_.size();
    } while (orderedNodes_.size() < nodes_.size());
    findCycles();

    return true;
}


/**
 * Finds all cycles in the dependency graph using Tarjan's
 * strongly connected components (SCC) algorithm
 */
void ScriptOptimizer::findCycles()
{
    GraphData graph;
    // Copy all vertices from the source graph
    foreach (ScriptNode * node, nodes_)
    {
        graph.vertices.insert(node, Vertex(node));
    }

    // Copy all edges from the source graph
    graph.edges.reserve(connections_.size());
    foreach (ScriptConnection * conn, connections_)
    {
        graph.edges.push_back(Edge(
            &graph.vertices[conn->source()],
            &graph.vertices[conn->target()]
        ));
        graph.outEdges[conn->source()].push_back(graph.edges.last());
    }

    // Determine strongly connected components
    graph.index = 0;
    for (auto it = graph.vertices.begin(); it != graph.vertices.end(); ++it)
    {
        if (it.value().index == -1)
            strongConnect(graph, it.value());
    }

    // Dump all SCCs
    for (auto it = graph.components.begin(); it != graph.components.end(); ++it)
    {
        // The algorithm returns unconnected vertices as SCCs, but those do not form cycles
        // so omit unconnected vertices
        if (it->size() > 1) //  || it->at(0)->node->hasOutboundLinksAll()
        {
            QString s;
            foreach (Vertex * v, *it)
            {
                s += "(" + blockMap_[v->node->blockId()]->name + " / " + v->node->name() + ") --> ";
            }

            // Add the first element again
            Vertex * v = it->at(0);
            s += "(" + blockMap_[v->node->blockId()]->name + " / " + v->node->name() + ")";
            qWarning("Cycle: %s", qPrintable(s));
        }
    }
}


/**
 * Try to add the vertex to a strongly connected component.
 *
 * @param graph SCC graph
 * @param v     Vertex to add
 */
void ScriptOptimizer::strongConnect(GraphData & graph, Vertex &v)
{
    // Set the depth index for v to the smallest unused index
    v.index = graph.index;
    v.lowlink = graph.index;
    graph.index++;
    graph.sets.push_back(&v);

    // Consider successors of v
    foreach (Edge const & e, graph.outEdges[v.node])
    {
        if (e.target->index == -1)
        {
            // Successor w has not yet been visited; recurse on it
            strongConnect(graph, *e.target);
            e.source->lowlink = std::min(e.source->lowlink, e.target->lowlink);
        }
        else if (graph.sets.contains(e.target))
        {
            // Successor w is in stack S and hence in the current SCC
            e.source->lowlink = std::min(e.source->lowlink, e.target->index);
        }
    }

    // If v is a root node, pop the stack and generate an SCC
    if (v.lowlink == v.index)
    {
        QVector<Vertex *> component;
        Vertex * last;
        do
        {
            last = graph.sets.last();
            component.push_back(last);
            graph.sets.pop_back();
        }
        while (last->node != v.node);
        graph.components.push_back(component);
    }
}


/**
 * Creates a node script compiler instance.
 *
 * @param optimizer Script optimizer
 * @param context   Block whose node we're optimizing
 * @param node      Node to optimize
 * @param flags     Optimization flags (see ScriptNodeCompiler::Flags)
 */
ScriptNodeCompiler::ScriptNodeCompiler(ScriptOptimizer * optimizer, ScriptOptimizer::Block * context,
    ScriptNode * node, unsigned int flags)
    : optimizer_(optimizer), context_(context), node_(node), flags_(flags)
{
}


/**
 * Compiles the node and updates its compiled script property.
 */
bool ScriptNodeCompiler::compile()
{
    Q_ASSERT(!node_->isCompiled());
    QStringList compiled = compileBody(node_);
    // TODO: Remove empty lines from compiled output
    node_->setCompiledScript(compiled.join("\r\n"));
    if (compiled.size() <= MaxInlineLength && !(node_->flags() & ScriptNode::DontInline))
        node_->setFlags(node_->flags() | ScriptNode::Inlined);
    return true;
}


/**
 * Checks if the script code specified node should inlined.
 *
 * @param port Node to check
 * @return True if script is inlined
 */
bool ScriptNodeCompiler::isInlined(ScriptNode * node)
{
    if (flags_ & OrderedCompile)
        Q_ASSERT(node->flags() & ScriptNode::Compiled);
    return (node->flags() & ScriptNode::Inlined);
}


QStringList ScriptNodeCompiler::compileBody(ScriptNode * port)
{
    QStringList body;
    if (port->type() == ScriptNode::OutputPort)
    {
        if (port->flags() & ScriptNode::Trigger)
        {
            // Caller port
            //qDebug(" --- OUTPORT TRIGGERS --- ");
            foreach (ScriptConnection * conn, port->outboundLinks())
            {
                if (conn->isAlive() && (conn->flags() & ScriptConnection::Connected))
                {
                    depends(node_, conn->target(), ScriptConnection::Trigger);
                    if (!isInlined(conn->target()))
                    {
                        body.push_back("self." + compileFunctionName(conn->target()) + "()");
                    }
                    else
                    {
                        if (flags_ & OrderedCompile)
                            Q_ASSERT(conn->target()->isCompiled());
                        body << reindentScript(conn->target()->compiledScript(), 0);
                    }
                }
            }
        }
        else
        {
            // Propagator port
            //qDebug(" --- OUTPORT PROPAGATOR --- ");
            QString srcprop = "self." + compileName(port);
            foreach (ScriptConnection * conn, port->outboundLinks())
            {
                if (conn->isAlive() && (conn->flags() & ScriptConnection::Connected))
                {
                    depends(node_, conn->target(), ScriptConnection::Propagate);
                    if (flags_ & OrderedCompile)
                        Q_ASSERT(conn->target()->isCompiled());
                    body.push_back("self." + compileName(conn->target()) + " = " + srcprop);
                    body << reindentScript(conn->target()->compiledScript(), 0);
                }
            }
        }
    }
    else
    {
        //qDebug(" --- TEXT SCRIPT --- ");
        // Trigger / Setter / Generic Script port
        body = compileScript(port->script(), 0);
    }

    return body;
}

QStringList ScriptNodeCompiler::compileScript(QString script, int indentation)
{
    QStringList lines = reindentScript(script, indentation);
    QRegExp expr("([a-zA-Z_.]+)\\{([^{}]*)\\}");
    QRegExp precomp("^[\\s]*#([a-zA-Z]+)[ \\t]*$");
    QRegExp precompArgs("^[\\s]*#([a-zA-Z]+)[ \\t]+(.*)$");
    QVector<bool> precompLevels;
    int precompFalseLevels = 0;

    QStringList compiled;
    for (int i = 0; i < lines.length(); i++)
    {
        QString line = lines[i];
        QString insn, args;
        if (precomp.indexIn(line) == 0)
        {
            insn = precomp.cap(1).toLower();
        }
        else if (precompArgs.indexIn(line) == 0)
        {
            insn = precompArgs.cap(1).toLower();
            args = precompArgs.cap(2);
        }

        if (insn.length() != 0)
        {
            if (insn == "if" || insn == "ifn")
            {
                compilePreprocConditional(insn, args, precompLevels, precompFalseLevels);
            }
            else if (insn == "else" || insn == "elseif" || insn == "elseifn")
            {
                if (precompLevels.size() > 0)
                {
                    if (precompLevels.back())
                        precompFalseLevels++;
                    else
                        precompFalseLevels--;
                    precompLevels.back() = !precompLevels.back();

                    if (insn == "elseif" || insn == "elseifn")
                    {
                        if (!precompLevels.back())
                            precompFalseLevels--;
                        precompLevels.pop_back();
                        compilePreprocConditional(insn, args, precompLevels, precompFalseLevels);
                    }
                }
                else
                    qWarning("#ELSE without matching #IF(N) in node '%s'", qPrintable(context_->name));
            }
            else if (insn == "endif")
            {
                if (precompLevels.size() > 0)
                {
                    if (!precompLevels.back())
                        precompFalseLevels--;
                    precompLevels.pop_back();
                }
                else
                    qWarning("#ENDIF without matching #IF(N) in node '%s'", qPrintable(context_->name));
            }
            else
            {
                qWarning("Illegal preprocessor instruction: '%s' in node '%s'", qPrintable(insn), qPrintable(context_->name));
            }
        }
        else
        {
            int pos;
            bool addLine = true;
            while ((pos = expr.indexIn(line)) != -1)
            {
                QString type = expr.cap(1), variant;
                int dotPos;
                if ((dotPos = type.indexOf('.')) != -1)
                {
                    variant = type.mid(dotPos + 1);
                    type = type.left(dotPos);
                }
                QStringList replacement = compileExpression(type, variant, expr.cap(2));
                if (replacement.length() == 0)
                {
                    line = "";
                    addLine = false;
                    break;
                }
                else if (replacement.length() == 1)
                {
                    line = line.replace(pos, expr.cap(0).length(), replacement[0]);
                }
                else
                {
                    bool notIndentation = false;
                    for (int i = 0; i < pos; i++)
                    {
                        if (line[i] != '\t')
                        {
                            notIndentation = true;
                            break;
                        }
                    }

                    if (notIndentation)
                    {
                        qWarning("Expression expands to multiple lines at: '%s' in node '%s'", qPrintable(line), qPrintable(context_->name));
                        break;
                    }

                    if (precompFalseLevels == 0 && line.length() > 0)
                    {
                        QStringList indented = reindentScript(replacement, pos);
                        foreach (QString sline, indented)
                        {
                            compiled.push_back(sline);
                        }
                    }

                    addLine = false;
                    break;
                }
            }

            if (precompFalseLevels == 0 && line.length() > 0 && addLine)
                compiled.push_back(line);
        }
    }

    if (!precompLevels.empty())
        qWarning("%d preprocessor blocks were not closed in node '%s'", precompLevels.size(), qPrintable(context_->name));

    return compiled;
}


void ScriptNodeCompiler::compilePreprocConditional(QString const & insn, QString const & args, QVector<bool> & precompLevels, int & precompFalseLevels)
{
    QRegExp conn("^CONNECTED\\(([^(]+)\\)[\\s]*$", Qt::CaseInsensitive);
    QRegExp prop("^PROPERTY\\(([^(]+)\\)[\\s]*==[\\s]*\"([^\"]*)\"[\\s]*$", Qt::CaseInsensitive);

    bool negate = (insn == "ifn" || insn == "elseifn");
    if (args == "DEBUG")
    {
        if (context_->debug ^ negate)
        {
            precompLevels.push_back(false);
            precompFalseLevels++;
        }
        else
            precompLevels.push_back(true);
    }
    else if (conn.indexIn(args) == 0)
    {
        ScriptNode * port = context_->port(conn.cap(1));
        if (port)
        {
            if (((port->type() == ScriptNode::InputPort && !port->hasInboundLinksAll(ScriptConnection::Connected)) ||
                (port->type() == ScriptNode::OutputPort && !port->hasOutboundLinksAll(ScriptConnection::Connected))) ^ negate)
            {
                precompLevels.push_back(false);
                precompFalseLevels++;
            }
            else
                precompLevels.push_back(true);
        }
        else
            qWarning("#IF: Unknown CONNECTED() port '%s' in node '%s'", qPrintable(conn.cap(1)), qPrintable(context_->name));
    }
    else if (prop.indexIn(args) == 0)
    {
        QVariant p = context_->property(prop.cap(1));
        if (p.isValid())
        {
            QString value = toString(p);
            if ((value != prop.cap(2)) ^ negate)
            {
                precompLevels.push_back(false);
                precompFalseLevels++;
            }
            else
                precompLevels.push_back(true);
        }
        else
            qWarning("#IF: Unknown PROPERTY() '%s' in node '%s'", qPrintable(prop.cap(1)), qPrintable(context_->name));
    }
    else
        qWarning("#IF: Illegal expression '%s' in node '%s'", qPrintable(args), qPrintable(context_->name));
}

QStringList ScriptNodeCompiler::compileExpression(QString type, QString variant, QString name)
{
    QStringList retval;
    if (type == "TRIGGER")
    {
        ScriptNode * port = context_->port(name);
        if (!port || port->type() != ScriptNode::OutputPort || !(port->flags() & ScriptNode::Trigger))
        {
            qWarning("Output trigger port '%s' was not found in node '%s'",
                     qPrintable(name), qPrintable(context_->name));
            retval.push_back("<<TRIGGER-ERROR>>");
            return retval;
        }

        depends(node_, port, ScriptConnection::Trigger);

        bool inlined = false, called = false, pass = false;
        if (variant.indexOf('P') != -1)
            pass = true;
        if (variant.indexOf('I') != -1 && !(port->flags() & ScriptNode::DontInline))
            inlined = true;
        if (variant.indexOf('F') != -1)
        {
            if (!(port->flags() & ScriptNode::Called))
            {
                Q_ASSERT(!(flags_ & OrderedCompile));
                port->setFlags(port->flags() | ScriptNode::Called);
            }
            called = true;
        }

        if (port->compiledScript().length() > 0)
        {
            if (!inlined && !called)
            {
                if (port->flags() & ScriptNode::Inlined)
                    inlined = true;
                else
                    called = true;
            }

            if (inlined)
                retval = port->compiledScript().split("\r\n");
            else
                retval.push_back("self." + compileFunctionName(port) + "()");
        }
        else if (pass)
            retval.push_back("pass");
    }
    else if (type == "VAR")
    {
        unsigned int depFlags = 0;
        if (variant.indexOf('R') != -1)
            depFlags |= ScriptConnection::Read;
        if (variant.indexOf('W') != -1)
            depFlags |= ScriptConnection::Write;
        if (variant.indexOf('A') != -1)
            depFlags |= ScriptConnection::Access;
        if (depFlags == 0)
            depFlags = ScriptConnection::Access;

        ScriptNode * port = context_->port(name);
        if (!port || port->flags() & ScriptNode::Trigger)
        {
            qWarning("Variable port '%s' was not found in node '%s'",
                     qPrintable(name), qPrintable(context_->name));
            retval.push_back("<<VAR-ERROR>>");
            return retval;
        }

        depends(node_, port, depFlags);

        // Don't emit code if an eliminated variable is written
        if (depFlags == ScriptConnection::Write && !port->isAlive())
            return retval;

        if (depFlags == ScriptConnection::Read && !port->isAlive())
        {
            retval.push_back("None");
            return retval;
        }

        retval.push_back("self." + compileName(port));
    }
    else if (type == "PROPERTY")
    {
        QVariant prop = context_->property(name);
        if (prop.isValid() && !isInternalProperty(name))
        {
            retval.push_back(toScriptString(prop));
            return retval;
        }

        qWarning("Property '%s' was not found in node '%s'",
                 qPrintable(name), qPrintable(context_->name));
        retval.push_back("<<PROPERTY-ERROR>>");
    }
    else if (type == "VAR_OR_PROP")
    {
        ScriptNode * port = context_->port(name);
        if (!port || port->flags() & ScriptNode::Trigger)
        {
            qWarning("Variable port '%s' was not found in node '%s'",
                     qPrintable(name), qPrintable(context_->name));
            retval.push_back("<<VAR-ERROR>>");
            return retval;
        }

        depends(node_, port, ScriptConnection::Read);

        QVariant prop = context_->property(name);
        if (!prop.isValid() || isInternalProperty(name))
        {
            qWarning("Property '%s' was not found in node '%s'",
                     qPrintable(name), qPrintable(context_->name));
            retval.push_back("<<PROPERTY-ERROR>>");
            return retval;
        }

        if (!port->isAlive())
            retval.push_back(toScriptString(context_->property(name)));
        else
            retval.push_back("(self." + compileName(port) + " or " + toScriptString(context_->property(name)) + ")");
    }
    else if (type == "PROPAGATE")
    {
        ScriptNode * port = context_->port(name);
        if (!port || port->type() != ScriptNode::OutputPort || port->flags() & ScriptNode::Trigger)
        {
            qWarning("Output variable port '%s' was not found in node '%s'",
                     qPrintable(name), qPrintable(context_->name));
            retval.push_back("<<PROPAGATE-ERROR>>");
            return retval;
        }

        depends(node_, port, ScriptConnection::Propagate);

        if (port->isAlive())
        {
            bool inlined = false, called = false, pass = false;
            if (variant.indexOf('P') != -1)
                pass = true;
            if (variant.indexOf('I') != -1 && !(port->flags() & ScriptNode::DontInline))
                inlined = true;
            if (variant.indexOf('F') != -1)
            {
                if (!(port->flags() & ScriptNode::Called))
                {
                    Q_ASSERT(!(flags_ & OrderedCompile));
                    port->setFlags(port->flags() | ScriptNode::Called);
                }
                called = true;
            }

            if (port->compiledScript().length() > 0)
            {
                if (!inlined && !called)
                {
                    if (port->flags() & ScriptNode::Inlined)
                        inlined = true;
                    else
                        called = true;
                }

                if (inlined)
                    retval = port->compiledScript().split("\r\n");
                else
                    retval.push_back("self." + compileFunctionName(port) + "()");
            }
            else if (pass)
                retval.push_back("pass");
        }
    }
    else if (type == "LOCAL")
    {
        retval.push_back("self.n" + QString::number(context_->id) + "_lvar_" + compileName(name));
    }
    else if (type == "NODEID")
    {
        retval.push_back(QString::number(context_->id));
    }
    else if (type == "NODENAME")
    {
        retval.push_back(context_->name);
    }
    else
    {
        qWarning("Illegal script expresssion type: '%s'", qPrintable(type));
        retval.push_back("<<TYPE-ERROR>>");
    }
    return retval;
}

QStringList ScriptNodeCompiler::reindentScript(QString script, int indentation)
{
    QStringList lines = script.split(QRegExp("[\\r\\n]+"));
    if (lines.empty() || lines.length() == 1 && lines[0].length() == 0)
        return QStringList();

    return reindentScript(lines, indentation);
}

QStringList ScriptNodeCompiler::reindentScript(QStringList script, int indentation)
{
    QString first = script[0];
    int tabs = 0;
    while (tabs < first.length() && first[tabs] == '\t')
        tabs++;

    for (int i = 0; i < script.length(); i++)
    {
        if (script[i].length() >= tabs)
        {
            for (int j = 0; j < tabs; j++)
            {
                if (script[i][j] != '\t')
                {
                    qWarning("Script line has invalid indentation:\r\n>> %s", qPrintable(script[i]));
                    break;
                }
            }
            script[i] = QString("\t").repeated(indentation) + script[i].mid(tabs);
        }
        else
            qWarning("Script line is missing indentation:\r\n>> %s", qPrintable(script[i]));
    }

    return script;
}

QString ScriptNodeCompiler::compileFunctionName(ScriptNode * port)
{
    if (port->type() == ScriptNode::OutputPort)
        return "n" + QString::number(port->blockId()) + "_propagator_" + compileName(port->name());
    else if (port->type() == ScriptNode::InputPort)
        return "n" + QString::number(port->blockId()) + "_trigger_" + compileName(port->name());
    else
        return compileName(port);
}

QString ScriptNodeCompiler::compileName(ScriptNode * port)
{
    QString type;
    if (port->type() == ScriptNode::OutputPort || port->type() == ScriptNode::InputPort)
        type = "_var_";
    else if (port->type() == ScriptNode::Script)
        type = "_script_";
    return "n" + QString::number(port->blockId()) + type + compileName(port->name());
}

// Converts the specified property or port name to a name
// that is allowed in Python scripts
// (Replaces special characters in names)
QString ScriptNodeCompiler::compileName(QString name)
{
    bool bogus = false;
    QString out;
    foreach (QChar c, name)
    {
        if (c == ' ' || c == '\t' || c == '_')
            out += "_";
        else if (c == '<')
            out += "less_";
        else if (c == '>')
            out += "greater_";
        else if (c == '=')
            out += "equal_";
        else if (c == '!')
            out += "not_";
        else if (c == '&')
            out += "and_";
        else if (c == '|')
            out += "or_";
        else if ((c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'))
            out += c;
        else
        {
            out += "_";
            bogus = true;
        }
    }

    if (bogus)
        qWarning("Property or port name '%s' contains invalid characters", qPrintable(name));

    return out;
}

bool ScriptNodeCompiler::isInternalProperty(QString name)
{
    return (name == "Comment" || name == "Enabled");
}

void ScriptNodeCompiler::depends(ScriptNode * node, ScriptNode * dependency, unsigned int flags)
{
    if (node != dependency)
        optimizer_->addDependency(node, dependency, flags);
}

QString ScriptNodeCompiler::toString(QVariant const &value)
{
    if (value.canConvert(QVariant::String))
    {
        QVariant var(value);
        if (!var.convert(QVariant::String))
            qWarning("Failed to convert variant type %d to string", value.type());
        return var.value<QString>();
    }
    else
    {
        qWarning("Variant type %d is not convertable to string!", value.type());
        return "";
    }
}

QString ScriptNodeCompiler::toScriptString(QVariant const &value)
{
    // TODO: Fix entity property parsing!
    // if (type == "Entity")
    //    return "None";

    if (value.type() == QVariant::Bool)
        return value.toBool() ? "True" : "False";

    if (value.type() == QVariant::String)
        return "'" + value.toString().replace(QRegExp("([\"\\\\])"), "\\\\1") + "'";

    if (value.canConvert(QVariant::String))
    {
        QVariant var(value);
        if (!var.convert(QVariant::String))
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

