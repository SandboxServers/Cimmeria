#ifndef SCRIPTCOMPILER_H
#define SCRIPTCOMPILER_H

#include "scriptdefinitions.h"
#include "scriptnodeseditor.h"
#include <QTextStream>

/*
 * Connection flags:
 *  (T)rigger   - outgoing port is triggered internally
 *  (C)onnected - port is connected to another port of a different node
 *  (R)ead      - property is read by scripts
 *  (A)ccess    - property is accessed by scripts (either read or write)
 *  (W)rite     - property is written by scripts
 *  (P)ropagate - outgoing port is propagated internally
 * *(F)orce     - node cannot be optimized out
 * *(E)liminate - node is eliminated from the final compilation results
 */
class ScriptNode
{
public:
    enum Flags
    {
        // Node cannot be optimized out (set for script nodes and during loop detection)
        Force = 0x01,
        // Node is eliminated from the final compilation results
        Eliminate = 0x02,
        // Port is a trigger port (not set --> port is a variable port)
        Trigger = 0x04,
        // Node is an initialization script (executed on startup)
        InitScript = 0x08,
        // Node is a teardown script
        TeardownScript = 0x10,
        // Node is a saved variable persist script
        PersistScript = 0x20,
        // Node is a saved variable restore script
        RestoreScript = 0x40,
        // Script should not be inlined
        DontInline = 0x80,
        // Script has at least one function symbol reference
        Called = 0x100,
        // Script was compiled at least once (last compiled version is cached)
        Compiled = 0x200,
        // Compiled script is inlined
        Inlined = 0x400,
        // Node must be connected (issues a warning if it isn't)
        Required = 0x800,
        // Node is a named Python method that should not be inlined or eliminated
        NamedMethod = 0x1000,
        // Script should be executed before the init script
        // (mainly variable assignments and propagators)
        PreInitScript = 0x2000,
        // Script should be executed when the module is loaded by the Python runtime
        StaticInitScript = 0x4000,
        // Script should be executed when the module is unloaded by the Python runtime
        StaticTeardownScript = 0x8000,
        // Flags that are added during precompilation and should be cleared
        DynamicFlags = Eliminate | DontInline | Compiled | Inlined
    };

    enum Type
    {
        // Input trigger/variable ports
        InputPort,
        // Output trigger/variable ports
        OutputPort,
        // StaticInit/(Pre)Init/Teardown/Persist/Restore scripts
        Script
    };

    ScriptNode(int blockId, QString name, Type type, unsigned int flags, QString script = "");

    int blockId() const;
    QString name() const;
    Type type() const;
    QString script() const;
    QVector<class ScriptConnection *> const & outboundLinks() const;
    QVector<class ScriptConnection *> const & inboundLinks() const;

    void setCompiledScript(QString script);
    QString compiledScript() const;

    void setFlags(unsigned int flags);
    unsigned int flags() const;

    // Checks if the node is still referenced
    // (is updated after each optimization step by optimize())
    bool isAlive() const;

    // Was the script compiled already?
    bool isCompiled() const;

    // Should we generate a function or is the script completely inlined?
    bool shouldEmitFunction() const;

    // Update optimization flags based on rules
    // Returns true if any optimizations were made
    bool optimize();

    // Checks if the node has outbound links where all flags are set
    bool hasOutboundLinksAll(unsigned int connectionFlags = 0, unsigned int nodeFlags = 0) const;
    // Checks if the node has inbound links where all flags are set
    bool hasInboundLinksAll(unsigned int connectionFlags = 0, unsigned int nodeFlags = 0) const;
    // Checks if the node has outbound links where any of the flags are set
    bool hasOutboundLinksAny(unsigned int connectionFlags = 0, unsigned int nodeFlags = 0) const;
    // Checks if the node has inbound links where any of the flags are set
    bool hasInboundLinksAny(unsigned int connectionFlags = 0, unsigned int nodeFlags = 0) const;

    // Dump node properties to debug channel
    void debugDump();

protected:
    friend class ScriptConnection;

    void addOutLink(class ScriptConnection * link);
    void removeOutLink(class ScriptConnection * link);

    void addInLink(class ScriptConnection * link);
    void removeInLink(class ScriptConnection * link);

private:
    int blockId_;
    QString name_;
    Type type_;
    unsigned int flags_;
    QString script_;
    QString compiledScript_;
    QVector<class ScriptConnection *> inLinks_, outLinks_;
};


