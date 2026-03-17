#!/usr/bin/env python3
"""
Investigate Mercury bundle corruption in createCellPlayer (msg_id 0x06).

Fragment reassembly: packets sharing the same (frag_begin, frag_end) range
belong to the same bundle. Their bodies are concatenated in seq_id order.

Client error:
  Bundle::iterator::unpack( entityMessage ): Not enough data on stream
  at 160 for request ID and NRO (5 left, needed 6)
  Nub::processOrderedPacket: Discarding bundle due to corrupted header
  for message id 136 (0x88)
"""

import struct
import sys
from collections import defaultdict
from Crypto.Cipher import AES

# ── Mercury flags ──
FLAG_HAS_REQUESTS  = 0x01
FLAG_HAS_ACKS      = 0x04
FLAG_ON_CHANNEL    = 0x08
FLAG_RELIABLE      = 0x10
FLAG_FRAGMENTED    = 0x20
FLAG_HAS_SEQUENCE  = 0x40

# ── Message IDs ──
SERVER_MSG_NAMES = {
    0x00: "LOGIN_REPLY",
    0x01: "AUTHENTICATE_REPLY",
    0x02: "REPLY_MESSAGE",
    0x03: "SET_GAME_TIME",
    0x04: "RESET_ENTITIES",
    0x05: "CREATE_BASE_PLAYER",
    0x06: "CREATE_CELL_PLAYER",
    0x07: "SPACE_DATA",
    0x08: "SPACE_VIEWPORT_INFO",
    0x09: "CREATE_ENTITY",
    0x0A: "LOGGED_OFF",
    0x0B: "CREATE_CELL_ENTITY",
    0x0C: "RESTORE_CLIENT",
    0x0D: "TICK_SYNC",
    0x0E: "SET_SPACE_VIEWPORT",
    0x10: "UPDATE_AVATAR",
    0x31: "FORCED_POSITION",
    0x36: "RESOURCE_FRAGMENT",
    0xFF: "CONNECT_REPLY",
}

SERVER_MSG_FORMAT = {
    0x03: ('constant', 4),
    0x04: ('constant', 1),
    0x05: ('word', None),
    0x06: ('word', None),
    0x07: ('word', None),
    0x08: ('constant', 13),
    0x09: ('word', None),
    0x0A: ('constant', 1),
    0x0B: ('word', None),
    0x0C: ('word', None),
    0x0D: ('constant', 8),
    0x0E: ('constant', 1),
    0x31: ('constant', 49),
    0x36: ('word', None),
    0xFF: ('word', None),
}


def decrypt_packet(key_bytes, payload):
    """Strip HMAC-MD5 (last 16), AES-256-CBC decrypt (IV=0), strip PKCS7."""
    if len(payload) <= 16:
        return None
    ciphertext = payload[:-16]
    if len(ciphertext) % 16 != 0:
        return None
    iv = b'\x00' * 16
    cipher = AES.new(key_bytes, AES.MODE_CBC, iv)
    plaintext = cipher.decrypt(ciphertext)
    # Strip PKCS7
    if plaintext:
        pad = plaintext[-1]
        if 1 <= pad <= 16 and all(b == pad for b in plaintext[-pad:]):
            plaintext = plaintext[:-pad]
    return plaintext


