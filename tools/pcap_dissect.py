#!/usr/bin/env python3
"""
Dissect a Mercury pcap capture from the SGW sniffer.

Usage:
    python3 tools/pcap_dissect.py <pcap_file> <keys_file>

Decrypts AES-256-CBC packets (IV=0, HMAC-MD5 trailer) and parses Mercury
packet headers + message payloads.

Mercury footer layout (stripped backward from end of plaintext):
  u8   ack_count          (outermost: if FLAG_HAS_ACKS = 0x04)
  u32  ack[ack_count]     (if FLAG_HAS_ACKS)
  u32  seq_id             (if FLAG_HAS_SEQUENCE = 0x40)
  u32  frag_end           (if FLAG_FRAGMENTED = 0x20)
  u32  frag_begin         (if FLAG_FRAGMENTED)
  u16  first_req_offset   (innermost: if FLAG_HAS_REQUESTS = 0x01)

Body = plaintext[1..footer_start]
"""

import struct
import sys

from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

# ── Mercury flag constants (from packet.rs / packet.hpp) ──────────────────────

FLAG_HAS_REQUESTS  = 0x01  # Footer includes first_req_offset (u16)
FLAG_PIGGYBACK     = 0x02  # Piggybacked sub-packets (unused)
FLAG_HAS_ACKS      = 0x04  # Footer includes ack_count + acks
FLAG_ON_CHANNEL    = 0x08  # Sent on persistent channel
FLAG_RELIABLE      = 0x10  # Must be ACKed
FLAG_FRAGMENTED    = 0x20  # Fragment of larger bundle
FLAG_HAS_SEQUENCE  = 0x40  # Footer includes seq_id
FLAG_INDEXED       = 0x80  # Indexed sub-channel (unused)

# Server→Client message IDs — extracted from live binary at SGW.exe global
# 0x01F72518 (BWNetDriver::ClientInterface InterfaceElementVec) on 2026-05-15.
# 57 entries covering msg_ids 0x00..0x38. msg_ids 0x39..0x7F are reserved/unused;
# 0x80..0xFE are dynamically registered EntityMethod slots (all share the generic
# "entityMessage" wire handler at 0x01ED1CBC).
SERVER_MSG_NAMES = {
    0x00: "authenticate",
    0x01: "bandwidthNotification",
    0x02: "updateFrequencyNotification",
    0x03: "setGameTime",
    0x04: "resetEntities",
    0x05: "createBasePlayer",
    0x06: "createCellPlayer",
    0x07: "spaceData",
    0x08: "spaceViewportInfo",
    0x09: "createEntity",
    0x0A: "updateEntity",
    0x0B: "entityInvisible",
    0x0C: "leaveAoI",
    0x0D: "tickSync",
    0x0E: "setSpaceViewport",
    0x0F: "setVehicle",
    0x10: "avatarUpdateNoAliasFullPosYawPitchRoll",
    0x11: "avatarUpdateNoAliasFullPosYawPitch",
    0x12: "avatarUpdateNoAliasFullPosYaw",
    0x13: "avatarUpdateNoAliasFullPosNoDir",
    0x14: "avatarUpdateNoAliasOnChunkYawPitchRoll",
    0x15: "avatarUpdateNoAliasOnChunkYawPitch",
    0x16: "avatarUpdateNoAliasOnChunkYaw",
    0x17: "avatarUpdateNoAliasOnChunkNoDir",
    0x18: "avatarUpdateNoAliasOnGroundYawPitchRoll",
    0x19: "avatarUpdateNoAliasOnGroundYawPitch",
    0x1A: "avatarUpdateNoAliasOnGroundYaw",
    0x1B: "avatarUpdateNoAliasOnGroundNoDir",
    0x1C: "avatarUpdateNoAliasNoPosYawPitchRoll",
    0x1D: "avatarUpdateNoAliasNoPosYawPitch",
    0x1E: "avatarUpdateNoAliasNoPosYaw",
    0x1F: "avatarUpdateNoAliasNoPosNoDir",
    0x20: "avatarUpdateAliasFullPosYawPitchRoll",
    0x21: "avatarUpdateAliasFullPosYawPitch",
    0x22: "avatarUpdateAliasFullPosYaw",
    0x23: "avatarUpdateAliasFullPosNoDir",
    0x24: "avatarUpdateAliasOnChunkYawPitchRoll",
    0x25: "avatarUpdateAliasOnChunkYawPitch",
    0x26: "avatarUpdateAliasOnChunkYaw",
    0x27: "avatarUpdateAliasOnChunkNoDir",
    0x28: "avatarUpdateAliasOnGroundYawPitchRoll",
    0x29: "avatarUpdateAliasOnGroundYawPitch",
    0x2A: "avatarUpdateAliasOnGroundYaw",
    0x2B: "avatarUpdateAliasOnGroundNoDir",
    0x2C: "avatarUpdateAliasNoPosYawPitchRoll",
    0x2D: "avatarUpdateAliasNoPosYawPitch",
    0x2E: "avatarUpdateAliasNoPosYaw",
    0x2F: "avatarUpdateAliasNoPosNoDir",
    0x30: "detailedPosition",
    0x31: "forcedPosition",
    0x32: "controlEntity",
    0x33: "voiceData",
    0x34: "restoreClient",
    0x35: "restoreBaseApp",
    0x36: "resourceFragment",
    0x37: "loggedOff",
    0x38: "entityMessage",
    0xFF: "connectReply",
}

