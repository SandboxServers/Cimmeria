#!/usr/bin/env python3
"""Debug Mercury server->client traffic from an encrypted PCAP.

Two modes, both reading server->client datagrams (source port 32832),
decrypting them with the session's AES-256 key and parsing the packet footer:

  fragments (default)
      Group fragmented packets by frag_begin, hex-dump each fragment body,
      reassemble the bundle and walk its messages (decoding VIEWPORT_INFO,
      CREATE_CELL_PLAYER and FORCED_POSITION). Also dumps a byte window of the
      reassembled body and lists the non-tick-sync packets sent between a
      given sequence number and the first fragment.

  dump
      One line per packet: sequence, flags, body size, fragment range, acks
      and a brief message list, with the first bytes in hex. Optionally limited
      to a sequence range.

Usage:
  python tools/frag_debug.py <capture.pcap> <key.hex>
          [--window START:END] [--gap-from SEQ]
  python tools/frag_debug.py <capture.pcap> <key.hex> --dump
          [--seq-min N] [--seq-max N]

<key.hex> is a text file holding the 64-hex-digit session key. Decryption
shells out to `openssl`, which must be on PATH.
"""

import argparse
import struct
import subprocess

FLAG_HAS_REQUESTS  = 0x01
FLAG_HAS_ACKS      = 0x04
FLAG_ON_CHANNEL    = 0x08
FLAG_RELIABLE      = 0x10
FLAG_FRAGMENTED    = 0x20
FLAG_HAS_SEQUENCE  = 0x40

SERVER_PORT = 32832

# Server->client message table, per docs/protocol/message-dispatch-table.md.
SERVER_MSG_FORMAT = {
    0x00: ('dword', None, "AUTHENTICATE"),
    0x01: ('constant', 4, "BANDWIDTH"),
    0x02: ('constant', 1, "UPD_FREQ"),
    0x03: ('constant', 4, "SET_GAME_TIME"),
    0x04: ('constant', 1, "RESET_ENTITIES"),
    0x05: ('word', None, "CREATE_BASE_PLAYER"),
    0x06: ('word', None, "CREATE_CELL_PLAYER"),
    0x07: ('word', None, "SPACE_DATA"),
    0x08: ('constant', 13, "VIEWPORT_INFO"),
    0x09: ('word', None, "CREATE_ENTITY"),
    0x0A: ('word', None, "UPDATE_ENTITY"),
    0x0B: ('constant', 5, "ENTITY_INVISIBLE"),
    0x0C: ('word', None, "LEAVE_AOI"),
    0x0D: ('constant', 8, "TICK_SYNC"),
    0x0E: ('constant', 1, "SET_SPACE_VIEWPORT"),
    0x0F: ('constant', 4, "SET_VEHICLE"),
    0x30: ('constant', 41, "DETAILED_POSITION"),
    0x31: ('constant', 49, "FORCED_POSITION"),
    0x32: ('constant', 5, "CONTROL_ENTITY"),
    0x33: ('word', None, "VOICE_DATA"),
    0x34: ('word', None, "RESTORE_CLIENT"),
    0x35: ('word', None, "RESTORE_BASEAPP"),
    0x36: ('word', None, "RESOURCE_FRAGMENT"),
    0x37: ('constant', 1, "LOGGED_OFF"),
    0x38: ('word', None, "CLIENT_MESSAGE"),
}
# UPDATE_AVATAR variants 0x10-0x2F
AVATAR_SIZES = [25,24,23,22, 25,24,23,22, 25,24,23,22, 13,12,11,10,
                22,21,20,19, 22,21,20,19, 22,21,20,19, 10,9,8,7]
for i, sz in enumerate(AVATAR_SIZES):
    SERVER_MSG_FORMAT[0x10 + i] = ('constant', sz, f"UPDATE_AVATAR_{0x10+i:#04x}")


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


def server_packets(pcap_path, key_hex):
    """Yield (pkt_num, rel_ts, flags, seq, fb, fe, body, acks) for each
    decryptable server->client packet. pkt_num counts every UDP packet."""
    pkt_num = 0
    first_ts = None
    for ts, src_port, dst_port, payload in read_pcap(pcap_path):
        if first_ts is None: first_ts = ts
        pkt_num += 1
        if src_port != SERVER_PORT: continue

        if len(payload) <= 16: continue
        ciphertext = payload[:-16]
        plaintext = decrypt_aes256_cbc(key_hex, ciphertext)
        if plaintext is None: continue
        plaintext = strip_pkcs7(plaintext)

        parsed = parse_footer(plaintext)
        if parsed is None: continue
        flags, seq, fb, fe, req_off, body, acks = parsed
        yield (pkt_num, ts - first_ts, flags, seq, fb, fe, body, acks)


