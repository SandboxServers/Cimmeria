import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.address.*;
import ghidra.program.model.symbol.*;
import java.util.*;

/**
 * Read-only static probe of SGW.exe for headless Ghidra (analyzeHeadless -postScript).
 * See README.md next to this file for the full command line and caveats.
 *
 * Arguments are tokens, processed in order; one run can carry many tokens.
 *   D:<addr>          decompile the function containing <addr>
 *   F:<addr>          create a function at <addr> if none exists (not saved under -readOnly), then decompile
 *   N:<addr>          decompile, print only small-integer compare lines (template-instantiation triage)
 *   ND:<name>         decompile the function with this exact name
 *   PREVFN:<addr>     decompile the nearest function that starts before <addr>
 *   PTR:<addr>        read the 4-byte pointer at <addr>, then decompile its target
 *   I:<addr>[,<n>]    disassemble <n> instructions from <addr> (default 30); "+" also separates, since cmd splits at commas
 *   IF:<addr>         disassemble the whole function containing <addr>
 *   X:<addr>          list references TO <addr>, with the containing function
 *   U16:<text>        find <text> as UTF-16LE (exact case) in initialized memory and list each hit with its references; for wide literals the string search does not define
 *   BYTES:<addr>+<n>  hex dump <n> bytes at <addr> (default 64) even where no code or function is defined
 *   RE:<regex>        scan every instruction text for <regex> (e.g. RE:^MOV dword ptr \[[A-Z]{3} \+ 0xfc\],); prints function + address, capped at 400 hits
 *   NX:<name>         list references to the function with this exact name
 *   S:<text>          case-insensitive substring search over defined strings
 *   FNSUB:<text>      case-insensitive substring search over function names
 *   VT:<addr>[,<n>]   dump <n> 4-byte slots at <addr> as a vtable (default 12)
 *   DATAAT:<addr>     show the defined data at <addr> and its components
 *   FINDPTR:<addr>    scan initialized memory for the 4-byte value <addr> (raw data xref);
 *                     hits are leads, not results, for RTTI Complete Object Locator chains
 *   DEM:<mangled>     demangle an MSVC symbol; does NOT accept raw RTTI type-name strings (.?AV...)
 *   BLOCKS            list the memory blocks
 * Example: D:0x00479930 X:0x00479930 S:CookedDataDialogs
 */