# Client→Server message IDs — extracted from live binary at SGW.exe global
# 0x01EF24CC (BaseAppExtInterface InterfaceElementVec) via static initializer
# at 0x017bac00 on 2026-05-15. Name string table at 0x019D0880.
# 14 entries covering msg_ids 0x00..0x0D; 0x80..0xFE are dynamically
# registered EntityMethod slots (mirror of server-side dispatch).
CLIENT_MSG_NAMES = {
    0x00: "baseAppLogin",
    0x01: "authenticate",
    0x02: "avatarUpdateImplicit",
    0x03: "avatarUpdateExplicit",
    0x04: "avatarUpdateWardImplicit",
    0x05: "avatarUpdateWardExplicit",
    0x06: "switchInterface",
    0x07: "requestEntityUpdate",
    0x08: "enableEntities",
    0x09: "setSpaceViewportAck",
    0x0A: "setVehicleAck",
    0x0B: "restoreClientAck",
    0x0C: "disconnectClient",
    0x0D: "entityMessage",
}

ENTITY_CLIENT_METHODS = {
    0x80: "onVersionInfo",
    0x81: "onCookedDataError",
    0x82: "onCharacterList",
    0x83: "onCharacterCreateFailed",
    0x84: "onCharacterVisuals",
    0x85: "onCharacterLoadFailed",
}

ENTITY_BASE_METHODS = {
    0xC0: "versionInfoRequest",
    0xC1: "elementDataRequest",
    0xC2: "logOff",
    0xC3: "createCharacter",
    0xC4: "playCharacter",
    0xC5: "deleteCharacter",
    0xC6: "requestCharacterVisuals",
    0xC7: "onClientVersion",
}

# Message-format tuple layout:
#   ('constant', fixed_len, None)         # fixed-length payload, no prefix
#   ('var',      None,      prefix_width) # variable-length, prefix_width ∈ {1, 2, 4}
#
# The `prefix_width` field preserves the discovery from the static initializer at
# 0x017BAC60 that BigWorld's `InterfaceElement::size` field encodes the length-
# prefix byte-width (u8/u16/u32) for variable-length entries. All current
# entries observed on this build use u16 prefixes, but the parser accepts any
# of the three widths so we can keep the discovered semantics rather than
# collapsing them to "word".

