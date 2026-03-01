# 03_bigworld_source_annotator.py
# Ghidra Jython script for Stargate Worlds (SGW.exe) reverse engineering
#
# Purpose:
#   Find BigWorld engine source path strings embedded in the binary from
#   debug assertions, logging, and error messages. These paths look like:
#     ..\..\Server\bigworld\src\lib\connection\servconn.cpp
#     ..\..\bigworld\src\lib\network\bundle.cpp
#   They precisely identify which BigWorld source file the code originated
#   from. This script labels functions based on these paths and also
#   searches for "Mercury::" debug strings for the Mercury networking layer.
#
# BigWorld Client Architecture Note:
#   SGW.exe is a BigWorld client binary. The BigWorld client SDK compiles
#   engine source directly into the executable. Debug/assertion macros
#   embed __FILE__ paths as string literals, giving us a direct mapping
#   from binary function -> original source file.
#
# Expected yield: 100-300 functions named or annotated
# Confidence: HIGH-MEDIUM (path strings are reliable; function naming is
#   contextual and may require manual refinement)
#
# Usage: Run from Ghidra Script Manager (Jython)

"""
BigWorld Source Path Annotator for SGW.exe

Scans for embedded BigWorld engine source file paths (from __FILE__ macros
in assertions/logging), traces them to containing functions, and labels
those functions based on the source context. Also finds Mercury:: debug
strings for the networking layer.
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
    """Check if a function has an auto-generated name."""
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
    # Collapse multiple underscores
    sanitized = re.sub(r'_+', '_', sanitized)
    # Strip trailing underscores
    sanitized = sanitized.rstrip('_')
    return sanitized


def add_plate_comment(func, comment):
    """Add or append to a function's plate comment."""
    existing = func.getComment()
    if existing is None:
        func.setComment(comment)
    elif comment not in existing:
        func.setComment(existing + "\n" + comment)


def extract_bw_source_info(path_string):
    """Extract meaningful source information from a BigWorld source path.

    Input examples:
      ..\..\Server\bigworld\src\lib\connection\servconn.cpp
      ..\..\bigworld\src\lib\network\bundle.cpp
      c:\bigworld\src\server\baseapp\entity.cpp
      ..\src\lib\entitydef\data_types.cpp

    Returns: (module, filename, stem) or None
      module: e.g., "connection", "network", "entitydef", "baseapp"
      filename: e.g., "servconn.cpp"
      stem: e.g., "servconn"
    """
    # Normalize path separators
    normalized = path_string.replace('\\', '/').lower()

    # Try to extract the BigWorld source path components
    # Pattern: .../bigworld/src/lib/<module>/<file>.cpp
    # Pattern: .../bigworld/src/server/<app>/<file>.cpp
    # Pattern: .../bigworld/src/common/<module>/<file>.cpp
    match = re.search(
        r'(?:bigworld|bwclient\d*)/src/(?:lib|server|client|common)/([^/]+)/([^/]+\.(?:cpp|hpp|h|c|inl))',
        normalized
    )
    if match:
        module = match.group(1)
        filename = match.group(2)
        stem = filename.rsplit('.', 1)[0]
        return (module, filename, stem)

    # Simpler pattern: just a source file in a recognizable directory
    match = re.search(r'/([^/]+)/([^/]+\.(?:cpp|hpp|h|c|inl))$', normalized)
    if match:
        module = match.group(1)
        filename = match.group(2)
        stem = filename.rsplit('.', 1)[0]
        return (module, filename, stem)

    # Fallback: just the filename
    match = re.search(r'([^/\\]+\.(?:cpp|hpp|h|c|inl))$', normalized)
    if match:
        filename = match.group(1)
        stem = filename.rsplit('.', 1)[0]
        return (None, filename, stem)

    return None


def generate_function_name(module, stem, func_addr, existing_names):
    """Generate a unique function name based on source file context.

    Uses module and stem to create a descriptive name like:
      BW_connection_servconn
      BW_network_bundle
    """
    if module:
        base_name = "BW_%s_%s" % (module, stem)
    else:
        base_name = "BW_%s" % stem

    safe_name = sanitize_label(base_name)

    # Ensure uniqueness by appending a counter if needed
    if safe_name not in existing_names:
        existing_names.add(safe_name)
        return safe_name

    counter = 2
    while True:
        candidate = "%s_%d" % (safe_name, counter)
        if candidate not in existing_names:
            existing_names.add(candidate)
            return candidate
        counter += 1


