#!/usr/bin/env python3
"""Debug fragment reassembly — dump hex of each fragment body and parse reassembled bundle."""

import struct
import subprocess
import sys

FLAG_HAS_REQUESTS  = 0x01
FLAG_HAS_ACKS      = 0x04
FLAG_ON_CHANNEL    = 0x08
FLAG_RELIABLE      = 0x10
FLAG_FRAGMENTED    = 0x20
FLAG_HAS_SEQUENCE  = 0x40

SERVER_MSG_NAMES = {
    0x00: "LOGIN_REPLY", 0x01: "AUTH_REPLY", 0x02: "REPLY_MSG",
    0x03: "SET_GAME_TIME", 0x04: "RESET_ENTITIES", 0x05: "CREATE_BASE_PLAYER",
    0x06: "CREATE_CELL_PLAYER", 0x07: "SPACE_DATA", 0x08: "VIEWPORT_INFO",
    0x09: "CREATE_ENTITY", 0x0A: "LOGGED_OFF", 0x0B: "CREATE_CELL_ENTITY",
    0x0C: "RESTORE_CLIENT", 0x0D: "TICK_SYNC", 0x0E: "SET_SPACE_VIEWPORT",
    0x31: "FORCED_POSITION", 0x36: "RESOURCE_FRAGMENT", 0xFF: "CONNECT_REPLY",
}

SERVER_MSG_FORMAT = {
    0x03: ('constant', 4), 0x04: ('constant', 1), 0x05: ('word', None),
    0x06: ('word', None), 0x07: ('word', None), 0x08: ('constant', 13),
    0x09: ('word', None), 0x0A: ('constant', 1), 0x0B: ('word', None),
    0x0C: ('word', None), 0x0D: ('constant', 8), 0x0E: ('constant', 1),
    0x31: ('constant', 49), 0x36: ('word', None), 0xFF: ('word', None),
}


def decrypt_aes256_cbc(key_hex, ciphertext):
    proc = subprocess.run(
        ["openssl", "enc", "-aes-256-cbc", "-d", "-nopad",
         "-K", key_hex, "-iv", "00" * 16],
        input=ciphertext, capture_output=True,
    )
    return proc.stdout if proc.returncode == 0 else None


def strip_pkcs7(data):
    if not data: return data
    pad_len = data[-1]
    if 1 <= pad_len <= 16 and all(b == pad_len for b in data[-pad_len:]):
        return data[:-pad_len]
    return data


def read_pcap(path):
    with open(path, 'rb') as f:
        magic, ver_maj, ver_min, _, _, snaplen, linktype = struct.unpack('<IHHiiII', f.read(24))
        while True:
            hdr = f.read(16)
            if len(hdr) < 16: break
            ts_sec, ts_usec, incl_len, orig_len = struct.unpack('<IIII', hdr)
            pkt_data = f.read(incl_len)
            if len(pkt_data) < incl_len: break
            if linktype == 1:
                if len(pkt_data) < 14: continue
                if struct.unpack('>H', pkt_data[12:14])[0] != 0x0800: continue
                ip_start = 14
            else: continue
            if len(pkt_data) < ip_start + 20: continue
            ihl = (pkt_data[ip_start] & 0x0F) * 4
            if pkt_data[ip_start + 9] != 17: continue
            udp_start = ip_start + ihl
            if len(pkt_data) < udp_start + 8: continue
            src_port, dst_port = struct.unpack('>HH', pkt_data[udp_start:udp_start + 4])
            payload = pkt_data[udp_start + 8:]
            yield (ts_sec + ts_usec / 1e6, src_port, dst_port, payload)


