# -*- coding: utf-8 -*-
# 07_vtable_annotator.py
# Ghidra Jython script for SGW.exe reverse engineering
#
# Purpose: Propagate names from known vtables to virtual function entries.
# After running the RTTI annotator (script 01), many vtables will be labeled
# with names like "vtable_ClassName". This script reads those labeled vtables
# and names each function pointer entry as ClassName::vfunc_N.
#
# Expected yield: 2000-5000 functions named (MEDIUM confidence)
#
# Run order: After script 01 (RTTI annotator). Before scripts 09-10.
#
# @category SGW
# @author Cimmeria Project

"""
VTable Annotator

Reads vtable labels created by the RTTI annotator and propagates class names
to virtual function entries. For each vtable:
  1. Extracts the class name from the vtable label
  2. Reads consecutive 4-byte pointer entries (32-bit binary)
  3. Names each pointed-to function as ClassName::vfunc_N
  4. Stops at NULL pointers, non-code addresses, or other known labels
  5. Respects existing user-defined names (inheritance preservation)
"""

from ghidra.program.model.symbol import SourceType, SymbolType
from ghidra.program.model.mem import MemoryAccessException

import re


def is_default_name(func):
    """Check if a function has a default auto-generated name."""
    name = func.getName()
    return (name.startswith("FUN_") or
            name.startswith("thunk_FUN_") or
            name.startswith("Ordinal_") or
            name.startswith("LAB_"))


def is_code_address(addr, mem, addr_space):
    """
    Heuristic check: does this 32-bit value look like a plausible code address?
    For a 32-bit PE, code addresses are typically in the range 0x00400000-0x7FFFFFFF.
    """
    try:
        val = addr.getOffset()
        # Must be non-zero
        if val == 0:
            return False
        # Typical 32-bit PE image base range
        if val < 0x00400000 or val > 0x7FFFFFFF:
            return False
        # Must be in an executable memory block
        block = mem.getBlock(addr)
        if block is None:
            return False
        if block.isExecute():
            return True
        # Some code may be in non-execute blocks if the binary has quirks
        # but generally we want executable
        return False
    except:
        return False


def read_pointer(listing, mem, addr):
    """Read a 4-byte pointer value at the given address (32-bit LE)."""
    try:
        val = mem.getInt(addr)
        return val & 0xFFFFFFFF
    except MemoryAccessException:
        return None
    except:
        return None


def clean_class_name(raw):
    """
    Clean a vtable label into a class name.
    Handles: vtable_ClassName, vtable_Namespace__ClassName, etc.
    """
    name = raw
    # Strip vtable_ prefix (various formats the RTTI script might use)
    for prefix in ["vtable_", "VTABLE_", "vftable_", "VFTABLE_"]:
        if name.startswith(prefix):
            name = name[len(prefix):]
            break

    # Replace double underscores with :: for display purposes but keep
    # underscores in the actual symbol name
    name = name.strip("_")
    return name


