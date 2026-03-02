# Reading Ghidra Decompiler Output of SGW.exe

This guide covers practical techniques for reading and interpreting Ghidra's decompiler output when analyzing SGW.exe, the Stargate Worlds client binary. SGW.exe is a 32-bit Windows executable built with MSVC (Visual Studio), linked against the BigWorld Engine and Unreal Engine 3 (UE3). Understanding MSVC's code generation patterns and BigWorld's architectural conventions is essential for extracting useful information from the decompilation.

---

## Binary Basics

- **Architecture:** x86 (32-bit), PE executable
- **Compiler:** MSVC (Visual Studio, likely VS2005 or VS2008 era)
- **Runtime:** Microsoft CRT (dynamic linking via msvcr80/msvcr90)
- **Key libraries:** BigWorld Engine (statically linked), Unreal Engine 3 (statically linked), Python 2.x (embedded), DirectX 9

The binary is not stripped -- it contains RTTI type information strings, debug assertion paths, and numerous literal strings that serve as excellent landmarks for navigation.

---

## MSVC Calling Conventions

Understanding calling conventions is critical because Ghidra sometimes misidentifies them, leading to incorrect parameter lists in decompiled output.

### __thiscall (Most C++ Methods)

The vast majority of functions in SGW.exe are C++ member functions using `__thiscall`:

- The `this` pointer is passed in the **ECX** register.
- All other arguments are pushed onto the stack, right-to-left.
- The callee cleans the stack.
- Return values in EAX (integers/pointers) or ST(0) (floats).

**In Ghidra's decompiler output**, `this` appears as the first parameter of the function. Ghidra typically names it `this` or `param_1`, and its type will be a pointer to the class (or `void *` if the class hasn't been defined).

```c
// Ghidra output for a typical __thiscall method
void __thiscall Entity::setPosition(Entity *this, float x, float y, float z)
{
    this->position.x = x;
    this->position.y = y;
    this->position.z = z;
}
```

**Common Ghidra artifact:** Ghidra sometimes fails to detect `__thiscall` and instead treats the function as `__cdecl`, causing `this` to appear as a stack parameter rather than a register parameter. If a function's first parameter is being used as an object pointer (dereferenced, used to access fields at fixed offsets), it is almost certainly `__thiscall` and the first parameter is `this`. You can fix this in Ghidra by editing the function signature and setting the calling convention to `__thiscall`.

### __cdecl (Free Functions and C-style APIs)

Free functions and C-compatible APIs use `__cdecl`:

- All arguments pushed on the stack, right-to-left.
- The **caller** cleans the stack.
- Common in: Python C API calls (`Py_*`, `PyObject_*`), CRT functions, global utility functions.

### __fastcall (Occasional)

Some performance-critical functions use `__fastcall`:

- First two integer/pointer arguments in **ECX** and **EDX**.
- Remaining arguments on the stack.
- The callee cleans the stack.

This is less common in SGW.exe but appears occasionally in inner loops and math-heavy code.

### __stdcall (Win32 API and COM)

Windows API calls and COM interface methods use `__stdcall`:

- Arguments on the stack, right-to-left.
- The callee cleans the stack.
- Ghidra generally handles these correctly because the Win32 API is well-typed.

---

## MSVC std::string Layout

MSVC's `std::string` (and `std::wstring`) implementation uses a **Small String Optimization (SSO)** layout that appears frequently in decompiled code. Understanding this layout lets you identify string operations even when Ghidra does not recognize the type.

### Memory Layout (MSVC std::string, 32-bit)

```
Offset  Size   Field          Description
0x00    16     _Buf/_Ptr      If length < 16: inline char buffer (_Buf)
                               If length >= 16: pointer to heap buffer (_Ptr)
0x10    4      _Mysize        Current string length (not including null terminator)
0x14    4      _Myres         Allocated capacity (>= _Mysize)
```

**Total size: 28 bytes** (0x1C)

The key insight is the **16-byte inline buffer**. Strings shorter than 16 characters are stored directly in the object (no heap allocation). Strings of 16 characters or longer cause a heap allocation, and the first 4 bytes of the inline buffer area are repurposed as a pointer to the heap buffer.

### Recognizing std::string in Decompiled Code

Look for these patterns:

