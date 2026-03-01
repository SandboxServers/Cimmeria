# 02_ue3_exec_annotator.py
# Ghidra Jython script for Stargate Worlds (SGW.exe) reverse engineering
#
# Purpose:
#   Find Unreal Engine 3 native function registration entries. SGW uses UE3
#   for client rendering alongside BigWorld for networking. UE3 native
#   functions are registered with "exec" prefix strings (e.g.,
#   "execGetPlayerViewPoint", "execSetHidden") that appear as string literals
#   in the binary. This script finds those strings, traces their references
#   to the registration code, and renames the associated functions.
#
# UE3 Native Function Registration Pattern:
#   UE3 registers native C++ implementations of UnrealScript functions using
#   a registration macro that emits a string literal "execFunctionName" and
#   a function pointer. The registration function or the exec stub itself
#   references this string.
#
# Expected yield: 500-1000 exec functions identified and renamed
# Confidence: HIGH - exec string pattern is unambiguous
#
# Usage: Run from Ghidra Script Manager (Jython)

"""
UE3 Exec Function Annotator for SGW.exe

Scans for "exec*" string literals that identify Unreal Engine 3 native
function stubs, traces references to find the implementing functions,
and renames them accordingly.
"""

from ghidra.program.model.symbol import SourceType
import re


def get_all_defined_strings():
    """Iterate over all defined data in the program and collect string entries."""
    listing = currentProgram.getListing()
    data_iter = listing.getDefinedData(True)
    strings = []
    while data_iter.hasNext():
        data = data_iter.next()
        dt = data.getDataType()
        if dt is not None and ("string" in dt.getName().lower() or "unicode" in dt.getName().lower()):
            value = data.getValue()
            if value is not None:
                strings.append((data.getAddress(), str(value)))
    return strings


def is_unnamed_function(func):
    """Check if a function has an auto-generated name (not user-defined)."""
    if func is None:
        return False
    name = func.getName()
    source = func.getSymbol().getSource()
    if source == SourceType.USER_DEFINED:
        return False
    if name.startswith("FUN_") or name.startswith("thunk_FUN_"):
        return True
    if source == SourceType.ANALYSIS or source == SourceType.DEFAULT:
        return True
    return False


def is_user_named(func):
    """Check if a function already has a user-defined name."""
    if func is None:
        return False
    return func.getSymbol().getSource() == SourceType.USER_DEFINED


def sanitize_label(name):
    """Sanitize a name to be a valid Ghidra label."""
    sanitized = re.sub(r'[^a-zA-Z0-9_]', '_', name)
    if sanitized and sanitized[0].isdigit():
        sanitized = '_' + sanitized
    return sanitized


def add_plate_comment(func, comment):
    """Add or append to a function's plate comment."""
    existing = func.getComment()
    if existing is None:
        func.setComment(comment)
    elif comment not in existing:
        func.setComment(existing + "\n" + comment)


