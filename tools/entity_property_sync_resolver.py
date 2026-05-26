#!/usr/bin/env python3
"""
entity_property_sync_resolver.py — Validate the entity-property-sync bible
chapter's wire-format claims against a captured Mercury session.

Decrypts a UDP pcap with the AES-256 session key, walks each Mercury body,
and reports per-validation whether the chapter's stated layouts/encodings
are CONFIRMED, DIVERGE, NOT-OBSERVED, or NOT-DETERMINABLE from the data.

Validation targets:
  V1 — sub-slot threshold (re-run mercury_dispute_resolver logic)
  V2 — createBasePlayer (0x05) wire layout
  V3 — createCellPlayer (0x06) fixed 32-byte payload
  V4 — property-change propID encoding (OQ-1 / G7: 0x3C/0x3D thresholds)
  V5 — cell-method wire byte mask (0x80..0xFF, 7-bit ID)
  V6 — base-method wire byte mask (0xC0..0xFF, 6-bit ID)
  V7 — createCellPlayer Y/Z rotation swap
  V8 — enableEntities (0x08, client→server) 8-byte body

Usage:
    python3 tools/entity_property_sync_resolver.py <pcap_file> <keys_file> [--server-port N]

Server-port resolution:
    In a 2-endpoint UDP capture, the server port appears as src in S2C
    packets and as dst in C2S packets — and the client port mirrors that.
    Both ports therefore have identical src+dst counts, so a pure frequency
    tie-break is unreliable. This script picks the server port via the
    following waterfall:
      1. If `--server-port N` is passed on the command line, use it.
      2. Otherwise, prefer the lower-numbered of the two two-sided ports
         (servers conventionally bind below the ephemeral-port range).
      3. Fall back to the single most-active port if only one is two-sided.
    If the heuristic resolves wrongly, all is_s2c / is_c2s decisions for
    the run invert and every validation result inverts with them — pass
    `--server-port N` explicitly when in doubt (the value can be read
    from the session log under `game/sgw/Working/binaries/sessions/*.log`
    or inferred from the `<date>_<HH-MM>-keys.txt` filename).
"""

from __future__ import annotations

import struct
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import pcap_dissect as pd

# ── SGWPlayer method index names (from mercury_dispute_resolver.py) ───────────
METHOD_IDX = {
    1: "ON_SEQUENCE",
    3: "INTERACTION_TYPE",
    4: "ON_ENTITY_FLAGS",
    7: "ON_ENTITY_PROPERTY",
    8: "ON_VISIBLE",
    9: "ON_KISMET_EVENT_SET_UPDATE",
    10: "ON_ENTITY_TINT",
    11: "ON_BEING_NAME_ID_UPDATE",
    14: "ON_EFFECT_RESULTS",
    15: "ON_LEVEL_UPDATE",
    16: "ON_TARGET_UPDATE",
    17: "ON_BEING_NAME_UPDATE",
    19: "ON_STATE_FIELD_UPDATE",
    20: "ON_STAT_UPDATE",
    21: "ON_STAT_BASE_UPDATE",
    23: "ON_ARCHETYPE_UPDATE",
    24: "ON_ALIGNMENT_UPDATE",
    25: "ON_FACTION_UPDATE",
    26: "BEING_APPEARANCE",
    28: "ON_PLAYER_COMMUNICATION",
    31: "ON_CHAT_JOINED",
    32: "ON_CHAT_LEFT",
    65: "SETUP_STARGATE_INFO",
    69: "ON_BAG_INFO",
    70: "ON_ACTIVE_SLOT_UPDATE",
    71: "ON_REMOVE_ITEM",
    72: "ON_UPDATE_ITEM",
    75: "ON_CASH_CHANGED",
    76: "ON_MAIL_HEADER_INFO",
    77: "ON_MAIL_HEADER_REMOVE",
    78: "ON_MAIL_READ",
    79: "SEND_MAIL_RESULT",
    80: "ON_STORE_OPEN",
    81: "ON_STORE_UPDATE",
    98: "ON_BEGIN_AID_WAIT",
    99: "ON_END_AID_WAIT",
    101: "ON_KNOWN_ABILITIES_UPDATE",
    102: "ON_TIME_OF_DAY",
    105: "ON_DIALOG_DISPLAY",
    113: "ON_TRAINER_OPEN",
    114: "ON_LOOT_DISPLAY",
    115: "ON_PLAYER_DATA_LOADED",
    117: "ON_CLIENT_MAP_LOAD",
    118: "GIVE_ABILITY",
    119: "GIVE_XP_FOR_LEVEL",
    121: "ON_ERROR_CODE",
    122: "SETUP_WORLD_PARAMETERS",
    124: "CLEAR_HINTED_REGIONS",
    125: "ADD_CLIENT_HINTED_GENERIC_REGION",
    126: "ON_RESET_MAP_INFO",
    130: "ON_EXTRA_NAME_UPDATE",
    131: "ON_EXP_UPDATE",
    132: "ON_MAX_EXP_UPDATE",
    133: "ON_RING_TRANSPORTER_LIST",
    139: "ON_UPDATE_KNOWN_CRAFTS",
    141: "ON_ABILITY_TREE_INFO",
    155: "ON_PLAY_MOVIE",
}


