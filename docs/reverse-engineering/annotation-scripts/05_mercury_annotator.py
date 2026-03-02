# -*- coding: utf-8 -*-
# 05_mercury_annotator.py
# Ghidra Jython script for Stargate Worlds (SGW.exe) reverse engineering
#
# Purpose:
#   Find Mercury messaging framework debug strings and label the associated
#   functions. Mercury is BigWorld's custom reliable UDP networking layer,
#   and SGW.exe contains numerous debug strings from it including class
#   names, assertion messages, and logging output.
#
# Mercury Components:
#   - Bundle: message batching and serialization
#   - Channel: reliable ordered communication channel
#   - NetworkInterface (Nub): socket management and dispatch
#   - PacketFilter: packet encryption/compression
#   - ReliableOrder: reliable delivery tracking
#   - PacketReceiver: inbound packet processing
#   - PacketSender: outbound packet processing
#   - InterfaceTable: message handler registration
#
# This script complements 03_bigworld_source_annotator.py by doing a
# deeper, Mercury-specific analysis. Script 03 catches source paths;
# this script catches debug/assertion strings, class names embedded
# in logging, and Mercury-specific file paths.
#
# Expected yield: 30-50 functions named, 50-100 commented
# Confidence: HIGH - Mercury strings are specific and unambiguous
#
# Usage: Run from Ghidra Script Manager (Jython)

"""
Mercury Networking Framework Annotator for SGW.exe

Performs deep annotation of Mercury messaging framework functions by
searching for Mercury-specific debug strings, source file paths,
class/method names in logging output, and known Mercury terminology.
Cross-references with RTTI results for Mercury classes.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.listing import CodeUnit
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
    sanitized = re.sub(r'_+', '_', sanitized)
    sanitized = sanitized.rstrip('_')
    return sanitized


def add_plate_comment(func, comment):
    """Add or append to a function's plate comment."""
    existing = func.getComment()
    if existing is None:
        func.setComment(comment)
    elif comment not in existing:
        func.setComment(existing + "\n" + comment)


def add_eol_comment(addr, comment):
    """Add an end-of-line comment at a specific address."""
    listing = currentProgram.getListing()
    cu = listing.getCodeUnitAt(addr)
    if cu is not None:
        existing = cu.getComment(CodeUnit.EOL_COMMENT)
        if existing is None:
            cu.setComment(CodeUnit.EOL_COMMENT, comment)
        elif comment not in existing:
            cu.setComment(CodeUnit.EOL_COMMENT, existing + " | " + comment)


# Known Mercury source files and their expected class associations
MERCURY_SOURCE_FILES = {
    'bundle.cpp':             'Bundle',
    'bundle.hpp':             'Bundle',
    'channel.cpp':            'Channel',
    'channel.hpp':            'Channel',
    'nub.cpp':                'Nub',
    'nub.hpp':                'Nub',
    'network_interface.cpp':  'NetworkInterface',
    'network_interface.hpp':  'NetworkInterface',
    'packet.cpp':             'Packet',
    'packet.hpp':             'Packet',
    'packet_filter.cpp':      'PacketFilter',
    'packet_filter.hpp':      'PacketFilter',
    'packet_receiver.cpp':    'PacketReceiver',
    'packet_receiver.hpp':    'PacketReceiver',
    'packet_sender.cpp':      'PacketSender',
    'packet_sender.hpp':      'PacketSender',
    'reliable_order.cpp':     'ReliableOrder',
    'reliable_order.hpp':     'ReliableOrder',
    'interface_table.cpp':    'InterfaceTable',
    'interface_table.hpp':    'InterfaceTable',
    'condemned_channels.cpp': 'CondemnedChannels',
    'irregular_channels.cpp': 'IrregularChannels',
    'once_off_packet.cpp':    'OnceOffPacket',
    'encryption_filter.cpp':  'EncryptionFilter',
    'message.cpp':            'Message',
    'message.hpp':            'Message',
    'endpoint.cpp':           'Endpoint',
    'endpoint.hpp':           'Endpoint',
    'machine_guard.cpp':      'MachineGuard',
    'interface_minder.cpp':   'InterfaceMinder',
    'piggyback.cpp':          'Piggyback',
    'unpacked_message_header.cpp': 'UnpackedMessageHeader',
}