def parse_messages(body):
    """Walk server messages, return list of (offset, msg_id, name, payload, note).

    note is None for a complete message, or "TRUNC" when the length prefix
    runs past the end of the body."""
    results = []
    offset = 0
    while offset < len(body):
        start = offset
        msg_id = body[offset]; offset += 1

        if msg_id == 0xFF:
            # Mercury reply message, not an entity method.
            fmt, length, name = 'word', None, "REPLY"
        elif msg_id >= 0x80:
            fmt, length, name = 'word', None, f"entity_{msg_id:#04x}"
        elif msg_id in SERVER_MSG_FORMAT:
            fmt, length, name = SERVER_MSG_FORMAT[msg_id]
        else:
            # Unknown -- try word length
            fmt, length, name = 'word', None, f"msg_{msg_id:#04x}"

        if fmt == 'constant':
            note = "TRUNC" if offset + length > len(body) else None
            results.append((start, msg_id, name, body[offset:offset+length], note))
            offset += length
            continue

        prefix = 4 if fmt == 'dword' else 2
        if offset + prefix > len(body):
            results.append((start, msg_id, name, body[offset:], "TRUNC"))
            break
        ln = struct.unpack('<I' if prefix == 4 else '<H', body[offset:offset+prefix])[0]
        offset += prefix
        results.append((start, msg_id, name, body[offset:offset+ln], None))
        offset += ln

    return results


def format_flags(f):
    parts = []
    if f & 0x01: parts.append("REQ")
    if f & 0x02: parts.append("PIG")
    if f & 0x04: parts.append("ACK")
    if f & 0x08: parts.append("CHAN")
    if f & 0x10: parts.append("REL")
    if f & 0x20: parts.append("FRAG")
    if f & 0x40: parts.append("SEQ")
    if f & 0x80: parts.append("IDX")
    return "|".join(parts)


def hexdump_rows(data, start, end):
    for row_off in range(start, end, 16):
        chunk = data[row_off:min(row_off + 16, end)]
        hex_str = ' '.join(f'{b:02x}' for b in chunk)
        ascii_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in chunk)
        print(f"    {row_off:04x}: {hex_str:<48s} {ascii_str}")


def is_tick_sync(body):
    return len(body) == 9 and body[0] == 0x0D


def run_fragments(pcap_path, key_hex, window, gap_from):
    fragments = {}  # frag_begin -> list of (seq, fb, fe, body)
    all_packets = []

    for pkt_num, ts, flags, seq, fb, fe, body, acks in server_packets(pcap_path, key_hex):
        all_packets.append({'pkt': pkt_num, 'flags': flags, 'seq': seq, 'body': body})
        if flags & FLAG_FRAGMENTED and fb is not None:
            fragments.setdefault(fb, []).append((seq, fb, fe, body))

    print(f"Found {len(fragments)} fragment group(s):")
    for fb_key in sorted(fragments.keys()):
        frags = sorted(fragments[fb_key], key=lambda x: x[0])
        total_body = b''.join(f[3] for f in frags)
        print(f"\n{'='*80}")
        print(f"Fragment group: frag_begin={fb_key}, frag_end={frags[0][2]}, num_frags={len(frags)}")
        print(f"Total reassembled body: {len(total_body)} bytes")
        print(f"Sequence numbers: {[f[0] for f in frags]}")

        # Each fragment's body hex (first 128 bytes)
        for i, (seq, fb, fe, body) in enumerate(frags):
            print(f"\n  Fragment {i+1} (seq={seq}): {len(body)} bytes")
            hexdump_rows(body, 0, min(len(body), 128))

        print(f"\n  Reassembled body parse:")
        for off, msg_id, name, payload, note in parse_messages(total_body):
            print(f"    @{off:4d} [{msg_id:#04x}] {name} ({len(payload)}B){' TRUNC' if note else ''}")
            if msg_id == 0x08:  # VIEWPORT_INFO
                if len(payload) >= 13:
                    eid1, eid2, sid = struct.unpack('<III', payload[0:12])
                    vid = payload[12]
                    print(f"           eid1={eid1} eid2={eid2} space={sid} viewport={vid}")
            elif msg_id in (0x06, 0x31):  # CREATE_CELL_PLAYER, FORCED_POSITION
                if len(payload) >= 28:
                    eid, sid, vid = struct.unpack('<III', payload[0:12])
                    px, py, pz = struct.unpack('<fff', payload[12:24])
                    print(f"           eid={eid} space={sid} vehicle={vid} pos=({px:.2f},{py:.2f},{pz:.2f})")

        # A byte window of the reassembled body, e.g. where a corrupted
        # message boundary is suspected.
        w_start, w_end = window
        print(f"\n  Hex at boundary (offset {w_start}-{w_end}):")
        hexdump_rows(total_body, max(0, w_start), min(len(total_body), w_end))

    if fragments:
        first_frag_seq = min(fragments.keys())
        print(f"\n{'='*80}")
        print(f"Non-tick-sync packets between seq={gap_from} and first fragment seq={first_frag_seq}:")
        for p in all_packets:
            if p['seq'] is None: continue
            if gap_from < p['seq'] < first_frag_seq and not (p['flags'] & FLAG_FRAGMENTED):
                if is_tick_sync(p['body']):
                    continue
                print(f"  Pkt #{p['pkt']} seq={p['seq']} flags={p['flags']:#04x} body={len(p['body'])}B")
                print(f"    hex: {' '.join(f'{b:02x}' for b in p['body'][:64])}")