public class Probe extends GhidraScript {
  public void run() throws Exception {
    println("PROGRAM " + currentProgram.getName() + " fns=" + currentProgram.getFunctionManager().getFunctionCount());
    DecompInterface d = new DecompInterface();
    d.openProgram(currentProgram);
    String[] a = getScriptArgs();
    for (String s : a) {
      try {
        if (s.startsWith("PTR:")) {
          // Read a 4-byte little-endian pointer at addr, print it, and try
          // to resolve+decompile the function it points to.
          Address addrObj = toAddr(s.substring(4));
          byte[] buf = new byte[4];
          currentProgram.getMemory().getBytes(addrObj, buf);
          long val = (buf[0] & 0xffL) | ((buf[1] & 0xffL) << 8) | ((buf[2] & 0xffL) << 16) | ((buf[3] & 0xffL) << 24);
          Address target = toAddr(Long.toHexString(val));
          println("=== PTR at " + s.substring(4) + " = 0x" + Long.toHexString(val));
          Function f = getFunctionContaining(target);
          if (f == null) f = createFunction(target, null);
          if (f != null) {
            println("  -> function " + f.getName() + "@" + f.getEntryPoint());
            DecompileResults r = d.decompileFunction(f, 60, monitor);
            println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "FAIL " + r.getErrorMessage());
          } else {
            println("  -> could not resolve/create a function there");
          }
        } else if (s.startsWith("PREVFN:")) {
          Address addrObj = toAddr(s.substring(7));
          Function f = getFunctionBefore(addrObj);
          println("=== PREVFN before " + s.substring(7) + " -> " + (f != null ? f.getName() + "@" + f.getEntryPoint() : "NONE"));
          if (f != null) {
            DecompileResults r = d.decompileFunction(f, 60, monitor);
            println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "FAIL " + r.getErrorMessage());
          }
        } else if (s.startsWith("NX:")) {
          // Lookup function by exact name, then print xrefs to it.
          String name = s.substring(3);
          java.util.List<Function> matches = new java.util.ArrayList<>();
          for (Function fn : currentProgram.getFunctionManager().getFunctions(true)) {
            if (fn.getName().equals(name)) matches.add(fn);
          }
          println("=== NAME-LOOKUP-XREF '" + name + "' matches=" + matches.size());
          for (Function fn : matches) {
            println("  @ " + fn.getEntryPoint());
            ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(fn.getEntryPoint());
            int count = 0;
            while (it.hasNext() && count < 100) {
              Reference ref = it.next();
              Address from = ref.getFromAddress();
              Function f2 = getFunctionContaining(from);
              println("    from " + from + " in " + (f2 != null ? f2.getName() + "@" + f2.getEntryPoint() : "???") + " type=" + ref.getReferenceType());
              count++;
            }
          }
        } else if (s.startsWith("ND:")) {
          // Lookup function by exact name, then decompile it.
          String name = s.substring(3);
          Function found = null;
          for (Function fn : currentProgram.getFunctionManager().getFunctions(true)) {
            if (fn.getName().equals(name)) { found = fn; break; }
          }
          if (found == null) { println("NAME-NOT-FOUND " + name); continue; }
          DecompileResults r = d.decompileFunction(found, 60, monitor);
          println("=== NAME-DECOMPILE " + name + " @ " + found.getEntryPoint());
          println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "FAIL " + r.getErrorMessage());
        } else if (s.startsWith("F:")) {
          // Force-create a function at addr (if none exists) then decompile it.
          // Transient within this script run; -readOnly means it won't be saved.
          String addr = s.substring(2);
          Address addrObj = toAddr(addr);
          Function f = getFunctionContaining(addrObj);
          if (f == null) {
            f = createFunction(addrObj, null);
            println("=== FORCE-CREATED FUNCTION at " + addr + " -> " + (f != null ? f.getName() : "FAILED"));
          }
          if (f == null) { println("NOFN-EVEN-AFTER-CREATE " + addr); continue; }
          DecompileResults r = d.decompileFunction(f, 60, monitor);
          println("=== DECOMPILE(F) " + f.getName() + " @ " + f.getEntryPoint() + " (asked " + addr + ")");
          println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "FAIL " + r.getErrorMessage());
        } else if (s.startsWith("RE:")) {
          // Instruction-text scan over the whole program: finds writers/readers of a struct offset.
          java.util.regex.Pattern pat = java.util.regex.Pattern.compile(s.substring(3));
          println("=== INSTRUCTION SCAN /" + s.substring(3) + "/");
          int hits = 0;
          ghidra.program.model.listing.InstructionIterator it2 = currentProgram.getListing().getInstructions(true);
          while (it2.hasNext() && hits < 400) {
            ghidra.program.model.listing.Instruction ins = it2.next();
            if (pat.matcher(ins.toString()).find()) {
              Function f = getFunctionContaining(ins.getAddress());
              println("  " + ins.getAddress() + ": " + ins + "   in " + (f != null ? f.getName() + "@" + f.getEntryPoint() : "NONE"));
              hits++;
            }
          }
          println("  hits=" + hits);
        } else if (s.startsWith("BYTES:")) {
          // Raw byte dump for undefined regions (code the auto-analysis never turned into functions).
          String[] parts = s.substring(6).split("[,+]");
          Address a0 = toAddr(parts[0]);
          int n = parts.length > 1 ? Integer.parseInt(parts[1]) : 64;
          println("=== BYTES " + parts[0] + " x" + n);
          StringBuilder sb = new StringBuilder();
          for (int i = 0; i < n; i++) {
            if (i % 16 == 0) { if (i > 0) { println("  " + sb); sb.setLength(0); } sb.append(a0.add(i)).append(": "); }
            sb.append(String.format("%02x ", getByte(a0.add(i)) & 0xff));
          }
          println("  " + sb);
        } else if (s.startsWith("U16:")) {
          // UTF-16LE literal search: wide strings in this binary are mostly not defined as string data.
          String text = s.substring(4);
          byte[] pat = new byte[text.length() * 2];
          for (int i = 0; i < text.length(); i++) { pat[i * 2] = (byte) text.charAt(i); pat[i * 2 + 1] = 0; }
          println("=== UTF16 SEARCH '" + text + "'");
          Address from = currentProgram.getMinAddress();
          int hits = 0;
          while (hits < 40) {
            Address hit = currentProgram.getMemory().findBytes(from, pat, null, true, monitor);
            if (hit == null) break;
            println("  hit " + hit);
            for (ghidra.program.model.symbol.Reference r : currentProgram.getReferenceManager().getReferencesTo(hit)) {
              Function f = getFunctionContaining(r.getFromAddress());
              println("    ref from " + r.getFromAddress() + " in " + (f != null ? f.getName() + "@" + f.getEntryPoint() : "NONE"));
            }
            hits++;
            from = hit.add(1);
          }
        } else if (s.startsWith("IF:")) {
          // Whole-function disassembly (all instructions in the function body, in address order).
          Address addrObj = toAddr(s.substring(3));
          Function f = getFunctionContaining(addrObj);
          println("=== DISASM FUNCTION " + s.substring(3));
          if (f == null) {
            println("  NO FUNCTION CONTAINING " + s.substring(3));
          } else {
            println("  function: " + f.getName() + "@" + f.getEntryPoint());
            ghidra.program.model.listing.InstructionIterator it =
                currentProgram.getListing().getInstructions(f.getBody(), true);
            while (it.hasNext()) {
              ghidra.program.model.listing.Instruction ins = it.next();
              println("  " + ins.getAddress() + ": " + ins.toString());
            }
          }
        } else if (s.startsWith("I:")) {
          // Raw disassembly window: addr, then N instructions forward (default 30).
          String[] parts = s.substring(2).split("[,+]");
          String addr = parts[0];
          int n = parts.length > 1 ? Integer.parseInt(parts[1]) : 30;
          Address addrObj = toAddr(addr);
          println("=== DISASM " + addr);
          Function f = getFunctionContaining(addrObj);
          println("  containing function: " + (f != null ? f.getName() + "@" + f.getEntryPoint() : "NONE"));
          ghidra.program.model.listing.Instruction ins = currentProgram.getListing().getInstructionAt(addrObj);
          if (ins == null) {
            println("  NO INSTRUCTION AT " + addr + " (data or undefined)");
            ghidra.program.model.listing.Data dat = currentProgram.getListing().getDataAt(addrObj);
            if (dat != null) println("  DATA: " + dat.toString());
          } else {
            for (int i = 0; i < n && ins != null; i++) {
              println("  " + ins.getAddress() + ": " + ins.toString());
              ins = ins.getNext();
            }
          }
        } else if (s.startsWith("N:")) {
          // Decompile, but only print the function name + any line that looks
          // like a small-integer comparison (category-id literal check).
          String addr = s.substring(2);
          Function f = getFunctionContaining(toAddr(addr));
          if (f == null) { println("NOFN " + addr); continue; }
          DecompileResults r = d.decompileFunction(f, 60, monitor);
          println("=== NARROW " + f.getName() + " @ " + f.getEntryPoint() + " (asked " + addr + ")");
          if (!r.decompileCompleted()) { println("FAIL " + r.getErrorMessage()); continue; }
          String c = r.getDecompiledFunction().getC();
          for (String line : c.split("\n")) {
            if (line.contains("!= (undefined") || line.contains("!= 0x") || line.contains("== 0x")
                || line.contains("!= 6") || line.contains("== 6")) {
              println("  " + line.trim());
            }
          }
        } else if (s.startsWith("D:")) {
          String addr = s.substring(2);
          Function f = getFunctionContaining(toAddr(addr));
          if (f == null) { println("NOFN " + addr); continue; }
          DecompileResults r = d.decompileFunction(f, 60, monitor);
          println("=== DECOMPILE " + f.getName() + " @ " + f.getEntryPoint() + " (asked " + addr + ")");
          println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "FAIL " + r.getErrorMessage());
        } else if (s.startsWith("X:")) {
          String addr = s.substring(2);
          Address target = toAddr(addr);
          println("=== XREFS TO " + addr);
          ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(target);
          int count = 0;
          while (it.hasNext() && count < 200) {
            Reference ref = it.next();
            Address from = ref.getFromAddress();
            Function f = getFunctionContaining(from);
            println("  from " + from + " in " + (f != null ? f.getName() + "@" + f.getEntryPoint() : "???") + " type=" + ref.getReferenceType());
            count++;
          }
          if (count == 0) println("  (no references found)");
        } else if (s.startsWith("S:")) {
          String needle = s.substring(2).toLowerCase();
          println("=== STRING SEARCH '" + needle + "'");
          ghidra.program.model.listing.Listing lst = currentProgram.getListing();
          ghidra.program.model.listing.DataIterator dit = lst.getDefinedData(currentProgram.getMinAddress(), true);
          int count = 0;
          while (dit.hasNext() && count < 200) {
            ghidra.program.model.listing.Data dat = dit.next();
            if (dat.hasStringValue()) {
              Object val = dat.getValue();
              if (val != null && val.toString().toLowerCase().contains(needle)) {
                println("  " + dat.getAddress() + " : " + val.toString());
                count++;
              }
            }
          }
          if (count == 0) println("  (no matches)");
        } else if (s.startsWith("DEM:")) {
          String mangled = s.substring(4);
          println("=== DEMANGLE '" + mangled + "'");
          try {
            ghidra.app.util.demangler.DemangledObject dobj =
                ghidra.app.util.demangler.DemanglerUtil.demangle(currentProgram, mangled);
            println("  -> " + (dobj != null ? dobj.getSignature(true) : "(demangle returned null)"));
          } catch (Throwable t) {
            println("  DEMANGLE-ERROR " + t);
          }
        } else if (s.equals("BLOCKS")) {
          println("=== MEMORY BLOCKS");
          for (ghidra.program.model.mem.MemoryBlock blk : currentProgram.getMemory().getBlocks()) {
            println("  " + blk.getName() + " " + blk.getStart() + "-" + blk.getEnd()
                + " init=" + blk.isInitialized() + " loaded=" + blk.isLoaded()
                + " len=0x" + Long.toHexString(blk.getEnd().subtract(blk.getStart()) + 1));
          }
        } else if (s.startsWith("FINDPTR:")) {
          // Scan all initialized memory blocks for a 4-byte little-endian pointer
          // value equal to the given address. Poor-man's data xref for cases where
          // -noanalysis left the Reference Manager without a recorded xref.
          long target = Long.parseUnsignedLong(s.substring(8).replace("0x", ""), 16);
          println("=== FINDPTR 0x" + Long.toHexString(target));
          int found = 0;
          for (ghidra.program.model.mem.MemoryBlock blk : currentProgram.getMemory().getBlocks()) {
            if (!blk.isInitialized() || !blk.isLoaded()) continue;
            long len = blk.getEnd().subtract(blk.getStart()) + 1;
            if (len > 64 * 1024 * 1024) continue; // skip huge/code blocks for speed unless needed
            byte[] buf = new byte[(int) len];
            try {
              blk.getBytes(blk.getStart(), buf);
            } catch (Exception ex) { continue; }
            for (int i = 0; i + 4 <= buf.length; i++) {
              long val = (buf[i] & 0xffL) | ((buf[i+1] & 0xffL) << 8) | ((buf[i+2] & 0xffL) << 16) | ((buf[i+3] & 0xffL) << 24);
              if (val == target) {
                Address at = blk.getStart().add(i);
                println("  hit @ " + at + " (block " + blk.getName() + ")");
                found++;
                if (found >= 100) break;
              }
            }
            if (found >= 100) break;
          }
          if (found == 0) println("  (no hits)");
        } else if (s.startsWith("VT:")) {
          // Dump N consecutive 4-byte pointer slots starting at addr as a vtable.
          String[] parts = s.substring(3).split("[,+]");
          String addr = parts[0];
          int n = parts.length > 1 ? Integer.parseInt(parts[1]) : 12;
          Address a2 = toAddr(addr);
          println("=== VTABLE " + addr + " x" + n);
          for (int i = 0; i < n; i++) {
            Address slot = a2.add((long) i * 4);
            byte[] buf = new byte[4];
            currentProgram.getMemory().getBytes(slot, buf);
            long val = (buf[0] & 0xffL) | ((buf[1] & 0xffL) << 8) | ((buf[2] & 0xffL) << 16) | ((buf[3] & 0xffL) << 24);
            Address target = toAddr(Long.toHexString(val));
            Function f = getFunctionAt(target);
            println("  [" + i + "] " + slot + " -> 0x" + Long.toHexString(val) + " : " + (f != null ? f.getName() : "(no fn / FUN_ not created)"));
          }
        } else if (s.startsWith("FNSUB:")) {
          // Substring search (case-insensitive) over all function names.
          String needle = s.substring(6).toLowerCase();
          println("=== FNSUB '" + needle + "'");
          int count = 0;
          for (Function fn : currentProgram.getFunctionManager().getFunctions(true)) {
            if (fn.getName().toLowerCase().contains(needle)) {
              println("  " + fn.getEntryPoint() + " : " + fn.getName());
              count++;
              if (count >= 300) { println("  ...truncated"); break; }
            }
          }
          if (count == 0) println("  (no matches)");
        } else if (s.startsWith("DATAAT:")) {
          // Show what's defined at a data address, and if it's a pointer, follow it.
          String addr = s.substring(7);
          Address a2 = toAddr(addr);
          ghidra.program.model.listing.Data dat = currentProgram.getListing().getDataAt(a2);
          println("=== DATAAT " + addr + " : " + (dat != null ? dat.toString() : "NO DEFINED DATA"));
          if (dat != null) {
            println("  type=" + dat.getDataType() + " len=" + dat.getLength());
            for (int i = 0; i < dat.getNumComponents() && i < 20; i++) {
              ghidra.program.model.listing.Data comp = dat.getComponent(i);
              println("  [" + i + "] " + comp.getAddress() + " " + comp.getDataType() + " = " + comp.getValue());
            }
          }
        } else {
          println("UNKNOWN TOKEN " + s);
        }
      } catch (Exception ex) {
        println("ERROR on token " + s + ": " + ex);
      }
    }
  }
}