**Construction (small string):**
```c
// Ghidra shows something like:
*(int *)(local_var + 0x10) = 0;    // _Mysize = 0
*(int *)(local_var + 0x14) = 0xF;  // _Myres = 15 (SSO capacity)
*(char *)local_var = '\0';         // null-terminate inline buffer
```

**Construction (from literal):**
```c
// Calls to a function like basic_string::assign or basic_string::basic_string
// with a string literal and a length
FUN_XXXXXXXX(local_var, "some string", 0xB);  // 0xB = 11 = strlen("some string")
```

**Size check before access:**
```c
// Deciding whether to use inline buffer or heap pointer
if (*(int *)(str + 0x14) >= 0x10) {
    ptr = *(char **)str;      // heap path: dereference pointer
} else {
    ptr = (char *)str;        // inline path: use buffer directly
}
```

When you see this conditional pattern, you are looking at a string data access. This is the SSO branch: if the reserved capacity (`_Myres` at offset 0x14) is 16 or more, the string has been heap-allocated, so the first 4 bytes are a pointer. Otherwise, the data is inline.

### MSVC std::wstring

Same layout, but the inline buffer holds 8 `wchar_t` values (16 bytes / 2 bytes per `wchar_t`), so the SSO threshold is 8 characters instead of 16. `_Mysize` and `_Myres` count `wchar_t` code units, not bytes.

---

## Virtual Function Call Patterns

C++ virtual method calls are the most common call pattern in SGW.exe. Recognizing them is essential for tracing execution flow.

### Basic Virtual Call

```c
// Ghidra decompiled output:
(*(this->vtable))[index](this, arg1, arg2);

// Or more commonly:
(**(code **)(*(int *)this + offset))(this, arg1, arg2);
```

Breaking this down:
1. `*(int *)this` -- Load the vtable pointer from the first 4 bytes of the object.
2. `+ offset` -- Index into the vtable (offset = slot_index * 4 for 32-bit).
3. `*(code **)( ... )` -- Load the function pointer from the vtable slot.
4. `( ... )(this, arg1, arg2)` -- Call it, passing `this` as the first argument.

**Example:**
```c
// vtable slot 3, so offset = 0x0C
(**(code **)(*(int *)this + 0xC))(this, param_2, param_3);
```

This calls the 4th virtual function (slot index 3, zero-based) on the object pointed to by `this`.

### Identifying Which Method Is Called

To determine which virtual method is being called:

1. Find the object's class. Check RTTI (see below) or look at how the object was constructed.
2. Navigate to the vtable address for that class.
3. Look at the function pointer at the computed offset within the vtable.
4. That function pointer is the actual method being called.

### Multiple Inheritance and Vtable Thunks

SGW.exe uses multiple inheritance extensively (BigWorld entities implement many interfaces). This means objects can have multiple vtable pointers at different offsets within the object:

```c
// Calling a method through a secondary vtable (interface)
// The 'this' pointer is adjusted before the call
int adjusted_this = (int)this + 0x20;  // offset to interface vtable
(**(code **)(*(int *)adjusted_this + 0x08))(adjusted_this, arg1);
```

The `this` pointer adjustment is a sign of calling through an inherited interface. Ghidra may show these as separate objects, but they are actually different views of the same entity.

---

## RTTI (Run-Time Type Information)

MSVC RTTI is one of the most valuable tools for navigating SGW.exe. It provides class names, inheritance hierarchies, and vtable associations.

### RTTI String Format

MSVC RTTI type descriptors contain mangled class names in the format:

```
.?AVClassName@@
.?AVNamespace@@ClassName@@
```

For example:
```
.?AVEntity@BW@@           -- BW::Entity
.?AVChannel@Mercury@@     -- Mercury::Channel
.?AVEventSignal@CME@@     -- CME::EventSignal
.?AVSGWPlayer@@           -- SGWPlayer (no namespace)
```

### Finding Classes Via RTTI

1. **Search for RTTI strings.** In Ghidra, use Search > Program Text or the Defined Strings window to search for `.?AV`. This yields hundreds of class names.
2. **Follow RTTI to vtable.** Each RTTI `type_info` structure is referenced by a `Complete Object Locator`, which is placed immediately before the vtable in memory. So: `type_info` string -> `type_info` struct -> `Complete Object Locator` -> vtable.
3. **Follow vtable to methods.** The vtable is an array of function pointers. Each entry is a virtual method of the class, in declaration order.
4. **Follow vtable xrefs to constructors.** Constructors write the vtable pointer into the object. Search for xrefs to the vtable address to find constructors and factory functions.