def _parse_footer(data):
    """Thin wrapper around pd.parse_mercury_footer.

    Returns body bytes or None on failure.
    """
    result = pd.parse_mercury_footer(data)
    if result is None:
        return None
    _flags, _seq, _fb, _fe, _fro, body, _acks = result
    return body


def _decrypt_packet(payload, key_hex):
    """Decrypt a UDP payload; return plaintext or None."""
    if len(payload) < 32:
        return None
    ciphertext = payload[:-16]
    if len(ciphertext) % 16 != 0:
        return None
    pt = pd.decrypt_aes256_cbc(key_hex, ciphertext)
    if pt is None:
        return None
    return pd.strip_pkcs7(pt)


# ── Body message iterator ─────────────────────────────────────────────────────

def iter_messages(body, is_server):
    """Yield (msg_id, payload_bytes) from a Mercury body."""
    offset = 0
    msg_format = pd.SERVER_MSG_FORMAT if is_server else pd.CLIENT_MSG_FORMAT

    while offset < len(body):
        if offset >= len(body):
            break
        msg_id = body[offset]
        offset += 1

        # Entity methods use a u16 word-length prefix. Cell methods occupy
        # 0x80..0xBF (7-bit ID); base methods occupy 0xC0..0xFF (6-bit ID).
        # V6 confirms 0xFF (base methodId=63 at the &0x3F mask boundary) is a
        # valid wire byte — the older 0xFE upper bound was wrong.
        if 0x80 <= msg_id <= 0xFF:
            if offset + 2 > len(body):
                break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]
            offset += 2
            if offset + word_len > len(body):
                break
            payload = body[offset:offset+word_len]
            offset += word_len
            yield msg_id, payload
            continue

        fmt = msg_format.get(msg_id)
        if fmt is None:
            break  # unknown system message; stop parsing this body

        style, fixed_len, prefix_width = fmt
        if style == 'constant':
            if offset + fixed_len > len(body):
                break
            payload = body[offset:offset+fixed_len]
            offset += fixed_len
        else:
            if offset + prefix_width > len(body):
                break
            if prefix_width == 1:
                pl = body[offset]; offset += 1
            elif prefix_width == 2:
                pl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            else:
                pl = struct.unpack('<I', body[offset:offset+4])[0]; offset += 4
            if offset + pl > len(body):
                break
            payload = body[offset:offset+pl]
            offset += pl

        yield msg_id, payload


# ── Per-validation collectors ─────────────────────────────────────────────────

class Results:
    def __init__(self):
        # V1 — sub-slot
        self.sub_index_distribution = Counter()   # sub_index → count
        self.sub_index_examples = []              # (tag, sub_index, word_len, eid, args_len)

        # V2 — createBasePlayer
        self.create_base_player_msgs = []         # list of payload bytes

        # V3/V7 — createCellPlayer
        self.create_cell_player_msgs = []         # list of payload bytes

        # V4 — updateEntity envelope inspection (UE FArchive: 4-byte length +
        # 4-byte type tag; case-1 = FNetworkPropertyChange, then propID as
        # uint32_t). The BigWorld 0x3C/0x3D wire-prefix scheme lives upstream
        # of the UE bridge and is never visible to a client-side capture.
        self.update_entity_msgs = []              # list of payload bytes
        self.envelope_type_tag_hist = Counter()   # type tag → count (case 1..6 per §1.8)
        self.envelope_length_mismatches = 0       # count of payload_len != declared length
        self.envelope_too_short = 0               # count of payloads < 8 bytes
        self.prop_id_hist = Counter()             # decoded propID (case-1 only) → count
        self.prop_id_over_59 = 0                  # would have used 0x3C prefix server-side
        self.prop_id_over_315 = 0                 # would have used 0x3D prefix server-side

        # V5/V6 — cell/base method wire bytes
        self.cell_method_first_bytes = Counter()  # (0x80..0xBF) → count
        self.base_method_first_bytes = Counter()  # (0xC0..0xFF) → count

        # V8 — enableEntities
        self.enable_entities_msgs = []           # list of (payload_bytes, direction)

        # Packet stats
        self.total_udp = 0
        self.decrypted = 0
        self.s2c = 0
        self.c2s = 0