# Known Mercury terminology patterns
MERCURY_TERMS = [
    'Bundle', 'Channel', 'NetworkInterface', 'Nub', 'PacketFilter',
    'ReliableOrder', 'PacketReceiver', 'PacketSender', 'InterfaceTable',
    'CondemnedChannels', 'IrregularChannels', 'OnceOffPacket',
    'EncryptionFilter', 'Endpoint', 'MachineGuard', 'InterfaceMinder',
    'Piggyback',
]


def classify_mercury_string(value):
    """Classify a string as a Mercury-related debug string.

    Returns: (category, class_name, detail) or None

    Categories:
      'class_method':  Mercury::ClassName::method
      'source_file':   path containing a Mercury source file
      'debug_msg':     Debug/assertion message containing Mercury terms
      'error_msg':     Error message from Mercury code
    """
    # Pattern 1: Mercury::ClassName::method or Mercury::ClassName
    match = re.match(r'Mercury::(\w+)(?:::(\w+))?', value)
    if match:
        class_name = match.group(1)
        method = match.group(2)
        return ('class_method', class_name, method)

    # Pattern 2: Source file paths for Mercury modules
    value_lower = value.lower().replace('\\', '/')
    for filename, class_name in MERCURY_SOURCE_FILES.items():
        if filename in value_lower:
            return ('source_file', class_name, filename)

    # Pattern 3: Debug/error messages containing Mercury terms
    for term in MERCURY_TERMS:
        if term in value and len(value) < 200:  # Skip very long strings
            # Determine if this is a meaningful debug message
            if any(kw in value.lower() for kw in
                   ['error', 'warning', 'assert', 'failed', 'invalid',
                    'unexpected', 'cannot', 'unable', 'should',
                    'overflow', 'underflow', 'corrupt', 'mismatch']):
                return ('error_msg', term, value)
            elif '::' in value or '(' in value:
                return ('debug_msg', term, value)

    return None


def generate_mercury_name(category, class_name, detail, used_names):
    """Generate a function name based on Mercury classification.

    Returns a unique sanitized name.
    """
    if category == 'class_method' and detail:
        base = "Mercury_%s_%s" % (class_name, detail)
    elif category == 'class_method':
        base = "Mercury_%s" % class_name
    elif category == 'source_file':
        base = "Mercury_%s" % class_name
    elif category in ('debug_msg', 'error_msg'):
        base = "Mercury_%s" % class_name
    else:
        base = "Mercury_unknown"

    safe = sanitize_label(base)

    if safe not in used_names:
        used_names.add(safe)
        return safe

    counter = 2
    while True:
        candidate = "%s_%d" % (safe, counter)
        if candidate not in used_names:
            used_names.add(candidate)
            return candidate
        counter += 1


