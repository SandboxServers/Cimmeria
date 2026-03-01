# 01_rtti_annotator.py
# Ghidra Jython script for Stargate Worlds (SGW.exe) reverse engineering
#
# Purpose:
#   Find MSVC RTTI type descriptors (.?AVClassName@@ strings in .rdata),
#   trace through the RTTI hierarchy (TypeDescriptor -> CompleteObjectLocator
#   -> vtable), and label vtables and their first virtual function.
#
# MSVC RTTI layout (32-bit):
#   1. RTTITypeDescriptor contains the mangled name string ".?AVClassName@@"
#      Layout: [pVFTable(4)] [spare(4)] [name(variable, null-terminated)]
#   2. RTTICompleteObjectLocator references the TypeDescriptor
#      Layout: [signature(4)] [offset(4)] [cdOffset(4)] [pTypeDescriptor(4)] [pClassHierarchy(4)]
#   3. The vtable pointer array is preceded by a pointer to the CompleteObjectLocator
#   4. The first entry in the vtable array points to vfunc_0
#
# Expected yield: 200-400 vtables labeled, 200-400 vfunc_0 functions named
# Confidence: HIGH - RTTI is compiler-generated and structurally reliable
#
# Usage: Run from Ghidra Script Manager (Jython)

"""
RTTI Annotator for SGW.exe

Scans memory for MSVC RTTI type descriptor strings (.?AV*@@), traces references
through the RTTI object hierarchy to locate vtables, and labels them
along with their first virtual function entry.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.mem import MemoryAccessException
from ghidra.program.model.address import AddressSet
import re
import struct


def find_rtti_strings_by_memory_scan():
    """Scan memory for .?AV*@@ and .?AU*@@ RTTI type descriptor name strings.

    Uses Ghidra's built-in findBytes API for reliable searching across all
    memory blocks, then reads the full null-terminated string at each hit.
    """
    memory = currentProgram.getMemory()
    results = []

    # Search for both .?AV (class) and .?AU (struct) RTTI patterns
    patterns = [".?AV", ".?AU"]
    search_start = currentProgram.getMinAddress()
    search_end = currentProgram.getMaxAddress()

    for pattern in patterns:
        # Convert pattern to byte string for findBytes
        addr = search_start
        while addr is not None and addr.compareTo(search_end) < 0:
            if monitor.isCancelled():
                return results

            # Use memory.findBytes to search for the pattern
            addr = memory.findBytes(addr, search_end, pattern.encode('ascii'), None, True, monitor)
            if addr is None:
                break

            # Read the full string from this address until null terminator
            name_chars = []
            offset = 0
            try:
                while offset < 512:
                    b = memory.getByte(addr.add(offset))
                    if b == 0:
                        break
                    name_chars.append(chr(b & 0xFF))
                    offset += 1
            except MemoryAccessException:
                pass

            if name_chars:
                name_str = ''.join(name_chars)
                if name_str.endswith('@@'):
                    results.append((addr, name_str))

            # Advance past this hit
            addr = addr.add(1)

    return results


def read_pointer(addr):
    """Read a 32-bit pointer value from the given address."""
    mem = currentProgram.getMemory()
    try:
        value = mem.getInt(addr)
        # Convert signed int to unsigned for address creation
        value = value & 0xFFFFFFFF
        return toAddr(value)
    except MemoryAccessException:
        return None


def is_valid_code_addr(addr):
    """Check if an address points to executable memory."""
    if addr is None:
        return False
    block = currentProgram.getMemory().getBlock(addr)
    if block is None:
        return False
    return block.isExecute() or block.getName() == '.text'


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


def extract_class_name(rtti_string):
    """Extract class name from RTTI mangled string .?AVClassName@@

    Handles nested namespaces: .?AVClassName@Namespace@@ -> Namespace::ClassName
    Also handles .?AU (struct) and .?AW (enum) prefixes.
    """
    match = re.match(r'\.?\?A[VUW](.+?)@@$', rtti_string)
    if not match:
        return None

    raw = match.group(1)

    # MSVC mangling: .?AVClass@Namespace@@ means Namespace::Class
    # Split on @ to get namespace parts (reversed order)
    parts = raw.split('@')
    parts = [p for p in parts if p]
    if len(parts) > 1:
        parts.reverse()

    return '::'.join(parts)


def sanitize_label(name):
    """Sanitize a name to be a valid Ghidra label (no special characters)."""
    sanitized = name.replace('::', '_')
    sanitized = re.sub(r'[^a-zA-Z0-9_]', '_', sanitized)
    if sanitized and sanitized[0].isdigit():
        sanitized = '_' + sanitized
    return sanitized


def set_label(addr, name):
    """Set a label at the given address."""
    sym_table = currentProgram.getSymbolTable()
    existing = sym_table.getPrimarySymbol(addr)
    if existing is not None and existing.getSource() == SourceType.USER_DEFINED:
        return False
    try:
        sym_table.createLabel(addr, name, SourceType.USER_DEFINED)
        return True
    except:
        return False


def run():
    monitor = getMonitor()
    monitor.setMessage("RTTI Annotator: Scanning memory for type descriptor strings...")

    stats = {
        'rtti_strings_found': 0,
        'class_names_extracted': 0,
        'type_descriptors_found': 0,
        'col_found': 0,
        'vtables_labeled': 0,
        'vfunc0_renamed': 0,
        'skipped_user_named': 0,
        'errors': 0,
    }

    func_mgr = currentProgram.getFunctionManager()
    ref_mgr = currentProgram.getReferenceManager()

    println("=" * 70)
    println("RTTI Annotator - Stargate Worlds (SGW.exe)")
    println("=" * 70)
    println("")
    println("Phase 1: Scanning memory for .?AV*@@ / .?AU*@@ RTTI strings...")

    # Memory scan instead of defined string search
    rtti_entries = []
    raw_hits = find_rtti_strings_by_memory_scan()

    for addr, value in raw_hits:
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        class_name = extract_class_name(value)
        if class_name:
            rtti_entries.append((addr, value, class_name))
            stats['rtti_strings_found'] += 1

    println("  Found %d RTTI type descriptor strings" % stats['rtti_strings_found'])

    if stats['rtti_strings_found'] == 0:
        println("No RTTI strings found. The binary may have RTTI disabled (/GR-).")
        return

    # Print some examples
    println("")
    println("  Sample classes found:")
    for _, _, name in rtti_entries[:20]:
        println("    %s" % name)
    if len(rtti_entries) > 20:
        println("    ... and %d more" % (len(rtti_entries) - 20))

    # Phase 2: For each RTTI string, trace the hierarchy
    # The string is at offset +8 within RTTITypeDescriptor.
    # So TypeDescriptor base = string_addr - 8
    # We look for xrefs TO the TypeDescriptor base.
    println("")
    println("Phase 2: Tracing RTTI hierarchy (TypeDescriptor -> COL -> vtable)...")
    monitor.setMessage("RTTI Annotator: Tracing RTTI hierarchy...")
    monitor.setMaximum(len(rtti_entries))

    vtable_map = {}  # class_name -> vtable_addr

    for idx, (rtti_addr, rtti_string, class_name) in enumerate(rtti_entries):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)

        stats['class_names_extracted'] += 1

        try:
            # TypeDescriptor base is 8 bytes before the name string
            type_desc_addr = rtti_addr.subtract(8)

            # Find xrefs to the TypeDescriptor
            # These come from CompleteObjectLocator.pTypeDescriptor field
            refs_to_td = getReferencesTo(type_desc_addr)

            if not refs_to_td:
                continue

            stats['type_descriptors_found'] += 1

            for td_ref in refs_to_td:
                if monitor.isCancelled():
                    return

                # td_ref.getFromAddress() is inside a COL (at the pTypeDescriptor field)
                # In 32-bit MSVC COL: pTypeDescriptor is at offset +12
                # So COL base = from_addr - 12
                col_td_field = td_ref.getFromAddress()
                col_base = col_td_field.subtract(12)

                # Verify: COL.signature should be 0 (for 32-bit, non-ASLR)
                sig = read_pointer(col_base)
                if sig is not None and sig.getOffset() != 0:
                    # Try without the offset assumption - maybe pTypeDescriptor is at a different offset
                    # Some MSVC versions or configurations may vary
                    pass

                # Find xrefs to the COL base - these come from vtable[-1]
                refs_to_col = getReferencesTo(col_base)

                if not refs_to_col:
                    # Try the original address (the pTypeDescriptor field itself)
                    # In case references point to this field directly
                    continue

                stats['col_found'] += 1

                for col_ref in refs_to_col:
                    # The pointer to COL sits at vtable[-1]
                    # So vtable[0] = this address + 4
                    vtable_addr = col_ref.getFromAddress().add(4)

                    # Verify: first vtable entry should point to executable code
                    first_entry = read_pointer(vtable_addr)
                    if first_entry is not None and is_valid_code_addr(first_entry):
                        if class_name not in vtable_map:
                            vtable_map[class_name] = vtable_addr

        except Exception as e:
            stats['errors'] += 1

    println("  TypeDescriptors with xrefs: %d" % stats['type_descriptors_found'])
    println("  CompleteObjectLocators found: %d" % stats['col_found'])
    println("  Vtables identified: %d" % len(vtable_map))

    # Phase 3: Label vtables and first virtual functions
    println("")
    println("Phase 3: Labeling vtables and vfunc_0 entries...")
    monitor.setMessage("RTTI Annotator: Labeling vtables...")
    monitor.setMaximum(len(vtable_map))

    for idx, (class_name, vtable_addr) in enumerate(sorted(vtable_map.items())):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)

        safe_name = sanitize_label(class_name)

        try:
            # Label the vtable
            vtable_label = "vtable_%s" % safe_name
            if set_label(vtable_addr, vtable_label):
                stats['vtables_labeled'] += 1

            # Read the first vtable entry to get vfunc_0
            vfunc0_addr = read_pointer(vtable_addr)
            if vfunc0_addr is not None and is_valid_code_addr(vfunc0_addr):
                func = func_mgr.getFunctionAt(vfunc0_addr)
                if func is None:
                    func = func_mgr.getFunctionContaining(vfunc0_addr)

                if func is not None:
                    if is_user_named(func):
                        stats['skipped_user_named'] += 1
                    elif is_unnamed_function(func):
                        new_name = "%s__vfunc_0" % safe_name
                        try:
                            func.setName(new_name, SourceType.USER_DEFINED)
                            stats['vfunc0_renamed'] += 1
                        except:
                            stats['errors'] += 1
                else:
                    # No function at this address; try to create one
                    try:
                        new_name = "%s__vfunc_0" % safe_name
                        func = createFunction(vfunc0_addr, new_name)
                        if func is not None:
                            stats['vfunc0_renamed'] += 1
                    except:
                        stats['errors'] += 1

        except Exception as e:
            stats['errors'] += 1

    # Print summary
    println("")
    println("=" * 70)
    println("RTTI Annotator Summary")
    println("=" * 70)
    println("  RTTI strings found:                %d" % stats['rtti_strings_found'])
    println("  Class names extracted:             %d" % stats['class_names_extracted'])
    println("  TypeDescriptors with xrefs:        %d" % stats['type_descriptors_found'])
    println("  CompleteObjectLocators found:       %d" % stats['col_found'])
    println("  Vtables labeled:                   %d" % stats['vtables_labeled'])
    println("  vfunc_0 functions renamed:         %d" % stats['vfunc0_renamed'])
    println("  Skipped (already user-named):      %d" % stats['skipped_user_named'])
    println("  Errors:                            %d" % stats['errors'])
    println("=" * 70)

    # Print some labeled vtables
    if vtable_map:
        println("")
        println("Sample vtables labeled:")
        for name, addr in sorted(vtable_map.items())[:30]:
            println("  %s -> %s" % (addr, name))
        if len(vtable_map) > 30:
            println("  ... and %d more" % (len(vtable_map) - 30))


# Entry point
run()