def parse_mercury(data):
    """Parse Mercury packet: flags byte at front, footers popped from end."""
    if len(data) < 1:
        return None
    flags = data[0]
    end = len(data)

    acks = []
    if flags & FLAG_HAS_ACKS:
        end -= 1
        if end < 1: return None
        ack_count = data[end]
        for _ in range(ack_count):
            end -= 4
            if end < 1: return None
            acks.append(struct.unpack('<I', data[end:end+4])[0])
        acks.reverse()

    seq_id = None
    if flags & FLAG_HAS_SEQUENCE:
        end -= 4
        if end < 1: return None
        seq_id = struct.unpack('<I', data[end:end+4])[0]

    frag_begin = frag_end = None
    if flags & FLAG_FRAGMENTED:
        end -= 4
        if end < 1: return None
        frag_end = struct.unpack('<I', data[end:end+4])[0]
        end -= 4
        if end < 1: return None
        frag_begin = struct.unpack('<I', data[end:end+4])[0]

    first_req_offset = None
    if flags & FLAG_HAS_REQUESTS:
        end -= 2
        if end < 1: return None
        first_req_offset = struct.unpack('<H', data[end:end+2])[0]

    body = data[1:end]
    return {
        'flags': flags,
        'seq_id': seq_id,
        'frag_begin': frag_begin,
        'frag_end': frag_end,
        'first_req_offset': first_req_offset,
        'body': body,
        'acks': acks,
    }


def read_pcap(path):
    """Yield (timestamp, src_port, dst_port, udp_payload) from pcap."""
    with open(path, 'rb') as f:
        magic = struct.unpack('<I', f.read(4))[0]
        if magic not in (0xa1b2c3d4, 0xd4c3b2a1):
            raise ValueError(f"Not a pcap: magic={magic:#x}")
        f.read(20)  # rest of global header

        while True:
            hdr = f.read(16)
            if len(hdr) < 16:
                break
            ts_sec, ts_usec, incl_len, _ = struct.unpack('<IIII', hdr)
            pkt_data = f.read(incl_len)
            if len(pkt_data) < incl_len:
                break
            if len(pkt_data) < 14:
                continue
            eth_type = struct.unpack('>H', pkt_data[12:14])[0]
            if eth_type != 0x0800:
                continue
            ip_start = 14
            if len(pkt_data) < ip_start + 20:
                continue
            ihl = (pkt_data[ip_start] & 0x0F) * 4
            protocol = pkt_data[ip_start + 9]
            if protocol != 17:
                continue
            udp_start = ip_start + ihl
            if len(pkt_data) < udp_start + 8:
                continue
            src_port, dst_port = struct.unpack('>HH', pkt_data[udp_start:udp_start+4])
            payload = pkt_data[udp_start + 8:]
            ts = ts_sec + ts_usec / 1_000_000
            yield (ts, src_port, dst_port, payload)


def hex_ascii_dump(data, context_start=0):
    """Hex + ASCII dump with offset labels."""
    lines = []
    for i in range(0, len(data), 16):
        addr = context_start + i
        chunk = data[i:i+16]
        hex_part = ' '.join(f'{b:02x}' for b in chunk)
        hex_part = hex_part.ljust(48)
        ascii_part = ''.join(chr(b) if 32 <= b < 127 else '.' for b in chunk)
        lines.append(f"  {addr:5d} | {hex_part} | {ascii_part}")
    return '\n'.join(lines)


def walk_messages(body):
    """Walk message headers in a bundle body, yielding (offset, msg_id, name, payload, truncated)."""
    offset = 0
    while offset < len(body):
        msg_start = offset
        msg_id = body[offset]
        offset += 1

        if msg_id >= 0x80:
            name = f"entity_method_{msg_id:#04x}"
            if offset + 2 > len(body):
                yield (msg_start, msg_id, name, body[offset:], True)
                break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]
            offset += 2
            if offset + word_len > len(body):
                yield (msg_start, msg_id, name, body[offset:], True)
                break
            payload = body[offset:offset + word_len]
            offset += word_len
            yield (msg_start, msg_id, name, payload, False)
            continue

        name = SERVER_MSG_NAMES.get(msg_id, f"msg_{msg_id:#04x}")

        if msg_id in SERVER_MSG_FORMAT:
            fmt, length = SERVER_MSG_FORMAT[msg_id]
            if fmt == 'constant':
                if offset + length > len(body):
                    yield (msg_start, msg_id, name, body[offset:], True)
                    break
                payload = body[offset:offset+length]
                offset += length
            else:
                if offset + 2 > len(body):
                    yield (msg_start, msg_id, name, body[offset:], True)
                    break
                word_len = struct.unpack('<H', body[offset:offset+2])[0]
                offset += 2
                if offset + word_len > len(body):
                    yield (msg_start, msg_id, name, body[offset:], True)
                    break
                payload = body[offset:offset + word_len]
                offset += word_len
        else:
            if offset + 2 > len(body):
                yield (msg_start, msg_id, name, body[offset:], True)
                break
            word_len = struct.unpack('<H', body[offset:offset+2])[0]
            offset += 2
            if offset + word_len > len(body):
                yield (msg_start, msg_id, name, body[offset:], True)
                break
            payload = body[offset:offset + word_len]
            offset += word_len

        yield (msg_start, msg_id, name, payload, False)


