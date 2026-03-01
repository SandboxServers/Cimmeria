# BigWorld Watcher System

> **Last updated**: 2026-03-01
> **Phase**: 5 -- BigWorld Engine Subsystems
> **RE Status**: Documented from BigWorld 2.0.1 reference source; not implemented in Cimmeria
> **Sources**: BW `lib/cstdmf/watcher.hpp`, BW `lib/cstdmf/watcher.cpp`, BW `lib/server/watcher_protocol.hpp`, BW `lib/server/watcher_protocol.cpp`, BW `lib/network/watcher_nub.hpp`, BW `lib/network/watcher_nub.cpp`, BW `lib/network/watcher_connection.hpp`, BW `lib/network/watcher_connection.cpp`, BW `lib/server/watcher_forwarding.hpp`, BW `lib/server/watcher_forwarding_collector.hpp`, BW `lib/server/watcher_forwarding_types.hpp`, BW `lib/pyscript/pywatcher.hpp`, BW `lib/pyscript/pywatcher.cpp`, BW `lib/network/basictypes.hpp`, BW `lib/cstdmf/config.hpp`

---

## Overview

The Watcher system is BigWorld's built-in runtime monitoring and debugging infrastructure. It provides a hierarchical tree of observable (and sometimes writable) values accessible remotely over the network. Conceptually it functions like a live `/proc` filesystem for game server processes -- any C++ variable, class member, or function return value can be registered into a path-addressable tree and queried or modified at runtime by external tools.

The system is entirely server-side. The SGW.exe client binary contains no watcher code (the only symbol resembling "watcher" is `APotentialClimbWatcher`, which is an Unreal Engine 3 gameplay class unrelated to the BigWorld watcher system). The watcher infrastructure is conditionally compiled via the `ENABLE_WATCHERS` preprocessor flag, allowing it to be stripped from production or console builds.

Cimmeria does **not** implement the Watcher system.

---

## Class Hierarchy

All watcher classes derive from the `Watcher` abstract base class, which inherits `SafeReferenceCount` for reference-counted lifetime management. The hierarchy:

```
Watcher (abstract base -- SafeReferenceCount)
|
+-- DirectoryWatcher              Tree node containing child watchers
|
+-- DataWatcher<TYPE>             Watches a C++ variable directly by reference
|
+-- FunctionWatcher<RETURN_TYPE>  Calls a free function to get/set value
|
+-- MemberWatcher<RET,OBJ>       Calls getter/setter methods on an object
|
+-- SequenceWatcher<SEQ>          Watches elements of a vector/sequence container
|
+-- MapWatcher<MAP>               Watches key-value pairs in a map container
|
+-- DereferenceWatcher (abstract) Pointer-following wrapper
|   +-- BaseDereferenceWatcher        Follows raw pointer (*(uintptr*)base)
|   +-- SmartPointerDereferenceWatcher Follows SmartPointer<T>
|   +-- ContainerBounceWatcher<C,K>   Looks up key in container, follows to value
|
+-- CallableWatcher               RPC-style callable function with arguments
|   +-- NoArgCallableWatcher          Callable with no arguments (abstract)
|       +-- NoArgFuncCallableWatcher  Concrete: wraps a C function pointer
|
+-- ForwardingWatcher             Forwards queries to managed sub-processes
|
+-- PyObjectWatcher               Watches a PyObject* with type dispatch
+-- SimplePythonWatcher           Watches arbitrary Python objects by base pointer
+-- PyAccessorWatcher             Binds Python get/set callables as a watcher
+-- PyFunctionWatcher             Python function callable with typed arguments
```

### Watcher Base Class