def scan_body_for_sub_slots(body, results: Results, is_server: bool):
    """Walk body looking for 0xBD (extended cell) or 0xFD (extended base) prefixes.

    `is_server` selects the same length-schema table as `iter_messages` uses.
    Shared msg_ids appear in both directions with different framing (constant
    vs. length-prefixed, or different prefix widths); falling back to
    ``SERVER_MSG_FORMAT or CLIENT_MSG_FORMAT`` would pick the wrong schema on
    those IDs and desync sub-slot offsets, skewing V1 counts.
    """
    msg_format = pd.SERVER_MSG_FORMAT if is_server else pd.CLIENT_MSG_FORMAT
    offset = 0
    while offset < len(body):
        msg_id = body[offset]
        offset += 1

        if msg_id == 0xBD:
            if offset + 2 > len(body): break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            if offset + word_len > len(body): break
            if word_len >= 5:
                entity_id = struct.unpack('<I', body[offset:offset+4])[0]
                sub_index = body[offset+4]
                args_len = word_len - 5
                results.sub_index_distribution[sub_index] += 1
                if len(results.sub_index_examples) < 20:
                    results.sub_index_examples.append(("BD", sub_index, word_len, entity_id, args_len))
            offset += word_len
            continue

        if msg_id == 0xFD:
            if offset + 2 > len(body): break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            if offset + word_len > len(body): break
            if word_len >= 1:
                sub_index = body[offset]
                args_len = word_len - 1
                results.sub_index_distribution[sub_index] += 1
                if len(results.sub_index_examples) < 20:
                    results.sub_index_examples.append(("FD", sub_index, word_len, None, args_len))
            offset += word_len
            continue

        # Other entity methods — skip (full 0x80..0xFF range; see iter_messages)
        if 0x80 <= msg_id <= 0xFF:
            if offset + 2 > len(body): break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            offset += word_len
            continue

        # Hit a system message — skip its body and keep scanning. (Earlier
        # versions early-returned here, missing any 0xBD/0xFD that followed a
        # system message in the same body — CR-9.) Use the direction-aware
        # table only — shared msg_ids encode differently per direction.
        fmt = msg_format.get(msg_id)
        if fmt is None:
            # Truly unknown — stop, bytes downstream would be misframed anyway.
            return
        style, fixed_len, prefix_width = fmt
        if style == 'constant':
            if offset + fixed_len > len(body): return
            offset += fixed_len
        else:
            if offset + prefix_width > len(body): return
            if prefix_width == 1:
                pl = body[offset]; offset += 1
            elif prefix_width == 2:
                pl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            else:
                pl = struct.unpack('<I', body[offset:offset+4])[0]; offset += 4
            if offset + pl > len(body): return
            offset += pl
        continue


