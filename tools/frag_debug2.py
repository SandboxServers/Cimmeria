#!/usr/bin/env python3
"""Dump ALL server->client packets from a PCAP, showing seq, flags, body size, and first bytes."""

import struct
import subprocess
import sys

FLAG_HAS_REQUESTS  = 0x01
FLAG_HAS_ACKS      = 0x04
FLAG_ON_CHANNEL    = 0x08
FLAG_RELIABLE      = 0x10
FLAG_FRAGMENTED    = 0x20
FLAG_HAS_SEQUENCE  = 0x40

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
    0x10: ('constant', 25, "UPDATE_AVATAR_0x10"),
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
# Fill in UPDATE_AVATAR variants 0x11-0x2F
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


def parse_messages_brief(body):
    """Parse messages and return brief summary."""
    msgs = []
    offset = 0
    while offset < len(body):
        msg_id = body[offset]; offset += 1

        if msg_id >= 0x80:
            # Entity method
            if offset + 2 > len(body):
                msgs.append(f"entity_{msg_id:#04x}(TRUNC)")
                break
            wl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            msgs.append(f"entity_{msg_id:#04x}({wl}B)")
            offset += wl
            continue

        if msg_id in SERVER_MSG_FORMAT:
            fmt, length, name = SERVER_MSG_FORMAT[msg_id]
            if fmt == 'constant':
                msgs.append(f"{name}({length}B)")
                offset += length
            elif fmt == 'word':
                if offset + 2 > len(body):
                    msgs.append(f"{name}(TRUNC)")
                    break
                wl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
                msgs.append(f"{name}({wl}B)")
                offset += wl
            elif fmt == 'dword':
                if offset + 4 > len(body):
                    msgs.append(f"{name}(TRUNC)")
                    break
                dl = struct.unpack('<I', body[offset:offset+4])[0]; offset += 4
                msgs.append(f"{name}({dl}B)")
                offset += dl
        else:
            # Unknown, try word
            if offset + 2 > len(body):
                msgs.append(f"msg_{msg_id:#04x}(TRUNC)")
                break
            wl = struct.unpack('<H', body[offset:offset+2])[0]; offset += 2
            msgs.append(f"msg_{msg_id:#04x}({wl}B)")
            offset += wl

    return msgs


def main():
    pcap_path = sys.argv[1]
    keys_path = sys.argv[2]
    # Optional: focus on seq range
    seq_min = int(sys.argv[3]) if len(sys.argv) > 3 else None
    seq_max = int(sys.argv[4]) if len(sys.argv) > 4 else None

    with open(keys_path) as f:
        key_hex = f.read().strip()

    SERVER_PORT = 32832
    pkt_num = 0
    first_ts = None

    for ts, src_port, dst_port, payload in read_pcap(pcap_path):
        if first_ts is None: first_ts = ts
        pkt_num += 1
        is_server = (src_port == SERVER_PORT)
        if not is_server: continue

        if len(payload) <= 16: continue
        ciphertext = payload[:-16]
        plaintext = decrypt_aes256_cbc(key_hex, ciphertext)
        if plaintext is None: continue
        plaintext = strip_pkcs7(plaintext)

        parsed = parse_footer(plaintext)
        if parsed is None: continue
        flags, seq, fb, fe, req_off, body, acks = parsed

        if seq_min is not None and seq is not None and seq < seq_min: continue
        if seq_max is not None and seq is not None and seq > seq_max: continue

        frag_str = f" FRAG[{fb}-{fe}]" if fb is not None else ""
        ack_str = f" acks={acks}" if acks else ""

        # For non-fragment, non-tick-sync: show messages
        is_tick = (len(body) == 9 and body[0] == 0x0D) if body else False
        is_frag = bool(flags & FLAG_FRAGMENTED)

        if is_tick and not is_frag:
            print(f"#{pkt_num:4d} seq={seq:4d} flags={flags:#04x}({format_flags(flags):16s}) body={len(body):5d}B TICK_SYNC{ack_str}")
        elif is_frag:
            first_bytes = ' '.join(f'{b:02x}' for b in body[:32])
            print(f"#{pkt_num:4d} seq={seq:4d} flags={flags:#04x}({format_flags(flags):16s}) body={len(body):5d}B{frag_str}{ack_str}")
            print(f"     hex: {first_bytes}")
        else:
            msgs = parse_messages_brief(body)
            msg_summary = " + ".join(msgs[:5])
            if len(msgs) > 5:
                msg_summary += f" + {len(msgs)-5} more"
            print(f"#{pkt_num:4d} seq={seq:4d} flags={flags:#04x}({format_flags(flags):16s}) body={len(body):5d}B [{msg_summary}]{ack_str}")
            if len(body) < 128 and not is_tick:
                first_bytes = ' '.join(f'{b:02x}' for b in body[:64])
                print(f"     hex: {first_bytes}")


if __name__ == "__main__":
    main()