```cpp
class Watcher : public SafeReferenceCount
{
public:
    enum Mode
    {
        WT_INVALID,     // 0 - Indicates an error
        WT_READ_ONLY,   // 1 - Value cannot be changed
        WT_READ_WRITE,  // 2 - Value can be read and modified remotely
        WT_DIRECTORY,   // 3 - Node is a directory containing sub-watchers
        WT_CALLABLE     // 4 - Watcher is a callable function (v2 only)
    };

    Watcher( const char * comment = "" );
    virtual ~Watcher();

    // Protocol v1 (string-based) interface
    virtual bool getAsString( const void * base, const char * path,
        std::string & result, std::string & desc, Mode & mode ) const = 0;
    virtual bool setFromString( void * base, const char * path,
        const char * valueStr ) = 0;

    // Protocol v2 (binary stream) interface
    virtual bool getAsStream( const void * base, const char * path,
        WatcherPathRequestV2 & pathRequest ) const = 0;
    virtual bool setFromStream( void * base, const char * path,
        WatcherPathRequestV2 & pathRequest ) = 0;

    // Directory traversal
    virtual bool visitChildren( const void * base, const char * path,
        WatcherPathRequest & pathRequest ) { return false; }
    virtual bool addChild( const char * path, WatcherPtr pChild,
        void * withBase = NULL ) { return false; }
    virtual bool removeChild( const char * path ) { return false; }

    // Global root
    static Watcher & rootWatcher();
    static void fini();

protected:
    static bool isEmptyPath( const char * path );
    static bool isDocPath( const char * path );  // checks for "__doc__"
    std::string comment_;
};
```

Key design points:
- Every watcher class must implement both v1 (string) and v2 (binary stream) accessors.
- The `base` pointer enables offset-based addressing, allowing a single watcher definition to work across an array of objects.
- The `comment_` field provides a description string accessible via the `__doc__` virtual path.
- The root watcher is a global singleton `DirectoryWatcher`, lazily initialized and cleaned up via `FiniJob`.

### DirectoryWatcher

The tree structure container. Stores children in a sorted `std::vector<DirData>`:

```cpp
class DirectoryWatcher : public Watcher
{
private:
    struct DirData
    {
        WatcherPtr   watcher;
        void *       base;
        std::string  label;
    };
    typedef std::vector<DirData> Container;
    Container container_;
};
```

Children are maintained in sorted order (when `SORTED_WATCHERS` is defined, which is the default). The path separator is `/`:

```cpp
const char WATCHER_SEPARATOR = '/';
```

Path resolution works by splitting the path at the first separator, finding the matching child by label prefix, and recursing into it with the remaining tail.

### DataWatcher

Watches a C++ variable directly by reference:

```cpp
template <class TYPE>
class DataWatcher : public Watcher
{
public:
    explicit DataWatcher( TYPE & rValue,
        Watcher::Mode access = Watcher::WT_READ_ONLY,
        const char * path = NULL );
private:
    TYPE & rValue_;
    Watcher::Mode access_;
};
```

The variable is read/written via pointer arithmetic against the `base` offset:

```cpp
const TYPE & useValue = *(const TYPE*)(
    ((const uintptr)&rValue_) + ((const uintptr)base) );
```

### MemberWatcher

Watches a value obtained via class member getter/setter methods:

```cpp
template <class RETURN_TYPE, class OBJECT_TYPE, class CONSTRUCTION_TYPE = RETURN_TYPE>
class MemberWatcher : public Watcher
{
private:
    OBJECT_TYPE * pObject_;
    RETURN_TYPE (OBJECT_TYPE::*getMethod_)() const;
    void (OBJECT_TYPE::*setMethod_)( RETURN_TYPE );
};
```

If `setMethod_` is NULL, the watcher is read-only.

### FunctionWatcher

Watches the return value of a free function:

```cpp
template <class RETURN_TYPE, class CONSTRUCTION_TYPE = RETURN_TYPE>
class FunctionWatcher : public Watcher
{
private:
    RETURN_TYPE (*getFunction_)();
    void (*setFunction_)( RETURN_TYPE );
};
```

### CallableWatcher

For RPC-style callable functions exposed over the watcher protocol. Supports a typed argument list:

```cpp
class CallableWatcher : public Watcher
{
public:
    enum ExposeHint
    {
        WITH_ENTITY,   // 0 - Component owning a specific entity
        ALL,           // 1 - All managed components
        WITH_SPACE,    // 2 - Components with a specific space
        LEAST_LOADED,  // 3 - The least loaded component
        LOCAL_ONLY     // 4 - Only this process
    };

    void addArg( WatcherDataType type, const char * description = "" );
};
```

Special virtual paths are recognized:
- `__args__` -- Returns the argument list as a tuple of (description, type) pairs
- `__expose__` -- Returns the `ExposeHint` as an int32
- `__doc__` -- Returns the comment string

### SequenceWatcher and MapWatcher

