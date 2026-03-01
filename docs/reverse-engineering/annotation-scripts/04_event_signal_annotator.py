# 04_event_signal_annotator.py
# Ghidra Jython script for Stargate Worlds (SGW.exe) reverse engineering
#
# Purpose:
#   Find ALL Event_NetOut_* and Event_NetIn_* string references and trace
#   them to their handler functions. This is the MOST IMPORTANT script
#   for server development because it maps every client<->server message
#   to a decompilable function.
#
# SGW uses BigWorld's CME (Client Method Extension) framework for
# client-server communication. Event strings are registered as:
#   - Event_NetOut_<name>: Client -> Server messages (253 known)
#   - Event_NetIn_<name>:  Server -> Client messages (167 known)
#
# Registration Pattern:
#   When an event is registered, the string literal and a function pointer
#   (the handler/serializer) are passed to a registration function. The
#   string reference appears in the registration context, and the handler
#   is typically a nearby function pointer argument or a call target within
#   the registration function.
#
# Expected yield: ~420 event registrations mapped, many handler functions named
# Confidence: HIGH - event strings are unambiguous naming anchors
#
# Usage: Run from Ghidra Script Manager (Jython)

"""
Event Signal Annotator for SGW.exe

Scans for Event_NetOut_* and Event_NetIn_* string literals, traces
cross-references to find registration and handler functions, and builds
a complete mapping of client-server messages to code addresses.

This is the most valuable script for Cimmeria server development.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.mem import MemoryAccessException
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


def read_pointer(addr):
    """Read a 32-bit pointer value from the given address."""
    mem = currentProgram.getMemory()
    try:
        value = mem.getInt(addr) & 0xFFFFFFFF
        return toAddr(value)
    except MemoryAccessException:
        return None


def find_function_pointers_near(ref_addr, func):
    """Search for function pointer arguments near a string reference.

    In the registration pattern, the event string and handler function
    pointer are typically passed as arguments in the same call instruction
    or are loaded into registers near each other.

    We look at instructions in a window around the string reference for:
    - PUSH instructions with immediate addresses that are function entry points
    - MOV instructions loading function addresses
    - CALL instructions (the registration function itself)

    Returns a list of (candidate_func, relationship) tuples.
    """
    func_mgr = currentProgram.getFunctionManager()
    listing = currentProgram.getListing()
    candidates = []

    if func is None:
        return candidates

    # Search in a window: 20 instructions before and 10 after the reference
    addr = ref_addr
    search_range = 20

    # Search backward from reference
    curr_addr = addr
    for i in range(search_range):
        instr = listing.getInstructionBefore(curr_addr)
        if instr is None:
            break
        curr_addr = instr.getAddress()

        # Check if this instruction is still in the same function
        if getFunctionContaining(curr_addr) != func:
            break

        mnemonic = instr.getMnemonicString().upper()

        # Look for PUSH <immediate> where the immediate is a function address
        if mnemonic == "PUSH":
            for op_idx in range(instr.getNumOperands()):
                refs = instr.getOperandReferences(op_idx)
                for op_ref in refs:
                    target = op_ref.getToAddress()
                    target_func = func_mgr.getFunctionAt(target)
                    if target_func is not None:
                        candidates.append((target_func, "push_arg"))

        # Look for MOV with function pointer
        elif mnemonic == "MOV" or mnemonic == "LEA":
            for op_idx in range(instr.getNumOperands()):
                refs = instr.getOperandReferences(op_idx)
                for op_ref in refs:
                    target = op_ref.getToAddress()
                    target_func = func_mgr.getFunctionAt(target)
                    if target_func is not None:
                        candidates.append((target_func, "mov_arg"))

    # Search forward from reference
    curr_addr = addr
    for i in range(10):
        instr = listing.getInstructionAfter(curr_addr)
        if instr is None:
            break
        curr_addr = instr.getAddress()

        if getFunctionContaining(curr_addr) != func:
            break

        mnemonic = instr.getMnemonicString().upper()

        if mnemonic == "PUSH":
            for op_idx in range(instr.getNumOperands()):
                refs = instr.getOperandReferences(op_idx)
                for op_ref in refs:
                    target = op_ref.getToAddress()
                    target_func = func_mgr.getFunctionAt(target)
                    if target_func is not None:
                        candidates.append((target_func, "push_arg_after"))

        elif mnemonic == "CALL":
            for op_idx in range(instr.getNumOperands()):
                refs = instr.getOperandReferences(op_idx)
                for op_ref in refs:
                    target = op_ref.getToAddress()
                    target_func = func_mgr.getFunctionAt(target)
                    if target_func is not None:
                        candidates.append((target_func, "call_target"))

    return candidates


def run():
    monitor = getMonitor()
    func_mgr = currentProgram.getFunctionManager()

    # Statistics
    stats = {
        'netout_strings': 0,
        'netin_strings': 0,
        'total_events': 0,
        'registration_functions_found': 0,
        'registration_functions_renamed': 0,
        'handler_functions_found': 0,
        'handler_functions_renamed': 0,
        'functions_commented': 0,
        'already_named': 0,
        'no_xrefs': 0,
        'errors': 0,
    }

    println("=" * 70)
    println("Event Signal Annotator - Stargate Worlds (SGW.exe)")
    println("=" * 70)
    println("Mapping Event_NetOut_* and Event_NetIn_* to handler functions")
    println("")

    # Phase 1: Collect all event strings
    println("Phase 1: Scanning for Event_NetOut_* and Event_NetIn_* strings...")
    monitor.setMessage("Event Annotator: Scanning strings...")

    all_strings = get_all_defined_strings()

    netout_events = []  # (addr, full_string, event_name)
    netin_events = []   # (addr, full_string, event_name)

    for addr, value in all_strings:
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        if value.startswith("Event_NetOut_"):
            event_name = value[len("Event_NetOut_"):]
            if event_name:  # Skip empty names
                netout_events.append((addr, value, event_name))

        elif value.startswith("Event_NetIn_"):
            event_name = value[len("Event_NetIn_"):]
            if event_name:
                netin_events.append((addr, value, event_name))

    stats['netout_strings'] = len(netout_events)
    stats['netin_strings'] = len(netin_events)
    stats['total_events'] = stats['netout_strings'] + stats['netin_strings']

    println("  Event_NetOut_* strings: %d (client -> server)" % stats['netout_strings'])
    println("  Event_NetIn_* strings:  %d (server -> client)" % stats['netin_strings'])
    println("  Total events:           %d" % stats['total_events'])

    # Combine all events for processing
    all_events = []
    for addr, full_string, event_name in netout_events:
        all_events.append((addr, full_string, event_name, "NetOut"))
    for addr, full_string, event_name in netin_events:
        all_events.append((addr, full_string, event_name, "NetIn"))

    # Phase 2: Process each event
    println("")
    println("Phase 2: Tracing event string references to handlers...")
    monitor.setMessage("Event Annotator: Processing events...")
    monitor.setMaximum(len(all_events))

    # The complete event mapping: event_name -> {direction, string_addr,
    #   registration_func, handler_func, status}
    event_map = {}

    # Track renamed function addresses to avoid conflicts
    renamed_addresses = set()

    for idx, (str_addr, full_string, event_name, direction) in enumerate(all_events):
        if monitor.isCancelled():
            println("Cancelled by user.")
            return
        monitor.setProgress(idx)
        monitor.setMessage("Processing: %s_%s" % (direction, event_name))

        map_key = "%s_%s" % (direction, event_name)
        event_entry = {
            'direction': direction,
            'event_name': event_name,
            'string_addr': str(str_addr),
            'registration_func': None,
            'registration_addr': None,
            'handler_func': None,
            'handler_addr': None,
            'status': 'no_xrefs',
        }

        try:
            # Find xrefs TO the event string
            refs = getReferencesTo(str_addr)
            if not refs:
                stats['no_xrefs'] += 1
                event_map[map_key] = event_entry
                continue

            for ref in refs:
                if monitor.isCancelled():
                    return

                ref_addr = ref.getFromAddress()

                # Add EOL comment at the reference site
                add_eol_comment(ref_addr, full_string)

                # Get the containing function (this is the registration context)
                reg_func = getFunctionContaining(ref_addr)
                if reg_func is None:
                    continue

                reg_func_addr = reg_func.getEntryPoint()
                reg_addr_str = str(reg_func_addr)

                stats['registration_functions_found'] += 1
                event_entry['status'] = 'registration_found'
                event_entry['registration_addr'] = reg_addr_str

                # Add plate comment to registration function
                comment = "Event registration: %s\nDirection: %s (client %s server)" % (
                    full_string, direction,
                    "->" if direction == "NetOut" else "<-")
                add_plate_comment(reg_func, comment)
                stats['functions_commented'] += 1

                # Rename the registration function if unnamed
                if reg_addr_str not in renamed_addresses:
                    if is_user_named(reg_func):
                        stats['already_named'] += 1
                        event_entry['registration_func'] = reg_func.getName()
                    elif is_unnamed_function(reg_func):
                        safe_event = sanitize_label(event_name)
                        reg_name = "register_%s_%s" % (direction, safe_event)
                        try:
                            reg_func.setName(reg_name, SourceType.USER_DEFINED)
                            stats['registration_functions_renamed'] += 1
                            renamed_addresses.add(reg_addr_str)
                            event_entry['registration_func'] = reg_name
                            println("  [%s] %s -> %s at %s" % (
                                direction, event_name, reg_name, reg_func_addr))
                        except:
                            stats['errors'] += 1
                    else:
                        event_entry['registration_func'] = reg_func.getName()

                # Phase 2b: Try to identify the handler function
                # Look for function pointers passed as arguments near the string ref
                handler_candidates = find_function_pointers_near(ref_addr, reg_func)

                # Filter candidates: exclude the registration function itself,
                # exclude common library functions, prefer push_arg candidates
                handler_func = None
                for candidate, relationship in handler_candidates:
                    candidate_addr = candidate.getEntryPoint()
                    candidate_addr_str = str(candidate_addr)

                    # Skip the registration function itself
                    if candidate_addr_str == reg_addr_str:
                        continue

                    # Skip thunks to imports
                    if candidate.isThunk():
                        continue

                    # Prefer push_arg (these are explicit function pointer arguments)
                    if relationship in ("push_arg", "push_arg_after", "mov_arg"):
                        handler_func = candidate
                        break

                if handler_func is not None:
                    handler_addr = handler_func.getEntryPoint()
                    handler_addr_str = str(handler_addr)
                    stats['handler_functions_found'] += 1
                    event_entry['status'] = 'handler_found'
                    event_entry['handler_addr'] = handler_addr_str

                    # Add plate comment
                    handler_comment = "Event handler: %s\nDirection: %s" % (
                        full_string, direction)
                    add_plate_comment(handler_func, handler_comment)

                    # Rename if unnamed
                    if handler_addr_str not in renamed_addresses:
                        if is_user_named(handler_func):
                            event_entry['handler_func'] = handler_func.getName()
                        elif is_unnamed_function(handler_func):
                            safe_event = sanitize_label(event_name)
                            handler_name = "handler_%s_%s" % (direction, safe_event)
                            try:
                                handler_func.setName(handler_name, SourceType.USER_DEFINED)
                                stats['handler_functions_renamed'] += 1
                                renamed_addresses.add(handler_addr_str)
                                event_entry['handler_func'] = handler_name
                                println("    Handler: %s at %s" % (
                                    handler_name, handler_addr))
                            except:
                                stats['errors'] += 1
                        else:
                            event_entry['handler_func'] = handler_func.getName()

                # Only process first code reference per event
                break

        except Exception as e:
            stats['errors'] += 1
            event_entry['status'] = 'error'
            println("  ERROR processing %s: %s" % (full_string, str(e)))

        event_map[map_key] = event_entry

    # Phase 3: Output complete event mapping
    println("")
    println("=" * 70)
    println("Complete Event Mapping")
    println("=" * 70)

    println("")
    println("--- Event_NetOut (Client -> Server) ---")
    println("%-40s %-30s %-30s %s" % ("Event Name", "Registration", "Handler", "Status"))
    println("-" * 130)

    netout_entries = sorted(
        [(k, v) for k, v in event_map.items() if v['direction'] == 'NetOut'],
        key=lambda x: x[0]
    )

    for map_key, entry in netout_entries:
        reg = entry.get('registration_func') or entry.get('registration_addr') or '-'
        handler = entry.get('handler_func') or entry.get('handler_addr') or '-'
        status = entry['status']
        println("  %-38s %-30s %-30s %s" % (
            entry['event_name'], reg, handler, status))

    println("")
    println("--- Event_NetIn (Server -> Client) ---")
    println("%-40s %-30s %-30s %s" % ("Event Name", "Registration", "Handler", "Status"))
    println("-" * 130)

    netin_entries = sorted(
        [(k, v) for k, v in event_map.items() if v['direction'] == 'NetIn'],
        key=lambda x: x[0]
    )

    for map_key, entry in netin_entries:
        reg = entry.get('registration_func') or entry.get('registration_addr') or '-'
        handler = entry.get('handler_func') or entry.get('handler_addr') or '-'
        status = entry['status']
        println("  %-38s %-30s %-30s %s" % (
            entry['event_name'], reg, handler, status))

    # Phase 4: Cross-reference analysis
    println("")
    println("=" * 70)
    println("Cross-Reference Analysis")
    println("=" * 70)

    # Find common registration functions (functions that register multiple events)
    reg_func_events = {}  # registration_addr -> [event_names]
    for map_key, entry in event_map.items():
        reg_addr = entry.get('registration_addr')
        if reg_addr:
            if reg_addr not in reg_func_events:
                reg_func_events[reg_addr] = []
            reg_func_events[reg_addr].append(
                "%s_%s" % (entry['direction'], entry['event_name']))

    multi_event_funcs = {addr: events for addr, events in reg_func_events.items()
                         if len(events) > 1}

    if multi_event_funcs:
        println("")
        println("Functions registering MULTIPLE events (likely class initializers):")
        for addr, events in sorted(multi_event_funcs.items(),
                                     key=lambda x: -len(x[1])):
            println("  %s (%d events):" % (addr, len(events)))
            for ev in sorted(events)[:10]:  # Show first 10
                println("    - %s" % ev)
            if len(events) > 10:
                println("    ... and %d more" % (len(events) - 10))

    # Summary statistics
    println("")
    println("=" * 70)
    println("Event Signal Annotator Summary")
    println("=" * 70)
    println("")
    println("  Events Found:")
    println("    Event_NetOut_* (client->server):  %d" % stats['netout_strings'])
    println("    Event_NetIn_* (server->client):   %d" % stats['netin_strings'])
    println("    Total events:                     %d" % stats['total_events'])
    println("")
    println("  Registration Functions:")
    println("    Found:                            %d" % stats['registration_functions_found'])
    println("    Renamed:                          %d" % stats['registration_functions_renamed'])
    println("    Already user-named:               %d" % stats['already_named'])
    println("    No xrefs found:                   %d" % stats['no_xrefs'])
    println("")
    println("  Handler Functions:")
    println("    Found:                            %d" % stats['handler_functions_found'])
    println("    Renamed:                          %d" % stats['handler_functions_renamed'])
    println("")
    println("  Annotations:")
    println("    Functions commented:              %d" % stats['functions_commented'])
    println("    Total functions renamed:          %d" % (
        stats['registration_functions_renamed'] + stats['handler_functions_renamed']))
    println("    Errors:                           %d" % stats['errors'])
    println("")

    # Coverage assessment
    total_with_handlers = len([e for e in event_map.values()
                               if e['status'] == 'handler_found'])
    total_with_reg = len([e for e in event_map.values()
                          if e['status'] in ('registration_found', 'handler_found')])
    total = len(event_map)

    println("  Coverage:")
    println("    Events with registration func:   %d / %d (%.1f%%)" % (
        total_with_reg, total,
        100.0 * total_with_reg / total if total > 0 else 0))
    println("    Events with handler func:        %d / %d (%.1f%%)" % (
        total_with_handlers, total,
        100.0 * total_with_handlers / total if total > 0 else 0))
    println("=" * 70)
    println("")
    println("TIP: Functions registering multiple events are likely entity class")
    println("initializers (e.g., SGWPlayer::registerEvents). Decompile those")
    println("first for maximum context about the event registration pattern.")


# Entry point
run()
