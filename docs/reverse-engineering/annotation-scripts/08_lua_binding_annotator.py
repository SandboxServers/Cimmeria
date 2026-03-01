# 08_lua_binding_annotator.py
# Ghidra Jython script for SGW.exe reverse engineering
#
# Purpose: Find toLua/Lua binding registration tables. SGW may use Lua for UI
# scripting (common in UE3-era games and BigWorld-based games). Lua bindings
# register C++ functions with string names in registration tables that consist
# of {string_ptr, function_ptr} pairs.
#
# Expected yield: 100-200 functions named (MED-HIGH confidence)
#
# Run order: After scripts 01-07. Can run independently.
#
# @category SGW
# @author Cimmeria Project

"""
Lua Binding Annotator

Searches for Lua/toLua++ binding registration tables in SGW.exe:
  1. Detects Lua presence by searching for lua-related strings
  2. Finds registration patterns: lua_register, lua_pushcfunction, tolua_function
  3. Identifies registration tables: arrays of {string_ptr, function_ptr} pairs
  4. Names C++ wrapper functions based on their Lua binding names

SGW might use toLua++, a custom binding system, or no Lua at all. The script
searches broadly and exits cleanly if no Lua bindings are found.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.mem import MemoryAccessException
from ghidra.program.util import DefinedDataIterator

import re


def is_default_name(func):
    """Check if a function has a default auto-generated name."""
    name = func.getName()
    return (name.startswith("FUN_") or
            name.startswith("thunk_FUN_") or
            name.startswith("Ordinal_"))


def sanitize_name(raw):
    """Clean a raw string into a valid Ghidra symbol name."""
    name = raw.replace("::", "_").replace(".", "_").replace("-", "_")
    name = re.sub(r'[^A-Za-z0-9_]', '', name)
    name = re.sub(r'_+', '_', name)
    name = name.strip('_')
    if len(name) > 120:
        name = name[:120]
    if name and name[0].isdigit():
        name = "lua_" + name
    return name


def read_dword(mem, addr):
    """Read a 4-byte little-endian value at the given address."""
    try:
        bytes_val = bytearray(4)
        count = mem.getBytes(addr, bytes_val)
        if count != 4:
            return None
        val = (bytes_val[0] & 0xFF) | \
              ((bytes_val[1] & 0xFF) << 8) | \
              ((bytes_val[2] & 0xFF) << 16) | \
              ((bytes_val[3] & 0xFF) << 24)
        return val
    except MemoryAccessException:
        return None
    except:
        return None


def read_cstring(mem, addr, max_len=256):
    """Read a null-terminated C string at the given address."""
    try:
        result = []
        for i in range(max_len):
            b = bytearray(1)
            count = mem.getBytes(addr.add(i), b)
            if count != 1:
                return None
            ch = b[0] & 0xFF
            if ch == 0:
                break
            if ch < 0x20 or ch > 0x7E:
                return None  # Not printable ASCII -- not a string
            result.append(chr(ch))
        if not result:
            return None
        return "".join(result)
    except:
        return None


def is_plausible_code_addr(val, mem, addr_space):
    """Check if a 32-bit value looks like it could be a code address."""
    if val == 0 or val < 0x00400000 or val > 0x7FFFFFFF:
        return False
    try:
        addr = addr_space.getAddress(val & 0xFFFFFFFF)
        block = mem.getBlock(addr)
        if block is None:
            return False
        return block.isExecute()
    except:
        return False


def is_plausible_data_addr(val, mem, addr_space):
    """Check if a 32-bit value looks like it could be a data address (for strings)."""
    if val == 0 or val < 0x00400000 or val > 0x7FFFFFFF:
        return False
    try:
        addr = addr_space.getAddress(val & 0xFFFFFFFF)
        block = mem.getBlock(addr)
        return block is not None
    except:
        return False


def main():
    monitor = getMonitor()
    fm = currentProgram.getFunctionManager()
    st = currentProgram.getSymbolTable()
    listing = currentProgram.getListing()
    mem = currentProgram.getMemory()
    addr_factory = currentProgram.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()

    println("=" * 70)
    println("Lua Binding Annotator - Script 08")
    println("=" * 70)
    println("")

    # Statistics
    lua_strings_found = 0
    registration_functions_found = 0
    binding_tables_found = 0
    bindings_extracted = 0
    functions_renamed = 0
    functions_commented = 0
    name_usage_counts = {}

    # ---------------------------------------------------------------
    # STEP 1: Detect Lua presence
    # ---------------------------------------------------------------
    println("[Step 1] Detecting Lua presence in binary...")
    monitor.setMessage("Step 1: Searching for Lua strings...")

    lua_indicator_strings = []
    lua_api_strings = []
    tolua_strings = []
    all_lua_strings = []

    # Lua API function names and error messages we might find
    lua_api_patterns = [
        "lua_pushstring", "lua_pushnumber", "lua_pushboolean",
        "lua_pushcfunction", "lua_pushcclosure", "lua_pushnil",
        "lua_register", "lua_setfield", "lua_getfield",
        "lua_newstate", "lua_close", "lua_pcall", "lua_call",
        "lua_gettop", "lua_settop", "lua_type", "lua_typename",
        "lua_tostring", "lua_tonumber", "lua_toboolean",
        "lua_createtable", "lua_newtable", "lua_rawset",
        "luaL_register", "luaL_openlibs", "luaL_newstate",
        "luaL_loadstring", "luaL_loadfile", "luaL_dofile",
        "luaL_ref", "luaL_unref", "luaL_checktype",
        "luaopen_", "luaL_newlib",
    ]

    tolua_api_patterns = [
        "tolua_function", "tolua_constant", "tolua_variable",
        "tolua_cclass", "tolua_module", "tolua_open", "tolua_beginmodule",
        "tolua_endmodule", "tolua_usertype", "tolua_isnoobj",
        "tolua_isstring", "tolua_isnumber", "tolua_isboolean",
        "tolua_tostring", "tolua_tonumber", "tolua_toboolean",
        "tolua_tousertype", "tolua_pushusertype",
        "tolua_error", "tolua_lerror",
    ]

    data_iter = DefinedDataIterator.definedStrings(currentProgram)
    count = 0
    while data_iter.hasNext():
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        data = data_iter.next()
        count += 1
        if count % 5000 == 0:
            monitor.setMessage("Step 1: Scanned %d strings..." % count)

        try:
            val = data.getDefaultValueRepresentation()
            if val is None:
                continue
            if val.startswith('"') and val.endswith('"'):
                val = val[1:-1]
        except:
            continue

        val_lower = val.lower()

        # Check for Lua API strings
        for pattern in lua_api_patterns:
            if pattern.lower() in val_lower:
                lua_api_strings.append((val, data.getAddress()))
                all_lua_strings.append((val, data.getAddress()))
                break
        else:
            # Check for toLua strings
            for pattern in tolua_api_patterns:
                if pattern.lower() in val_lower:
                    tolua_strings.append((val, data.getAddress()))
                    all_lua_strings.append((val, data.getAddress()))
                    break
            else:
                # General "lua" mention
                if "lua" in val_lower and len(val) < 200:
                    lua_indicator_strings.append((val, data.getAddress()))
                    all_lua_strings.append((val, data.getAddress()))

    lua_strings_found = len(all_lua_strings)
    println("  Total strings with 'lua': %d" % lua_strings_found)
    println("    Lua API strings:        %d" % len(lua_api_strings))
    println("    toLua++ strings:        %d" % len(tolua_strings))
    println("    Other lua mentions:     %d" % len(lua_indicator_strings))
    println("")

    if lua_strings_found == 0:
        println("No Lua-related strings found in the binary.")
        println("SGW.exe does not appear to use Lua scripting.")
        println("")
        println("Script 08 complete (no Lua bindings to process).")
        return

    # Determine binding style
    has_tolua = len(tolua_strings) > 0
    has_lua_api = len(lua_api_strings) > 0

    if has_tolua:
        println("toLua++ binding system detected.")
    if has_lua_api:
        println("Lua C API usage detected.")
    if not has_tolua and not has_lua_api:
        println("Lua strings found but no clear binding API detected.")
        println("Will attempt pattern-based discovery.")
    println("")

    # ---------------------------------------------------------------
    # STEP 2: Find Lua registration function calls
    # ---------------------------------------------------------------
    println("[Step 2] Finding Lua registration function references...")
    monitor.setMessage("Step 2: Finding registration functions...")

    # Look for imported or named functions related to Lua registration
    registration_funcs = {}  # name -> address
    reg_patterns = [
        "lua_register", "luaL_register", "luaL_newlib",
        "lua_pushcfunction", "lua_pushcclosure",
        "lua_setfield", "lua_setglobal", "lua_rawset",
        "tolua_function", "tolua_cclass", "tolua_module",
        "tolua_beginmodule", "tolua_constant", "tolua_variable",
    ]

    # Search symbol table
    for pattern in reg_patterns:
        sym_iter = st.getSymbols(pattern)
        while sym_iter.hasNext():
            sym = sym_iter.next()
            registration_funcs[sym.getName()] = sym.getAddress()
            registration_functions_found += 1

    # Also check imports
    ext_mgr = currentProgram.getExternalManager()
    for pattern in reg_patterns:
        # Search function manager for these names
        func_iter = fm.getFunctions(True)
        while func_iter.hasNext():
            if monitor.isCancelled():
                return
            func = func_iter.next()
            if func.getName().lower() == pattern.lower():
                registration_funcs[func.getName()] = func.getEntryPoint()
                registration_functions_found += 1

    println("  Found %d Lua registration functions:" % len(registration_funcs))
    for name, addr in sorted(registration_funcs.items()):
        println("    %s at %s" % (name, addr.toString()))
    println("")

    # ---------------------------------------------------------------
    # STEP 3: Follow xrefs to registration functions to find callers
    # ---------------------------------------------------------------
    println("[Step 3] Tracing registration function call sites...")
    monitor.setMessage("Step 3: Tracing registration calls...")

    # For each registration function, find all callers and try to extract
    # the string argument (Lua function name) and the C function pointer
    binding_map = {}  # lua_name -> c_function_addr

    for reg_name, reg_addr in registration_funcs.items():
        if monitor.isCancelled():
            return

        refs = getReferencesTo(reg_addr)
        if refs is None:
            continue

        for ref in refs:
            if not ref.getReferenceType().isCall():
                continue

            call_addr = ref.getFromAddress()
            caller_func = getFunctionContaining(call_addr)
            if caller_func is None:
                continue

            # The caller is a registration function (e.g., luaopen_xxx or tolua_xxx_open)
            # Mark the caller as a registration function
            if is_default_name(caller_func):
                # Try to name it based on the registration pattern
                plate = caller_func.getComment()
                if plate is None or "Lua" not in plate:
                    comment = "[Lua] Calls %s -- likely a binding registration function" % reg_name
                    if plate:
                        comment = plate + "\n" + comment
                    caller_func.setComment(comment)
                    functions_commented += 1

    println("  Traced registration call sites.")
    println("")

    # ---------------------------------------------------------------
    # STEP 4: Search for registration table patterns
    # ---------------------------------------------------------------
    println("[Step 4] Searching for registration table patterns...")
    monitor.setMessage("Step 4: Scanning for {string_ptr, func_ptr} tables...")

    # Look for luaL_Reg-style tables: arrays of {const char* name, lua_CFunction func}
    # These are pairs of 4-byte pointers: [string_ptr, func_ptr, string_ptr, func_ptr, ...]
    # terminated by {NULL, NULL}
    #
    # Strategy: For each Lua-related string we found, check if adjacent memory
    # has the pattern of a registration table.

    # Build a set of all string addresses for quick lookup
    all_string_addrs = set()
    string_addr_to_val = {}
    data_iter2 = DefinedDataIterator.definedStrings(currentProgram)
    while data_iter2.hasNext():
        if monitor.isCancelled():
            return
        data = data_iter2.next()
        addr = data.getAddress()
        all_string_addrs.add(addr.getOffset())
        try:
            val = data.getDefaultValueRepresentation()
            if val and val.startswith('"') and val.endswith('"'):
                val = val[1:-1]
            string_addr_to_val[addr.getOffset()] = val
        except:
            pass

    # For each Lua-related string, find xrefs to it and check if the
    # reference is part of a table
    tables_found = set()  # set of table start addresses (to avoid duplicates)

    # Focus on strings that look like function/method names
    candidate_strings = []
    for val, addr in all_lua_strings:
        # Skip error messages, file paths, etc.
        if "/" in val or "\\" in val:
            continue
        if len(val) > 100:
            continue
        candidate_strings.append((val, addr))

    # Also look for short identifier-like strings referenced near Lua API calls
    # that could be Lua function names
    monitor.setMaximum(len(candidate_strings))

    for idx, (str_val, str_addr) in enumerate(candidate_strings):
        monitor.setProgress(idx)
        if monitor.isCancelled():
            return

        refs = getReferencesTo(str_addr)
        if refs is None:
            continue

        for ref in refs:
            from_addr = ref.getFromAddress()

            # Check if this reference is in a data section (table) rather than code
            from_block = mem.getBlock(from_addr)
            if from_block is None:
                continue

            # If it's in a data/rdata section, this might be part of a registration table
            block_name = from_block.getName().lower()
            is_data_section = ("data" in block_name or "rdata" in block_name or
                               "rodata" in block_name or not from_block.isExecute())

            if not is_data_section:
                continue

            # Check if this looks like the start of a {string, func} pair
            # from_addr should contain a pointer to the string
            ptr_val = read_dword(mem, from_addr)
            if ptr_val is None:
                continue

            if ptr_val != str_addr.getOffset():
                continue  # The data at from_addr doesn't actually point to our string

            # Read the next dword -- should be a function pointer
            func_ptr_val = read_dword(mem, from_addr.add(4))
            if func_ptr_val is None:
                continue

            if not is_plausible_code_addr(func_ptr_val, mem, default_space):
                continue

            # This looks like a {string, func} pair! Try to find the table start.
            # Walk backwards to find the beginning of the table.
            table_start = from_addr
            while True:
                prev_str_ptr = read_dword(mem, table_start.add(-8))
                prev_func_ptr = read_dword(mem, table_start.add(-4))

                if prev_str_ptr is None or prev_func_ptr is None:
                    break
                if not is_plausible_data_addr(prev_str_ptr, mem, default_space):
                    break
                if not is_plausible_code_addr(prev_func_ptr, mem, default_space):
                    break

                # Check if prev_str_ptr points to a readable string
                try:
                    test_addr = default_space.getAddress(prev_str_ptr & 0xFFFFFFFF)
                    test_str = read_cstring(mem, test_addr, 128)
                    if test_str is None or len(test_str) < 1:
                        break
                except:
                    break

                table_start = table_start.add(-8)

            # Skip if we've already processed this table
            table_offset = table_start.getOffset()
            if table_offset in tables_found:
                continue
            tables_found.add(table_offset)

            # Now walk forward through the table extracting all entries
            binding_tables_found += 1
            current = table_start
            table_entries = []
            MAX_TABLE_ENTRIES = 500

            while len(table_entries) < MAX_TABLE_ENTRIES:
                s_ptr = read_dword(mem, current)
                f_ptr = read_dword(mem, current.add(4))

                if s_ptr is None or f_ptr is None:
                    break

                # {NULL, NULL} terminator
                if s_ptr == 0 and f_ptr == 0:
                    break

                if s_ptr == 0 or f_ptr == 0:
                    break

                if not is_plausible_data_addr(s_ptr, mem, default_space):
                    break

                if not is_plausible_code_addr(f_ptr, mem, default_space):
                    break

                # Read the string
                try:
                    s_addr = default_space.getAddress(s_ptr & 0xFFFFFFFF)
                    lua_name = read_cstring(mem, s_addr, 128)
                    if lua_name is None or len(lua_name) < 1:
                        break
                except:
                    break

                f_addr = default_space.getAddress(f_ptr & 0xFFFFFFFF)
                table_entries.append((lua_name, f_addr))
                current = current.add(8)

            if len(table_entries) >= 2:
                println("  Registration table at %s: %d entries" % (
                    table_start.toString(), len(table_entries)))

                for lua_name, func_addr in table_entries:
                    bindings_extracted += 1
                    binding_map[lua_name] = func_addr

    println("")
    println("  Found %d registration tables with %d total bindings" % (
        binding_tables_found, bindings_extracted))
    println("")

    # ---------------------------------------------------------------
    # STEP 5: Name functions from discovered bindings
    # ---------------------------------------------------------------
    println("[Step 5] Naming functions from Lua bindings...")
    monitor.setMessage("Step 5: Naming bound functions...")

    if not binding_map:
        println("  No binding entries to process.")
    else:
        monitor.setMaximum(len(binding_map))
        for idx, (lua_name, func_addr) in enumerate(sorted(binding_map.items())):
            monitor.setProgress(idx)
            if monitor.isCancelled():
                return

            func = fm.getFunctionAt(func_addr)
            if func is None:
                # Try to create
                try:
                    from ghidra.app.cmd.function import CreateFunctionCmd
                    cmd = CreateFunctionCmd(func_addr)
                    cmd.applyTo(currentProgram)
                    func = fm.getFunctionAt(func_addr)
                except:
                    pass

            if func is None:
                continue

            if not is_default_name(func):
                # Already named -- add comment only
                plate = func.getComment()
                if plate is None or "Lua" not in plate:
                    comment = "[Lua binding] Registered as: %s" % lua_name
                    if plate:
                        comment = plate + "\n" + comment
                    func.setComment(comment)
                    functions_commented += 1
                continue

            # Determine naming prefix based on the Lua name pattern
            if "." in lua_name:
                # Module.method pattern (e.g., "MyClass.DoThing")
                parts = lua_name.split(".")
                func_name = "lua_%s_%s" % (parts[0], "_".join(parts[1:]))
            elif ":" in lua_name:
                # Class:method pattern
                parts = lua_name.split(":")
                func_name = "lua_%s_%s" % (parts[0], "_".join(parts[1:]))
            else:
                func_name = "lua_%s" % lua_name

            func_name = sanitize_name(func_name)
            if not func_name:
                continue

            # Handle duplicates
            if func_name in name_usage_counts:
                name_usage_counts[func_name] += 1
                func_name = "%s_%d" % (func_name, name_usage_counts[func_name])
            else:
                name_usage_counts[func_name] = 0

            try:
                func.setName(func_name, SourceType.USER_DEFINED)
                functions_renamed += 1

                comment = "[Lua binding] Lua name: \"%s\"\nFunction at: %s" % (
                    lua_name, func_addr.toString())
                func.setComment(comment)
                functions_commented += 1

                println("    %s -> %s" % (lua_name, func_name))
            except Exception as e:
                try:
                    addr_suffix = func_addr.toString().replace("0x", "")
                    fallback = "%s_at_%s" % (func_name, addr_suffix)
                    func.setName(fallback, SourceType.USER_DEFINED)
                    functions_renamed += 1
                except:
                    pass

    println("")

    # ---------------------------------------------------------------
    # STEP 6: Name functions that reference Lua API strings
    # ---------------------------------------------------------------
    println("[Step 6] Naming functions that reference Lua API patterns...")
    monitor.setMessage("Step 6: Naming Lua API callers...")

    # For strings like "luaopen_xxx", find the function they reference
    # and name it accordingly
    renamed_step6 = 0
    for val, str_addr in all_lua_strings:
        if monitor.isCancelled():
            return

        # Look for luaopen_ patterns (module initializers)
        if val.startswith("luaopen_"):
            module_name = val[8:]  # strip "luaopen_"
            # Find xrefs -- this string might be used in a registration
            refs = getReferencesTo(str_addr)
            if refs is None:
                continue
            for ref in refs:
                func = getFunctionContaining(ref.getFromAddress())
                if func is None:
                    continue
                if is_default_name(func):
                    clean = sanitize_name("luaopen_%s" % module_name)
                    try:
                        func.setName(clean, SourceType.USER_DEFINED)
                        renamed_step6 += 1
                        functions_renamed += 1
                        comment = "[Lua] Module initializer for: %s" % module_name
                        func.setComment(comment)
                    except:
                        pass

    println("  Named %d Lua module initializers" % renamed_step6)
    println("")

    # ---------------------------------------------------------------
    # Summary
    # ---------------------------------------------------------------
    println("=" * 70)
    println("Lua Binding Annotator - Summary")
    println("=" * 70)
    println("")
    println("Lua strings found:       %d" % lua_strings_found)
    println("Registration functions:  %d" % len(registration_funcs))
    println("Binding tables found:    %d" % binding_tables_found)
    println("Bindings extracted:      %d" % bindings_extracted)
    println("Functions renamed:       %d" % functions_renamed)
    println("Functions commented:     %d" % functions_commented)
    println("")

    if functions_renamed == 0 and bindings_extracted == 0:
        println("NOTE: While Lua-related strings were found, no structured binding")
        println("tables were identified. SGW.exe may use:")
        println("  - A custom Lua integration (not toLua++ or standard luaL_Reg)")
        println("  - Lua only for configuration, not gameplay scripting")
        println("  - A different scripting approach entirely (Python via Boost.Python)")
        println("")
        println("The BigWorld engine uses Python for entity scripting. Lua usage in")
        println("SGW.exe, if any, is likely limited to UI scripting or embedded tools.")

    println("")
    println("Script 08 complete.")


# Entry point
main()