def main():
    monitor = getMonitor()
    fm = currentProgram.getFunctionManager()
    st = currentProgram.getSymbolTable()
    listing = currentProgram.getListing()
    mem = currentProgram.getMemory()
    addr_factory = currentProgram.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()

    println("=" * 70)
    println("VTable Annotator - Script 07")
    println("=" * 70)
    println("")

    # Statistics
    vtables_processed = 0
    vtables_skipped = 0
    total_vfuncs_named = 0
    total_vfuncs_skipped = 0  # already named
    total_entries_read = 0
    vtable_sizes = {}  # class_name -> entry_count
    already_named_from_parent = 0

    # Track all names we assign to detect conflicts
    assigned_names = set()

    # ---------------------------------------------------------------
    # STEP 1: Find all vtable labels in the symbol table
    # ---------------------------------------------------------------
    println("[Step 1] Finding vtable labels in symbol table...")
    monitor.setMessage("Finding vtable labels...")

    vtable_symbols = []

    sym_iter = st.getAllSymbols(True)
    sym_count = 0
    while sym_iter.hasNext():
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        sym = sym_iter.next()
        sym_count += 1
        if sym_count % 10000 == 0:
            monitor.setMessage("Scanning symbols: %d..." % sym_count)

        sym_name = sym.getName()
        if sym_name is None:
            continue

        # Match vtable labels (case-insensitive prefix check)
        lower_name = sym_name.lower()
        if (lower_name.startswith("vtable_") or
            lower_name.startswith("vftable_") or
            lower_name.startswith("vtbl_")):
            vtable_symbols.append((sym_name, sym.getAddress()))

    println("  Found %d vtable labels (scanned %d symbols)" % (len(vtable_symbols), sym_count))

    if not vtable_symbols:
        println("")
        println("No vtable labels found. Run script 01 (RTTI annotator) first to label vtables.")
        println("Script 07 complete (nothing to do).")
        return

    println("")

    # ---------------------------------------------------------------
    # STEP 2: Process each vtable
    # ---------------------------------------------------------------
    println("[Step 2] Processing vtables and naming virtual functions...")
    monitor.setMessage("Processing vtables...")
    monitor.setMaximum(len(vtable_symbols))

    # Sort by address for deterministic processing
    vtable_symbols.sort(key=lambda x: x[1].getOffset())

    # Build a set of all vtable addresses for boundary detection
    vtable_addrs = set()
    for _, addr in vtable_symbols:
        vtable_addrs.add(addr.getOffset())

    for vt_idx, (vt_name, vt_addr) in enumerate(vtable_symbols):
        monitor.setProgress(vt_idx)
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        class_name = clean_class_name(vt_name)
        if not class_name:
            vtables_skipped += 1
            continue

        # Read consecutive pointer entries from the vtable
        entry_count = 0
        named_count = 0
        skipped_count = 0
        current_offset = vt_addr

        # Safety limit: no vtable should have more than 500 entries
        MAX_VTABLE_ENTRIES = 500

        while entry_count < MAX_VTABLE_ENTRIES:
            if monitor.isCancelled():
                return

            # Calculate address of this vtable slot
            slot_addr = current_offset

            # Check if we've hit another vtable label (boundary)
            if entry_count > 0:
                slot_offset = slot_addr.getOffset()
                if slot_offset in vtable_addrs:
                    break

                # Check if there's any non-vtable label at this address
                syms_at = st.getSymbols(slot_addr)
                if syms_at:
                    has_user_label = False
                    for s in syms_at:
                        s_name = s.getName()
                        if (s.getSource() == SourceType.USER_DEFINED and
                            not s_name.startswith("DAT_") and
                            not s_name.startswith("ADDR_")):
                            has_user_label = True
                            break
                    if has_user_label:
                        break

            # Read the pointer value at this slot
            ptr_val = read_pointer(listing, mem, slot_addr)
            if ptr_val is None:
                break  # Can't read memory

            if ptr_val == 0:
                break  # NULL terminator

            # Convert pointer value to an address
            try:
                target_addr = default_space.getAddress(ptr_val & 0xFFFFFFFF)
            except:
                break

            # Check if this looks like a code address
            if not is_code_address(target_addr, mem, default_space):
                break

            total_entries_read += 1
            entry_count += 1

            # Get or create a function at the target address
            func = fm.getFunctionAt(target_addr)
            if func is None:
                # Try to create a function here
                try:
                    from ghidra.app.cmd.function import CreateFunctionCmd
                    cmd = CreateFunctionCmd(target_addr)
                    cmd.applyTo(currentProgram)
                    func = fm.getFunctionAt(target_addr)
                except:
                    pass

            if func is None:
                # Still no function -- skip this entry but continue reading
                current_offset = slot_addr.add(4)
                continue

            # Check if the function already has a user-defined name
            if not is_default_name(func):
                # Already named (possibly from a parent class vtable or another script)
                skipped_count += 1
                already_named_from_parent += 1
                total_vfuncs_skipped += 1

                # Add a comment noting this vtable also references it
                plate = func.getComment()
                vt_note = "Also in vtable: %s (slot %d)" % (class_name, entry_count - 1)
                if plate is None:
                    func.setComment(vt_note)
                elif class_name not in plate:
                    func.setComment(plate + "\n" + vt_note)

                current_offset = slot_addr.add(4)
                continue

            # Name the function as ClassName::vfunc_N
            vfunc_name = "%s__vfunc_%d" % (class_name, entry_count - 1)

            # Ensure uniqueness
            if vfunc_name in assigned_names:
                # This shouldn't happen often, but handle it
                vfunc_name = "%s__vfunc_%d_at_%s" % (
                    class_name, entry_count - 1,
                    target_addr.toString().replace("0x", ""))
            assigned_names.add(vfunc_name)

            try:
                func.setName(vfunc_name, SourceType.USER_DEFINED)
                named_count += 1
                total_vfuncs_named += 1

                # Add plate comment
                comment = "[VTable] %s virtual function #%d\nVTable: %s at %s" % (
                    class_name, entry_count - 1, vt_name, vt_addr.toString())
                func.setComment(comment)
            except Exception as e:
                # Name conflict with existing symbol
                try:
                    alt_name = "%s__vfunc_%d_at_%s" % (
                        class_name, entry_count - 1,
                        target_addr.toString().replace("0x", ""))
                    func.setName(alt_name, SourceType.USER_DEFINED)
                    named_count += 1
                    total_vfuncs_named += 1
                    assigned_names.add(alt_name)
                except:
                    pass

            current_offset = slot_addr.add(4)

        # Record stats for this vtable
        if entry_count > 0:
            vtable_sizes[class_name] = entry_count
            vtables_processed += 1

            if named_count > 0 or entry_count > 10:
                println("  %s: %d entries (%d named, %d already named)" % (
                    class_name, entry_count, named_count, skipped_count))
        else:
            vtables_skipped += 1

    println("")

    # ---------------------------------------------------------------
    # Summary
    # ---------------------------------------------------------------
    println("=" * 70)
    println("VTable Annotator - Summary")
    println("=" * 70)
    println("")
    println("VTables processed:       %d" % vtables_processed)
    println("VTables skipped:         %d" % vtables_skipped)
    println("Total vfunc entries:     %d" % total_entries_read)
    println("Functions renamed:       %d" % total_vfuncs_named)
    println("Already named (kept):    %d" % total_vfuncs_skipped)
    println("")

    # Show vtables with the most entries
    if vtable_sizes:
        sorted_vtables = sorted(vtable_sizes.items(), key=lambda x: x[1], reverse=True)
        println("Largest vtables (top 20):")
        for class_name, count in sorted_vtables[:20]:
            println("  %-45s %3d entries" % (class_name, count))
        println("")

        # Size distribution
        small = sum(1 for c in vtable_sizes.values() if c <= 5)
        medium = sum(1 for c in vtable_sizes.values() if 5 < c <= 20)
        large = sum(1 for c in vtable_sizes.values() if 20 < c <= 50)
        xlarge = sum(1 for c in vtable_sizes.values() if c > 50)
        println("VTable size distribution:")
        println("  1-5 entries:    %d vtables" % small)
        println("  6-20 entries:   %d vtables" % medium)
        println("  21-50 entries:  %d vtables" % large)
        println("  51+ entries:    %d vtables" % xlarge)
        println("")

    println("Script 07 complete.")


# Entry point
main()
