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
#   2. RTTICompleteObjectLocator references the TypeDescriptor (among other fields)
#   3. The vtable pointer array is preceded by a pointer to the CompleteObjectLocator
#   4. The first entry in the vtable array points to vfunc_0
#
# Expected yield: 200-400 vtables labeled, 200-400 vfunc_0 functions named
# Confidence: HIGH - RTTI is compiler-generated and structurally reliable
#
# Usage: Run from Ghidra Script Manager (Jython)

"""
RTTI Annotator for SGW.exe

Scans for MSVC RTTI type descriptor strings (.?AV*@@), traces references
through the RTTI object hierarchy to locate vtables, and labels them
along with their first virtual function entry.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.data import StringDataType
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


def is_unnamed_function(func):
    """Check if a function has an auto-generated name (not user-defined)."""
    if func is None:
        return False
    name = func.getName()
    source = func.getSymbol().getSource()
    # Auto-generated names: FUN_*, thunk_FUN_*, etc.
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
    """
    # Match the RTTI pattern
    match = re.match(r'\.?\?AV(.+?)@@$', rtti_string)
    if not match:
        return None

    raw = match.group(1)

    # MSVC mangling: .?AVClass@Namespace@@ means Namespace::Class
    # Split on @ to get namespace parts (reversed order)
    parts = raw.split('@')
    # Filter empty parts and reverse (MSVC stores inner-to-outer)
    parts = [p for p in parts if p]
    if len(parts) > 1:
        parts.reverse()

    return '::'.join(parts)


def sanitize_label(name):
    """Sanitize a name to be a valid Ghidra label (no special characters)."""
    # Replace :: with _ for label compatibility
    sanitized = name.replace('::', '_')
    # Replace any remaining invalid characters
    sanitized = re.sub(r'[^a-zA-Z0-9_]', '_', sanitized)
    # Ensure it doesn't start with a digit
    if sanitized and sanitized[0].isdigit():
        sanitized = '_' + sanitized
    return sanitized