# Server→Client message payload formats — confirmed against the live
# InterfaceElementVec at 0x01F72518 and against observed packet body sizes
# in this pcap.
#
# Note: msg_ids 0x0A..0x0C (updateEntity, entityInvisible, leaveAoI) were
# previously misidentified as LOGGED_OFF/CREATE_CELL_ENTITY/RESTORE_CLIENT
# (those are at 0x37, n/a, 0x34 respectively). Corrected here.
SERVER_MSG_FORMAT = {
    0x00: ('var',      None, 2),  # authenticate
    0x01: ('constant', 4,    None),  # bandwidthNotification
    0x02: ('constant', 1,    None),  # updateFrequencyNotification
    0x03: ('constant', 4,    None),  # setGameTime
    0x04: ('constant', 1,    None),  # resetEntities
    0x05: ('var',      None, 2),  # createBasePlayer (variable per-entity props)
    0x06: ('var',      None, 2),  # createCellPlayer (variable)
    0x07: ('var',      None, 2),  # spaceData
    0x08: ('constant', 13,   None),  # spaceViewportInfo
    0x09: ('var',      None, 2),  # createEntity (variable)
    0x0A: ('var',      None, 2),  # updateEntity (variable)
    0x0B: ('constant', 5,    None),  # entityInvisible
    0x0C: ('var',      None, 2),  # leaveAoI
    0x0D: ('constant', 8,    None),  # tickSync
    0x0E: ('constant', 1,    None),  # setSpaceViewport
    0x0F: ('constant', 4,    None),  # setVehicle
    # avatarUpdate family — 32 variants, BigWorld bit-packed wire format.
    # Sizes match the +0x04 fixed-length field in the static InterfaceElement
    # table; differential 1-byte deltas confirm 8-bit quantized YPR angles.
    0x10: ('constant', 25,   None),  # NoAliasFullPosYawPitchRoll
    0x11: ('constant', 24,   None),  # NoAliasFullPosYawPitch
    0x12: ('constant', 23,   None),  # NoAliasFullPosYaw
    0x13: ('constant', 22,   None),  # NoAliasFullPosNoDir
    0x14: ('constant', 25,   None),  # NoAliasOnChunkYawPitchRoll
    0x15: ('constant', 24,   None),  # NoAliasOnChunkYawPitch
    0x16: ('constant', 23,   None),  # NoAliasOnChunkYaw
    0x17: ('constant', 22,   None),  # NoAliasOnChunkNoDir
    0x18: ('constant', 25,   None),  # NoAliasOnGroundYawPitchRoll
    0x19: ('constant', 24,   None),  # NoAliasOnGroundYawPitch
    0x1A: ('constant', 23,   None),  # NoAliasOnGroundYaw
    0x1B: ('constant', 22,   None),  # NoAliasOnGroundNoDir
    0x1C: ('constant', 13,   None),  # NoAliasNoPosYawPitchRoll
    0x1D: ('constant', 12,   None),  # NoAliasNoPosYawPitch
    0x1E: ('constant', 11,   None),  # NoAliasNoPosYaw
    0x1F: ('constant', 10,   None),  # NoAliasNoPosNoDir
    0x20: ('constant', 25,   None),  # AliasFullPosYawPitchRoll
    0x21: ('constant', 24,   None),  # AliasFullPosYawPitch
    0x22: ('constant', 23,   None),  # AliasFullPosYaw
    0x23: ('constant', 22,   None),  # AliasFullPosNoDir
    0x24: ('constant', 25,   None),  # AliasOnChunkYawPitchRoll
    0x25: ('constant', 24,   None),  # AliasOnChunkYawPitch
    0x26: ('constant', 23,   None),  # AliasOnChunkYaw
    0x27: ('constant', 22,   None),  # AliasOnChunkNoDir
    0x28: ('constant', 25,   None),  # AliasOnGroundYawPitchRoll
    0x29: ('constant', 24,   None),  # AliasOnGroundYawPitch
    0x2A: ('constant', 23,   None),  # AliasOnGroundYaw
    0x2B: ('constant', 22,   None),  # AliasOnGroundNoDir
    0x2C: ('constant', 13,   None),  # AliasNoPosYawPitchRoll
    0x2D: ('constant', 12,   None),  # AliasNoPosYawPitch
    0x2E: ('constant', 11,   None),  # AliasNoPosYaw
    0x2F: ('constant', 10,   None),  # AliasNoPosNoDir
    0x30: ('constant', 41,   None),  # detailedPosition
    0x31: ('constant', 49,   None),  # forcedPosition
    0x32: ('constant', 5,    None),  # controlEntity
    0x33: ('var',      None, 2),  # voiceData (variable)
    0x34: ('var',      None, 2),  # restoreClient
    0x35: ('var',      None, 2),  # restoreBaseApp
    0x36: ('var',      None, 2),  # resourceFragment (variable)
    0x37: ('constant', 1,    None),  # loggedOff (1-byte reason code)
    0x38: ('var',      None, 2),  # entityMessage (msg_id 0x80..0xFE share this handler)
    0xFF: ('var',      None, 2),  # connectReply
}