### RTTI Hierarchy

The `Complete Object Locator` also contains a pointer to a `Class Hierarchy Descriptor`, which lists all base classes. This lets you reconstruct inheritance trees. Ghidra's "RTTI Analyzer" (run via Analysis > Auto Analyze with RTTI enabled) can reconstruct these automatically.

---

## BigWorld-Specific Patterns

SGW.exe incorporates the BigWorld Engine. These patterns appear throughout the binary.

### EntityManager Singleton Access

BigWorld uses a global `EntityManager` singleton. Access looks like:

```c
// Common pattern: get the singleton, then call a method on it
EntityManager *mgr = EntityManager::instance();
Entity *entity = mgr->findEntity(entityId);
```

In decompiled code:
```c
// Loading the singleton from a global pointer
puVar = *(EntityManager **)DAT_XXXXXXXX;
// Calling findEntity through the vtable
entity = (**(code **)(*(int *)puVar + 0x1C))(puVar, entityId);
```

Look for a global pointer that is dereferenced, used as `this`, and then has virtual methods called on it. If the same global appears in many entity-related functions, it is likely `EntityManager::instance()` or a similar singleton.

### Mercury::Channel Operations

Mercury is BigWorld's networking layer. Channel operations appear when the client sends or receives messages.

```c
// Typical Mercury send pattern:
// 1. Get the channel (usually via ServerConnection)
// 2. Start a message bundle
// 3. Write data to the bundle
// 4. Send the bundle

Mercury::Bundle *bundle = channel->startMessage(messageId);
bundle->writeInt32(someValue);
bundle->writeString(someString);
channel->send(bundle);
```

In decompiled code, look for sequences of writes to the same object (the bundle) followed by a send call. The `messageId` passed to `startMessage` correlates with the Event_NetOut/Event_NetIn indices.

### CME EventSignal Dispatch

The CME (Cheyenne Mountain Entertainment) framework extends BigWorld with an event signal system. Event dispatch looks like:

```c
// EventSignal::fire() pattern
// Signal objects are typically stored in static/global arrays indexed by event ID
EventSignal *signal = g_eventSignals[eventIndex];
signal->fire(param1, param2, ...);
```

The `Event_NetOut_*` and `Event_NetIn_*` strings are the names of these signals. Finding the string reference leads you to the signal registration, which leads to the handler function.

### Property Getter/Setter Patterns

Entity properties defined in .def files have generated getter and setter code. The pattern involves flag checks and dirty-marking:

```c
// Property setter with flag check
void Entity::setPropertyXXX(Entity *this, int newValue)
{
    if (this->propertyXXX != newValue) {
        this->propertyXXX = newValue;
        this->propertyFlags |= DIRTY_FLAG;
        // May also trigger a client update depending on the property's Flags
        if (this->hasWitnesses) {
            this->sendPropertyUpdate(PROPERTY_INDEX, &newValue, sizeof(int));
        }
    }
}
```

Properties with `CELL_PUBLIC` or `ALL_CLIENTS` flags trigger network updates to witnesses. Properties with `CELL_PRIVATE` do not.

### Entity Method Dispatch

When the server receives an exposed method call from the client, it dispatches through a method table:

```c
// Method dispatch pattern
MethodDescription *desc = entityType->getMethodDescription(methodIndex);
if (desc->isExposed()) {
    desc->callMethod(entity, argStream);
}
```

The `methodIndex` is the ordinal position of the method in the entity's .def file (across all method sections). This index is critical for the wire protocol -- the client sends this index to identify which method to call.

---

## Common Ghidra Decompiler Artifacts

The Ghidra decompiler is powerful but imperfect. These are common artifacts you will encounter in SGW.exe output and how to interpret them.

### Incorrect Parameter Counts on __thiscall

**Problem:** Ghidra may show too few or too many parameters, especially when it misidentifies the calling convention.

**Symptoms:**
- A function uses ECX as an object pointer but it does not appear in the parameter list.
- Extra parameters appear that are never used in the function body.

**Fix:** Right-click the function signature, select "Edit Function Signature", and set the calling convention to `__thiscall`. Ghidra will then correctly show `this` as the first parameter passed via ECX.

### Switch Statement Recovery Issues

