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
import subprocess
import sys

# ── Mercury flag constants (from packet.rs / packet.hpp) ──────────────────────

FLAG_HAS_REQUESTS  = 0x01  # Footer includes first_req_offset (u16)
FLAG_PIGGYBACK     = 0x02  # Piggybacked sub-packets (unused)
FLAG_HAS_ACKS      = 0x04  # Footer includes ack_count + acks
FLAG_ON_CHANNEL    = 0x08  # Sent on persistent channel
FLAG_RELIABLE      = 0x10  # Must be ACKed
FLAG_FRAGMENTED    = 0x20  # Fragment of larger bundle
FLAG_HAS_SEQUENCE  = 0x40  # Footer includes seq_id
FLAG_INDEXED       = 0x80  # Indexed sub-channel (unused)

# Server→Client message IDs
# Message IDs verified against C++ messages.hpp / messages.cpp
SERVER_MSG_NAMES = {
    0x00: "BASEMSG_LOGIN_REPLY",
    0x01: "BASEMSG_AUTHENTICATE_REPLY",
    0x02: "BASEMSG_REPLY_MESSAGE",
    0x03: "BASEMSG_SET_GAME_TIME",
    0x04: "BASEMSG_RESET_ENTITIES",
    0x05: "BASEMSG_CREATE_BASE_PLAYER",
    0x06: "BASEMSG_CREATE_CELL_PLAYER",
    0x07: "BASEMSG_SPACE_DATA",
    0x08: "BASEMSG_SPACE_VIEWPORT_INFO",
    0x09: "BASEMSG_CREATE_ENTITY",
    0x0A: "BASEMSG_LOGGED_OFF",
    0x0B: "BASEMSG_CREATE_CELL_ENTITY",
    0x0C: "BASEMSG_RESTORE_CLIENT",
    0x0D: "BASEMSG_TICK_SYNC",
    0x0E: "BASEMSG_SET_SPACE_VIEWPORT",
    0x36: "BASEMSG_RESOURCE_FRAGMENT",
    0x31: "BASEMSG_FORCED_POSITION",
    0xFF: "BASEMSG_CONNECT_REPLY",
}

CLIENT_MSG_NAMES = {
    0x00: "BASEAPP_LOGIN",
    0x01: "AUTHENTICATE",
    0x03: "AVATAR_UPDATE_EXPLICIT",
    0x06: "SWITCH_INTERFACE",
    0x07: "REQUEST_ENTITY_UPDATE",
    0x08: "ENABLE_ENTITIES",
    0x09: "VIEWPORT_ACK",
    0x0A: "VEHICLE_ACK",
    0x0B: "RESTORE_CLIENT_ACK",
    0x0C: "DISCONNECT",
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

# Server→Client message payload formats (from messages.cpp)
# 'constant' = fixed-length, 'word' = u16 length prefix
SERVER_MSG_FORMAT = {
    0x03: ('constant', 4),     # SET_GAME_TIME
    0x04: ('constant', 1),     # RESET_ENTITIES
    0x05: ('word', None),      # CREATE_BASE_PLAYER
    0x06: ('word', None),      # CREATE_CELL_PLAYER
    0x07: ('word', None),      # SPACE_DATA
    0x08: ('constant', 13),    # SPACE_VIEWPORT_INFO
    0x09: ('word', None),      # CREATE_ENTITY
    0x0A: ('constant', 1),     # LOGGED_OFF
    0x0B: ('word', None),      # CREATE_CELL_ENTITY
    0x0C: ('word', None),      # RESTORE_CLIENT
    0x0D: ('constant', 8),     # TICK_SYNC
    0x0E: ('constant', 1),     # SET_SPACE_VIEWPORT
    0x31: ('constant', 49),    # FORCED_POSITION
    0x36: ('word', None),      # RESOURCE_FRAGMENT
    0xFF: ('word', None),      # CONNECT_REPLY
}

CLIENT_MSG_FORMAT = {
    0x01: ('word', None),      # AUTHENTICATE
    0x03: ('constant', 40),    # AVATAR_UPDATE_EXPLICIT
    0x06: ('constant', 0),     # SWITCH_INTERFACE
    0x08: ('constant', 8),     # ENABLE_ENTITIES
    0x09: ('constant', 8),     # VIEWPORT_ACK
    0x0A: ('constant', 8),     # VEHICLE_ACK
    0x0B: ('word', None),      # RESTORE_CLIENT_ACK
    0x0C: ('constant', 1),     # DISCONNECT
}


def decrypt_aes256_cbc(key_hex, ciphertext):
    iv_hex = "00" * 16
    proc = subprocess.run(
        ["openssl", "enc", "-aes-256-cbc", "-d", "-nopad",
         "-K", key_hex, "-iv", iv_hex],
        input=ciphertext, capture_output=True,
    )
    if proc.returncode != 0:
        return None
    return proc.stdout


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


def parse_messages(body, is_server):
    """Parse message payloads from a Mercury body."""
    offset = 0
    msg_format = SERVER_MSG_FORMAT if is_server else CLIENT_MSG_FORMAT
    msg_names = SERVER_MSG_NAMES if is_server else CLIENT_MSG_NAMES

    while offset < len(body):
        msg_id = body[offset]
        offset += 1

        # Entity methods (0x80+) always use WORD_LENGTH
        if msg_id >= 0x80:
            if is_server:
                name = ENTITY_CLIENT_METHODS.get(msg_id)
            else:
                name = ENTITY_BASE_METHODS.get(msg_id)
            if name is None:
                name = f"entity_method_{msg_id:#04x}"
            if offset + 2 > len(body):
                yield (msg_id, name, body[offset:])
                break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]
            offset += 2
            payload = body[offset:offset+word_len]
            offset += word_len
            yield (msg_id, name, payload)
            continue

        name = msg_names.get(msg_id, f"msg_{msg_id:#04x}")

        if msg_id in msg_format:
            fmt, length = msg_format[msg_id]
            if fmt == 'constant':
                payload = body[offset:offset+length]
                offset += length
            else:
                if offset + 2 > len(body):
                    yield (msg_id, name, body[offset:])
                    break
                word_len = struct.unpack('<H', body[offset:offset+2])[0]
                offset += 2
                payload = body[offset:offset+word_len]
                offset += word_len
        else:
            # Unknown system message — try WORD_LENGTH
            if offset + 2 > len(body):
                yield (msg_id, name, body[offset:])
                break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]
            offset += 2
            payload = body[offset:offset+word_len]
            offset += word_len

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
