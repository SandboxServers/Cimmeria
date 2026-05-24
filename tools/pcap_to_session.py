#!/usr/bin/env python3
"""
Convert a Mercury .pcap + AES keys.txt to a JSONL session trace consumable
by ``cimmeria-wireclient``'s ``session_trace::Trace`` loader.

Usage:
    python3 tools/pcap_to_session.py <pcap_file> <keys_file> [--out PATH]
                                     [--label LABEL] [--server-port PORT]

Output schema (one JSON object per line):

    line 1:  {"header": {label, source_pcap, session_key_hex,
                         client_addr, server_addr, packet_count, schema_version}}
    line N:  {"event": {t_seconds, packet_no, direction ("c2s"|"s2c"),
                        seq, flags, acks, messages: [
                            {msg_id, name?, body_hex}, ...]}}

This sits on top of the existing decoder in tools/pcap_dissect.py — we
reuse its packet framing, AES decrypt, and message-name table. The only new
work here is JSONL serialisation. Keeping the producer in Python (rather
than rewriting libpcap reading in Rust) buys us:

  * One source of truth for the wire decoder.
  * Hermetic Rust tests (replay path reads JSONL, never opens a .pcap).
  * Hand-editable fixtures (JSONL diffs cleanly under git).

The decoder is intentionally lossy in places where the server's behavior
is non-deterministic (server-assigned entity IDs, tick timestamps); the
wireclient comparison policy decides per-msg-id whether a byte drift is
load-bearing or expected drift. See
``crates/wireclient/src/session_trace.rs``.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

# Re-use the existing dissector primitives.
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from pcap_dissect import (  # noqa: E402  (import after sys.path tweak)
    SERVER_MSG_NAMES,
    decrypt_aes256_cbc,
    parse_mercury_footer,
    parse_messages,
    read_pcap,
    strip_pkcs7,
)


def message_name(msg_id: int, is_server: bool) -> str | None:
    """Best-effort human name for a msg_id.

    Static msg_ids 0x00..0x38 have stable names defined in
    ``pcap_dissect.SERVER_MSG_NAMES``. Dynamic entity-method slots
    (0x80..0xFE) have per-entity-class names that the dissector doesn't
    statically resolve — leave the name absent so the consumer falls back
    to the raw id.
    """
    if is_server and msg_id in SERVER_MSG_NAMES:
        return SERVER_MSG_NAMES[msg_id]
    return None


def render_event(
    pkt_num: int,
    rel_ts: float,
    is_server: bool,
    seq: int | None,
    flags: int,
    acks: list[int],
    body: bytes,
) -> dict:
    direction = "s2c" if is_server else "c2s"
    messages = []
    if body:
        for msg_id, name, msg_payload in parse_messages(body, is_server):
            # Prefer the dissector's name (which has client/server-aware
            # heuristics for entity-method slots) over our static map.
            resolved = name or message_name(msg_id, is_server)
            messages.append({
                "msg_id": msg_id,
                "name": resolved if resolved and not resolved.startswith("msg_0x") else None,
                "body_hex": msg_payload.hex(),
            })
    return {
        "event": {
            "t_seconds": round(rel_ts, 6),
            "packet_no": pkt_num,
            "direction": direction,
            "seq": seq,
            "flags": flags,
            "acks": list(acks) if acks else [],
            "messages": messages,
        }
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("pcap_file")
    parser.add_argument("keys_file")
    parser.add_argument(
        "--out",
        default=None,
        help="Destination JSONL path. Default: stdout.",
    )
    parser.add_argument(
        "--label",
        default=None,
        help="Human-readable label for the trace header (default: pcap basename).",
    )
    parser.add_argument(
        "--server-port",
        type=int,
        default=32832,
        help="BaseApp UDP server port to filter on (default: 32832).",
    )
    args = parser.parse_args()

    key_hex = Path(args.keys_file).read_text().strip()
    server_port = args.server_port

    label = args.label or Path(args.pcap_file).stem

    out_fh = open(args.out, "w", encoding="utf-8") if args.out else sys.stdout

    # ── First pass: emit packets, capture endpoints + count ─────────────
    events: list[dict] = []
    client_addr_seen: str | None = None
    server_addr_seen: str | None = None
    first_ts: float | None = None
    pkt_num = 0

    for ts, src_port, dst_port, payload in read_pcap(args.pcap_file):
        if first_ts is None:
            first_ts = ts
        is_server = src_port == server_port
        if src_port != server_port and dst_port != server_port:
            continue
        rel_ts = ts - first_ts
        pkt_num += 1

        # Client UDP port floats; capture the first observed port for the
        # header. Server port is fixed via --server-port.
        if not is_server and client_addr_seen is None:
            client_addr_seen = f"127.0.0.1:{src_port}"
        if server_addr_seen is None:
            server_addr_seen = f"127.0.0.1:{server_port}"

        # Unencrypted client path (Phase 3 baseAppLogin).
        if len(payload) > 0 and payload[0] in (0x41, 0x01):
            parsed = parse_mercury_footer(payload)
            if parsed:
                flags, seq, _fb, _fe, _req_off, body, acks = parsed
                if not is_server and flags == 0x41 and len(body) > 10:
                    events.append(
                        render_event(pkt_num, rel_ts, is_server, seq, flags, acks, body)
                    )
                    continue

        # Encrypted path.
        if len(payload) <= 16:
            continue
        ciphertext = payload[:-16]
        plaintext = decrypt_aes256_cbc(key_hex, ciphertext)
        if plaintext is None:
            continue
        plaintext = strip_pkcs7(plaintext)
        parsed = parse_mercury_footer(plaintext)
        if parsed is None:
            continue
        flags, seq, _fb, _fe, _req_off, body, acks = parsed
        events.append(render_event(pkt_num, rel_ts, is_server, seq, flags, acks, body))

    header = {
        "header": {
            "label": label,
            "source_pcap": os.fspath(Path(args.pcap_file).name),
            "session_key_hex": key_hex,
            "client_addr": client_addr_seen or f"0.0.0.0:0",
            "server_addr": server_addr_seen or f"127.0.0.1:{server_port}",
            "packet_count": len(events),
            "schema_version": 1,
        }
    }

    out_fh.write(json.dumps(header) + "\n")
    for ev in events:
        out_fh.write(json.dumps(ev) + "\n")

    if args.out:
        out_fh.close()
        print(
            f"Wrote {len(events)} events to {args.out} ({label})",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