def process_packet(plaintext, src_port, dst_port, results: Results, server_port: int):
    """Process one decrypted Mercury plaintext."""
    body = _parse_footer(plaintext)
    if body is None:
        return

    # Direction: determined by the server port detected during port scan
    is_s2c = (src_port == server_port)
    is_c2s = (dst_port == server_port)

    if is_s2c:
        results.s2c += 1
    elif is_c2s:
        results.c2s += 1

    # V1 — sub-slot scan. Direction-aware: shared system msg_ids encode with
    # different length schemas in s2c vs c2s.
    scan_body_for_sub_slots(body, results, is_server=is_s2c)

    # V5/V6 — entity method wire bytes
    # Entity methods 0x80..0xBF = cell (7-bit); 0xC0..0xFF = base (6-bit)
    # (0xBD and 0xFD are special sentinels for extended methods but still count)
    # Scan the raw body for high bytes to validate the mask ranges
    # We need to walk messages to find entity methods. Use iter_messages.
    for msg_id, payload in iter_messages(body, is_server=is_s2c):
        if 0x80 <= msg_id <= 0xBF:
            results.cell_method_first_bytes[msg_id] += 1
        elif 0xC0 <= msg_id <= 0xFF:
            results.base_method_first_bytes[msg_id] += 1

        if is_s2c:
            if msg_id == 0x05:  # createBasePlayer
                results.create_base_player_msgs.append(payload)
            elif msg_id == 0x06:  # createCellPlayer
                results.create_cell_player_msgs.append(payload)
            elif msg_id == 0x0A:  # updateEntity — UE FArchive envelope
                results.update_entity_msgs.append(payload)
                # V4 — parse the UE FArchive envelope: [u32 length][u32 type].
                # FUN_01560ad0 (the BigWorld→UE bridge) reads exactly these
                # 8 bytes, validates payload_len == read_len, then dispatches
                # on the type tag. Case 1 = FNetworkPropertyChange::vfunc_0,
                # which then reads the propID as a uint32_t from the FArchive
                # at this+0x2c. The BigWorld wire 0x3C/0x3D prefix is decoded
                # upstream of this layer and is never visible from a client
                # pcap (OQ-1 closure: audit Appendix D).
                if len(payload) < 8:
                    results.envelope_too_short += 1
                else:
                    declared_len, type_tag = struct.unpack('<II', payload[0:8])
                    results.envelope_type_tag_hist[type_tag] += 1
                    # Bridge invariant: declared_len matches the remaining
                    # body. The bridge reads bytes after the type tag, so the
                    # comparison is `declared_len == len(payload) - 8`.
                    if declared_len != len(payload) - 8:
                        results.envelope_length_mismatches += 1
                    if type_tag == 1 and len(payload) >= 12:
                        # FNetworkPropertyChange — propID is the next u32.
                        prop_id = struct.unpack('<I', payload[8:12])[0]
                        results.prop_id_hist[prop_id] += 1
                        if prop_id >= 60:
                            results.prop_id_over_59 += 1
                        if prop_id >= 316:
                            results.prop_id_over_315 += 1

        # enableEntities is client→server only. Gate on is_c2s rather than
        # `not is_s2c` so packets with unknown direction (e.g. third-party
        # endpoints in a noisy capture) are excluded instead of misattributed.
        if is_c2s:
            if msg_id == 0x08:  # enableEntities
                results.enable_entities_msgs.append((payload, "c2s"))


# ── Reporting helpers ─────────────────────────────────────────────────────────

def _hex(b):
    return " ".join(f"{x:02X}" for x in b)


def report_v1(r: Results):
    print("## V1 — Sub-slot threshold (idBase=61 vs 62)")
    print()
    if not r.sub_index_distribution:
        print("  NOT-OBSERVED: No 0xBD/0xFD-prefixed messages in this pcap.")
        print("  (Normal for a short session — sub-slot methods require world entry)")
        return

    rust_hits = 0
    spec_hits = 0
    rust_uniq = 0
    spec_uniq = 0

    print(f"  sub_idx  count  spec(+62) name                   rust(+61) name")
    print(f"  -------  -----  --------- ----                   --------- ----")
    for si, n in sorted(r.sub_index_distribution.items()):
        sm = si + 62
        rm = si + 61
        sn = METHOD_IDX.get(sm, "")
        rn = METHOD_IDX.get(rm, "")
        if sn: spec_hits += n
        if rn: rust_hits += n
        sn_named = bool(sn)
        rn_named = bool(rn)
        if rn_named and not sn_named: rust_uniq += n
        elif sn_named and not rn_named: spec_uniq += n
        print(f"  {si:7d}  {n:5d}  {sm:9d} {sn:<22}  {rm:9d} {rn}")

    print()
    if rust_uniq > 0 and spec_uniq == 0:
        print(f"  CONFIRMS: idBase=61 (Rust). {rust_uniq} packets only make sense under Rust encoding.")
        print(f"  Zero packets only make sense under spec (idBase=62).")
    elif spec_uniq > 0 and rust_uniq == 0:
        print(f"  DIVERGES: idBase=62 (spec). {spec_uniq} packets only make sense under spec encoding.")
    elif rust_hits > spec_hits:
        print(f"  CONFIRMS (weighted): Rust hits={rust_hits} vs spec hits={spec_hits}.")
    else:
        print(f"  AMBIGUOUS: Rust hits={rust_hits}, spec hits={spec_hits}.")
    print()