def run():
    monitor = getMonitor()
    func_mgr = currentProgram.getFunctionManager()
    sym_table = currentProgram.getSymbolTable()

    # Statistics
    stats = {
        'mercury_class_method_strings': 0,
        'mercury_source_file_strings': 0,
        'mercury_debug_msg_strings': 0,
        'mercury_error_msg_strings': 0,
        'total_mercury_strings': 0,
        'functions_renamed': 0,
        'functions_commented': 0,
        'already_named': 0,
        'no_xrefs': 0,
        'rtti_classes_found': 0,
        'rtti_cross_refs': 0,
        'errors': 0,
    }

    println("=" * 70)
    println("Mercury Networking Annotator - Stargate Worlds (SGW.exe)")
    println("=" * 70)
    println("")

    # Phase 1: Collect and classify Mercury-related strings
    println("Phase 1: Scanning for Mercury-related strings...")
    monitor.setMessage("Mercury Annotator: Scanning strings...")

    all_strings = get_all_defined_strings()
    mercury_entries = []  # (addr, value, category, class_name, detail)

    for addr, value in all_strings:
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        classification = classify_mercury_string(value)
        if classification:
            category, class_name, detail = classification
            mercury_entries.append((addr, value, category, class_name, detail))

            if category == 'class_method':
                stats['mercury_class_method_strings'] += 1
            elif category == 'source_file':
                stats['mercury_source_file_strings'] += 1
            elif category == 'debug_msg':
                stats['mercury_debug_msg_strings'] += 1
            elif category == 'error_msg':
                stats['mercury_error_msg_strings'] += 1

    stats['total_mercury_strings'] = len(mercury_entries)

    println("  Mercury:: class/method strings:    %d" % stats['mercury_class_method_strings'])
    println("  Mercury source file paths:         %d" % stats['mercury_source_file_strings'])
    println("  Mercury debug messages:            %d" % stats['mercury_debug_msg_strings'])
    println("  Mercury error messages:            %d" % stats['mercury_error_msg_strings'])
    println("  Total Mercury-related strings:     %d" % stats['total_mercury_strings'])

    # Phase 2: Check for existing Mercury RTTI vtables
    println("")
    println("Phase 2: Cross-referencing with RTTI data...")
    monitor.setMessage("Mercury Annotator: Checking RTTI...")

    rtti_mercury_vtables = {}  # class_name -> vtable_addr

    # Search for symbols starting with "vtable_Mercury_" (from RTTI annotator)
    symbol_iter = sym_table.getSymbolIterator("vtable_Mercury_*", True)
    while symbol_iter.hasNext():
        sym = symbol_iter.next()
        name = sym.getName()
        if name.startswith("vtable_Mercury_"):
            class_name = name[len("vtable_Mercury_"):]
            rtti_mercury_vtables[class_name] = sym.getAddress()
            stats['rtti_classes_found'] += 1

    # Also search for vtable symbols that match Mercury terms
    for term in MERCURY_TERMS:
        symbol_iter = sym_table.getSymbolIterator("vtable_*%s*" % term, True)
        while symbol_iter.hasNext():
            sym = symbol_iter.next()
            name = sym.getName()
            if name not in rtti_mercury_vtables.values():
                rtti_mercury_vtables[name] = sym.getAddress()
                stats['rtti_classes_found'] += 1

    println("  Mercury RTTI vtables found:        %d" % stats['rtti_classes_found'])
    for class_name, vtable_addr in sorted(rtti_mercury_vtables.items()):
        println("    %s at %s" % (class_name, vtable_addr))

    # Phase 3: Process Mercury strings and annotate functions
    println("")
    println("Phase 3: Annotating functions from Mercury strings...")
    monitor.setMessage("Mercury Annotator: Processing strings...")
    monitor.setMaximum(len(mercury_entries))

    used_names = set()
    processed_functions = set()

    # Process in priority order: class_method > source_file > error_msg > debug_msg
    priority_order = ['class_method', 'source_file', 'error_msg', 'debug_msg']
    sorted_entries = sorted(mercury_entries,
                             key=lambda x: priority_order.index(x[2])
                             if x[2] in priority_order else 99)

    # Track per-class function associations
    class_functions = {}  # class_name -> [(func_addr, func_name, context)]

    for idx, (str_addr, value, category, class_name, detail) in enumerate(sorted_entries):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)
        monitor.setMessage("Processing: %s" % value[:50])

        try:
            refs = getReferencesTo(str_addr)
            if not refs:
                stats['no_xrefs'] += 1
                continue

            for ref in refs:
                if monitor.isCancelled():
                    return

                ref_addr = ref.getFromAddress()

                # Add EOL comment at reference site
                add_eol_comment(ref_addr, "[Mercury] %s" % value[:60])

                func = getFunctionContaining(ref_addr)
                if func is None:
                    continue

                func_addr = func.getEntryPoint()
                addr_str = str(func_addr)

                if addr_str in processed_functions:
                    # Still track the class association
                    if class_name not in class_functions:
                        class_functions[class_name] = []
                    class_functions[class_name].append(
                        (addr_str, func.getName(), value[:60]))
                    continue
                processed_functions.add(addr_str)

                # Build descriptive comment
                if category == 'class_method':
                    comment = "Mercury::%s" % class_name
                    if detail:
                        comment += "::%s" % detail
                elif category == 'source_file':
                    comment = "Mercury source: %s" % detail
                elif category == 'error_msg':
                    comment = "Mercury error in %s: %s" % (
                        class_name, value[:80])
                elif category == 'debug_msg':
                    comment = "Mercury debug (%s): %s" % (
                        class_name, value[:80])
                else:
                    comment = "Mercury: %s" % value[:80]

                # Add plate comment
                add_plate_comment(func, comment)
                stats['functions_commented'] += 1

                # Track class association
                if class_name not in class_functions:
                    class_functions[class_name] = []
                class_functions[class_name].append(
                    (addr_str, func.getName(), value[:60]))

                # Rename if unnamed
                if is_user_named(func):
                    stats['already_named'] += 1
                    continue

                if is_unnamed_function(func):
                    new_name = generate_mercury_name(
                        category, class_name, detail, used_names)
                    try:
                        func.setName(new_name, SourceType.USER_DEFINED)
                        stats['functions_renamed'] += 1
                        println("  [%s] %s at %s  <- \"%s\"" % (
                            category, new_name, func_addr, value[:50]))
                    except:
                        stats['errors'] += 1

        except Exception as e:
            stats['errors'] += 1
            println("  ERROR processing '%s': %s" % (value[:40], str(e)))

    # Phase 4: RTTI cross-reference
    # For each Mercury RTTI vtable, find functions that reference it
    println("")
    println("Phase 4: Cross-referencing RTTI vtables with Mercury functions...")
    monitor.setMessage("Mercury Annotator: RTTI cross-reference...")

    for class_name, vtable_addr in sorted(rtti_mercury_vtables.items()):
        if monitor.isCancelled():
            break

        refs = getReferencesTo(vtable_addr)
        if not refs:
            continue

        for ref in refs:
            ref_addr = ref.getFromAddress()
            func = getFunctionContaining(ref_addr)
            if func is None:
                continue

            func_addr = func.getEntryPoint()
            addr_str = str(func_addr)

            if addr_str in processed_functions:
                continue

            # This function references a Mercury vtable - it's likely a
            # constructor, destructor, or method of that class
            comment = "References Mercury RTTI vtable: %s" % class_name
            add_plate_comment(func, comment)
            stats['rtti_cross_refs'] += 1

            if class_name not in class_functions:
                class_functions[class_name] = []
            class_functions[class_name].append(
                (addr_str, func.getName(), "vtable_ref"))

    # Phase 5: Report per-class function groupings
    println("")
    println("=" * 70)
    println("Mercury Class Function Groupings")
    println("=" * 70)

    for class_name in sorted(class_functions.keys()):
        funcs = class_functions[class_name]
        println("")
        println("  Mercury::%s  (%d functions)" % (class_name, len(funcs)))
        println("  " + "-" * 50)
        for func_addr, func_name, context in sorted(funcs, key=lambda x: x[0]):
            println("    %s  %-35s  %s" % (func_addr, func_name, context[:40]))

    # Phase 6: Report known Mercury source files and their coverage
    println("")
    println("=" * 70)
    println("Mercury Source File Coverage")
    println("=" * 70)

    source_file_hits = {}
    for _, value, category, class_name, detail in mercury_entries:
        if category == 'source_file':
            if detail not in source_file_hits:
                source_file_hits[detail] = 0
            source_file_hits[detail] += 1

    for filename in sorted(MERCURY_SOURCE_FILES.keys()):
        class_name = MERCURY_SOURCE_FILES[filename]
        hits = source_file_hits.get(filename, 0)
        status = "FOUND (%d refs)" % hits if hits > 0 else "not found"
        println("  %-35s -> %-20s  %s" % (filename, class_name, status))

    # Summary
    println("")
    println("=" * 70)
    println("Mercury Networking Annotator Summary")
    println("=" * 70)
    println("")
    println("  String Analysis:")
    println("    Mercury:: class/method strings:  %d" % stats['mercury_class_method_strings'])
    println("    Mercury source file paths:       %d" % stats['mercury_source_file_strings'])
    println("    Mercury debug messages:          %d" % stats['mercury_debug_msg_strings'])
    println("    Mercury error messages:          %d" % stats['mercury_error_msg_strings'])
    println("    Total Mercury strings:           %d" % stats['total_mercury_strings'])
    println("")
    println("  Function Annotation:")
    println("    Functions renamed:               %d" % stats['functions_renamed'])
    println("    Functions commented:             %d" % stats['functions_commented'])
    println("    Already user-named:              %d" % stats['already_named'])
    println("    No xrefs found:                  %d" % stats['no_xrefs'])
    println("")
    println("  RTTI Cross-Reference:")
    println("    Mercury RTTI vtables found:      %d" % stats['rtti_classes_found'])
    println("    RTTI vtable cross-references:    %d" % stats['rtti_cross_refs'])
    println("")
    println("  Unique Mercury classes identified: %d" % len(class_functions))
    println("  Total functions associated:        %d" % sum(
        len(v) for v in class_functions.values()))
    println("  Errors:                            %d" % stats['errors'])
    println("=" * 70)
    println("")
    println("TIP: Compare the Mercury class groupings above with Cimmeria's")
    println("src/mercury/ implementation to identify missing functionality.")
    println("Key classes to compare: Bundle, Channel, Nub/NetworkInterface,")
    println("PacketFilter, ReliableOrder.")


# Entry point
run()