# Client→Server message formats — sizes from InterfaceElement::add immediate
# args in static initializer at 0x017bac00, validated 2026-05-15 against live
# memory at 0xFFE3A080..0xFFE3A207 (the heap-allocated InterfaceElement array
# pointed to by BaseAppExtInterface's slot pointers at 0x01EF24E0+).
#
# Wire-framing discriminator confirmed: the `flag` byte at struct offset +0x01
# determines whether `size` (at +0x04) is a length-prefix width or a fixed
# payload length:
#   flag=1 → variable-length, size ∈ {1, 2, 4} is the prefix width (u8/u16/u32)
#   flag=0 → fixed-length payload of exactly `size` bytes
# All four variable client→server entries on this build use u16 prefixes
# (flag=1, size=2); the rest are fixed-length.
CLIENT_MSG_FORMAT = {
    0x00: ('var',      None, 2),  # baseAppLogin (flag=1, size=2 — live-validated)
    0x01: ('var',      None, 2),  # authenticate (flag=1, size=2 — live-validated)
    0x02: ('constant', 36,   None),  # avatarUpdateImplicit (flag=0, size=0x24)
    0x03: ('constant', 40,   None),  # avatarUpdateExplicit (flag=0, size=0x28)
    0x04: ('constant', 36,   None),  # avatarUpdateWardImplicit (flag=0, size=0x24 — live-validated)
    0x05: ('constant', 40,   None),  # avatarUpdateWardExplicit (flag=0, size=0x28 — live-validated)
    0x06: ('constant', 0,    None),  # switchInterface (flag=0, size=0 — parameterless)
    0x07: ('var',      None, 2),  # requestEntityUpdate (flag=1, size=2 — live-validated)
    0x08: ('constant', 8,    None),  # enableEntities (flag=0, size=8 — 8 dummy bytes per emitter at 0x00dd928f, NOT i32 entity_id + i32 flag; SGW expansion of BigWorld's 1-byte keepBase, body content undefined)
    0x09: ('constant', 8,    None),  # setSpaceViewportAck (flag=0, size=8)
    0x0A: ('constant', 8,    None),  # setVehicleAck (flag=0, size=8)
    0x0B: ('constant', 4,    None),  # restoreClientAck (flag=0, size=4 — fixed 4-byte payload, NOT u32 prefix; likely i32 entity_id)
    0x0C: ('constant', 1,    None),  # disconnectClient (flag=0, size=1 — reason byte)
    0x0D: ('var',      None, 2),  # entityMessage (flag=1, size=2 — entity-method dispatch envelope)
}