def analyze_bundle(body, label):
    """Deep analysis of a bundle containing createCellPlayer."""
    print(f"\n  === Message walk for {label} ===")
    for off, mid, name, payload, trunc in walk_messages(body):
        trunc_marker = " [TRUNCATED]" if trunc else ""
        print(f"    @{off:5d}: [{mid:#04x}] {name} ({len(payload)}B){trunc_marker}")

        if mid == 0x06:
            if len(payload) >= 28:
                eid = struct.unpack('<I', payload[0:4])[0]
                sid = struct.unpack('<I', payload[4:8])[0]
                vid = struct.unpack('<I', payload[8:12])[0]
                px = struct.unpack('<f', payload[12:16])[0]
                py = struct.unpack('<f', payload[16:20])[0]
                pz = struct.unpack('<f', payload[20:24])[0]
                print(f"           entity_id={eid}, space_id={sid}, vehicle_id={vid}")
                print(f"           pos=({px:.2f}, {py:.2f}, {pz:.2f})")
                print(f"           createCellPlayer payload: {len(payload)} bytes")

        if trunc:
            # Show what remains
            remaining = len(body) - off
            print(f"           ** PARSE STOPPED: {remaining} bytes remain in body from offset {off}")
            break

    # Full hex dump: first 48 bytes
    print(f"\n  === Body bytes 0-47 (bundle header context) ===")
    print(hex_ascii_dump(body[:48], context_start=0))

    # Corruption zone
    print(f"\n  === Body bytes 140-210 (around corruption at 160) ===")
    if len(body) >= 140:
        start = 140
        end_pos = min(len(body), 216)
        print(hex_ascii_dump(body[start:end_pos], context_start=start))
    else:
        print(f"    Body only {len(body)} bytes")
        print(hex_ascii_dump(body, context_start=0))

    # Byte-by-byte annotation at the critical zone
    print(f"\n  === CRITICAL ZONE: bytes 150-175 (annotated) ===")
    if len(body) >= 150:
        for i in range(150, min(176, len(body))):
            b = body[i]
            char = chr(b) if 32 <= b < 127 else '.'
            annotations = []
            if b == 0x88:
                annotations.append("<-- 0x88 = msg_id 136 (client error refers to this)")
            if b == 0x06:
                annotations.append("<-- 0x06")
            if i == 160:
                annotations.append("<-- OFFSET 160 (client error: 'at 160')")
            ann = "  " + "; ".join(annotations) if annotations else ""
            print(f"    [{i:3d}]: 0x{b:02x} ({b:3d}) '{char}'{ann}")

    # Try to interpret what's at offset 160 as a message header
    print(f"\n  === What the client sees at offset 160 ===")
    if len(body) > 165:
        print(f"    The client says: 'Not enough data on stream at 160 for request ID and NRO (5 left, needed 6)'")
        print(f"    This means the message parser is at body offset 160 with only 5 bytes left in the bundle")
        print(f"    Total body length: {len(body)}")
        print(f"    body[160:] = {body[160:].hex()}")
        print(f"    body[155:166] = {body[155:min(166, len(body))].hex()}")
    elif len(body) > 160:
        remaining = len(body) - 160
        print(f"    body[160:] = {body[160:].hex()} ({remaining} bytes)")
    else:
        print(f"    Body is only {len(body)} bytes (ends before offset 160)")

    # Scan for 0x88 in the body
    print(f"\n  === All occurrences of 0x88 in body ===")
    for i in range(len(body)):
        if body[i] == 0x88:
            ctx_start = max(0, i - 4)
            ctx_end = min(len(body), i + 8)
            ctx_hex = ' '.join(f'{body[j]:02x}' for j in range(ctx_start, ctx_end))
            print(f"    offset {i}: ...{ctx_hex}...")
            # If this could be a msg_id, what would the word_len be?
            if i + 2 < len(body):
                wl = struct.unpack('<H', body[i+1:i+3])[0]
                remaining = len(body) - (i + 3)
                print(f"      if entity_method: word_len={wl}, remaining_after={remaining}")

    # First 200 bytes full dump
    print(f"\n  === Full body dump: first 200 bytes ===")
    print(hex_ascii_dump(body[:200], context_start=0))

    # Last 20 bytes
    if len(body) > 20:
        print(f"\n  === Last 20 bytes of body (offsets {len(body)-20}-{len(body)-1}) ===")
        print(hex_ascii_dump(body[-20:], context_start=len(body)-20))


