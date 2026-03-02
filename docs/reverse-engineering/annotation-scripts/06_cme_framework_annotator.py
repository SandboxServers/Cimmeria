# -*- coding: utf-8 -*-
# 06_cme_framework_annotator.py
# Ghidra Jython script for SGW.exe reverse engineering
#
# Purpose: Find and annotate Cheyenne Mountain Entertainment (CME) framework
# symbols. The CME game framework sits on top of BigWorld and includes classes
# like CME::PropertyNode, CME::EventSignal, CME::SoapLibrary, and CME::UIManager.
# These often appear as MSVC mangled symbols or debug strings embedded in the binary.
#
# Expected yield: 50-100 functions named (HIGH confidence)
#
# Run order: After scripts 01-05. Before scripts 07-10.
#
# @category SGW
# @author Cimmeria Project

"""
CME Framework Annotator

Searches for Cheyenne Mountain Entertainment framework symbols in SGW.exe:
  - Debug strings containing "CME::" class/method references
  - MSVC mangled names containing "CME@@" in the symbol table
  - Specific CME subsystem patterns (PropertyNode, EventSignal, SoapLibrary, UIManager)

For each CME-related symbol, the script finds cross-references, identifies
containing functions, and renames them with descriptive CME class/method names.
"""

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.data import StringDataType
from ghidra.program.util import DefinedDataIterator
import re

def is_default_name(func):
    """Check if a function has a default auto-generated name (FUN_, thunk_FUN_, etc.)."""
    name = func.getName()
    return (name.startswith("FUN_") or
            name.startswith("thunk_FUN_") or
            name.startswith("Ordinal_") or
            name.startswith("LAB_"))


def sanitize_name(raw):
    """
    Clean a raw string into a valid Ghidra symbol name.
    Replaces :: with _, strips non-alphanumeric characters, truncates to 120 chars.
    """
    # Replace :: with _
    name = raw.replace("::", "_")
    # Replace common separators
    name = name.replace(" ", "_").replace("-", "_").replace(".", "_")
    # Strip anything that isn't alphanumeric or underscore
    name = re.sub(r'[^A-Za-z0-9_]', '', name)
    # Collapse multiple underscores
    name = re.sub(r'_+', '_', name)
    # Strip leading/trailing underscores
    name = name.strip('_')
    # Truncate
    if len(name) > 120:
        name = name[:120]
    # Ensure it doesn't start with a digit
    if name and name[0].isdigit():
        name = "CME_" + name
    return name