# Per-key cache so repeat decrypts on the same key_hex don't re-parse hex
# on every packet. Keyed by the session-key hex string itself, which makes
# the cache trivially correct even if a future caller rotates keys mid-run.
_AES_KEY_BYTES_CACHE: dict[str, bytes] = {}
_AES_IV = b"\x00" * 16


def decrypt_aes256_cbc(key_hex, ciphertext):
    if len(ciphertext) % 16 != 0:
        return None
    key_bytes = _AES_KEY_BYTES_CACHE.get(key_hex)
    if key_bytes is None:
        key_bytes = bytes.fromhex(key_hex)
        _AES_KEY_BYTES_CACHE[key_hex] = key_bytes
    try:
        cipher = Cipher(algorithms.AES(key_bytes), modes.CBC(_AES_IV))
        dec = cipher.decryptor()
        return dec.update(ciphertext) + dec.finalize()
    except Exception:
        return None


def strip_pkcs7(data):
    if not data:
        return data
    pad_len = data[-1]
    if pad_len < 1 or pad_len > 16:
        return data
    if all(b == pad_len for b in data[-pad_len:]):
        return data[:-pad_len]
    return data


def read_pcap(path):
    with open(path, 'rb') as f:
        magic, ver_maj, ver_min, _, _, snaplen, linktype = struct.unpack('<IHHiiII', f.read(24))
        if magic not in (0xa1b2c3d4, 0xd4c3b2a1):
            raise ValueError(f"Not a pcap file (magic={magic:#x})")
        while True:
            hdr = f.read(16)
            if len(hdr) < 16:
                break
            ts_sec, ts_usec, incl_len, orig_len = struct.unpack('<IIII', hdr)
            pkt_data = f.read(incl_len)
            if len(pkt_data) < incl_len:
                break
            if linktype == 1:
                if len(pkt_data) < 14:
                    continue
                eth_type = struct.unpack('>H', pkt_data[12:14])[0]
                if eth_type != 0x0800:
                    continue
                ip_start = 14
            else:
                continue
            if len(pkt_data) < ip_start + 20:
                continue
            ihl = (pkt_data[ip_start] & 0x0F) * 4
            protocol = pkt_data[ip_start + 9]
            if protocol != 17:
                continue
            udp_start = ip_start + ihl
            if len(pkt_data) < udp_start + 8:
                continue
            src_port, dst_port = struct.unpack('>HH', pkt_data[udp_start:udp_start + 4])
            payload = pkt_data[udp_start + 8:]
            ts = ts_sec + ts_usec / 1_000_000
            yield (ts, src_port, dst_port, payload)


def parse_mercury_footer(data):
    """Parse Mercury packet by stripping footers from the end (backward).

    Returns (flags, seq_id, frag_begin, frag_end, first_req_offset, body, acks) or None.
    """
    if len(data) < 1:
        return None

    flags = data[0]
    end = len(data)  # exclusive; shrinks as we pop footers

    # Pop helpers
    def pop_u8():
        nonlocal end
        if end < 1: return None
        end -= 1
        return data[end]

    def pop_u32():
        nonlocal end
        if end < 4: return None
        end -= 4
        return struct.unpack('<I', data[end:end+4])[0]

    def pop_u16():
        nonlocal end
        if end < 2: return None
        end -= 2
        return struct.unpack('<H', data[end:end+2])[0]

    # 1. Acks (outermost)
    acks = []
    if flags & FLAG_HAS_ACKS:
        ack_count = pop_u8()
        if ack_count is None: return None
        for _ in range(ack_count):
            a = pop_u32()
            if a is None: return None
            acks.append(a)
        acks.reverse()

    # 2. Sequence ID
    seq_id = None
    if flags & FLAG_HAS_SEQUENCE:
        seq_id = pop_u32()
        if seq_id is None: return None

    # 3. Fragment IDs
    frag_begin = frag_end = None
    if flags & FLAG_FRAGMENTED:
        frag_end = pop_u32()
        if frag_end is None: return None
        frag_begin = pop_u32()
        if frag_begin is None: return None

    # 4. First request offset (innermost)
    first_req_offset = None
    if flags & FLAG_HAS_REQUESTS:
        first_req_offset = pop_u16()
        if first_req_offset is None: return None

    # Body is everything between flags byte (index 0) and first footer
    body = data[1:end]

    return (flags, seq_id, frag_begin, frag_end, first_req_offset, body, acks)