def run():
    monitor = getMonitor()
    func_mgr = currentProgram.getFunctionManager()

    # Statistics
    stats = {
        'exec_strings_found': 0,
        'exec_functions_renamed': 0,
        'exec_already_named': 0,
        'exec_no_function': 0,
        'int_accessor_strings': 0,
        'int_accessors_renamed': 0,
        'errors': 0,
    }

    println("=" * 70)
    println("UE3 Exec Function Annotator - Stargate Worlds (SGW.exe)")
    println("=" * 70)
    println("")

    # Phase 1: Collect all defined strings
    println("Phase 1: Scanning for exec* and int property accessor strings...")
    monitor.setMessage("UE3 Exec Annotator: Scanning strings...")

    all_strings = get_all_defined_strings()
    exec_strings = []
    int_accessor_strings = []

    for addr, value in all_strings:
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        # Match exec function strings: must start with "exec" followed by
        # an uppercase letter (to avoid false positives like "execute", "exception")
        if value.startswith("exec") and len(value) > 4 and value[4].isupper():
            exec_strings.append((addr, value))
        # UE3 int property accessors: strings like "intFunctionName"
        # These are property getter/setter registration entries
        elif value.startswith("int") and len(value) > 3 and value[3].isupper():
            int_accessor_strings.append((addr, value))

    stats['exec_strings_found'] = len(exec_strings)
    stats['int_accessor_strings'] = len(int_accessor_strings)

    println("  Found %d exec* strings" % stats['exec_strings_found'])
    println("  Found %d int* property accessor strings" % stats['int_accessor_strings'])

    # Phase 2: Process exec strings
    println("")
    println("Phase 2: Tracing exec string references to functions...")
    monitor.setMessage("UE3 Exec Annotator: Processing exec strings...")
    monitor.setMaximum(len(exec_strings))

    # Track renamed functions to avoid duplicates
    renamed_addresses = set()

    for idx, (str_addr, exec_name) in enumerate(exec_strings):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)
        monitor.setMessage("Processing: %s" % exec_name)

        try:
            # Find xrefs TO the exec string
            refs = getReferencesTo(str_addr)
            if not refs:
                stats['exec_no_function'] += 1
                continue

            for ref in refs:
                if monitor.isCancelled():
                    return

                ref_addr = ref.getFromAddress()
                func = getFunctionContaining(ref_addr)

                if func is None:
                    # The reference might be in a data table rather than code.
                    # In UE3, the exec string and function pointer are often stored
                    # adjacently in a registration table. Try reading the next pointer
                    # in the table as a function address.
                    continue

                func_addr = func.getEntryPoint()
                addr_str = str(func_addr)

                if addr_str in renamed_addresses:
                    continue

                if is_user_named(func):
                    stats['exec_already_named'] += 1
                    renamed_addresses.add(addr_str)
                    continue

                if is_unnamed_function(func):
                    safe_name = sanitize_label(exec_name)
                    try:
                        func.setName(safe_name, SourceType.USER_DEFINED)
                        add_plate_comment(func, "UE3 Native: %s" % exec_name)
                        stats['exec_functions_renamed'] += 1
                        renamed_addresses.add(addr_str)
                        println("  Renamed: %s -> %s at %s" % (
                            func.getName(), safe_name, func_addr))
                    except Exception as e:
                        # Name collision - try with address suffix
                        try:
                            alt_name = "%s_%s" % (safe_name, str(func_addr).replace(":", "_"))
                            func.setName(alt_name, SourceType.USER_DEFINED)
                            add_plate_comment(func, "UE3 Native: %s" % exec_name)
                            stats['exec_functions_renamed'] += 1
                            renamed_addresses.add(addr_str)
                            println("  Renamed (alt): %s at %s" % (alt_name, func_addr))
                        except:
                            stats['errors'] += 1

                # Only process the first code reference per exec string
                break

        except Exception as e:
            stats['errors'] += 1
            println("  ERROR processing %s: %s" % (exec_name, str(e)))

    # Phase 3: Process int property accessor strings
    println("")
    println("Phase 3: Tracing int property accessor references...")
    monitor.setMessage("UE3 Exec Annotator: Processing int accessors...")
    monitor.setMaximum(len(int_accessor_strings))

    for idx, (str_addr, accessor_name) in enumerate(int_accessor_strings):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)

        try:
            refs = getReferencesTo(str_addr)
            if not refs:
                continue

            for ref in refs:
                if monitor.isCancelled():
                    return

                ref_addr = ref.getFromAddress()
                func = getFunctionContaining(ref_addr)

                if func is None:
                    continue

                func_addr = func.getEntryPoint()
                addr_str = str(func_addr)

                if addr_str in renamed_addresses:
                    continue

                if is_user_named(func):
                    renamed_addresses.add(addr_str)
                    continue

                if is_unnamed_function(func):
                    safe_name = sanitize_label(accessor_name)
                    try:
                        func.setName(safe_name, SourceType.USER_DEFINED)
                        add_plate_comment(func, "UE3 Int Accessor: %s" % accessor_name)
                        stats['int_accessors_renamed'] += 1
                        renamed_addresses.add(addr_str)
                        println("  Int accessor: %s at %s" % (safe_name, func_addr))
                    except:
                        stats['errors'] += 1

                break

        except Exception as e:
            stats['errors'] += 1

    # Phase 4: Look for registration table patterns
    # UE3 uses static arrays of FNativeFunctionLookup structs:
    #   struct FNativeFunctionLookup { const char* Name; Native Pointer; };
    # We can identify these by looking for consecutive exec string pointers
    println("")
    println("Phase 4: Scanning for UE3 native registration tables...")
    monitor.setMessage("UE3 Exec Annotator: Scanning registration tables...")

    table_functions_found = 0

    for str_addr, exec_name in exec_strings:
        if monitor.isCancelled():
            break

        refs = getReferencesTo(str_addr)
        if not refs:
            continue

        for ref in refs:
            ref_addr = ref.getFromAddress()
            # Check if this reference is from a DATA section (registration table)
            func = getFunctionContaining(ref_addr)
            if func is not None:
                continue  # This is a code reference, already handled

            # This is a data reference - likely a registration table entry.
            # The function pointer should be at ref_addr + 4 (next field in struct)
            try:
                mem = currentProgram.getMemory()
                func_ptr_val = mem.getInt(ref_addr.add(4)) & 0xFFFFFFFF
                func_ptr_addr = toAddr(func_ptr_val)

                target_func = func_mgr.getFunctionAt(func_ptr_addr)
                if target_func is None:
                    # Try to create the function
                    try:
                        target_func = createFunction(func_ptr_addr, None)
                    except:
                        continue

                if target_func is not None:
                    addr_str = str(target_func.getEntryPoint())
                    if addr_str in renamed_addresses:
                        continue

                    if is_user_named(target_func):
                        renamed_addresses.add(addr_str)
                        continue

                    if is_unnamed_function(target_func):
                        safe_name = sanitize_label(exec_name)
                        try:
                            target_func.setName(safe_name, SourceType.USER_DEFINED)
                            add_plate_comment(target_func,
                                "UE3 Native (from registration table): %s" % exec_name)
                            table_functions_found += 1
                            stats['exec_functions_renamed'] += 1
                            renamed_addresses.add(addr_str)
                            println("  Table entry: %s at %s" % (safe_name, func_ptr_addr))
                        except:
                            # Name may already exist
                            try:
                                alt_name = "%s_impl" % safe_name
                                target_func.setName(alt_name, SourceType.USER_DEFINED)
                                add_plate_comment(target_func,
                                    "UE3 Native (from registration table): %s" % exec_name)
                                table_functions_found += 1
                                stats['exec_functions_renamed'] += 1
                                renamed_addresses.add(addr_str)
                            except:
                                stats['errors'] += 1

            except (MemoryAccessException, Exception):
                continue

    # Print summary
    println("")
    println("=" * 70)
    println("UE3 Exec Function Annotator Summary")
    println("=" * 70)
    println("  exec* strings found:              %d" % stats['exec_strings_found'])
    println("  exec functions renamed:            %d" % stats['exec_functions_renamed'])
    println("    (from code references):          %d" % (stats['exec_functions_renamed'] - table_functions_found))
    println("    (from registration tables):      %d" % table_functions_found)
    println("  exec already user-named:           %d" % stats['exec_already_named'])
    println("  exec with no function found:       %d" % stats['exec_no_function'])
    println("  int* accessor strings found:       %d" % stats['int_accessor_strings'])
    println("  int accessors renamed:             %d" % stats['int_accessors_renamed'])
    println("  Errors:                            %d" % stats['errors'])
    println("  Total functions named:             %d" % (stats['exec_functions_renamed'] + stats['int_accessors_renamed']))
    println("=" * 70)


# Entry point
run()