def parse_footer(data):
    if len(data) < 1: return None
    flags = data[0]
    end = len(data)

    def pop_u8():
        nonlocal end
        if end < 1: return None
        end -= 1; return data[end]

    def pop_u32():
        nonlocal end
        if end < 4: return None
        end -= 4; return struct.unpack('<I', data[end:end+4])[0]

    def pop_u16():
        nonlocal end
        if end < 2: return None
        end -= 2; return struct.unpack('<H', data[end:end+2])[0]

    acks = []
    if flags & FLAG_HAS_ACKS:
        ac = pop_u8()
        if ac is None: return None
        for _ in range(ac):
            a = pop_u32()
            if a is None: return None
            acks.append(a)
        acks.reverse()

    seq = pop_u32() if flags & FLAG_HAS_SEQUENCE else None
    if flags & FLAG_HAS_SEQUENCE and seq is None: return None

    fb = fe = None
    if flags & FLAG_FRAGMENTED:
        fe = pop_u32()
        fb = pop_u32()
        if fb is None or fe is None: return None

    req_off = None
    if flags & FLAG_HAS_REQUESTS:
        req_off = pop_u16()
        if req_off is None: return None

    return (flags, seq, fb, fe, req_off, data[1:end], acks)


def parse_messages(body):
    """Walk server messages, return list of (offset, msg_id, name, payload_bytes)."""
    results = []
    offset = 0
    while offset < len(body):
        start = offset
        msg_id = body[offset]; offset += 1

        if msg_id >= 0x80:
            name = f"entity_method_{msg_id:#04x}"
            if offset + 2 > len(body):
                results.append((start, msg_id, name, body[offset:]))
                break
            wl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            results.append((start, msg_id, name, body[offset:offset+wl]))
            offset += wl
            continue

        name = SERVER_MSG_NAMES.get(msg_id, f"msg_{msg_id:#04x}")

        if msg_id in SERVER_MSG_FORMAT:
            fmt, length = SERVER_MSG_FORMAT[msg_id]
            if fmt == 'constant':
                results.append((start, msg_id, name, body[offset:offset+length]))
                offset += length
            else:
                if offset + 2 > len(body):
                    results.append((start, msg_id, name, body[offset:]))
                    break
                wl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
                results.append((start, msg_id, name, body[offset:offset+wl]))
                offset += wl
        else:
            # Unknown — try word length
            if offset + 2 > len(body):
                results.append((start, msg_id, name, body[offset:]))
                break
            wl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            results.append((start, msg_id, name, body[offset:offset+wl]))
            offset += wl

    return results