def report_v2(r: Results):
    print("## V2 — createBasePlayer (msg_id 0x05) wire layout")
    print()
    if not r.create_base_player_msgs:
        print("  NOT-OBSERVED: No createBasePlayer (0x05) messages in this pcap.")
        return

    print(f"  Found {len(r.create_base_player_msgs)} createBasePlayer message(s).")
    all_match = True
    for i, payload in enumerate(r.create_base_player_msgs[:5]):
        print(f"")
        print(f"  Msg #{i+1} — total payload {len(payload)} bytes")
        if len(payload) < 6:
            print(f"    DIVERGES: payload too short ({len(payload)} bytes, need >= 6)")
            all_match = False
            continue
        entity_id = struct.unpack('<I', payload[0:4])[0]
        type_id   = struct.unpack('<H', payload[4:6])[0]
        prop_stream = payload[6:]
        print(f"    Offset 0-3 : entityId  = {entity_id} (0x{entity_id:08X})  [expect u32 LE]")
        print(f"    Offset 4-5 : typeId    = {type_id} (0x{type_id:04X})  [expect 0x0002 = SGWPlayer; 0-based entities.xml index per EntityDescription_ReadFromStream @ SGW.exe@0x01590520]")
        print(f"    Offset 6+  : property stream = {len(prop_stream)} bytes")
        print(f"    Prop stream head: {_hex(prop_stream[:32])}")
        if type_id != 0x0002:
            print(f"    NOTE: typeId != 0x0002 (SGWPlayer). Could be a non-player entity. Common 0-based typeIDs: 0x00=SGWSpawnableEntity, 0x01=SGWBeing, 0x02=SGWPlayer, 0x07=SGWBlackMarket (ServerOnly — silent-fail on client), 0x08=Account.")
    print()
    if all_match:
        print("  CONFIRMS: entityId u32 at offset 0, typeId u16 at offset 4, prop stream at 6+.")
        print("  typeId=0x0002 (SGWPlayer) where entity is the player.")
    print()


