# -*- coding: utf-8 -*-
# 07b_targeted_vtable_annotator.py
# Trimmed version of 07_vtable_annotator.py for the right-click loot-routing
# investigation. Only processes vtables for classes we care about, so it
# completes in seconds instead of half an hour.
#
# Usage (Ghidra Jython console):
#   execfile(r'c:\Users\steven.cady\repos\personal\Cimmeria\docs\reverse-engineering\annotation-scripts\07b_targeted_vtable_annotator.py')
#
# Run after 01_rtti_annotator.py.

from ghidra.program.model.symbol import SourceType
from ghidra.program.model.mem import MemoryAccessException

# Vtables to process. Substring match, case-insensitive, against the class
# name extracted from the vtable label.
TARGETS = [
    # Right-click router (player input controller)
    "ASGWController_Player",
    # Server-entity proxies whose state the click handler may read
    "GameProxyPlayer",
    "GamePlayer",
    "GameMob",
    "GameBeing",
    "GameEntity",
    "GameEntityBase",
    "GamePet",
    # Event<->callback wiring for the input action and the wire-out events
    "Event_Action_MouseLook",
    "Event_Action_MouseClick",
    "Event_NetOut_UseAbility",
    "Event_NetOut_Interact",
    "Event_NetOut_useAbilityOnGroundTarget",
    # The callback wrappers — vfunc_3 of these typically invokes the bound
    # member function pointer, which is what we need to chase
    "MemberCallback",
    "CallbackImpl",
]


def is_default_name(func):
    name = func.getName()
    return (name.startswith("FUN_") or
            name.startswith("thunk_FUN_") or
            name.startswith("Ordinal_") or
            name.startswith("LAB_"))


def is_code_address(addr, mem):
    try:
        val = addr.getOffset()
        if val == 0 or val < 0x00400000 or val > 0x7FFFFFFF:
            return False
        block = mem.getBlock(addr)
        return block is not None and block.isExecute()
    except:
        return False


def read_pointer(mem, addr):
    try:
        return mem.getInt(addr) & 0xFFFFFFFF
    except MemoryAccessException:
        return None
    except:
        return None


def clean_class_name(raw):
    name = raw
    for prefix in ("vtable_", "VTABLE_", "vftable_", "VFTABLE_", "vtbl_"):
        if name.startswith(prefix):
            name = name[len(prefix):]
            break
    return name.strip("_")


def matches_target(class_name):
    lower = class_name.lower()
    for t in TARGETS:
        if t.lower() in lower:
            return True
    return False


def main():
    monitor = getMonitor()
    fm = currentProgram.getFunctionManager()
    st = currentProgram.getSymbolTable()
    mem = currentProgram.getMemory()
    addr_factory = currentProgram.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()

    println("=" * 70)
    println("Targeted VTable Annotator - Script 07b (loot routing)")
    println("=" * 70)

    # Step 1 - find vtable symbols matching our targets
    monitor.setMessage("Scanning symbols for target vtables...")
    targets = []
    seen_addrs = set()
    sym_iter = st.getAllSymbols(True)
    while sym_iter.hasNext():
        if monitor.isCancelled():
            return
        sym = sym_iter.next()
        sn = sym.getName()
        if sn is None:
            continue
        ln = sn.lower()
        if not (ln.startswith("vtable_") or ln.startswith("vftable_") or ln.startswith("vtbl_")):
            continue
        cls = clean_class_name(sn)
        if not matches_target(cls):
            continue
        addr = sym.getAddress()
        key = addr.getOffset()
        if key in seen_addrs:
            continue
        seen_addrs.add(key)
        targets.append((sn, addr, cls))

    targets.sort(key=lambda x: x[1].getOffset())
    println("Matched %d target vtables." % len(targets))
    if not targets:
        println("Nothing to do. Did 01_rtti_annotator run successfully?")
        return

    # Build vtable boundary set across the FULL symbol table so we don't
    # walk past one vtable into another that we're not annotating.
    monitor.setMessage("Indexing all vtable boundaries...")
    all_vtable_addrs = set()
    sym_iter = st.getAllSymbols(True)
    while sym_iter.hasNext():
        if monitor.isCancelled():
            return
        sym = sym_iter.next()
        sn = sym.getName()
        if sn is None:
            continue
        ln = sn.lower()
        if ln.startswith("vtable_") or ln.startswith("vftable_") or ln.startswith("vtbl_"):
            all_vtable_addrs.add(sym.getAddress().getOffset())

    # Step 2 - process each target vtable
    monitor.setMaximum(len(targets))
    total_named = 0
    total_already = 0
    assigned = set()

    MAX_ENTRIES = 500
    for i, (vt_name, vt_addr, cls) in enumerate(targets):
        monitor.setProgress(i)
        monitor.setMessage("[%d/%d] %s" % (i + 1, len(targets), cls))
        if monitor.isCancelled():
            return

        entry_idx = 0
        cur = vt_addr
        named_for_this = 0
        already_for_this = 0

        while entry_idx < MAX_ENTRIES:
            if entry_idx > 0:
                # Stop at the next vtable boundary
                if cur.getOffset() in all_vtable_addrs:
                    break
                # Stop at any user-defined non-DAT label
                hit_label = False
                for s in st.getSymbols(cur):
                    if s.getSource() == SourceType.USER_DEFINED:
                        sn = s.getName()
                        if sn and not sn.startswith("DAT_") and not sn.startswith("ADDR_"):
                            hit_label = True
                            break
                if hit_label:
                    break

            ptr = read_pointer(mem, cur)
            if ptr is None or ptr == 0:
                break
            try:
                target = default_space.getAddress(ptr & 0xFFFFFFFF)
            except:
                break
            if not is_code_address(target, mem):
                break

            func = fm.getFunctionAt(target)
            if func is None:
                try:
                    from ghidra.app.cmd.function import CreateFunctionCmd
                    cmd = CreateFunctionCmd(target)
                    cmd.applyTo(currentProgram)
                    func = fm.getFunctionAt(target)
                except:
                    pass
            if func is None:
                cur = cur.add(4)
                entry_idx += 1
                continue

            if not is_default_name(func):
                already_for_this += 1
                total_already += 1
                # Tag with note even if already named
                pc = func.getComment()
                note = "Also in vtable: %s (slot %d)" % (cls, entry_idx)
                if pc is None:
                    func.setComment(note)
                elif cls not in pc:
                    func.setComment(pc + "\n" + note)
                cur = cur.add(4)
                entry_idx += 1
                continue

            name = "%s__vfunc_%d" % (cls, entry_idx)
            if name in assigned:
                name = "%s__vfunc_%d_at_%s" % (cls, entry_idx, target.toString().replace("0x", ""))
            assigned.add(name)
            try:
                func.setName(name, SourceType.USER_DEFINED)
                func.setComment("[VTable] %s virtual function #%d\nVTable: %s at %s" % (
                    cls, entry_idx, vt_name, vt_addr.toString()))
                named_for_this += 1
                total_named += 1
            except:
                pass

            cur = cur.add(4)
            entry_idx += 1

        if entry_idx > 0:
            println("  %s: %d entries (%d named, %d already named)" % (cls, entry_idx, named_for_this, already_for_this))

    println("")
    println("Done. %d functions newly named, %d already had user names." % (total_named, total_already))


main()