def main():
    pcap_path = sys.argv[1]
    keys_path = sys.argv[2]

    with open(keys_path) as f:
        key_hex = f.read().strip()

    SERVER_PORT = 32832
    pkt_num = 0
    first_ts = None

    # Collect all server->client packets with their parsed data
    fragments = {}  # frag_begin -> list of (seq, fb, fe, body)
    all_packets = []

    for ts, src_port, dst_port, payload in read_pcap(pcap_path):
        if first_ts is None: first_ts = ts
        pkt_num += 1
        is_server = (src_port == SERVER_PORT)
        if not is_server: continue  # Only care about S->C

        if len(payload) <= 16: continue
        ciphertext = payload[:-16]
        plaintext = decrypt_aes256_cbc(key_hex, ciphertext)
        if plaintext is None: continue
        plaintext = strip_pkcs7(plaintext)

        parsed = parse_footer(plaintext)
        if parsed is None: continue

        flags, seq, fb, fe, req_off, body, acks = parsed

        all_packets.append({
            'pkt': pkt_num, 'ts': ts - first_ts, 'flags': flags,
            'seq': seq, 'fb': fb, 'fe': fe, 'body': body, 'acks': acks,
        })

        if flags & FLAG_FRAGMENTED and fb is not None:
            if fb not in fragments:
                fragments[fb] = []
            fragments[fb].append((seq, fb, fe, body))

    # Print all fragment groups
    print(f"Found {len(fragments)} fragment group(s):")
    for fb_key in sorted(fragments.keys()):
        frags = sorted(fragments[fb_key], key=lambda x: x[0])
        total_body = b''.join(f[3] for f in frags)
        print(f"\n{'='*80}")
        print(f"Fragment group: frag_begin={fb_key}, frag_end={frags[0][2]}, num_frags={len(frags)}")
        print(f"Total reassembled body: {len(total_body)} bytes")
        print(f"Sequence numbers: {[f[0] for f in frags]}")

        # Dump each fragment's body hex (first 128 bytes)
        for i, (seq, fb, fe, body) in enumerate(frags):
            print(f"\n  Fragment {i+1} (seq={seq}): {len(body)} bytes")
            # Show first 128 bytes in rows of 16
            for row_off in range(0, min(len(body), 128), 16):
                chunk = body[row_off:row_off+16]
                hex_str = ' '.join(f'{b:02x}' for b in chunk)
                ascii_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in chunk)
                print(f"    {row_off:04x}: {hex_str:<48s} {ascii_str}")

        # Now parse the reassembled body as Mercury messages
        print(f"\n  Reassembled body parse:")
        msgs = parse_messages(total_body)
        for off, msg_id, name, payload in msgs:
            print(f"    @{off:4d} [{msg_id:#04x}] {name} ({len(payload)}B)")
            if msg_id == 0x08:  # VIEWPORT
                if len(payload) >= 13:
                    eid1 = struct.unpack('<I', payload[0:4])[0]
                    eid2 = struct.unpack('<I', payload[4:8])[0]
                    sid = struct.unpack('<I', payload[8:12])[0]
                    vid = payload[12]
                    print(f"           eid1={eid1} eid2={eid2} space={sid} viewport={vid}")
            elif msg_id == 0x06:  # CREATE_CELL
                if len(payload) >= 28:
                    eid = struct.unpack('<I', payload[0:4])[0]
                    sid = struct.unpack('<I', payload[4:8])[0]
                    vid = struct.unpack('<I', payload[8:12])[0]
                    px,py,pz = struct.unpack('<fff', payload[12:24])
                    print(f"           eid={eid} space={sid} vehicle={vid} pos=({px:.2f},{py:.2f},{pz:.2f})")
            elif msg_id == 0x31:  # FORCED_POS
                if len(payload) >= 28:
                    eid = struct.unpack('<I', payload[0:4])[0]
                    sid = struct.unpack('<I', payload[4:8])[0]
                    vid = struct.unpack('<I', payload[8:12])[0]
                    px,py,pz = struct.unpack('<fff', payload[12:24])
                    print(f"           eid={eid} space={sid} vehicle={vid} pos=({px:.2f},{py:.2f},{pz:.2f})")

        # Show bytes around offset 96 where corruption might happen
        # The original body was 99 bytes (VIEWPORT+CELL+FORCED), then mapLoaded follows
        print(f"\n  Hex at boundary (around offset 96-110):")
        start_dump = max(0, 90)
        end_dump = min(len(total_body), 120)
        for row_off in range(start_dump, end_dump, 16):
            chunk = total_body[row_off:row_off+16]
            hex_str = ' '.join(f'{b:02x}' for b in chunk)
            ascii_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in chunk)
            print(f"    {row_off:04x}: {hex_str:<48s} {ascii_str}")

    # Also show all non-tick-sync, non-fragment packets between seq 76 and first frag
    if fragments:
        first_frag_seq = min(fragments.keys())
        print(f"\n{'='*80}")
        print(f"Non-tick-sync packets between seq=76 and first fragment seq={first_frag_seq}:")
        for p in all_packets:
            if p['seq'] is None: continue
            if 76 < p['seq'] < first_frag_seq and not (p['flags'] & FLAG_FRAGMENTED):
                # Check if it's a pure tick-sync
                if len(p['body']) == 9 and p['body'][0] == 0x0D:
                    continue  # Skip tick-sync
                print(f"  Pkt #{p['pkt']} seq={p['seq']} flags={p['flags']:#04x} body={len(p['body'])}B")
                print(f"    hex: {' '.join(f'{b:02x}' for b in p['body'][:64])}")


if __name__ == "__main__":
    main()