def set_label(addr, name):
    """Set a label at the given address."""
    sym_table = currentProgram.getSymbolTable()
    # Check if there is already a user-defined symbol here
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
    monitor.setMessage("RTTI Annotator: Scanning for type descriptor strings...")

    # Statistics
    stats = {
        'rtti_strings_found': 0,
        'class_names_extracted': 0,
        'type_descriptors_found': 0,
        'complete_object_locators_found': 0,
        'vtables_labeled': 0,
        'vfunc0_renamed': 0,
        'skipped_user_named': 0,
        'errors': 0,
    }

    func_mgr = currentProgram.getFunctionManager()
    ref_mgr = currentProgram.getReferenceManager()

    # Step 1: Find all RTTI type descriptor strings
    println("=" * 70)
    println("RTTI Annotator - Stargate Worlds (SGW.exe)")
    println("=" * 70)
    println("")
    println("Phase 1: Scanning for .?AV*@@ RTTI strings...")

    all_strings = get_all_defined_strings()
    rtti_entries = []

    for addr, value in all_strings:
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        if '.?AV' in value and value.endswith('@@'):
            class_name = extract_class_name(value)
            if class_name:
                rtti_entries.append((addr, value, class_name))
                stats['rtti_strings_found'] += 1

    println("  Found %d RTTI type descriptor strings" % stats['rtti_strings_found'])

    if stats['rtti_strings_found'] == 0:
        println("No RTTI strings found. Is this binary stripped of RTTI?")
        return

    # Step 2-6: For each RTTI string, trace through the hierarchy
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
        monitor.setMessage("Processing: %s" % class_name)

        stats['class_names_extracted'] += 1
        safe_name = sanitize_label(class_name)

        try:
            # Step 3: Find xrefs TO the RTTI string -> these point to TypeDescriptor
            # The RTTI string is embedded within the RTTITypeDescriptor structure.
            # The TypeDescriptor starts 8 bytes before the string (pVFTable + spare)
            # But references may point to the string directly or to the struct start.
            # We look for references to the string address itself.
            refs_to_string = getReferencesTo(rtti_addr)

            if not refs_to_string:
                # The string might be part of a larger TypeDescriptor structure.
                # Try looking at the address 8 bytes before (RTTITypeDescriptor layout:
                # offset 0: pVFTable, offset 4: spare, offset 8: name[])
                type_desc_addr = rtti_addr.subtract(8)
                refs_to_td = getReferencesTo(type_desc_addr)
                if refs_to_td:
                    stats['type_descriptors_found'] += 1
                    refs_to_string = []  # We will use refs_to_td below
                    # Now find xrefs TO the TypeDescriptor -> CompleteObjectLocator
                    for td_ref in refs_to_td:
                        col_candidate = td_ref.getFromAddress()
                        # The COL references the TD; find xrefs to this COL
                        refs_to_col = getReferencesTo(col_candidate)
                        if refs_to_col:
                            stats['complete_object_locators_found'] += 1
                            for col_ref in refs_to_col:
                                # The pointer to the COL sits at vtable[-1]
                                # So the vtable starts at col_ref + 4
                                vtable_addr = col_ref.getFromAddress().add(4)
                                vtable_map[class_name] = vtable_addr
                continue

            # References to the RTTI name string found
            for str_ref in refs_to_string:
                if monitor.isCancelled():
                    return

                # str_ref.getFromAddress() is inside the TypeDescriptor (the name field pointer)
                # The TypeDescriptor struct start is at the referring address's context
                # Actually, the string IS the name field, so the ref FROM address is where
                # a pointer to this string lives. That pointer is inside a TypeDescriptor
                # or a CompleteObjectLocator.

                td_candidate = str_ref.getFromAddress()
                stats['type_descriptors_found'] += 1

                # Step 4: Find xrefs TO the TypeDescriptor address
                # The CompleteObjectLocator has a pointer to the TypeDescriptor
                # But we need the base of the TypeDescriptor, which is 8 bytes before the name
                type_desc_base = rtti_addr.subtract(8)

                refs_to_td = getReferencesTo(type_desc_base)
                if not refs_to_td:
                    # Try the string address itself as some tools resolve differently
                    refs_to_td = getReferencesTo(rtti_addr)

                for td_ref in refs_to_td:
                    if monitor.isCancelled():
                        return

                    # td_ref.getFromAddress() is inside the CompleteObjectLocator
                    # (specifically the pTypeDescriptor field)
                    col_candidate_addr = td_ref.getFromAddress()

                    # The COL structure (32-bit MSVC):
                    # offset 0: signature (0)
                    # offset 4: offset
                    # offset 8: cdOffset
                    # offset 12: pTypeDescriptor  <- this is what references our TD
                    # offset 16: pClassHierarchyDescriptor
                    # So COL base = col_candidate_addr - 12
                    col_base = col_candidate_addr.subtract(12)

                    # Step 5: Find xrefs TO the COL base
                    refs_to_col = getReferencesTo(col_base)
                    if refs_to_col:
                        stats['complete_object_locators_found'] += 1
                        for col_ref in refs_to_col:
                            # The reference to COL is at vtable[-1]
                            # So vtable[0] = col_ref.getFromAddress() + 4
                            vtable_start = col_ref.getFromAddress().add(4)
                            if class_name not in vtable_map:
                                vtable_map[class_name] = vtable_start

                # Only process the first ref to avoid duplicates
                break

        except Exception as e:
            stats['errors'] += 1
            println("  ERROR processing %s: %s" % (class_name, str(e)))

    # Step 6-7: Label vtables and first virtual functions
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
                println("  vtable: %s at %s" % (vtable_label, vtable_addr))

            # Read the first vtable entry to get vfunc_0
            vfunc0_addr = read_pointer(vtable_addr)
            if vfunc0_addr is not None:
                func = func_mgr.getFunctionAt(vfunc0_addr)
                if func is None:
                    # Try to get the function containing this address
                    func = func_mgr.getFunctionContaining(vfunc0_addr)

                if func is not None:
                    if is_user_named(func):
                        stats['skipped_user_named'] += 1
                    elif is_unnamed_function(func):
                        new_name = "%s__vfunc_0" % safe_name
                        try:
                            func.setName(new_name, SourceType.USER_DEFINED)
                            stats['vfunc0_renamed'] += 1
                            println("  vfunc_0: %s at %s" % (new_name, vfunc0_addr))
                        except:
                            stats['errors'] += 1
                elif vfunc0_addr is not None:
                    # No function at this address; try to create one
                    try:
                        new_name = "%s__vfunc_0" % safe_name
                        func = createFunction(vfunc0_addr, new_name)
                        if func is not None:
                            stats['vfunc0_renamed'] += 1
                            println("  vfunc_0 (created): %s at %s" % (new_name, vfunc0_addr))
                    except:
                        stats['errors'] += 1

        except Exception as e:
            stats['errors'] += 1
            println("  ERROR labeling %s: %s" % (class_name, str(e)))

    # Print summary
    println("")
    println("=" * 70)
    println("RTTI Annotator Summary")
    println("=" * 70)
    println("  RTTI strings found:                %d" % stats['rtti_strings_found'])
    println("  Class names extracted:             %d" % stats['class_names_extracted'])
    println("  TypeDescriptors located:           %d" % stats['type_descriptors_found'])
    println("  CompleteObjectLocators located:     %d" % stats['complete_object_locators_found'])
    println("  Vtables labeled:                   %d" % stats['vtables_labeled'])
    println("  vfunc_0 functions renamed:         %d" % stats['vfunc0_renamed'])
    println("  Skipped (already user-named):      %d" % stats['skipped_user_named'])
    println("  Errors:                            %d" % stats['errors'])
    println("=" * 70)


# Entry point
run()