def format_flags(flags):
    parts = []
    if flags & FLAG_HAS_REQUESTS: parts.append("REQ")
    if flags & FLAG_PIGGYBACK: parts.append("PIG")
    if flags & FLAG_HAS_ACKS: parts.append("ACK")
    if flags & FLAG_ON_CHANNEL: parts.append("CHAN")
    if flags & FLAG_RELIABLE: parts.append("REL")
    if flags & FLAG_FRAGMENTED: parts.append("FRAG")
    if flags & FLAG_HAS_SEQUENCE: parts.append("SEQ")
    if flags & FLAG_INDEXED: parts.append("IDX")
    return "|".join(parts) if parts else "NONE"


_PREFIX_STRUCT = {1: '<B', 2: '<H', 4: '<I'}


def _read_var_prefix(body, offset, prefix_width):
    """Read a length-prefix of `prefix_width` bytes from body[offset:].

    Returns (length, new_offset) or (None, offset) if truncated.
    """
    fmt = _PREFIX_STRUCT.get(prefix_width)
    if fmt is None:
        raise ValueError(f"Unsupported prefix width: {prefix_width}")
    if offset + prefix_width > len(body):
        return None, offset
    length = struct.unpack(fmt, body[offset:offset + prefix_width])[0]
    return length, offset + prefix_width


def parse_messages(body, is_server):
    """Parse message payloads from a Mercury body."""
    offset = 0
    msg_format = SERVER_MSG_FORMAT if is_server else CLIENT_MSG_FORMAT
    msg_names = SERVER_MSG_NAMES if is_server else CLIENT_MSG_NAMES

    while offset < len(body):
        msg_id = body[offset]
        offset += 1

        # Entity methods (0x80+) always use u16 length prefix
        if msg_id >= 0x80:
            if is_server:
                name = ENTITY_CLIENT_METHODS.get(msg_id)
            else:
                name = ENTITY_BASE_METHODS.get(msg_id)
            if name is None:
                name = f"entity_method_{msg_id:#04x}"
            length, offset = _read_var_prefix(body, offset, 2)
            if length is None:
                yield (msg_id, name, body[offset:])
                break
            payload = body[offset:offset + length]
            offset += length
            yield (msg_id, name, payload)
            continue

        name = msg_names.get(msg_id, f"msg_{msg_id:#04x}")

        if msg_id in msg_format:
            style, fixed_len, prefix_width = msg_format[msg_id]
            if style == 'constant':
                payload = body[offset:offset + fixed_len]
                offset += fixed_len
            else:
                length, offset = _read_var_prefix(body, offset, prefix_width)
                if length is None:
                    yield (msg_id, name, body[offset:])
                    break
                payload = body[offset:offset + length]
                offset += length
        else:
            # Unknown system message — fall back to u16 length prefix
            length, offset = _read_var_prefix(body, offset, 2)
            if length is None:
                yield (msg_id, name, body[offset:])
                break
            payload = body[offset:offset + length]
            offset += length

        yield (msg_id, name, payload)


def hex_dump(data, max_bytes=64):
    h = data[:max_bytes].hex()
    grouped = " ".join(h[i:i+2] for i in range(0, len(h), 2))
    if len(data) > max_bytes:
        grouped += f" ... ({len(data)} bytes total)"
    return grouped