Template watchers for STL containers. `SequenceWatcher<SEQ>` presents each element as a named child (using index numbers, custom labels, or a label sub-path). `MapWatcher<MAP>` presents each key-value pair with the key converted to a string label. Both delegate to a single child watcher for element rendering.

---

## MF_WATCH Macro System

The primary way to register watchers in BigWorld C++ code:

```cpp
#define MF_WATCH      ::addWatcher
#define MF_WATCH_REF  ::addReferenceWatcher
```

`MF_WATCH` resolves to the `addWatcher()` family of overloaded template functions. Each creates the appropriate watcher type and adds it to the root watcher tree.

### Usage Patterns

**Simple variable watch (read-write by default):**
```cpp
static int myValue = 72;
MF_WATCH( "myValue", myValue );
```

**Read-only variable:**
```cpp
MF_WATCH( "myValue", myValue, Watcher::WT_READ_ONLY );
```

**Member accessor (read-only):**
```cpp
MF_WATCH( "some value", exampleObject_,
    ExampleClass::getValue );
```

**Member accessor (read-write):**
```cpp
MF_WATCH( "some value", exampleObject_,
    ExampleClass::getValue,
    ExampleClass::setValue );
```

**Overloaded accessor helper:**
```cpp
MF_WATCH( "Comms/Desired in", g_server,
    MF_ACCESSORS( uint32, ServerConnection, bandwidthFromServer ) );
```

The `MF_ACCESSORS` macro performs the necessary `static_cast` for disambiguating overloaded methods:

```cpp
#define MF_ACCESSORS( TYPE, CLASS, METHOD )                     \
    static_cast< TYPE (CLASS::*)() const >(&CLASS::METHOD),     \
    static_cast< void (CLASS::*)(TYPE)   >(&CLASS::METHOD)
```

There is also `MF_WRITE_ACCESSOR` for write-only watchers (NULL getter).

### addWatcher Overloads

The `addWatcher` function has multiple overloads matching different watcher types:

| Signature | Creates |
|-----------|---------|
| `addWatcher(path, TYPE & value, Mode)` | `DataWatcher<TYPE>` |
| `addWatcher(path, RETURN_TYPE (*get)(), void (*set)())` | `FunctionWatcher<RETURN_TYPE>` |
| `addWatcher(path, OBJ &, RET (OBJ::*get)() const)` | `MemberWatcher<RET,OBJ>` (read-only) |
| `addWatcher(path, OBJ &, RET (OBJ::*get)() const, void (OBJ::*set)(RET))` | `MemberWatcher<RET,OBJ>` (read-write) |
| `addReferenceWatcher(path, OBJ &, const RET& (OBJ::*get)() const)` | `MemberWatcher` (const ref variant) |

All overloads add the watcher to `Watcher::rootWatcher()` and optionally set a comment string.

---

## Watcher Data Types

The `WatcherDataType` enum defines the type identifiers used in the v2 binary protocol:

```cpp
enum WatcherDataType {
    WATCHER_TYPE_UNKNOWN = 0,
    WATCHER_TYPE_INT     = 1,  // int32 or int64
    WATCHER_TYPE_UINT    = 2,  // uint32 or uint64
    WATCHER_TYPE_FLOAT   = 3,  // float or double
    WATCHER_TYPE_BOOL    = 4,  // bool (1 byte)
    WATCHER_TYPE_STRING  = 5,  // length-prefixed string
    WATCHER_TYPE_TUPLE   = 6,  // compound type (nested elements)
    WATCHER_TYPE_TYPE    = 7   // meta-type (describes a WatcherDataType value)

    // Reserved but not implemented:
    // WATCHER_TYPE_VECTOR2,
    // WATCHER_TYPE_VECTOR3,
    // WATCHER_TYPE_VECTOR4
};
```

### Type Serialization

Each value on the stream is encoded as:

```
[type: 1 byte] [mode: 1 byte] [size: packed int] [data: size bytes]
```

The `watcherValueToStream()` overloads handle encoding, while `watcherStreamToValue()` overloads handle decoding. Specific type mappings:

| C++ Type | Wire Type | Size |
|----------|-----------|------|
| `int32` | `WATCHER_TYPE_INT` | 4 bytes |
| `int64` | `WATCHER_TYPE_INT` | 8 bytes |
| `uint32` | `WATCHER_TYPE_UINT` | 4 bytes |
| `uint64` | `WATCHER_TYPE_UINT` | 8 bytes |
| `float` | `WATCHER_TYPE_FLOAT` | 4 bytes |
| `double` | `WATCHER_TYPE_FLOAT` | 8 bytes |
| `bool` | `WATCHER_TYPE_BOOL` | 1 byte |
| `std::string` | `WATCHER_TYPE_STRING` | variable (length-prefixed) |
| Compound | `WATCHER_TYPE_TUPLE` | variable (count + nested elements) |
| Meta-type | `WATCHER_TYPE_TYPE` | 1 byte |

The decoder supports cross-size casting: a 4-byte int on stream can populate an int64 (upcast), and vice versa (with potential data loss). If types do not match, the system falls back to string conversion via `watcherStreamToStringToValue()`.

### Packed Integer Size Encoding

Size fields use a packed format (shared with `BinaryStream::readStringLength`):
- If the first byte is < 0xFF: it is the size directly (1-254 bytes)
- If the first byte is 0xFF: the next 3 bytes encode the size using `BW_UNPACK3()`

---

## Watcher Protocol

### Version 1 (String-Based)

The v1 protocol uses string representations for all values. Messages carry:
- `WATCHER_MSG_GET (16)` -- Get values as strings
- `WATCHER_MSG_SET (17)` -- Set values from strings
- `WATCHER_MSG_TELL (18)` -- Reply with string values
- `WATCHER_MSG_GET_WITH_DESC (20)` -- Get values with description strings

Requests carry a count of paths, each as a null-terminated string. Replies carry (identifier, description, value) string triples.

### Version 2 (Binary)

The v2 protocol uses typed binary streams for efficient transport:
- `WATCHER_MSG_GET2 (26)` -- Binary get with sequence numbers
- `WATCHER_MSG_SET2 (27)` -- Binary set with typed data
- `WATCHER_MSG_TELL2 (28)` -- Binary reply
- `WATCHER_MSG_SET2_TELL2 (29)` -- Combined set + reply

Extension messages use IDs starting at `WATCHER_MSG_EXTENSION_START (107)`.

### Message IDs Summary

```cpp
enum WatcherMsg
{
    // Deprecated (old HTTP watcher)
    // WATCHER_MSG_REGISTER      = 0,
    // WATCHER_MSG_DEREGISTER    = 1,
    // WATCHER_MSG_FLUSHCOMPONENTS = 2,

    WATCHER_MSG_GET              = 16,
    WATCHER_MSG_SET              = 17,
    WATCHER_MSG_TELL             = 18,
    WATCHER_MSG_GET_WITH_DESC    = 20,

    WATCHER_MSG_GET2             = 26,
    WATCHER_MSG_SET2             = 27,
    WATCHER_MSG_TELL2            = 28,
    WATCHER_MSG_SET2_TELL2       = 29,

    WATCHER_MSG_EXTENSION_START  = 107
};
```

### Packet Structure

All watcher packets begin with a `WatcherDataMsg` header:

```cpp
struct WatcherDataMsg
{
    int  message;     // WatcherMsg enum value
    int  count;       // Number of path queries in this packet
    char string[0];   // Variable-length payload (flexible array)
};
```

**V1 GET packet layout:**
```
[message: 4B] [count: 4B] [path_1\0] [path_2\0] ...
```

**V2 GET2 packet layout:**
```
[message: 4B] [count: 4B] [seqNum_1: 4B] [path_1\0] [seqNum_2: 4B] [path_2\0] ...
```

**V2 SET2 packet layout:**
```
[message: 4B] [count: 4B] [seqNum_1: 4B] [path_1\0] [type: 1B] [size] [data] ...
```

### Registration Message

Used to register/deregister a watcher nub with the machine daemon:

```cpp
struct WatcherRegistrationMsg
{
    int   version;    // Currently 0
    int   uid;        // User ID
    int   message;    // WATCHER_MSG_* value
    int   id;         // Component ID (e.g. 14)
    char  abrv[32];   // Short name (e.g. "cell14")
    char  name[64];   // Long name (e.g. "Cell 14")
};
```

### Protocol Decoder

`WatcherProtocolDecoder` provides a virtual dispatch mechanism for decoding v2 streams. Subclasses override type-specific handlers:

```cpp
class WatcherProtocolDecoder
{
public:
    virtual bool decode( BinaryIStream & stream );
    virtual bool decodeNext( BinaryIStream & stream );
    virtual int readSize( BinaryIStream & stream );

    virtual bool intHandler( BinaryIStream & stream, Watcher::Mode mode );
    virtual bool uintHandler( BinaryIStream & stream, Watcher::Mode mode );
    virtual bool floatHandler( BinaryIStream & stream, Watcher::Mode mode );
    virtual bool boolHandler( BinaryIStream & stream, Watcher::Mode mode );
    virtual bool stringHandler( BinaryIStream & stream, Watcher::Mode mode );
    virtual bool tupleHandler( BinaryIStream & stream, Watcher::Mode mode );
    virtual bool defaultHandler( BinaryIStream & stream, Watcher::Mode mode );
};
```

The `decode()` method loops reading `[type][mode]` byte pairs and dispatching to the appropriate handler. The default handler implementation reads the size prefix and skips the data blob.

---

## Network Transport

### WatcherNub

The `WatcherNub` is the core network component. It is a **singleton** that binds both a UDP and TCP socket to the same port, listens for incoming watcher requests, and dispatches them.

```cpp
class WatcherNub :
    public Mercury::InputNotificationHandler,
    public Singleton< WatcherNub >
{
    bool init( const char * listeningInterface, uint16 listeningPort );
    int registerWatcher( int id, const char * abrv, const char * longName, ... );
    int deregisterWatcher();
    void attachTo( Mercury::EventDispatcher & dispatcher );
    bool receiveUDPRequest();
    bool processRequest( char * packet, int len, const RemoteEndpoint & remoteEndpoint );

private:
    int  id_;
    bool registered_;
    bool insideReceiveRequest_;
    char * requestPacket_;       // Pre-allocated receive buffer
    bool isInitialised_;
    Endpoint udpSocket_;
    Endpoint tcpSocket_;
    char abrv_[32];
    char name_[64];
    Mercury::EventDispatcher * pDispatcher_;
    WatcherRequestHandler * pExtensionHandler_;
};
```

**Packet size constants** (from `basictypes.hpp`):

```cpp
static const int WN_PACKET_SIZE     = 0x10000;    // 64 KB (UDP)
static const int WN_PACKET_SIZE_TCP = 0x1000000;   // 16 MB (TCP)
```

**Initialization flow:**
1. Creates a UDP socket (`SOCK_DGRAM`)
2. Resolves the listening interface to an address
3. Binds UDP and TCP sockets to the same port (retries up to 5 times)
4. Starts TCP `listen(5)` with backlog of 5
5. Registers file descriptors with the Mercury `EventDispatcher`

**Registration:** On registration, the nub sends a `ProcessMessage` to the local machine daemon (machined) at `127.0.0.1`, identifying itself with its component ID, name, abbreviation, and BigWorld version numbers. On deregistration, it sends a deregister message to the same daemon.

**Request processing:** When a UDP datagram or TCP message arrives, `processRequest()` validates the packet minimum size, identifies the message type, creates a `WatcherPacketHandler`, processes each path query in the packet, and then runs the handler which sends the reply. Unknown message types above `WATCHER_MSG_EXTENSION_START` are delegated to a `WatcherRequestHandler` extension callback.

### WatcherConnection

Handles TCP streaming for large watcher responses. When the `WatcherNub` accepts a TCP connection, it creates a `WatcherConnection` which manages the two-phase TCP protocol:

```cpp
class WatcherConnection : public Mercury::InputNotificationHandler
{
    bool recvSize();   // Phase 1: receive 4-byte message size
    bool recvMsg();    // Phase 2: receive message body

    WatcherNub & nub_;
    Mercury::EventDispatcher & dispatcher_;
    Endpoint * pEndpoint_;
    uint32 receivedSize_;
    uint32 messageSize_;
    char * pBuffer_;
};
```

**TCP message framing:**
```
[messageSize: 4 bytes (uint32)] [message body: messageSize bytes]
```

The message size is validated against `WN_PACKET_SIZE_TCP` (16 MB). Once the full message is received, it is passed to `WatcherNub::processRequest()` for the same handling as UDP messages. The `WatcherConnection` deletes itself on connection loss.

### WatcherPacketHandler

Created per-request by `WatcherNub::processRequest()`. It is initialized with:
- The remote endpoint for sending replies
- The expected count of path queries
- The protocol version (`WP_VERSION_1` or `WP_VERSION_2`)
- Whether this is a SET (vs GET) operation