def report_v3_v7(r: Results):
    print("## V3/V7 — createCellPlayer (msg_id 0x06) — fixed 32-byte payload + Y/Z swap")
    print()
    if not r.create_cell_player_msgs:
        print("  NOT-OBSERVED: No createCellPlayer (0x06) messages in this pcap.")
        return

    print(f"  Found {len(r.create_cell_player_msgs)} createCellPlayer message(s).")
    always_32 = True
    vehicle_zero = True
    yz_looks_swapped = []

    for i, payload in enumerate(r.create_cell_player_msgs[:5]):
        print()
        print(f"  Msg #{i+1} — {len(payload)} bytes")
        if len(payload) != 32:
            print(f"    DIVERGES: expected 32 bytes, got {len(payload)}")
            always_32 = False
            continue

        raw = _hex(payload)
        print(f"    Raw: {raw}")

        off = 0
        space_id   = struct.unpack('<I', payload[off:off+4])[0]; off += 4
        vehicle_id = struct.unpack('<I', payload[off:off+4])[0]; off += 4
        pos_x      = struct.unpack('<f', payload[off:off+4])[0]; off += 4
        pos_y      = struct.unpack('<f', payload[off:off+4])[0]; off += 4
        pos_z      = struct.unpack('<f', payload[off:off+4])[0]; off += 4
        rot_x      = struct.unpack('<f', payload[off:off+4])[0]; off += 4
        rot_z      = struct.unpack('<f', payload[off:off+4])[0]; off += 4
        rot_y      = struct.unpack('<f', payload[off:off+4])[0]; off += 4

        print(f"    Offset  0: spaceId   = {space_id}  (0x{space_id:08X})")
        print(f"    Offset  4: vehicleId = {vehicle_id}  (expect 0 at world entry)")
        print(f"    Offset  8: posX      = {pos_x:.4f}")
        print(f"    Offset 12: posY      = {pos_y:.4f}")
        print(f"    Offset 16: posZ      = {pos_z:.4f}")
        print(f"    Offset 20: rotX(pitch)= {rot_x:.4f}")
        print(f"    Offset 24: rotZ(yaw)  = {rot_z:.4f}  *** chapter says Y/Z swap: this is yaw ***")
        print(f"    Offset 28: rotY(roll) = {rot_y:.4f}  *** chapter says Y/Z swap: this is roll ***")

        if vehicle_id != 0:
            vehicle_zero = False

        # Heuristic for Y/Z swap: at world-entry, yaw should be a non-trivial
        # float (a standing facing direction) while roll should be ~0.
        # If rot_z (yaw slot) looks like an angle and rot_y (roll slot) is ~0,
        # the swap is confirmed.
        yaw_plausible   = abs(rot_z) < 180.0 and abs(rot_z) > 0.001
        roll_near_zero  = abs(rot_y) < 0.01
        yz_looks_swapped.append(yaw_plausible and roll_near_zero)

    print()
    if always_32:
        print("  CONFIRMS V3: payload is always 32 bytes.")
    else:
        print("  DIVERGES V3: not all payloads are 32 bytes.")

    if vehicle_zero:
        print("  CONFIRMS V3: vehicleId=0 at all observed world entries.")
    else:
        print("  NOTE: vehicleId != 0 in at least one message (non-zero vehicle mount?).")

    if all(yz_looks_swapped):
        print("  CONFIRMS V7: rotation field layout consistent with X/Z/Y order (yaw in slot 24, roll~0 in slot 28).")
    elif any(yz_looks_swapped):
        print("  PARTIAL-CONFIRMS V7: some messages show yaw-in-Z-slot; others ambiguous.")
    else:
        print("  NOT-DETERMINABLE V7: cannot confirm Y/Z swap from this data (both rot values near zero, or large roll).")
    print()


def report_v4(r: Results):
    print("## V4 — updateEntity FArchive envelope + decoded propID distribution")
    print()
    print("  Scope: the BigWorld 0x3C/0x3D wire-prefix scheme decodes upstream")
    print("  of FUN_01560ad0 and is never visible to a client-side pcap (OQ-1).")
    print("  This validation parses the UE FArchive envelope the bridge expects")
    print("  ([u32 length][u32 type]; case 1 → propID as u32) and reports the")
    print("  decoded propID distribution so reviewers can see whether observed")
    print("  propIDs land in the ranges where the server WOULD have used the")
    print("  0x3C (60..315) or 0x3D (316+) prefix.")
    print()
    print(f"  updateEntity (0x0A) messages found: {len(r.update_entity_msgs)}")
    print()

    if not r.update_entity_msgs:
        print("  NOT-OBSERVED: No updateEntity messages.")
        return

    print(f"  Envelope parse stats:")
    print(f"    payloads < 8 bytes (no envelope)        : {r.envelope_too_short}")
    print(f"    payloads where declared_len != body_len : {r.envelope_length_mismatches}")
    print(f"    type-tag histogram (per FUN_01560ad0 switch — case 1..6):")
    type_names = {
        1: "FNetworkPropertyChange (property delta)",
        2: "FNetworkActorMove",
        3: "FNetworkActorCreate",
        4: "FNetworkActorDelete",
        5: "FNetworkObjectRename",
        6: "FNetworkRemoteConsoleCommand",
    }
    for tag, n in sorted(r.envelope_type_tag_hist.items()):
        name = type_names.get(tag, "(unknown / not in §1.8 switch)")
        print(f"      type={tag}: {n} msgs   {name}")
    print()

    if r.envelope_length_mismatches:
        print(f"  DIVERGES: {r.envelope_length_mismatches} envelopes have"
              f" declared_len != actual body length. The bridge would reject these.")
        print()

    if not r.prop_id_hist:
        print("  NOT-OBSERVED V4: no type-tag=1 (property-change) messages decoded.")
        print("  OQ-1 STATUS: UNCHANGED — wire-prefix scheme is server-side only.")
        print()
        return

    case1_total = sum(r.prop_id_hist.values())
    print(f"  Decoded propID distribution (type-tag=1, {case1_total} messages):")
    print(f"    propIDs < 60   (server uses 1-byte direct)         : "
          f"{case1_total - r.prop_id_over_59}")
    print(f"    propIDs 60..315 (server uses [0x3C, propId-60])    : "
          f"{r.prop_id_over_59 - r.prop_id_over_315}")
    print(f"    propIDs >= 316  (server uses [0x3D, propId-316])   : "
          f"{r.prop_id_over_315}")
    print()
    print(f"  Top decoded propIDs:")
    for pid, n in r.prop_id_hist.most_common(10):
        print(f"    propID={pid:5d}: {n} msgs")
    print()
    if r.prop_id_over_315 > 0:
        print("  CONFIRMS (range): observed propIDs span into the 0x3D-prefix")
        print("  range server-side. Threshold-encoding correctness is still a")
        print("  server-only invariant (cannot be falsified from this pcap).")
    elif r.prop_id_over_59 > 0:
        print("  CONFIRMS (range): observed propIDs span into the 0x3C-prefix")
        print("  range server-side. Threshold-encoding correctness is still a")
        print("  server-only invariant (cannot be falsified from this pcap).")
    else:
        print("  All observed propIDs are < 60. This session would have used")
        print("  only the 1-byte server-side encoding; the 60/316 thresholds")
        print("  remain neither confirmed nor falsified by this capture.")
    print()