def main():
    pcap_path = sys.argv[1] if len(sys.argv) > 1 else \
        "/mnt/c/Users/Steve/source/projects/SGW/Stargate Worlds-QA/Working/binaries/sessions/2026-03-15_20-30.pcap"
    key_hex = sys.argv[2] if len(sys.argv) > 2 else \
        "D01CBDC57690E788AA13AAB8B67CEE6FF01C69BF3EA52761D8F845D3B34CD7AC"

    key_bytes = bytes.fromhex(key_hex)
    SERVER_PORT = 32832
    first_ts = None

    # frag_groups: (frag_begin, frag_end) -> [(seq_id, body, rel_ts, pkt_num)]
    frag_groups = defaultdict(list)
    standalone_bundles = []  # (seq_id, body, rel_ts, pkt_num, first_req_offset)
    pkt_num = 0

    print(f"Pcap: {pcap_path}")
    print(f"Key:  {key_hex[:16]}...{key_hex[-8:]}")
    print("=" * 90)
    print()
    print("=== ALL SERVER->CLIENT PACKETS ===")
    print()

    for ts, src_port, dst_port, payload in read_pcap(pcap_path):
        if first_ts is None:
            first_ts = ts
        pkt_num += 1
        rel_ts = ts - first_ts

        if src_port != SERVER_PORT:
            continue

        plaintext = decrypt_packet(key_bytes, payload)
        if plaintext is None:
            continue

        parsed = parse_mercury(plaintext)
        if parsed is None:
            continue

        flags = parsed['flags']
        seq_id = parsed['seq_id']
        body = parsed['body']
        fb = parsed['frag_begin']
        fe = parsed['frag_end']
        req_off = parsed['first_req_offset']

        flag_parts = []
        if flags & FLAG_HAS_REQUESTS: flag_parts.append("REQ")
        if flags & FLAG_HAS_ACKS: flag_parts.append("ACK")
        if flags & FLAG_ON_CHANNEL: flag_parts.append("CHAN")
        if flags & FLAG_RELIABLE: flag_parts.append("REL")
        if flags & FLAG_FRAGMENTED: flag_parts.append("FRAG")
        if flags & FLAG_HAS_SEQUENCE: flag_parts.append("SEQ")
        flag_str = "|".join(flag_parts)

        if flags & FLAG_FRAGMENTED:
            frag_groups[(fb, fe)].append((seq_id, body, rel_ts, pkt_num))
            print(f"  pkt#{pkt_num:3d} [{rel_ts:8.3f}s] S->C seq={seq_id} flags=0x{flags:02x}({flag_str}) "
                  f"FRAG range=[{fb}-{fe}] body={len(body)}B")
        else:
            standalone_bundles.append((seq_id, body, rel_ts, pkt_num, req_off))
            # Quick peek at first msg_id
            first_msg = f"first_msg=0x{body[0]:02x}" if body else "empty"
            print(f"  pkt#{pkt_num:3d} [{rel_ts:8.3f}s] S->C seq={seq_id} flags=0x{flags:02x}({flag_str}) "
                  f"body={len(body)}B {first_msg}")

    # ── REASSEMBLY ──
    print()
    print("=" * 90)
    print("FRAGMENT REASSEMBLY")
    print("=" * 90)

    reassembled = {}  # label -> full_body

    for (fb, fe), frags in sorted(frag_groups.items()):
        frags_sorted = sorted(frags, key=lambda x: x[0])  # sort by seq_id
        expected_count = fe - fb + 1
        print(f"\n  Fragment group [{fb}-{fe}]: expecting {expected_count} packets, got {len(frags_sorted)}")
        total_body = bytearray()
        for seq_id, body, ts, pn in frags_sorted:
            frag_index = seq_id - fb
            print(f"    seq={seq_id} (index {frag_index}): pkt#{pn} [{ts:.3f}s] body={len(body)}B")
            total_body.extend(body)

        if len(frags_sorted) < expected_count:
            missing = set(range(fb, fe + 1)) - set(f[0] for f in frags_sorted)
            print(f"    ** MISSING fragments: seq_ids {missing}")

        label = f"frag_group_{fb}_{fe}"
        reassembled[label] = bytes(total_body)
        print(f"    => reassembled total: {len(total_body)} bytes")

    # ── SEARCH FOR createCellPlayer ──
    print()
    print("=" * 90)
    print("SEARCHING FOR createCellPlayer (msg_id 0x06)")
    print("=" * 90)

    found = False

    for label, body in sorted(reassembled.items()):
        for off, mid, name, payload, trunc in walk_messages(body):
            if mid == 0x06:
                found = True
                print(f"\n  ** FOUND in {label} at body offset {off}, total body={len(body)} bytes")
                analyze_bundle(body, label)
                break

    for seq_id, body, ts, pn, req_off in standalone_bundles:
        for off, mid, name, payload, trunc in walk_messages(body):
            if mid == 0x06:
                found = True
                label = f"standalone_seq{seq_id}_pkt{pn}"
                print(f"\n  ** FOUND in standalone pkt#{pn} seq={seq_id} at body offset {off}, total body={len(body)} bytes")
                analyze_bundle(body, label)
                break

    if not found:
        print("\n  createCellPlayer (0x06) NOT FOUND in any bundle!")
        print("  Dumping all reassembled bundle message walks for inspection:")
        for label, body in sorted(reassembled.items()):
            print(f"\n  --- {label} ({len(body)} bytes) ---")
            for off, mid, name, payload, trunc in walk_messages(body):
                trunc_marker = " [TRUNCATED]" if trunc else ""
                print(f"    @{off:5d}: [{mid:#04x}] {name} ({len(payload)}B){trunc_marker}")
                if trunc:
                    break
            # Show first 64 bytes
            print(hex_ascii_dump(body[:64], context_start=0))

        print("\n  Non-tick standalone bundles:")
        for seq_id, body, ts, pn, req_off in standalone_bundles:
            if len(body) > 0 and body[0] == 0x0D and len(body) <= 9:
                continue
            print(f"\n  --- pkt#{pn} seq={seq_id} [{ts:.3f}s] {len(body)}B req_off={req_off} ---")
            for off, mid, name, payload, trunc in walk_messages(body):
                trunc_marker = " [TRUNCATED]" if trunc else ""
                print(f"    @{off:5d}: [{mid:#04x}] {name} ({len(payload)}B){trunc_marker}")
                if trunc:
                    break
            if len(body) <= 300:
                print(hex_ascii_dump(body[:200], context_start=0))


if __name__ == '__main__':
    main()