def demangle_msvc_simple(mangled):
    """
    Attempt a simple extraction of class/method from an MSVC mangled name
    containing CME@@. This is not a full demangler -- just enough to extract
    the readable parts for annotation purposes.

    MSVC mangled names look like: ?method@Class@CME@@...
    We extract the Class and method portions.
    """
    # Strip leading ? if present
    if mangled.startswith("?"):
        mangled = mangled[1:]

    # Split on @ to get name components (MSVC encoding)
    parts = mangled.split("@")

    # Filter out empty strings and encoding suffixes (like QAE, UAE, etc.)
    name_parts = []
    for p in parts:
        if not p:
            continue
        # Stop at known MSVC type-encoding markers
        if len(p) >= 2 and p[0] in ('Q', 'U', 'S', 'A', 'P', 'I', 'E', 'C', 'X', 'Z', 'Y'):
            # Heuristic: if it looks like an encoding suffix, stop
            if all(c.isupper() or c in ('0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '@', '_') for c in p):
                if len(p) <= 4:
                    break
        name_parts.append(p)

    # Reverse because MSVC encodes method@Class@Namespace
    name_parts.reverse()

    # Filter to keep only reasonable identifiers
    filtered = []
    for p in name_parts:
        if re.match(r'^[A-Za-z_][A-Za-z0-9_]*$', p):
            filtered.append(p)

    if filtered:
        return "::".join(filtered)
    return None


# Known CME subsystem keywords and their descriptions
CME_SUBSYSTEMS = {
    "PropertyNode":    "UI data binding system",
    "EventSignal":     "Event dispatch/signal system",
    "SoapLibrary":     "XML web service / SOAP calls",
    "GameUI":          "Game UI framework",
    "UIManager":       "UI manager",
    "UIWidget":        "UI widget base class",
    "UIScreen":        "UI screen/panel",
    "UIButton":        "UI button control",
    "UIList":          "UI list control",
    "UIElement":       "UI element base",
    "DataModel":       "Data model layer",
    "ResourceManager": "Resource management",
    "ConfigManager":   "Configuration management",
    "GameState":       "Game state management",
    "NetworkManager":  "Network management layer",
    "AudioManager":    "Audio management",
    "InputManager":    "Input handling",
    "SceneManager":    "Scene/world management",
    "CameraManager":   "Camera system",
    "AnimManager":     "Animation management",
    "EffectManager":   "Visual effect management",
    "ParticleSystem":  "Particle effects",
}


def main():
    monitor = getMonitor()
    fm = currentProgram.getFunctionManager()
    st = currentProgram.getSymbolTable()
    listing = currentProgram.getListing()
    mem = currentProgram.getMemory()

    println("=" * 70)
    println("CME Framework Annotator - Script 06")
    println("=" * 70)
    println("")

    # Statistics
    cme_strings_found = 0
    cme_mangled_found = 0
    functions_renamed = 0
    functions_commented = 0
    classes_found = set()
    subsystems_found = set()
    name_usage_counts = {}  # track duplicate names

    # ---------------------------------------------------------------
    # PASS 1: Search all defined strings for "CME::" patterns
    # ---------------------------------------------------------------
    println("[Pass 1] Searching defined strings for CME:: references...")
    monitor.setMessage("Pass 1: Scanning strings for CME::")

    string_refs = []  # list of (string_value, string_address)

    # Use DefinedDataIterator to walk all defined strings
    data_iter = DefinedDataIterator.definedStrings(currentProgram)
    count = 0
    while data_iter.hasNext():
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        data = data_iter.next()
        count += 1
        if count % 5000 == 0:
            monitor.setMessage("Pass 1: Scanned %d strings..." % count)

        try:
            val = data.getDefaultValueRepresentation()
            if val is None:
                continue
            # Strip surrounding quotes if present
            if val.startswith('"') and val.endswith('"'):
                val = val[1:-1]
        except:
            continue

        # Check for CME:: pattern
        if "CME::" in val or "CME\\" in val or "cme::" in val.lower():
            string_refs.append((val, data.getAddress()))
            cme_strings_found += 1

    println("  Found %d strings containing CME references (scanned %d total)" % (cme_strings_found, count))

    # Process each CME string
    monitor.setMessage("Pass 1: Processing CME string references...")
    monitor.setMaximum(len(string_refs))

    for idx, (string_val, string_addr) in enumerate(string_refs):
        monitor.setProgress(idx)
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        # Extract class::method from the string
        # Look for patterns like "CME::ClassName::Method" or "CME::ClassName"
        cme_match = re.search(r'(CME::[\w:]+)', string_val)
        if not cme_match:
            continue

        cme_name = cme_match.group(1)
        parts = cme_name.split("::")
        if len(parts) >= 2:
            # parts[0] = "CME", parts[1] = ClassName, parts[2+] = method/subclass
            class_name = parts[1]
            classes_found.add(class_name)

            # Check if this is a known subsystem
            for subsys in CME_SUBSYSTEMS:
                if subsys.lower() in class_name.lower():
                    subsystems_found.add(subsys)

        # Find xrefs to this string
        refs = getReferencesTo(string_addr)
        if refs is None:
            continue

        for ref in refs:
            if monitor.isCancelled():
                return

            from_addr = ref.getFromAddress()
            func = getFunctionContaining(from_addr)
            if func is None:
                continue

            if not is_default_name(func):
                # Already named -- just add a comment if missing
                plate = func.getComment()
                if plate is None or "CME" not in plate:
                    comment = "References CME string: %s" % cme_name
                    if plate:
                        comment = plate + "\n" + comment
                    func.setComment(comment)
                    functions_commented += 1
                continue

            # Build a function name from the CME reference
            func_name = sanitize_name(cme_name)
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

                # Add plate comment
                comment = "[CME Framework] Source: string \"%s\" at %s" % (
                    cme_name, string_addr.toString())
                func.setComment(comment)
                functions_commented += 1

                println("  Renamed %s -> %s (from string)" % (
                    func.getEntryPoint().toString(), func_name))
            except Exception as e:
                # Name conflict -- try with address suffix
                try:
                    addr_suffix = func.getEntryPoint().toString().replace("0x", "")
                    fallback_name = "%s_at_%s" % (func_name, addr_suffix)
                    func.setName(fallback_name, SourceType.USER_DEFINED)
                    functions_renamed += 1
                    println("  Renamed %s -> %s (fallback)" % (
                        func.getEntryPoint().toString(), fallback_name))
                except:
                    pass

    println("  Pass 1 complete: %d functions renamed from string references" % functions_renamed)
    println("")

    # ---------------------------------------------------------------
    # PASS 2: Search symbol table for MSVC mangled names with "CME@@"
    # ---------------------------------------------------------------
    println("[Pass 2] Searching symbol table for MSVC mangled CME@@ symbols...")
    monitor.setMessage("Pass 2: Scanning symbol table for CME@@")

    renamed_pass2 = 0

    sym_iter = st.getAllSymbols(True)
    sym_count = 0
    while sym_iter.hasNext():
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        sym = sym_iter.next()
        sym_count += 1
        if sym_count % 10000 == 0:
            monitor.setMessage("Pass 2: Scanned %d symbols..." % sym_count)

        sym_name = sym.getName()
        if sym_name is None:
            continue

        # Check for CME in mangled MSVC names
        if "CME@@" not in sym_name and "CME@" not in sym_name:
            continue

        cme_mangled_found += 1
        sym_addr = sym.getAddress()

        # Try to demangle
        demangled = demangle_msvc_simple(sym_name)
        if demangled is None:
            demangled = sym_name  # Use raw name as fallback

        # Extract class name
        d_parts = demangled.split("::")
        for p in d_parts:
            if p and p != "CME" and re.match(r'^[A-Z][A-Za-z0-9]+$', p):
                classes_found.add(p)
                for subsys in CME_SUBSYSTEMS:
                    if subsys.lower() == p.lower():
                        subsystems_found.add(subsys)

        # Find the function at or containing this address
        func = fm.getFunctionAt(sym_addr)
        if func is None:
            func = getFunctionContaining(sym_addr)
        if func is None:
            continue

        if not is_default_name(func):
            # Already named -- add comment only
            plate = func.getComment()
            if plate is None or "CME" not in plate:
                comment = "CME mangled symbol: %s (demangled: %s)" % (sym_name, demangled)
                if plate:
                    comment = plate + "\n" + comment
                func.setComment(comment)
                functions_commented += 1
            continue

        # Rename the function
        clean_name = sanitize_name(demangled)
        if not clean_name:
            continue

        if clean_name in name_usage_counts:
            name_usage_counts[clean_name] += 1
            clean_name = "%s_%d" % (clean_name, name_usage_counts[clean_name])
        else:
            name_usage_counts[clean_name] = 0

        try:
            func.setName(clean_name, SourceType.USER_DEFINED)
            renamed_pass2 += 1
            functions_renamed += 1

            comment = "[CME Framework] Demangled from: %s" % sym_name
            func.setComment(comment)
            functions_commented += 1

            println("  Renamed %s -> %s (from mangled symbol)" % (
                func.getEntryPoint().toString(), clean_name))
        except Exception as e:
            try:
                addr_suffix = func.getEntryPoint().toString().replace("0x", "")
                fallback_name = "%s_at_%s" % (clean_name, addr_suffix)
                func.setName(fallback_name, SourceType.USER_DEFINED)
                renamed_pass2 += 1
                functions_renamed += 1
            except:
                pass

    println("  Pass 2 complete: %d functions renamed from mangled symbols" % renamed_pass2)
    println("")

    # ---------------------------------------------------------------
    # PASS 3: Search for specific CME subsystem string patterns
    # ---------------------------------------------------------------
    println("[Pass 3] Searching for specific CME subsystem patterns...")
    monitor.setMessage("Pass 3: Searching CME subsystem patterns")

    renamed_pass3 = 0
    subsystem_keywords = [
        "PropertyNode", "EventSignal", "SoapLibrary",
        "GameUI", "UIManager", "UIWidget", "UIScreen",
        "DataModel", "ConfigManager", "ResourceManager",
    ]

    # Search for each keyword in defined strings
    data_iter2 = DefinedDataIterator.definedStrings(currentProgram)
    keyword_hits = []  # (keyword, string_val, addr)

    count2 = 0
    while data_iter2.hasNext():
        if monitor.isCancelled():
            println("Cancelled by user.")
            return

        data = data_iter2.next()
        count2 += 1
        if count2 % 5000 == 0:
            monitor.setMessage("Pass 3: Scanned %d strings..." % count2)

        try:
            val = data.getDefaultValueRepresentation()
            if val is None:
                continue
            if val.startswith('"') and val.endswith('"'):
                val = val[1:-1]
        except:
            continue

        # Skip strings already caught by CME:: search
        if "CME::" in val:
            continue

        for kw in subsystem_keywords:
            if kw in val and len(val) < 200:
                keyword_hits.append((kw, val, data.getAddress()))
                break

    println("  Found %d subsystem keyword hits" % len(keyword_hits))

    monitor.setMaximum(len(keyword_hits))
    for idx, (keyword, string_val, str_addr) in enumerate(keyword_hits):
        monitor.setProgress(idx)
        if monitor.isCancelled():
            return

        subsystems_found.add(keyword)

        refs = getReferencesTo(str_addr)
        if refs is None:
            continue

        for ref in refs:
            from_addr = ref.getFromAddress()
            func = getFunctionContaining(from_addr)
            if func is None:
                continue

            if not is_default_name(func):
                # Already named, just add comment
                plate = func.getComment()
                if plate is None or keyword not in str(plate):
                    desc = CME_SUBSYSTEMS.get(keyword, "CME subsystem")
                    comment = "[CME %s] %s (string: \"%s\")" % (keyword, desc, string_val[:80])
                    if plate:
                        comment = plate + "\n" + comment
                    func.setComment(comment)
                    functions_commented += 1
                continue

            # Build name from keyword and string context
            # Try to extract a more specific name from the string
            name_candidate = None
            # Check for class::method in the string
            method_match = re.search(r'(\w+)::(\w+)', string_val)
            if method_match:
                name_candidate = "%s_%s" % (method_match.group(1), method_match.group(2))
            else:
                # Use keyword + partial string
                # Clean the string for use as a name
                clean_str = sanitize_name(string_val)
                if clean_str and len(clean_str) > 3:
                    name_candidate = "CME_%s_%s" % (keyword, clean_str[:40])
                else:
                    name_candidate = "CME_%s" % keyword

            if not name_candidate:
                continue

            name_candidate = sanitize_name(name_candidate)

            if name_candidate in name_usage_counts:
                name_usage_counts[name_candidate] += 1
                name_candidate = "%s_%d" % (name_candidate, name_usage_counts[name_candidate])
            else:
                name_usage_counts[name_candidate] = 0

            try:
                func.setName(name_candidate, SourceType.USER_DEFINED)
                renamed_pass3 += 1
                functions_renamed += 1

                desc = CME_SUBSYSTEMS.get(keyword, "CME subsystem")
                comment = "[CME Framework - %s] %s\nSource string: \"%s\" at %s" % (
                    keyword, desc, string_val[:100], str_addr.toString())
                func.setComment(comment)
                functions_commented += 1

                println("  Renamed %s -> %s (subsystem: %s)" % (
                    func.getEntryPoint().toString(), name_candidate, keyword))
            except:
                try:
                    addr_suffix = func.getEntryPoint().toString().replace("0x", "")
                    fallback_name = "%s_at_%s" % (name_candidate, addr_suffix)
                    func.setName(fallback_name, SourceType.USER_DEFINED)
                    renamed_pass3 += 1
                    functions_renamed += 1
                except:
                    pass

    println("  Pass 3 complete: %d functions renamed from subsystem patterns" % renamed_pass3)
    println("")

    # ---------------------------------------------------------------
    # Summary
    # ---------------------------------------------------------------
    println("=" * 70)
    println("CME Framework Annotator - Summary")
    println("=" * 70)
    println("")
    println("CME strings found:       %d" % cme_strings_found)
    println("CME mangled symbols:     %d" % cme_mangled_found)
    println("Functions renamed:       %d" % functions_renamed)
    println("Functions commented:     %d" % functions_commented)
    println("Unique CME classes:      %d" % len(classes_found))
    println("")

    if classes_found:
        println("CME classes discovered:")
        for cls in sorted(classes_found):
            println("  - %s" % cls)
        println("")

    if subsystems_found:
        println("CME subsystems identified:")
        for subsys in sorted(subsystems_found):
            desc = CME_SUBSYSTEMS.get(subsys, "")
            println("  - %-20s %s" % (subsys, desc))
        println("")

    println("Script 06 complete.")


# Entry point
main()