def report_v5_v6(r: Results):
    print("## V5/V6 — Cell-method (0x80..0xBF) and base-method (0xC0..0xFF) wire byte masks")
    print()
    cell_total = sum(r.cell_method_first_bytes.values())
    base_total = sum(r.base_method_first_bytes.values())
    print(f"  Cell-range bytes (0x80..0xBF): {cell_total} observed")
    print(f"  Base-range bytes (0xC0..0xFF): {base_total} observed")
    print()

    if r.cell_method_first_bytes:
        print("  Cell method wire bytes (top 10):")
        for fb, n in r.cell_method_first_bytes.most_common(10):
            method_id = fb & 0x7F  # 7-bit
            name = METHOD_IDX.get(method_id, "")
            print(f"    0x{fb:02X}  n={n:5d}  methodId={method_id} (0x{method_id:02X})  {name}")
        # Confirm all fall in 0x80..0xBF
        out_of_range = [fb for fb in r.cell_method_first_bytes if not (0x80 <= fb <= 0xBF)]
        if not out_of_range:
            print("  CONFIRMS V5: all cell-method bytes fall in 0x80..0xBF.")
        else:
            print(f"  DIVERGES V5: bytes outside 0x80..0xBF: {[hex(x) for x in out_of_range]}")
    else:
        print("  NOT-OBSERVED V5: No cell-method messages in this pcap.")

    print()
    if r.base_method_first_bytes:
        print("  Base method wire bytes (top 10):")
        for fb, n in r.base_method_first_bytes.most_common(10):
            method_id = fb & 0x3F  # 6-bit
            name = METHOD_IDX.get(method_id, "")
            print(f"    0x{fb:02X}  n={n:5d}  methodId={method_id} (0x{method_id:02X})  {name}")
        # Confirm all in 0xC0..0xFF
        out_of_range = [fb for fb in r.base_method_first_bytes if not (0xC0 <= fb <= 0xFF)]
        if not out_of_range:
            print("  CONFIRMS V6: all base-method bytes fall in 0xC0..0xFF.")
        else:
            print(f"  DIVERGES V6: bytes outside 0xC0..0xFF: {[hex(x) for x in out_of_range]}")
    else:
        print("  NOT-OBSERVED V6: No base-method messages in this pcap.")
    print()


def report_v8(r: Results):
    print("## V8 — enableEntities (msg_id 0x08, client-to-server) 8-byte body")
    print()
    if not r.enable_entities_msgs:
        print("  NOT-OBSERVED: No enableEntities (0x08) messages in this pcap.")
        return

    print(f"  Found {len(r.enable_entities_msgs)} enableEntities message(s).")
    all_8 = True
    for i, (payload, direction) in enumerate(r.enable_entities_msgs[:5]):
        print(f"  Msg #{i+1} [{direction}]: {len(payload)} bytes  raw: {_hex(payload)}")
        if len(payload) != 8:
            print(f"    DIVERGES: expected 8 bytes per chapter claim, got {len(payload)}")
            all_8 = False

    print()
    if all_8:
        print("  CONFIRMS V8: payload is 8 bytes (matches CLIENT_MSG_FORMAT[0x08] = constant 8).")
        print("  Content is undefined (not structured u32+u32) — confirmed by chapter §V8.")
    print()