For v1, the max reply packet size is `WN_PACKET_SIZE` minus header overhead. For v2 over TCP, it uses `WN_PACKET_SIZE_TCP`. For v2 over UDP, it uses `WN_PACKET_SIZE`.

The handler creates `WatcherPathRequestV1` or `WatcherPathRequestV2` objects for each path in the request, collects results, and self-deletes after sending the response.

---

## Watcher Forwarding

In multi-process BigWorld deployments (multiple CellApps, BaseApps), watcher queries can be forwarded from manager processes to their managed components.

### ForwardingWatcher

A special watcher subclass mounted in manager process watcher trees. It does **not** support the v1 protocol (returns false for `getAsString`/`setFromString`):

```cpp
class ForwardingWatcher : public Watcher
{
public:
    enum ExposeHints
    {
        WITH_ENTITY  = 0,  // Component owning a specific entity
        ALL          = 1,  // All components
        WITH_SPACE   = 2,  // Components with a specific space
        LEAST_LOADED = 3   // The least loaded component
    };

    virtual ForwardingCollector * newCollector(
        WatcherPathRequestV2 & pathRequest,
        const std::string & destWatcher,
        const std::string & targetInfo ) = 0;
};
```

When a v2 query arrives for a forwarding watcher path, the watcher extracts the target information from the path, creates a `ForwardingCollector`, and fans the query out to the appropriate component processes via Mercury.

### ForwardingCollector

Manages the fan-out and collection of responses:

```cpp
class ForwardingCollector : public Mercury::ShutdownSafeReplyMessageHandler
{
    WatcherPathRequestV2 & pathRequest_;
    std::string queryPath_;
    bool callingComponents_;
    uint32 pendingRequests_;
    Mercury::InterfaceElement interfaceElement_;
    AddressList * pAddressList_;
    Mercury::NetworkInterface * pInterface_;
    std::string outputStr_;
    MemoryOStream resultStream_;
    uint32 tupleCount_;
};
```

**Forwarding types** (from `watcher_forwarding_types.hpp`):

```cpp
typedef int32 ComponentID;
typedef std::vector<ComponentID> ComponentIDList;
typedef std::pair<Mercury::Address, ComponentID> AddressPair;
typedef std::vector<AddressPair> AddressList;
```

The collector sends the watcher query to each component address, tracks pending replies, and aggregates results. It uses a `ResponseDecoder` (a `WatcherProtocolDecoder` subclass) to parse component responses in a state machine:
1. `EXPECTING_TUPLE` -- First element should be the function result tuple
2. `EXPECTING_OUTPUT` -- Stdout/stderr output string
3. `EXPECTING_ANY` -- Additional result data

Once all responses arrive (or time out), the collector packs the aggregated results back into the original `WatcherPathRequestV2` for return to the querying client.

---

## Python Integration

BigWorld exposes the watcher system to Python scripts through the `BigWorld` module. All Python watcher functions are gated behind `ENABLE_WATCHERS`.

### BigWorld.setWatcher(path, value)

Sets a watcher value from Python. Converts the value to a string via `PyObject_Str()` and calls `Watcher::rootWatcher().setFromString()`:

```python
BigWorld.setWatcher( "Client Settings/Terrain/draw", 0 )
```

### BigWorld.getWatcher(path)

Retrieves a watcher value as a string:

```python
value = BigWorld.getWatcher( "Client Settings/Terrain/draw" )
# Returns: "1"
```

### BigWorld.getWatcherDir(path)

Lists the children of a directory watcher. Returns a list of `(mode, label, value)` tuples:

```python
children = BigWorld.getWatcherDir( "stats" )
# Returns: [(1, "numEntities", "42"), (2, "tickRate", "20"), (3, "bandwidth", "<DIR>")]
# mode: 1=READ_ONLY, 2=READ_WRITE, 3=DIRECTORY
```

### BigWorld.addWatcher(path, getFunction, [setFunction], [description])

Creates a Python-backed watcher using callable get/set functions:

```python
maxBandwidth = 20000

def getMaxBps():
    return str(maxBandwidth)

def setMaxBps(bps):
    maxBandwidth = int(bps)

BigWorld.addWatcher( "Comms/Max bandwidth per second", getMaxBps, setMaxBps )
```

