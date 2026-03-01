# 09_string_discovery.py
# Ghidra Jython script for SGW.exe reverse engineering
#
# Purpose: Broad string-based function naming. Finds remaining debug strings
# containing "::" separators (indicating C++ class::method patterns) and uses
# them to name their containing functions. This catches symbols not handled
# by the more targeted scripts 03-06.
#
# Expected yield: 500-2000 functions named (LOW-MED confidence)
#
# Run order: After scripts 01-08 (this catches what they missed).
#            Before script 10 (xref propagation benefits from more named functions).
#
# @category SGW
# @author Cimmeria Project

"""
String Discovery Annotator

Iterates all defined strings in SGW.exe looking for C++ class::method patterns.
Filters out strings already handled by earlier scripts (Event_Net*, CME::,
Mercury::, BigWorld paths) and names functions based on the remaining debug
strings. This is a broad sweep that picks up miscellaneous debug/assert strings
containing class and method names.

Handles duplicates (multiple functions referencing the same string) by adding
index suffixes. Adds plate comments with the full original string for context.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.util import DefinedDataIterator

import re


def is_default_name(func):
    """Check if a function has a default auto-generated name."""
    name = func.getName()
    return (name.startswith("FUN_") or
            name.startswith("thunk_FUN_") or
            name.startswith("Ordinal_") or
            name.startswith("LAB_"))


def sanitize_for_symbol(raw):
    """
    Clean a class::method string into a valid Ghidra symbol name.
    Input like "MyClass::doThing" becomes "MyClass_doThing".
    """
    # Replace :: with underscore
    name = raw.replace("::", "_")
    # Replace other separators
    name = name.replace(" ", "_").replace("-", "_").replace(".", "_")
    name = name.replace("<", "_").replace(">", "_")
    name = name.replace(",", "_").replace("(", "").replace(")", "")
    # Strip non-alphanumeric except underscore
    name = re.sub(r'[^A-Za-z0-9_]', '', name)
    # Collapse multiple underscores
    name = re.sub(r'_+', '_', name)
    # Strip leading/trailing underscores
    name = name.strip('_')
    # Truncate to reasonable length
    if len(name) > 120:
        name = name[:120]
    # Ensure doesn't start with digit
    if name and name[0].isdigit():
        name = "cls_" + name
    return name


def extract_class_method(s):
    """
    Extract the first class::method pattern from a string.
    Returns (class_name, method_name, full_match) or None.

    Handles patterns like:
      "MyClass::doSomething"
      "Namespace::Class::method"
      "SomeClass::method: error occurred"
      "Failed in Foo::Bar::baz()"
    """
    # Match word::word patterns, optionally with more :: segments
    # The pattern captures the full chain like A::B::C
    match = re.search(r'\b([A-Za-z_]\w*(?:::[A-Za-z_]\w*)+)\b', s)
    if match:
        full = match.group(1)
        parts = full.split("::")
        if len(parts) >= 2:
            class_name = "::".join(parts[:-1])
            method_name = parts[-1]
            return (class_name, method_name, full)
    return None


def should_exclude(s, class_method_full):
    """
    Check if this string should be excluded because it's handled by earlier scripts.
    Returns True if the string should be skipped.
    """
    s_lower = s.lower()
    full_lower = class_method_full.lower()

    # Script 04: Event_NetOut_* / Event_NetIn_*
    if "event_netout_" in s_lower or "event_netin_" in s_lower:
        return True

    # Script 05: Mercury::
    if full_lower.startswith("mercury::") or "mercury::" in full_lower:
        return True

    # Script 06: CME::
    if full_lower.startswith("cme::") or "cme::" in full_lower:
        return True

    # Script 03: BigWorld source paths (contain backslashes)
    if "\\bigworld\\" in s_lower or "\\server\\" in s_lower:
        return True
    if "..\\..\\server\\" in s_lower or "..\\..\\src\\" in s_lower:
        return True

    # File paths (forward or back slashes)
    if "\\" in s or ("/" in s and "::" in s and s.index("/") < s.index("::")):
        # Only exclude if the path portion comes before the class::method
        pass  # Actually, let's be more selective

    # Exclude if the string is primarily a file path
    if re.match(r'^[A-Za-z]:\\', s) or s.startswith("..\\") or s.startswith("../"):
        return True
    if s.startswith("/"):
        return True

    # Script 02: UE3 exec stubs
    if s.startswith("exec"):
        return True

    # Common C++ standard library noise
    std_excludes = [
        "std::", "stlp_std::", "__gnu_cxx::", "boost::", "_STL::",
        "stdext::", "atl::", "wtl::",
    ]
    for exc in std_excludes:
        if full_lower.startswith(exc.lower()):
            return True

    # MSVC internal names
    if full_lower.startswith("_crt") or full_lower.startswith("__crt"):
        return True

    return False


def main():
    monitor = getMonitor()
    fm = currentProgram.getFunctionManager()
    listing = currentProgram.getListing()

    println("=" * 70)
    println("String Discovery Annotator - Script 09")
    println("=" * 70)
    println("")

    # Statistics
    total_strings_scanned = 0
    strings_with_colons = 0
    strings_excluded = 0
    strings_qualifying = 0
    functions_renamed = 0
    functions_already_named = 0
    functions_commented = 0

    # Track class frequencies for reporting
    class_counts = {}  # class_name -> count of functions renamed

    # Track name assignments for duplicate handling
    name_assignments = {}  # sanitized_name -> count

    # Track which strings map to which functions (for duplicate detection)
    string_to_funcs = {}  # (class, method) -> list of function addresses

    # ---------------------------------------------------------------
    # PASS 1: Collect all qualifying strings and their xref targets
    # ---------------------------------------------------------------
    println("[Pass 1] Scanning all defined strings for class::method patterns...")
    monitor.setMessage("Pass 1: Scanning strings...")

    qualifying_strings = []  # (class_name, method_name, full_match, original_string, string_addr)

    data_iter = DefinedDataIterator.definedStrings(currentProgram)

    while data_iter.hasNext():
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        data = data_iter.next()
        total_strings_scanned += 1

        if total_strings_scanned % 5000 == 0:
            monitor.setMessage("Pass 1: Scanned %d strings..." % total_strings_scanned)

        try:
            val = data.getDefaultValueRepresentation()
            if val is None:
                continue
            # Strip surrounding quotes
            if val.startswith('"') and val.endswith('"'):
                val = val[1:-1]
        except:
            continue

        # Quick filter: must contain ::
        if "::" not in val:
            continue

        strings_with_colons += 1

        # Length filter
        if len(val) < 5 or len(val) > 200:
            continue

        # Extract class::method pattern
        result = extract_class_method(val)
        if result is None:
            continue

        class_name, method_name, full_match = result

        # Validate the components look like real identifiers
        # Class name parts should start with uppercase (C++ convention) or be
        # known lowercase namespaces
        class_parts = class_name.split("::")
        valid = True
        for part in class_parts:
            if not re.match(r'^[A-Za-z_]\w*$', part):
                valid = False
                break
        if not valid:
            continue

        if not re.match(r'^[A-Za-z_]\w*$', method_name):
            continue

        # Exclusion checks
        if should_exclude(val, full_match):
            strings_excluded += 1
            continue

        strings_qualifying += 1
        qualifying_strings.append((class_name, method_name, full_match, val, data.getAddress()))

    println("  Total strings scanned:    %d" % total_strings_scanned)
    println("  Strings with '::':        %d" % strings_with_colons)
    println("  Excluded (prior scripts): %d" % strings_excluded)
    println("  Qualifying strings:       %d" % strings_qualifying)
    println("")

    if not qualifying_strings:
        println("No qualifying strings found. Earlier scripts may have covered everything.")
        println("Script 09 complete.")
        return

    # ---------------------------------------------------------------
    # PASS 2: Map strings to functions via xrefs
    # ---------------------------------------------------------------
    println("[Pass 2] Mapping qualifying strings to functions via xrefs...")
    monitor.setMessage("Pass 2: Resolving xrefs...")
    monitor.setMaximum(len(qualifying_strings))

    # Build mapping: (full_match) -> list of (func, string_addr, original_string)
    match_to_funcs = {}

    for idx, (class_name, method_name, full_match, original_str, str_addr) in enumerate(qualifying_strings):
        monitor.setProgress(idx)
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        refs = getReferencesTo(str_addr)
        if refs is None:
            continue

        for ref in refs:
            from_addr = ref.getFromAddress()
            func = getFunctionContaining(from_addr)
            if func is None:
                continue

            key = full_match
            if key not in match_to_funcs:
                match_to_funcs[key] = []
            match_to_funcs[key].append((func, str_addr, original_str, class_name, method_name))

    println("  Mapped %d unique class::method patterns to functions" % len(match_to_funcs))
    println("")

    # ---------------------------------------------------------------
    # PASS 3: Rename functions
    # ---------------------------------------------------------------
    println("[Pass 3] Renaming functions...")
    monitor.setMessage("Pass 3: Renaming functions...")
    monitor.setMaximum(len(match_to_funcs))

    processed = 0
    for full_match, func_list in sorted(match_to_funcs.items()):
        processed += 1
        monitor.setProgress(processed)
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        # Deduplicate by function entry point
        seen_addrs = set()
        unique_funcs = []
        for item in func_list:
            func = item[0]
            addr = func.getEntryPoint().getOffset()
            if addr not in seen_addrs:
                seen_addrs.add(addr)
                unique_funcs.append(item)

        for func_idx, (func, str_addr, original_str, class_name, method_name) in enumerate(unique_funcs):
            if not is_default_name(func):
                # Already named -- just add comment
                functions_already_named += 1
                plate = func.getComment()
                if plate is None or full_match not in str(plate or ""):
                    comment = "[String discovery] References: \"%s\"" % full_match
                    if plate:
                        comment = plate + "\n" + comment
                    func.setComment(comment)
                    functions_commented += 1
                continue

            # Build the function name
            base_name = sanitize_for_symbol(full_match)
            if not base_name or len(base_name) < 3:
                continue

            # Handle multiple functions referencing the same string
            if len(unique_funcs) > 1:
                # Append index for disambiguation
                func_name = "%s_%d" % (base_name, func_idx + 1)
            else:
                func_name = base_name

            # Handle global name conflicts
            if func_name in name_assignments:
                name_assignments[func_name] += 1
                func_name = "%s_x%d" % (func_name, name_assignments[func_name])
            else:
                name_assignments[func_name] = 0

            try:
                func.setName(func_name, SourceType.USER_DEFINED)
                functions_renamed += 1

                # Track class for statistics
                top_class = class_name.split("::")[0]
                class_counts[top_class] = class_counts.get(top_class, 0) + 1

                # Add plate comment with full context
                comment = "[String discovery] Debug string: \"%s\"" % original_str[:150]
                if len(unique_funcs) > 1:
                    comment += "\nNote: %d functions reference this string (this is #%d)" % (
                        len(unique_funcs), func_idx + 1)
                comment += "\nString at: %s" % str_addr.toString()
                func.setComment(comment)
                functions_commented += 1

                println("  %s -> %s" % (func.getEntryPoint().toString(), func_name))
            except Exception as e:
                # Name conflict -- try with address suffix
                try:
                    addr_suffix = func.getEntryPoint().toString().replace("0x", "")
                    fallback = "%s_at_%s" % (base_name, addr_suffix)
                    func.setName(fallback, SourceType.USER_DEFINED)
                    functions_renamed += 1

                    top_class = class_name.split("::")[0]
                    class_counts[top_class] = class_counts.get(top_class, 0) + 1

                    comment = "[String discovery] Debug string: \"%s\"\nString at: %s" % (
                        original_str[:150], str_addr.toString())
                    func.setComment(comment)
                    functions_commented += 1
                except:
                    pass

    println("")

    # ---------------------------------------------------------------
    # Summary
    # ---------------------------------------------------------------
    println("=" * 70)
    println("String Discovery Annotator - Summary")
    println("=" * 70)
    println("")
    println("Strings scanned:           %d" % total_strings_scanned)
    println("Strings with '::':         %d" % strings_with_colons)
    println("Excluded (prior scripts):  %d" % strings_excluded)
    println("Qualifying patterns:       %d" % strings_qualifying)
    println("Unique patterns with xrefs:%d" % len(match_to_funcs))
    println("")
    println("Functions renamed:         %d" % functions_renamed)
    println("Already named (skipped):   %d" % functions_already_named)
    println("Comments added:            %d" % functions_commented)
    println("")

    if class_counts:
        # Sort by count descending
        sorted_classes = sorted(class_counts.items(), key=lambda x: x[1], reverse=True)
        println("Top classes by renamed function count:")
        for class_name, count in sorted_classes[:30]:
            println("  %-40s %3d functions" % (class_name, count))
        println("")

        println("Total unique top-level classes: %d" % len(class_counts))
        println("")

        # Distribution
        single = sum(1 for c in class_counts.values() if c == 1)
        small = sum(1 for c in class_counts.values() if 2 <= c <= 5)
        medium = sum(1 for c in class_counts.values() if 6 <= c <= 20)
        large = sum(1 for c in class_counts.values() if c > 20)
        println("Class size distribution:")
        println("  1 function:     %d classes" % single)
        println("  2-5 functions:  %d classes" % small)
        println("  6-20 functions: %d classes" % medium)
        println("  20+ functions:  %d classes" % large)
        println("")

    println("Script 09 complete.")


# Entry point
main()