**Problem:** Ghidra sometimes fails to fully recover switch/case statements, showing them as chains of if/else comparisons or even completely unstructured goto-based code.

**Symptoms:**
- A function with a message ID parameter shows a series of `if (param == 0x1) ... else if (param == 0x2) ...` instead of a clean switch.
- Jump tables are not recognized, resulting in "switchD" labels and computed gotos.
- Cases may be missing entirely if the jump table was not detected.

**Workaround:** Check the disassembly view for jump table patterns (a `jmp dword ptr [reg*4 + table_addr]` instruction). If you find one, you can manually define the jump table in Ghidra (right-click the instruction > References > Add Jump Table) to help the decompiler recover the switch.

### Enum Values Shown as Raw Integers

**Problem:** Ghidra shows raw integer constants where the original code used named enumerations.

**Symptoms:**
- A comparison like `if (param_2 == 3)` where 3 is actually `Atrea.enums.DT_Physical` or `EMobAggressionLevel.Aggressive`.
- Flag checks like `if (flags & 0x100)` where 0x100 is `ForceStanding` from `EAbilityFlags`.

**Workaround:** Cross-reference the integer values against `enumerations.xml` and the `enumerations.py` Python module. You can define enum types in Ghidra's Data Type Manager and apply them to function parameters and local variables to make the decompilation readable.

Common enumerations to define in Ghidra:
- `EAbilityFlags` (UINT64 bitfield)
- `EDamageType` (UINT8: 0=Untyped, 1=Energy, 2=Hazmat, 3=Physical, 4=Psionic)
- `EGender` (UINT8: 1=Male, 2=Female, 3=Other)
- `ETargetType` (UINT8: 0=None, 1=Self, 2=Target, 3=Ground)
- `EMobAggressionLevel` (INT8)
- `EWeaponState` (various)

### Inlined Constructors and Destructors

**Problem:** MSVC aggressively inlines small constructors and destructors. Ghidra shows the inlined code as part of the calling function rather than as a separate call.

**Symptoms:**
- A function begins with a long sequence of zeroing out fields and writing vtable pointers. This is an inlined constructor.
- A function ends with calls to `operator delete` or heap free functions preceded by vtable writes. This is an inlined destructor.
- std::string construction (setting `_Mysize=0`, `_Myres=0xF`, null-terminating the buffer) appears inline within unrelated functions.

**Interpretation:** Recognize these patterns and mentally group them as "object X is being constructed here" or "object X is being destroyed here." The vtable pointer written during construction tells you which class is being constructed (look up the vtable address in the RTTI data).

### False Conditionals and Dead Code

**Problem:** Compiler optimizations sometimes produce conditional branches that Ghidra decompiles into conditions that appear nonsensical.

**Symptoms:**
- `if (true)` or `if (false)` branches.
- Conditions comparing a variable to itself.
- Unreachable code blocks.

**Interpretation:** These are usually artifacts of inlining, template instantiation, or conditional compilation (e.g., debug code compiled out). They can be safely ignored.

### Incorrect Struct Field Access

**Problem:** Without proper type definitions, Ghidra shows field accesses as raw pointer arithmetic.

**Symptoms:**
```c
*(int *)(param_1 + 0x4C) = *(int *)(param_1 + 0x4C) + 1;
```

**Interpretation:** This is `this->someField++` where `someField` is at offset 0x4C within the object. If you define the struct type in Ghidra's Data Type Manager with the correct field layout, the decompiler will show named fields instead of raw offsets.

---

## Navigation Strategies

### Starting from RTTI

1. Search for `.?AV` in the Defined Strings window.
2. Find the class you are interested in (e.g., `.?AVSGWAbilityManager@@`).
3. Follow the xref from the string to the `type_info` structure.
4. Follow the xref from `type_info` to the `Complete Object Locator`.
5. The vtable starts immediately after the `Complete Object Locator`.
6. Follow xrefs TO the vtable address to find constructors (they write the vtable pointer).
7. Each entry in the vtable is a virtual method you can decompile.

### Starting from String References

1. Search for known strings: `"Event_NetOut_UseAbility"`, `"EntityManager"`, `"Mercury::Channel"`, etc.
2. Follow the xref to the function that references the string.
3. The function is either the handler itself or a registration function that associates the string with a handler.
4. For Event_NetOut/Event_NetIn strings, the function referencing the string typically passes it to an EventSignal registration call, and the function pointer passed alongside it is the actual handler.