class ScriptConnection
{
public:
    enum Flags
    {
        // Outgoing port is triggered internally
        Trigger = 0x01,
        // Port is connected to another port of a different node
        Connected = 0x02,
        // Variable is read by scripts
        Read = 0x04,
        // Variable is written by scripts
        Write = 0x08,
        // Variable is accessed by scripts (without read or write qualifier)
        Access = 0x10,
        // Outgoing port is propagated internally
        Propagate = 0x20,
        // Connection is eliminated from the final compilation results
        Eliminate = 0x40,
        // Connection should not be removed after a compilation step
        // (set for static connections, eg. links between output and input ports)
        Persistent = 0x80,
        // Flags that are added during precompilation and should be cleared
        DynamicFlags = Trigger | Read | Write | Access | Propagate | Eliminate
    };

    ScriptConnection(ScriptNode * source, ScriptNode * target);
    ~ScriptConnection();

    ScriptNode * source() const;
    ScriptNode * target() const;

    void setFlags(unsigned int flags);
    unsigned int flags() const;

    // Checks if the connection is still referenced
    // (is updated after each optimization step by optimize())
    bool isAlive() const;

    // Update optimization flags based on rules
    // Returns true if any optimizations were made
    bool optimize();

private:
    ScriptNode * source_, * target_;
    unsigned int flags_;
};

class ScriptOptimizer
{
public:
    const static unsigned int MaxOptimizationIterations = 10;

    struct Block
    {
        QMap<QString, ScriptNode *> nodes;
        QMap<QString, QVariant> properties;
        unsigned int id;
        QString name;
        bool debug;

        ScriptNode * port(QString name) const;
        QVariant property(QString name) const;
    };

    ScriptOptimizer(ScriptNodesEditor & editor);
    ~ScriptOptimizer();

    void addNodeSet(ScriptNodesEditor & editor);
    void addNode(ScriptNode * node);
    void addBlock(QNEBlock & block);
    void addBlockScript(QNEBlock & block, QString name, unsigned int flags, QString script);
    void addBlockConnections(QNEBlock & block);
    void addDependency(ScriptNode * node, ScriptNode * dependency, unsigned int flags);

    bool compile(QString const & path);
    bool optimize();

    // Dump optimizer settings, all nodes and connections to debug channel
    void debugDump();

private:
    struct Vertex
    {
        inline Vertex() : node(nullptr), index(-1), lowlink(-1) {}
        inline Vertex(ScriptNode * n) : node(n), index(-1), lowlink(-1) {}

        ScriptNode * node;
        int index, lowlink;
    };

    struct Edge
    {
        inline Edge() : source(nullptr), target(nullptr) {}
        inline Edge(Vertex * s, Vertex * t) : source(s), target(t) {}

        Vertex * source, * target;
    };

    struct GraphData
    {
        QMap<ScriptNode *, Vertex> vertices;
        QMap<ScriptNode *, QVector<Edge> > outEdges;
        QVector<Edge> edges;
        QVector<Vertex *> sets;
        QVector<QVector<Vertex *> > components;
        int index;
    };

    ScriptNodesEditor & editor_;
    QVector<ScriptNode *> nodes_;
    QVector<ScriptConnection *> connections_;
    QMap<int, Block *> blockMap_;
    QVector<ScriptNode *> orderedNodes_;
    QVector<QNEBlock *> refBlocks_;

    bool findCompilationOrder();
    void findCycles();
    void strongConnect(GraphData & graph, Vertex &vert);
};

class ScriptNodeCompiler
{
public:
    const static unsigned int MaxInlineLength = 4;

    enum Flags
    {
        AllowInlining = 0x01,
        DontCompile = 0x02,
        OrderedCompile = 0x04
    };

    enum ExpressionFlags
    {
        DontEmitLine = 0x01
    };

    ScriptNodeCompiler(ScriptOptimizer * optimizer, ScriptOptimizer::Block * context,
        ScriptNode * node, unsigned int flags = 0);

    bool compile();

    bool isInlined(ScriptNode * node);
    QStringList compileBody(ScriptNode * port);
    QStringList compileScript(QString script, int indentation);
    void compilePreprocConditional(const QString &expression, const QString &args, QVector<bool> &precompLevels, int &precompFalseLevels);
    QStringList compileExpression(QString type, QString variant, QString name);
    QStringList reindentScript(QStringList script, int indentation);
    QStringList reindentScript(QString script, int indentation);
    QString compileFunctionName(ScriptNode * port);
    QString compileName(ScriptNode * port);
    static QString compileName(QString name);
    bool isInternalProperty(QString name);

private:
    ScriptOptimizer * optimizer_;
    ScriptOptimizer::Block * context_;
    ScriptNode * node_;
    unsigned int flags_;

    void depends(ScriptNode * node, ScriptNode * dependency, unsigned int flags);
    QString toString(QVariant const &value);
    QString toScriptString(QVariant const &value);
};

#endif // SCRIPTCOMPILER_H