def extract_mercury_info(mercury_string):
    """Extract class and method information from Mercury:: debug strings.

    Input examples:
      Mercury::Channel::send
      Mercury::Bundle::startMessage
      Mercury::NetworkInterface::processPacket

    Returns: (class_name, method_name) or None
    """
    match = re.match(r'Mercury::(\w+)(?:::(\w+))?', mercury_string)
    if match:
        class_name = match.group(1)
        method_name = match.group(2)  # May be None
        return (class_name, method_name)
    return None


def run():
    monitor = getMonitor()
    func_mgr = currentProgram.getFunctionManager()

    # Statistics
    stats = {
        'bw_path_strings': 0,
        'bw_unique_source_files': 0,
        'bw_functions_renamed': 0,
        'bw_functions_commented': 0,
        'bw_already_named': 0,
        'mercury_strings': 0,
        'mercury_functions_renamed': 0,
        'mercury_functions_commented': 0,
        'errors': 0,
    }

    println("=" * 70)
    println("BigWorld Source Path Annotator - Stargate Worlds (SGW.exe)")
    println("=" * 70)
    println("")

    # Phase 1: Collect all strings
    println("Phase 1: Scanning for BigWorld source path strings...")
    monitor.setMessage("BW Source Annotator: Scanning strings...")

    all_strings = get_all_defined_strings()

    # Categorize strings
    bw_path_strings = []     # BigWorld source paths
    mercury_strings = []     # Mercury:: debug strings

    for addr, value in all_strings:
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        value_lower = value.lower()

        # BigWorld source paths - look for file extensions with path context
        if ('bigworld' in value_lower or 'bwclient' in value_lower) and \
           (value_lower.endswith('.cpp') or value_lower.endswith('.hpp') or
            value_lower.endswith('.h') or value_lower.endswith('.c') or
            value_lower.endswith('.inl')):
            bw_path_strings.append((addr, value))

        # Also catch paths that reference BW directories without "bigworld" in them
        elif ('src/lib/' in value_lower.replace('\\', '/') or
              'src\\lib\\' in value_lower) and \
             (value_lower.endswith('.cpp') or value_lower.endswith('.hpp') or
              value_lower.endswith('.h')):
            bw_path_strings.append((addr, value))

        # Mercury:: debug strings
        if value.startswith('Mercury::'):
            mercury_strings.append((addr, value))

    stats['bw_path_strings'] = len(bw_path_strings)
    stats['mercury_strings'] = len(mercury_strings)

    # Deduplicate by source file
    seen_files = set()
    for _, value in bw_path_strings:
        info = extract_bw_source_info(value)
        if info:
            seen_files.add(info[1])  # filename
    stats['bw_unique_source_files'] = len(seen_files)

    println("  Found %d BigWorld source path strings" % stats['bw_path_strings'])
    println("  Representing %d unique source files" % stats['bw_unique_source_files'])
    println("  Found %d Mercury:: debug strings" % stats['mercury_strings'])

    # Phase 2: Process BigWorld source path strings
    println("")
    println("Phase 2: Annotating functions from BigWorld source paths...")
    monitor.setMessage("BW Source Annotator: Processing source paths...")
    monitor.setMaximum(len(bw_path_strings))

    used_names = set()
    # Track which functions have been processed to avoid duplicate work
    processed_functions = set()

    for idx, (str_addr, path_string) in enumerate(bw_path_strings):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)

        source_info = extract_bw_source_info(path_string)
        if source_info is None:
            continue

        module, filename, stem = source_info

        try:
            # Find all xrefs TO the path string
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

                if addr_str in processed_functions:
                    continue
                processed_functions.add(addr_str)

                # Build the plate comment with full source path
                comment = "BigWorld source: %s" % path_string
                if module:
                    comment += "\nModule: %s" % module

                if is_user_named(func):
                    # Still add the comment even if already named
                    add_plate_comment(func, comment)
                    stats['bw_already_named'] += 1
                    stats['bw_functions_commented'] += 1
                    continue

                if is_unnamed_function(func):
                    # Generate and apply a name
                    new_name = generate_function_name(module, stem, func_addr, used_names)
                    try:
                        func.setName(new_name, SourceType.USER_DEFINED)
                        add_plate_comment(func, comment)
                        stats['bw_functions_renamed'] += 1
                        stats['bw_functions_commented'] += 1
                        println("  Renamed: %s at %s  [%s]" % (
                            new_name, func_addr, filename))
                    except Exception as e:
                        # Name conflict - just add comment
                        add_plate_comment(func, comment)
                        stats['bw_functions_commented'] += 1
                        stats['errors'] += 1
                else:
                    # Has some name but not user-defined - add comment
                    add_plate_comment(func, comment)
                    stats['bw_functions_commented'] += 1

        except Exception as e:
            stats['errors'] += 1
            println("  ERROR processing path '%s': %s" % (path_string, str(e)))

    # Phase 3: Process Mercury:: debug strings
    println("")
    println("Phase 3: Annotating functions from Mercury:: debug strings...")
    monitor.setMessage("BW Source Annotator: Processing Mercury strings...")
    monitor.setMaximum(len(mercury_strings))

    mercury_used_names = set()

    for idx, (str_addr, merc_string) in enumerate(mercury_strings):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)

        merc_info = extract_mercury_info(merc_string)
        if merc_info is None:
            continue

        class_name, method_name = merc_info

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

                if addr_str in processed_functions:
                    continue
                processed_functions.add(addr_str)

                # Build comment
                comment = "Mercury debug string: %s" % merc_string

                if is_user_named(func):
                    add_plate_comment(func, comment)
                    stats['mercury_functions_commented'] += 1
                    continue

                if is_unnamed_function(func):
                    # Generate Mercury function name
                    if method_name:
                        base_name = "Mercury_%s_%s" % (class_name, method_name)
                    else:
                        base_name = "Mercury_%s" % class_name

                    safe_name = sanitize_label(base_name)

                    # Ensure uniqueness
                    if safe_name in mercury_used_names:
                        counter = 2
                        while "%s_%d" % (safe_name, counter) in mercury_used_names:
                            counter += 1
                        safe_name = "%s_%d" % (safe_name, counter)
                    mercury_used_names.add(safe_name)

                    try:
                        func.setName(safe_name, SourceType.USER_DEFINED)
                        add_plate_comment(func, comment)
                        stats['mercury_functions_renamed'] += 1
                        stats['mercury_functions_commented'] += 1
                        println("  Mercury: %s at %s  [%s]" % (
                            safe_name, func_addr, merc_string))
                    except:
                        add_plate_comment(func, comment)
                        stats['mercury_functions_commented'] += 1
                        stats['errors'] += 1
                else:
                    add_plate_comment(func, comment)
                    stats['mercury_functions_commented'] += 1

        except Exception as e:
            stats['errors'] += 1
            println("  ERROR processing Mercury string '%s': %s" % (merc_string, str(e)))

    # Phase 4: Report unique source files found
    println("")
    println("Phase 4: BigWorld source file inventory")
    println("-" * 50)

    file_to_funcs = {}
    for _, (_, path_string) in enumerate(bw_path_strings):
        info = extract_bw_source_info(path_string)
        if info:
            module, filename, stem = info
            key = "%s/%s" % (module, filename) if module else filename
            if key not in file_to_funcs:
                file_to_funcs[key] = 0
            file_to_funcs[key] += 1

    for source_file in sorted(file_to_funcs.keys()):
        count = file_to_funcs[source_file]
        println("  %-45s  %d ref(s)" % (source_file, count))

    # Print summary
    println("")
    println("=" * 70)
    println("BigWorld Source Path Annotator Summary")
    println("=" * 70)
    println("")
    println("  BigWorld Source Paths:")
    println("    Path strings found:              %d" % stats['bw_path_strings'])
    println("    Unique source files:             %d" % stats['bw_unique_source_files'])
    println("    Functions renamed:               %d" % stats['bw_functions_renamed'])
    println("    Functions commented:             %d" % stats['bw_functions_commented'])
    println("    Already user-named:              %d" % stats['bw_already_named'])
    println("")
    println("  Mercury Debug Strings:")
    println("    Mercury:: strings found:         %d" % stats['mercury_strings'])
    println("    Functions renamed:               %d" % stats['mercury_functions_renamed'])
    println("    Functions commented:             %d" % stats['mercury_functions_commented'])
    println("")
    println("  Total functions renamed:           %d" % (
        stats['bw_functions_renamed'] + stats['mercury_functions_renamed']))
    println("  Total functions commented:         %d" % (
        stats['bw_functions_commented'] + stats['mercury_functions_commented']))
    println("  Errors:                            %d" % stats['errors'])
    println("=" * 70)


# Entry point
run()