### Starting from Known Functions

If you have already identified a function (e.g., from an annotation script or a previous finding), use xrefs to explore outward:

- **Xrefs TO this function:** Who calls this function? This tells you the calling context.
- **Xrefs FROM this function:** What does this function call? This tells you the downstream behavior.
- **Vtable membership:** If this function is in a vtable, what class does the vtable belong to? What other methods are in the same vtable?

### Starting from .def Files

1. Pick a method from a .def file (e.g., `SGWCombatant.def:onAttacked`).
2. The method name often appears as a string in the binary (BigWorld uses method names for dispatch in debug builds, and SGW.exe has debug artifacts).
3. Search for the string `"onAttacked"` in Ghidra.
4. Follow the xref to find the method registration or dispatch code.
5. The actual method implementation is either the function itself or reached through one level of indirection.

### Using BigWorld Source Paths

SGW.exe contains assertion strings with BigWorld source file paths like:

```
..\..\Server\bigworld\src\lib\connection\server_connection.cpp
..\..\Server\bigworld\src\lib\entitydef\entity_description.cpp
```

These paths tell you exactly which BigWorld source file contains the original code. Search for these paths in the Defined Strings, follow their xrefs, and you will land in the corresponding compiled function. You can then cross-reference with the BigWorld reference source at the same file path.

---

## Practical Example: Tracing a Client Method Call

Here is a walkthrough of tracing how the client serializes and sends a `setCrouched` call, from .def file to decompiled code.

**Step 1: Read the .def file.**

From `SGWCombatant.def`:
```xml
<setCrouched>
    <Exposed/>
    <Arg>  INT8  <ArgName>enabled</ArgName>  </Arg>
</setCrouched>
```

This is an `<Exposed/>` cell method with one INT8 argument. The client is allowed to call this method directly.

**Step 2: Search for the string in Ghidra.**

Search for `"setCrouched"` in the Defined Strings window. You may find it referenced in method registration code, or as part of an `Event_NetOut_SetCrouched` signal name.

**Step 3: Follow the xref.**

The string xref leads to a function that registers the method. Near the registration, you will find either:
- A function pointer to the serialization handler (the function that writes the INT8 to the Mercury bundle).
- An index number that maps to the method's position in the entity's method table.

**Step 4: Decompile the handler.**

The handler function will look something like:

```c
void __thiscall SGWCombatant_setCrouched_handler(void *this, int8_t enabled)
{
    Mercury::Bundle *bundle = this->serverConnection->startMessage(METHOD_CALL_MSG_ID);
    bundle->writeUint8(entityType->getExposedMethodIndex("setCrouched"));
    bundle->writeInt8(enabled);
    this->serverConnection->send(bundle);
}
```

**Step 5: Validate.**

The decompiled handler should serialize exactly one INT8, matching the .def file declaration. If it serializes additional fields not in the .def, those are likely internal BigWorld framing (entity ID, method index) rather than game-specific arguments.

---

## Tips and Reminders

- **When in doubt, check the disassembly.** The decompiler output is an interpretation. The disassembly is ground truth. If the decompiled code looks wrong, switch to the Listing view and read the assembly directly.

- **BigWorld method indices are not stable across builds.** The method index used on the wire depends on the order methods appear in the .def file and how the entity type was compiled. Always verify indices against the specific version of the .def files that SGW.exe was built with (the client's copy, not necessarily Cimmeria's).

- **RTTI does not mean the class is instantiated.** Some RTTI entries are for classes that were compiled in but never actually used at runtime. Do not assume a class with RTTI is necessarily important.

- **SGW.exe is a hybrid.** It uses both BigWorld (server connectivity, entity system) and UE3 (rendering, input, audio). Functions from the UE3 side have different patterns -- UObject, UFunction, FName. These are less relevant for server emulation but may appear in call chains.

- **Python dispatch.** Many methods ultimately dispatch to embedded Python. Look for calls to `PyObject_CallMethod`, `PyObject_CallFunction`, or BigWorld's Python dispatch wrappers. The string arguments to these calls reveal the Python method name being invoked, which maps directly to scripts in `python/cell/` or `python/base/`.

- **Save your work.** When you identify a function, rename it in Ghidra immediately. When you identify a type, define it in the Data Type Manager. Accumulated annotations make every future analysis easier.