def parse_create_base_player(payload):
    if len(payload) < 6:
        return f"  (too short: {len(payload)} bytes)"
    entity_id = struct.unpack('<I', payload[0:4])[0]
    entity_type = struct.unpack('<H', payload[4:6])[0]
    rest = payload[6:]
    return f"  entity_id={entity_id}, entity_type={entity_type}, props={len(rest)} bytes: {hex_dump(rest, 48)}"


def parse_cell_player_create(payload):
    if len(payload) < 28:
        return f"  (too short: {len(payload)} bytes)"
    off = 0
    eid = struct.unpack('<I', payload[off:off+4])[0]; off += 4
    sid = struct.unpack('<I', payload[off:off+4])[0]; off += 4
    vid = struct.unpack('<I', payload[off:off+4])[0]; off += 4
    px = struct.unpack('<f', payload[off:off+4])[0]; off += 4
    py = struct.unpack('<f', payload[off:off+4])[0]; off += 4
    pz = struct.unpack('<f', payload[off:off+4])[0]; off += 4
    # Direction is 3 packed int8s (yaw, pitch, roll), each = angle * 256 / 360
    rest = payload[off:]
    return (f"  entity_id={eid}, space_id={sid}, vehicle_id={vid}, "
            f"pos=({px:.2f}, {py:.2f}, {pz:.2f}), rest={len(rest)}B: {hex_dump(rest, 32)}")


def parse_viewport_info(payload):
    if len(payload) < 13:
        return f"  (too short: {len(payload)}B)"
    eid1 = struct.unpack('<I', payload[0:4])[0]
    eid2 = struct.unpack('<I', payload[4:8])[0]
    space_id = struct.unpack('<I', payload[8:12])[0]
    viewport_id = payload[12]
    return f"  entity_id={eid1}, entity_id2={eid2}, space_id={space_id}, viewport={viewport_id}"


def parse_forced_position(payload):
    if len(payload) < 28:
        return f"  (payload={len(payload)}B): {hex_dump(payload)}"
    off = 0
    eid = struct.unpack('<I', payload[off:off+4])[0]; off += 4
    sid = struct.unpack('<I', payload[off:off+4])[0]; off += 4
    vid = struct.unpack('<I', payload[off:off+4])[0]; off += 4
    px = struct.unpack('<f', payload[off:off+4])[0]; off += 4
    py = struct.unpack('<f', payload[off:off+4])[0]; off += 4
    pz = struct.unpack('<f', payload[off:off+4])[0]; off += 4
    rest = payload[off:]
    return (f"  entity_id={eid}, space_id={sid}, vehicle_id={vid}, "
            f"pos=({px:.2f}, {py:.2f}, {pz:.2f}), rest={hex_dump(rest)}")


def parse_tick_sync(payload):
    if len(payload) < 8:
        return f"  (too short)"
    tick = struct.unpack('<I', payload[0:4])[0]
    rate = struct.unpack('<I', payload[4:8])[0]
    return f"  tick={tick}, rate={rate}"


def parse_game_state(payload):
    return f"  {hex_dump(payload)}"


def parse_reset_entities(payload):
    if len(payload) >= 1:
        keep_base = payload[0]
        return f"  keepBase={keep_base}"
    return f"  {hex_dump(payload)}"


def parse_char_list(payload):
    """Parse onCharacterList payload."""
    if len(payload) < 4:
        return f"  (too short)"
    off = 0
    count = struct.unpack('<I', payload[off:off+4])[0]; off += 4
    lines = [f"  count={count}"]
    for i in range(count):
        if off + 4 > len(payload): break
        pid = struct.unpack('<i', payload[off:off+4])[0]; off += 4
        lines.append(f"  [{i}] player_id={pid}, rest: {hex_dump(payload[off:off+64], 64)}")
        # Skip the rest of this character entry (we'd need to know the exact size)
        break
    if count == 0:
        lines.append("  (empty — triggers char creation screen)")
    return "\n".join(lines)