Internally creates a `PyAccessorWatcher` that holds references to the Python callables.

### BigWorld.addFunctionWatcher(path, function, argList, exposeHint, [description])

Creates a callable function watcher with a typed argument list, suitable for RPC-style invocation from WebConsole:

```python
BigWorld.addFunctionWatcher( "command/addGuards",
    util.patrollingGuards,
    [("Number of guards to add", int)],
    BigWorld.EXPOSE_LEAST_LOADED,
    "Add an arbitrary number of patrolling guards into the world.")
```

Internally creates a `PyFunctionWatcher`. The `exposeHint` maps to `CallableWatcher::ExposeHint` and controls how the forwarding watcher fans out the call across managed components.

### BigWorld.delWatcher(path)

Removes a watcher from the tree:

```python
BigWorld.delWatcher( "Comms/Max bandwidth per second" )
```

### Python Object Watchers

`PyObjectWatcher` provides automatic type dispatch for Python objects:
- `str` objects -- rendered directly via `DataWatcher<PyObjectPtrRef>`
- Sequence objects (lists, tuples) -- dispatched to `SequenceWatcher<PySequenceSTL>`
- Mapping objects (dicts) -- dispatched to `MapWatcher<PyMappingSTL>`

`SimplePythonWatcher` takes an arbitrary `PyObject*` as its base pointer and navigates Python container hierarchies using `PyMapping_GetItemString()` and `PySequence_GetItem()`.

---

## Conditional Compilation

The entire watcher system is controlled by `ENABLE_WATCHERS`, defined in `lib/cstdmf/config.hpp`:

```cpp
#define ENABLE_WATCHERS  (!CONSUMER_CLIENT_BUILD || FORCE_ENABLE_WATCHERS)
```

- **Development/internal builds**: `CONSUMER_CLIENT_BUILD` is 0, so watchers are enabled.
- **Consumer/production builds**: watchers are disabled unless forced.
- **PlayStation 3**: explicitly disabled regardless of other settings.
- **Force override**: `FORCE_ENABLE_WATCHERS` can be set to 1 to enable in any build.

When `ENABLE_WATCHERS` is 0, the macros become no-ops:

```cpp
// VS2003 compatible stub (no variadic macros)
#define MF_WATCH false &&

// VS2005+ stub
#define MF_WATCH( ... )

// Stub Watcher class with Mode enum only (no methods)
class Watcher
{
public:
    enum Mode { WT_INVALID, WT_READ_ONLY, WT_READ_WRITE, WT_DIRECTORY };
};
```

This zero-overhead approach means no watcher code is compiled into the binary at all -- no registration, no tree, no network socket -- just the stub `Watcher::Mode` enum for compatibility with code that references it unconditionally.

---

## Cimmeria Implementation Status

**Not implemented.** Cimmeria does not use the BigWorld watcher system. The SGW.exe client binary contains no watcher client code. The watcher system is entirely server-side infrastructure.

### Recommended Monitoring Approach

Rather than implementing the full BigWorld watcher protocol, Cimmeria can achieve similar runtime monitoring via its existing Python console (port 8989):

1. **Variable inspection**: Expose C++ values through Boost.Python bindings, queryable via the Python console.
2. **Statistics**: Implement periodic stat collection in Python scripts, accessible via console commands.
3. **Runtime configuration**: Use the Python console to modify server parameters at runtime.
4. **GM tools**: Add Python console commands for common administrative operations.

### Highest-Value BigWorld Watcher Features

If a watcher-like system were ever desired, the most valuable features to replicate would be:
- **DataWatcher pattern**: Simple registration of C++ variables for inspection (entity counts, tick rates, bandwidth metrics).
- **MemberWatcher pattern**: Exposing object properties via getter/setter methods.
- **Directory tree browsing**: Hierarchical organization for discoverability.
- **Read-write capability**: Tuning server parameters without restart.

The full watcher protocol (UDP/TCP transport, binary encoding, multi-process forwarding) is unnecessary for Cimmeria's single-machine development environment.

---

## Related Documents

- [BigWorld Architecture](bigworld-architecture.md) -- Server component overview
- [Service Architecture](../architecture/service-architecture.md) -- Cimmeria configuration
- [BigWorld Reference Index](../analysis/bigworld-reference-index.md) -- Source file locations