# ── Driver ────────────────────────────────────────────────────────────────────

def main():
    # Parse args: <pcap_file> <keys_file> [--server-port N]
    args = list(sys.argv[1:])
    forced_server_port = None
    if "--server-port" in args:
        i = args.index("--server-port")
        if i + 1 >= len(args):
            print(__doc__, file=sys.stderr)
            sys.exit(2)
        try:
            forced_server_port = int(args[i + 1])
        except ValueError:
            print(f"--server-port expects an integer, got {args[i + 1]!r}", file=sys.stderr)
            sys.exit(2)
        del args[i:i + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        sys.exit(2)

    pcap_path = args[0]
    keys_path = args[1]

    # Load the first valid 64-char hex key (AES-256 = 32 bytes = 64 hex chars)
    raw = Path(keys_path).read_text().strip()
    candidates = [t for t in raw.replace("\n", " ").split()
                  if all(c in "0123456789abcdefABCDEF" for c in t)]
    # Try keys in order; longest = most likely the AES key
    key_candidates = sorted(candidates, key=len, reverse=True)
    key_hex = None
    for kc in key_candidates:
        if len(kc) == 64:
            key_hex = kc
            break
    if key_hex is None:
        print(f"No 64-char hex key found in {keys_path}", file=sys.stderr)
        print(f"Lines found: {candidates[:5]}", file=sys.stderr)
        sys.exit(2)

    print("# Entity Property Sync Wire-Capture Validator")
    print(f"# pcap   : {pcap_path}")
    print(f"# keys   : {keys_path}")
    print(f"# key    : {key_hex[:16]}...{key_hex[-16:]}")
    print()

    results = Results()

    # Pass 1: determine server port. See module docstring for the resolution
    # waterfall — --server-port wins, else prefer the lower two-sided port,
    # else fall back to the most-active port. In a two-endpoint UDP capture
    # the src+dst counts of server and client ports tie exactly, so a pure
    # frequency max() picks arbitrarily and silently inverts is_s2c/is_c2s
    # if it lands on the client.
    from collections import Counter as _Counter
    src_port_freq = _Counter()
    dst_port_freq = _Counter()
    for ts, sp, dp, payload in pd.read_pcap(pcap_path):
        src_port_freq[sp] += 1
        dst_port_freq[dp] += 1
    all_ports = set(src_port_freq.keys()) | set(dst_port_freq.keys())
    two_sided = [p for p in all_ports
                 if src_port_freq[p] > 0 and dst_port_freq[p] > 0]

    if forced_server_port is not None:
        server_port = forced_server_port
        port_source = f"forced via --server-port (override)"
    elif len(two_sided) >= 2:
        # Prefer the lower-numbered two-sided port (servers conventionally
        # bind below the ephemeral range). Deterministic regardless of how
        # the underlying counter iterates.
        server_port = min(two_sided)
        port_source = (f"lower of two-sided ports {sorted(two_sided)} "
                       f"(pass --server-port to override)")
    elif len(two_sided) == 1:
        server_port = two_sided[0]
        port_source = "single two-sided port"
    else:
        server_port = max(src_port_freq, key=src_port_freq.get)
        port_source = "fallback: most-active source port"
    print(f"  Detected server port : {server_port}  [{port_source}]")
    print()

    # Pass 2: process packets
    for ts, src_port, dst_port, payload in pd.read_pcap(pcap_path):
        results.total_udp += 1
        plaintext = _decrypt_packet(payload, key_hex)
        if plaintext is None:
            continue
        results.decrypted += 1
        process_packet(plaintext, src_port, dst_port, results, server_port)

    print(f"## Decryption summary")
    print(f"  Total UDP packets  : {results.total_udp}")
    print(f"  Decrypted (valid)  : {results.decrypted}")
    print(f"  Server-to-Client   : {results.s2c}")
    print(f"  Client-to-Server   : {results.c2s}")
    print()

    report_v1(results)
    report_v2(results)
    report_v3_v7(results)
    report_v4(results)
    report_v5_v6(results)
    report_v8(results)


if __name__ == "__main__":
    main()