def describe_payload(msg_id, name, payload, is_server):
    if not is_server:
        if msg_id == 0x08:
            return parse_reset_entities(payload) if len(payload) >= 8 else ""
        return ""
    if msg_id == 0x05: return parse_create_base_player(payload)
    if msg_id == 0x06: return parse_cell_player_create(payload)
    if msg_id == 0x08: return parse_viewport_info(payload)
    if msg_id == 0x31: return parse_forced_position(payload)
    if msg_id == 0x0D: return parse_tick_sync(payload)
    if msg_id == 0x04: return parse_reset_entities(payload)
    if msg_id == 0x82: return parse_char_list(payload)
    return ""


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <pcap_file> <keys_file>")
        sys.exit(1)

    pcap_path = sys.argv[1]
    keys_path = sys.argv[2]

    with open(keys_path) as f:
        key_hex = f.read().strip()

    SERVER_PORT = 32832
    pkt_num = 0
    first_ts = None

    print(f"Session key: {key_hex[:16]}...")
    print(f"Pcap: {pcap_path}")
    print("=" * 80)
    print()

    for ts, src_port, dst_port, payload in read_pcap(pcap_path):
        if first_ts is None:
            first_ts = ts

        is_server = (src_port == SERVER_PORT)
        direction = "S→C" if is_server else "C→S"
        rel_ts = ts - first_ts
        pkt_num += 1

        if src_port != SERVER_PORT and dst_port != SERVER_PORT:
            continue

        # Try unencrypted parse first (Phase 3 baseAppLogin from client)
        if len(payload) > 0 and payload[0] in (0x41, 0x01):
            parsed = parse_mercury_footer(payload)
            if parsed:
                flags, seq, fb, fe, req_off, body, acks = parsed
                # Heuristic: if body looks reasonable for baseAppLogin, treat as unencrypted
                if not is_server and flags == 0x41 and len(body) > 10:
                    print(f"#{pkt_num:3d} [{rel_ts:8.3f}s] {direction} UNENCRYPTED seq={seq} flags={flags:#04x}({format_flags(flags)}) body={len(body)}B")
                    print(f"     body: {hex_dump(body)}")
                    print()
                    continue

        # Encrypted: strip HMAC trailer (16 bytes), decrypt, strip PKCS7
        if len(payload) <= 16:
            continue

        ciphertext = payload[:-16]
        plaintext = decrypt_aes256_cbc(key_hex, ciphertext)
        if plaintext is None:
            print(f"#{pkt_num:3d} [{rel_ts:8.3f}s] {direction} (decrypt failed, {len(payload)}B)")
            print()
            continue

        plaintext = strip_pkcs7(plaintext)

        parsed = parse_mercury_footer(plaintext)
        if parsed is None:
            print(f"#{pkt_num:3d} [{rel_ts:8.3f}s] {direction} (parse failed, {len(plaintext)}B plaintext)")
            print(f"     raw: {hex_dump(plaintext)}")
            print()
            continue

        flags, seq, fb, fe, req_off, body, acks = parsed

        ack_str = f" acks={acks}" if acks else ""
        frag_str = f" frag=[{fb}-{fe}]" if fb is not None else ""
        req_str = f" req_off={req_off}" if req_off is not None else ""

        print(f"#{pkt_num:3d} [{rel_ts:8.3f}s] {direction} seq={seq} flags={flags:#04x}({format_flags(flags)}){ack_str}{frag_str}{req_str} body={len(body)}B")

        if len(body) == 0:
            print()
            continue

        for msg_id, name, msg_payload in parse_messages(body, is_server):
            print(f"     [{msg_id:#04x}] {name} ({len(msg_payload)}B)")
            extra = describe_payload(msg_id, name, msg_payload, is_server)
            if extra:
                print(extra)

        print()


if __name__ == "__main__":
    main()
