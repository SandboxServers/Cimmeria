# -*- coding: utf-8 -*-
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
# String Discovery Strategy:
#   Uses Ghidra's defined string data (get_all_defined_strings) for finding
#   the exec*/int* strings. These are strings Ghidra has already typed as
#   string data during auto-analysis. The findBytes API doesn't work reliably
#   in Jython for short ASCII patterns like "intU".
#
# Function Resolution Strategy:
#   1. getReferencesTo() - check if string is referenced from code or data
#   2. Data reference: read adjacent 4-byte function pointer (registration table)
#   3. Pointer scan fallback: search memory for string address as 4-byte LE value
#
# Expected yield: 500-1000 exec functions identified and renamed
# Confidence: HIGH - exec string pattern is unambiguous
#
# Usage: Run from Ghidra Script Manager (Jython)

"""
UE3 Exec Function Annotator for SGW.exe

Scans for "exec*" and "int*" string literals that identify Unreal Engine 3
native function stubs, traces references through registration tables to
find the implementing functions, and renames them accordingly.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.mem import MemoryAccessException
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
                try:
                    strings.append((data.getAddress(), unicode(value).encode('utf-8', 'replace')))
                except:
                    pass
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


def is_valid_code_addr(addr):
    """Check if an address points to executable memory."""
    if addr is None:
        return False
    block = currentProgram.getMemory().getBlock(addr)
    if block is None:
        return False
    return block.isExecute() or block.getName() == '.text'


def try_rename_function(func, name, comment, stats, renamed_addresses, stat_key):
    """Try to rename a function, handling conflicts. Returns True on success."""
    if func is None:
        return False
    addr_str = str(func.getEntryPoint())
    if addr_str in renamed_addresses:
        return False
    if is_user_named(func):
        renamed_addresses.add(addr_str)
        return False
    if not is_unnamed_function(func):
        # Has some non-default name but not user-defined — add comment only
        add_plate_comment(func, comment)
        renamed_addresses.add(addr_str)
        return False

    safe_name = sanitize_label(name)
    try:
        func.setName(safe_name, SourceType.USER_DEFINED)
        add_plate_comment(func, comment)
        stats[stat_key] += 1
        renamed_addresses.add(addr_str)
        return True
    except:
        # Name collision — try with _impl suffix
        try:
            func.setName(safe_name + "_impl", SourceType.USER_DEFINED)
            add_plate_comment(func, comment)
            stats[stat_key] += 1
            renamed_addresses.add(addr_str)
            return True
        except:
            # Try with address suffix
            try:
                alt_name = "%s_%s" % (safe_name, addr_str.replace(":", "_"))
                func.setName(alt_name, SourceType.USER_DEFINED)
                add_plate_comment(func, comment)
                stats[stat_key] += 1
                renamed_addresses.add(addr_str)
                return True
            except:
                stats['errors'] += 1
                return False


def resolve_function_from_refs(str_addr, parsed_name, comment, stats, renamed_addresses, stat_key, memory, func_mgr):
    """Try to resolve a function from xrefs to a string address.

    Handles both code references (function directly references the string)
    and data references (registration table with [string_ptr][func_ptr] layout).
    Returns True if a function was successfully renamed.
    """
    refs = getReferencesTo(str_addr)
    if not refs:
        return False

    for ref in refs:
        ref_addr = ref.getFromAddress()
        func = getFunctionContaining(ref_addr)

        if func is not None:
            # Code reference — the function directly references this string
            result = try_rename_function(func, parsed_name, comment, stats, renamed_addresses, stat_key)
            if result:
                println("  Code ref: %s at %s" % (sanitize_label(parsed_name), func.getEntryPoint()))
            return True  # Handled (even if not renamed due to existing name)

        else:
            # Data reference — likely a registration table entry
            # UE3 registration layout: [string_ptr(4)] [func_ptr(4)]
            # But the ref_addr points to where the string pointer is stored.
            # The function pointer should be at ref_addr + 4.
            try:
                func_ptr_val = memory.getInt(ref_addr.add(4)) & 0xFFFFFFFF
                func_ptr_addr = toAddr(func_ptr_val)

                if not is_valid_code_addr(func_ptr_addr):
                    continue

                target_func = func_mgr.getFunctionAt(func_ptr_addr)
                if target_func is None:
                    try:
                        target_func = createFunction(func_ptr_addr, None)
                    except:
                        pass

                if target_func is not None:
                    result = try_rename_function(target_func, parsed_name,
                        comment + " (from registration table)", stats, renamed_addresses, stat_key)
                    if result:
                        println("  Table: %s at %s" % (sanitize_label(parsed_name), func_ptr_addr))
                    return True
            except:
                continue

    return False


def resolve_function_by_pointer_scan(str_addr, parsed_name, comment, stats, renamed_addresses, stat_key, memory, func_mgr):
    """Fallback: search memory for the string address as a 4-byte LE pointer,
    then read the adjacent function pointer from the registration table.
    Returns True if a function was successfully renamed.
    """
    search_start = currentProgram.getMinAddress()
    search_end = currentProgram.getMaxAddress()

    str_addr_int = str_addr.getOffset()
    # Build the 4-byte LE representation of the string address
    search_bytes = bytearray([
        str_addr_int & 0xFF,
        (str_addr_int >> 8) & 0xFF,
        (str_addr_int >> 16) & 0xFF,
        (str_addr_int >> 24) & 0xFF
    ])

    try:
        ptr_addr = memory.findBytes(search_start, search_end, bytes(search_bytes), None, True, getMonitor())
    except:
        return False

    if ptr_addr is None:
        return False

    try:
        func_ptr_val = memory.getInt(ptr_addr.add(4)) & 0xFFFFFFFF
        func_ptr_addr = toAddr(func_ptr_val)

        if not is_valid_code_addr(func_ptr_addr):
            return False

        target_func = func_mgr.getFunctionAt(func_ptr_addr)
        if target_func is None:
            try:
                target_func = createFunction(func_ptr_addr, None)
            except:
                return False

        if target_func is not None:
            result = try_rename_function(target_func, parsed_name,
                comment + " (from pointer scan)", stats, renamed_addresses, stat_key)
            if result:
                println("  PtrScan: %s at %s" % (sanitize_label(parsed_name), func_ptr_addr))
            return result
    except:
        pass

    return False


def run():
    monitor = getMonitor()
    func_mgr = currentProgram.getFunctionManager()
    memory = currentProgram.getMemory()

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

    # Phase 1: Collect strings using Ghidra's defined string data
    # findBytes doesn't work reliably in Jython for short patterns like "intU",
    # but get_all_defined_strings() successfully finds 1,275+ int* strings.
    println("Phase 1: Scanning defined strings for UE3 patterns...")
    monitor.setMessage("UE3 Exec Annotator: Scanning defined strings...")

    all_strings = get_all_defined_strings()
    println("  Total defined strings: %d" % len(all_strings))

    exec_strings = []          # standalone "exec*" strings
    int_accessor_strings = []  # "int*" registration strings (contain exec or property accessors)

    for addr, value in all_strings:
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        # Standalone exec strings: "execMethodName"
        if value.startswith("exec") and len(value) > 4 and len(value) < 256:
            # Must have an uppercase letter after "exec" to be a UE3 exec stub
            if value[4:5].isupper():
                exec_strings.append((addr, value))

        # Int property accessor strings: "intClassNameexecMethodName" or "intClassName..."
        # These are UE3 native function registration entries
        elif value.startswith("int") and len(value) > 6 and len(value) < 256:
            # The character after "int" should be uppercase (class name start)
            if value[3:4].isupper():
                # Filter out unlikely matches - must be alphanumeric/underscore only
                if re.match(r'^int[A-Z][a-zA-Z0-9_]+$', value):
                    int_accessor_strings.append((addr, value))

    stats['exec_strings_found'] = len(exec_strings)
    stats['int_accessor_strings'] = len(int_accessor_strings)

    println("  Found %d exec* strings" % stats['exec_strings_found'])
    println("  Found %d int* property accessor strings" % stats['int_accessor_strings'])

    if exec_strings:
        println("")
        println("  Sample exec strings:")
        for _, name in exec_strings[:10]:
            println("    %s" % name)
        if len(exec_strings) > 10:
            println("    ... and %d more" % (len(exec_strings) - 10))

    if int_accessor_strings:
        println("")
        println("  Sample int* strings:")
        for _, name in int_accessor_strings[:10]:
            println("    %s" % name)
        if len(int_accessor_strings) > 10:
            println("    ... and %d more" % (len(int_accessor_strings) - 10))

    # Track renamed functions to avoid duplicates
    renamed_addresses = set()

    # Phase 2: Process standalone exec strings
    println("")
    println("Phase 2: Tracing exec string references to functions...")
    monitor.setMessage("UE3 Exec Annotator: Processing exec strings...")
    monitor.setMaximum(len(exec_strings))

    for idx, (str_addr, exec_name) in enumerate(exec_strings):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)

        try:
            comment = "UE3 Native: %s" % exec_name
            found = resolve_function_from_refs(str_addr, exec_name, comment,
                stats, renamed_addresses, 'exec_functions_renamed', memory, func_mgr)

            if not found:
                # Fallback: pointer scan
                found = resolve_function_by_pointer_scan(str_addr, exec_name, comment,
                    stats, renamed_addresses, 'exec_functions_renamed', memory, func_mgr)

            if not found:
                stats['exec_no_function'] += 1

        except Exception as e:
            stats['errors'] += 1
            println("  ERROR processing %s: %s" % (exec_name, str(e)))

    # Phase 3: Process int property accessor strings
    # Format: "intClassNameexecMethodName" or "intClassNamePropertyName"
    println("")
    println("Phase 3: Tracing int accessor strings via references and registration tables...")
    monitor.setMessage("UE3 Exec Annotator: Processing int accessors...")
    monitor.setMaximum(len(int_accessor_strings))

    for idx, (str_addr, accessor_name) in enumerate(int_accessor_strings):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)

        # Parse "intClassexecMethod" -> "Class_execMethod"
        # or "intClassName" -> keep as-is for labeling
        parsed_name = accessor_name
        if accessor_name.startswith("int"):
            inner = accessor_name[3:]  # Remove "int" prefix
            # Try to find "exec" within the name
            exec_idx = inner.find("exec")
            if exec_idx > 0:
                class_name = inner[:exec_idx]
                method_name = inner[exec_idx:]
                parsed_name = "%s_%s" % (class_name, method_name)
            else:
                parsed_name = inner

        try:
            comment = "UE3: %s" % accessor_name

            # Method 1: xref-based resolution
            found = resolve_function_from_refs(str_addr, parsed_name, comment,
                stats, renamed_addresses, 'int_accessors_renamed', memory, func_mgr)

            if not found:
                # Method 2: pointer scan fallback
                found = resolve_function_by_pointer_scan(str_addr, parsed_name, comment,
                    stats, renamed_addresses, 'int_accessors_renamed', memory, func_mgr)

        except Exception as e:
            stats['errors'] += 1

    # Print summary
    println("")
    println("=" * 70)
    println("UE3 Exec Function Annotator Summary")
    println("=" * 70)
    println("  exec* strings found:              %d" % stats['exec_strings_found'])
    println("  exec functions renamed:            %d" % stats['exec_functions_renamed'])
    println("  exec already user-named:           %d" % stats['exec_already_named'])
    println("  exec with no function found:       %d" % stats['exec_no_function'])
    println("  int* accessor strings found:       %d" % stats['int_accessor_strings'])
    println("  int accessors renamed:             %d" % stats['int_accessors_renamed'])
    println("  Errors:                            %d" % stats['errors'])
    println("  Total functions named:             %d" % (stats['exec_functions_renamed'] + stats['int_accessors_renamed']))
    println("=" * 70)


# Entry point
run()
