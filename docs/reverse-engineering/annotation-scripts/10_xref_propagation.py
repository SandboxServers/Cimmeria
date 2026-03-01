# 10_xref_propagation.py
# Ghidra Jython script for SGW.exe reverse engineering
#
# Purpose: Infer function class membership from call graph context. If a
# function calls 3+ already-named functions from the same class, it's likely
# a method of that class too. This is the lowest-confidence annotation script
# and runs last to maximize the number of named anchors available.
#
# Expected yield: 1000-3000 functions named (LOW confidence)
#
# Run order: LAST (script 10 of 10). Benefits from all prior scripts.
#
# @category SGW
# @author Cimmeria Project

"""
Xref Propagation Annotator

Infers function class membership from the call graph. For each unnamed function:
  1. Examines outgoing calls (functions it calls)
  2. Examines incoming calls (functions that call it)
  3. If >=3 named callees/callers share a class prefix AND that class represents
     >50% of all named callees/callers, infers class membership

Runs up to 3 iterative passes since naming one function may unlock inference
for others. Reports per-pass statistics and convergence.

All renamings are LOW confidence and use the naming pattern:
  ClassName__unknown_HEXADDR
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.symbol import RefType

import re


def is_default_name(func):
    """Check if a function has a default auto-generated name."""
    name = func.getName()
    return (name.startswith("FUN_") or
            name.startswith("thunk_FUN_") or
            name.startswith("Ordinal_") or
            name.startswith("LAB_"))


def is_thunk(func):
    """Check if a function is a thunk (wrapper/trampoline)."""
    return func.isThunk()


def extract_class_prefix(func_name):
    """
    Extract the class prefix from a named function.

    Handles naming conventions from all prior scripts:
      - "ClassName_method"         -> "ClassName"
      - "ClassName__method"        -> "ClassName"
      - "ClassName__vfunc_N"       -> "ClassName"
      - "CME_ClassName_method"     -> "CME_ClassName"
      - "lua_ClassName_method"     -> "lua_ClassName"
      - "Event_NetOut_xxx"         -> "Event_NetOut"
      - "Event_NetIn_xxx"          -> "Event_NetIn"
      - "Mercury_xxx"              -> "Mercury"
      - "BigWorld_xxx"             -> "BigWorld"
      - "ClassName__unknown_ADDR"  -> "ClassName" (from prior passes)

    Returns the class prefix or None if it can't be determined.
    """
    if not func_name:
        return None

    # Skip names that are too short to have a meaningful class prefix
    if len(func_name) < 5:
        return None

    # Skip system/library function names
    if func_name.startswith("_") and not func_name.startswith("__"):
        return None

    # Handle double-underscore separator (used by vtable annotator and others)
    if "__" in func_name:
        parts = func_name.split("__")
        if len(parts) >= 2 and len(parts[0]) >= 3:
            # Special case: skip if first part is just "vfunc", "unknown", etc.
            if parts[0] in ("vfunc", "unknown", "at", "thunk"):
                return None
            return parts[0]

    # Handle single underscore separator
    # Be careful: many names use _ as word separator within a class name
    # Heuristic: split on _ and take up to the first lowercase-starting part
    # after an uppercase-starting part, since class names typically start with
    # uppercase (CamelCase)
    parts = func_name.split("_")
    if len(parts) < 2:
        return None

    # Known multi-part prefixes
    if len(parts) >= 3:
        two_part = parts[0] + "_" + parts[1]
        if two_part in ("Event_NetOut", "Event_NetIn"):
            return two_part
        if parts[0] in ("CME", "lua", "tolua"):
            if len(parts) >= 3:
                return parts[0] + "_" + parts[1]

    # For single-word class names: take the first part if it looks like a class name
    if len(parts[0]) >= 3 and parts[0][0].isupper():
        return parts[0]

    # For lowercase prefixes that are known namespaces
    known_prefixes = {"mercury", "bigworld", "bw", "cme", "sgw", "ue", "lua", "tolua"}
    if parts[0].lower() in known_prefixes:
        return parts[0]

    return None


def get_outgoing_calls(func, monitor):
    """Get all functions called by the given function."""
    called_funcs = []
    body = func.getBody()
    if body is None:
        return called_funcs

    ref_mgr = currentProgram.getReferenceManager()
    addr_iter = body.getAddresses(True)

    while addr_iter.hasNext():
        if monitor.isCancelled():
            return called_funcs

        addr = addr_iter.next()
        refs = ref_mgr.getReferencesFrom(addr)
        if refs is None:
            continue

        for ref in refs:
            if ref.getReferenceType().isCall():
                target = ref.getToAddress()
                fm = currentProgram.getFunctionManager()
                target_func = fm.getFunctionAt(target)
                if target_func is not None and not is_thunk(target_func):
                    called_funcs.append(target_func)

    return called_funcs


def get_incoming_callers(func, monitor):
    """Get all functions that call the given function."""
    callers = []
    entry = func.getEntryPoint()
    refs = getReferencesTo(entry)
    if refs is None:
        return callers

    fm = currentProgram.getFunctionManager()
    for ref in refs:
        if monitor.isCancelled():
            return callers

        if ref.getReferenceType().isCall():
            from_addr = ref.getFromAddress()
            caller = fm.getFunctionContaining(from_addr)
            if caller is not None and not is_thunk(caller):
                callers.append(caller)

    return callers


def count_class_prefixes(func_list):
    """
    Count class prefixes among a list of named functions.
    Returns: dict of prefix -> count, and total named functions counted.
    """
    prefix_counts = {}
    total_named = 0

    for func in func_list:
        name = func.getName()
        if is_default_name(func):
            continue

        prefix = extract_class_prefix(name)
        if prefix is None:
            continue

        total_named += 1
        prefix_counts[prefix] = prefix_counts.get(prefix, 0) + 1

    return prefix_counts, total_named


def main():
    monitor = getMonitor()
    fm = currentProgram.getFunctionManager()

    println("=" * 70)
    println("Xref Propagation Annotator - Script 10")
    println("=" * 70)
    println("")

    # Configuration
    MIN_SAME_CLASS_CALLS = 3      # Minimum callees/callers from same class
    MIN_RATIO = 0.50              # Must be >50% of all named callees/callers
    MAX_PASSES = 3                # Maximum inference passes
    CALLEE_WEIGHT = 1.0           # Weight for outgoing call evidence
    CALLER_WEIGHT = 0.7           # Weight for incoming call evidence (less reliable)

    println("Configuration:")
    println("  Min same-class references: %d" % MIN_SAME_CLASS_CALLS)
    println("  Min ratio threshold:       %.0f%%" % (MIN_RATIO * 100))
    println("  Max passes:                %d" % MAX_PASSES)
    println("")

    # Global statistics
    total_renamed = 0
    total_commented = 0
    classes_inferred = set()
    per_pass_stats = []

    # ---------------------------------------------------------------
    # Build initial function inventory
    # ---------------------------------------------------------------
    println("[Setup] Building function inventory...")
    monitor.setMessage("Building function inventory...")

    all_functions = []
    func_iter = fm.getFunctions(True)
    while func_iter.hasNext():
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        func = func_iter.next()
        if not is_thunk(func):
            all_functions.append(func)

    total_funcs = len(all_functions)
    named_before = sum(1 for f in all_functions if not is_default_name(f))
    unnamed_before = total_funcs - named_before

    println("  Total non-thunk functions: %d" % total_funcs)
    println("  Named:                     %d" % named_before)
    println("  Unnamed (FUN_*):           %d" % unnamed_before)
    println("")

    if unnamed_before == 0:
        println("No unnamed functions to process.")
        println("Script 10 complete.")
        return

    # Track which functions we've renamed to avoid re-processing
    renamed_this_session = set()  # entry point offsets

    # ---------------------------------------------------------------
    # Iterative passes
    # ---------------------------------------------------------------
    for pass_num in range(1, MAX_PASSES + 1):
        println("=" * 50)
        println("[Pass %d of %d]" % (pass_num, MAX_PASSES))
        println("=" * 50)
        monitor.setMessage("Pass %d: Analyzing call graphs..." % pass_num)

        pass_renamed = 0
        pass_classes = set()
        candidates = []

        # Collect unnamed functions for this pass
        unnamed_funcs = []
        for func in all_functions:
            if is_default_name(func):
                entry_offset = func.getEntryPoint().getOffset()
                if entry_offset not in renamed_this_session:
                    unnamed_funcs.append(func)

        println("  Unnamed functions to analyze: %d" % len(unnamed_funcs))
        monitor.setMaximum(len(unnamed_funcs))

        for idx, func in enumerate(unnamed_funcs):
            monitor.setProgress(idx)
            if monitor.isCancelled():
                println("Cancelled by user.")
                return

            if idx % 1000 == 0 and idx > 0:
                monitor.setMessage("Pass %d: Analyzed %d/%d functions..." % (
                    pass_num, idx, len(unnamed_funcs)))

            entry_addr = func.getEntryPoint()

            # --- Analyze outgoing calls ---
            callees = get_outgoing_calls(func, monitor)
            if monitor.isCancelled():
                return

            callee_prefixes, callee_named_total = count_class_prefixes(callees)

            # --- Analyze incoming callers ---
            callers = get_incoming_callers(func, monitor)
            if monitor.isCancelled():
                return

            caller_prefixes, caller_named_total = count_class_prefixes(callers)

            # --- Combine evidence ---
            # Merge prefix counts with weighting
            combined = {}
            for prefix, count in callee_prefixes.items():
                combined[prefix] = combined.get(prefix, 0.0) + count * CALLEE_WEIGHT
            for prefix, count in caller_prefixes.items():
                combined[prefix] = combined.get(prefix, 0.0) + count * CALLER_WEIGHT

            if not combined:
                continue

            # Find the dominant class prefix
            best_prefix = max(combined, key=combined.get)
            best_score = combined[best_prefix]

            # Raw counts for threshold checking
            callee_count = callee_prefixes.get(best_prefix, 0)
            caller_count = caller_prefixes.get(best_prefix, 0)
            total_raw = callee_count + caller_count

            # Need at least MIN_SAME_CLASS_CALLS raw references
            if total_raw < MIN_SAME_CLASS_CALLS:
                continue

            # Check ratio: best prefix must dominate
            total_named = callee_named_total + caller_named_total
            if total_named == 0:
                continue

            ratio = float(total_raw) / float(total_named)
            if ratio < MIN_RATIO:
                continue

            # This function is a candidate for class inference
            candidates.append((func, best_prefix, callee_count, caller_count, ratio))

        # Sort candidates by confidence (higher ratio and count = better)
        candidates.sort(key=lambda x: (x[4], x[2] + x[3]), reverse=True)

        println("  Candidates found: %d" % len(candidates))

        # Apply renamings
        for func, prefix, callee_count, caller_count, ratio in candidates:
            if monitor.isCancelled():
                return

            # Double-check it's still unnamed (a prior candidate in this pass
            # might have named it via a different path)
            if not is_default_name(func):
                continue

            entry_addr = func.getEntryPoint()
            addr_hex = entry_addr.toString().replace("0x", "")

            new_name = "%s__unknown_%s" % (prefix, addr_hex)

            try:
                func.setName(new_name, SourceType.USER_DEFINED)
                pass_renamed += 1
                total_renamed += 1
                pass_classes.add(prefix)
                classes_inferred.add(prefix)
                renamed_this_session.add(entry_addr.getOffset())

                # Build detailed comment
                comment_lines = [
                    "[Xref propagation - LOW confidence]",
                    "Inferred class: %s" % prefix,
                    "Evidence: %d callees + %d callers from this class (%.0f%% of named refs)" % (
                        callee_count, caller_count, ratio * 100),
                    "Pass: %d" % pass_num,
                ]
                func.setComment("\n".join(comment_lines))
                total_commented += 1

            except Exception as e:
                # Name conflict
                try:
                    alt_name = "%s__inferred_%s" % (prefix, addr_hex)
                    func.setName(alt_name, SourceType.USER_DEFINED)
                    pass_renamed += 1
                    total_renamed += 1
                    pass_classes.add(prefix)
                    classes_inferred.add(prefix)
                    renamed_this_session.add(entry_addr.getOffset())
                except:
                    pass

        per_pass_stats.append({
            'pass': pass_num,
            'analyzed': len(unnamed_funcs),
            'candidates': len(candidates),
            'renamed': pass_renamed,
            'classes': len(pass_classes),
        })

        println("  Pass %d results: %d functions renamed across %d classes" % (
            pass_num, pass_renamed, len(pass_classes)))
        println("")

        # Check for convergence: if this pass renamed nothing, stop
        if pass_renamed == 0:
            println("  Convergence reached -- no new renamings in pass %d." % pass_num)
            break

    println("")

    # ---------------------------------------------------------------
    # Summary
    # ---------------------------------------------------------------
    println("=" * 70)
    println("Xref Propagation Annotator - Summary")
    println("=" * 70)
    println("")

    # Per-pass breakdown
    println("Per-pass results:")
    println("  %-6s %-10s %-12s %-10s %-10s" % (
        "Pass", "Analyzed", "Candidates", "Renamed", "Classes"))
    println("  " + "-" * 48)
    for stats in per_pass_stats:
        println("  %-6d %-10d %-12d %-10d %-10d" % (
            stats['pass'], stats['analyzed'], stats['candidates'],
            stats['renamed'], stats['classes']))
    println("")

    # Overall stats
    named_after = named_before + total_renamed
    println("Overall statistics:")
    println("  Functions named before:  %d (%.1f%%)" % (
        named_before, 100.0 * named_before / total_funcs if total_funcs else 0))
    println("  Functions renamed:       %d" % total_renamed)
    println("  Functions named after:   %d (%.1f%%)" % (
        named_after, 100.0 * named_after / total_funcs if total_funcs else 0))
    println("  Comments added:          %d" % total_commented)
    println("  Unique classes inferred: %d" % len(classes_inferred))
    println("")

    if classes_inferred:
        # Count how many functions per inferred class
        class_func_counts = {}
        for func in all_functions:
            name = func.getName()
            if "__unknown_" in name or "__inferred_" in name:
                prefix = extract_class_prefix(name)
                if prefix:
                    class_func_counts[prefix] = class_func_counts.get(prefix, 0) + 1

        sorted_classes = sorted(class_func_counts.items(), key=lambda x: x[1], reverse=True)
        println("Inferred classes (top 30 by function count):")
        for cls, cnt in sorted_classes[:30]:
            println("  %-40s %3d functions" % (cls, cnt))
        println("")

    println("IMPORTANT: All renamings from this script are LOW confidence.")
    println("They indicate likely class membership based on call graph patterns")
    println("but should be verified through decompilation before relying on them.")
    println("")
    println("Script 10 complete.")


# Entry point
main()