def run_dump(pcap_path, key_hex, seq_min, seq_max):
    for pkt_num, ts, flags, seq, fb, fe, body, acks in server_packets(pcap_path, key_hex):
        if seq_min is not None and seq is not None and seq < seq_min: continue
        if seq_max is not None and seq is not None and seq > seq_max: continue

        frag_str = f" FRAG[{fb}-{fe}]" if fb is not None else ""
        ack_str = f" acks={acks}" if acks else ""
        seq_str = f"{seq:4d}" if seq is not None else "   -"
        head = f"#{pkt_num:4d} seq={seq_str} flags={flags:#04x}({format_flags(flags):16s}) body={len(body):5d}B"

        is_tick = is_tick_sync(body) if body else False
        is_frag = bool(flags & FLAG_FRAGMENTED)

        if is_tick and not is_frag:
            print(f"{head} TICK_SYNC{ack_str}")
        elif is_frag:
            print(f"{head}{frag_str}{ack_str}")
            print(f"     hex: {' '.join(f'{b:02x}' for b in body[:32])}")
        else:
            msgs = [f"{name}({note or f'{len(payload)}B'})"
                    for _, _, name, payload, note in parse_messages(body)]
            msg_summary = " + ".join(msgs[:5])
            if len(msgs) > 5:
                msg_summary += f" + {len(msgs)-5} more"
            print(f"{head} [{msg_summary}]{ack_str}")
            if len(body) < 128:
                print(f"     hex: {' '.join(f'{b:02x}' for b in body[:64])}")


def parse_window(text):
    start, sep, end = text.partition(':')
    try:
        window = int(start), int(end)
    except ValueError:
        window = None
    if not sep or window is None or window[1] <= window[0]:
        raise argparse.ArgumentTypeError(
            f"expected START:END with END > START, got {text!r}")
    return window


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('pcap', help='capture file (classic pcap, Ethernet link type)')
    ap.add_argument('keys', help='text file holding the hex AES-256 session key')
    ap.add_argument('--dump', action='store_true',
                    help='list every server->client packet instead of reassembling fragments')
    ap.add_argument('--seq-min', type=int, help='dump: skip packets below this sequence')
    ap.add_argument('--seq-max', type=int, help='dump: skip packets above this sequence')
    ap.add_argument('--window', type=parse_window, metavar='START:END',
                    help='fragments: reassembled-body byte window to hex-dump (default 90:120)')
    ap.add_argument('--gap-from', type=int, metavar='SEQ',
                    help='fragments: list non-tick-sync packets after this sequence '
                         'and before the first fragment (default 76)')
    args = ap.parse_args()

    if args.dump and (args.window is not None or args.gap_from is not None):
        ap.error('--window and --gap-from apply to fragments mode, not --dump')
    if not args.dump and (args.seq_min is not None or args.seq_max is not None):
        ap.error('--seq-min and --seq-max need --dump')
    if args.window is None:
        args.window = (90, 120)
    if args.gap_from is None:
        args.gap_from = 76

    with open(args.keys) as f:
        key_hex = f.read().strip()

    if args.dump:
        run_dump(args.pcap, key_hex, args.seq_min, args.seq_max)
    else:
        run_fragments(args.pcap, key_hex, args.window, args.gap_from)


if __name__ == "__main__":
    main()
