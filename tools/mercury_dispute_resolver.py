#!/usr/bin/env python3
"""
mercury_dispute_resolver.py — answer the three DISPUTED items from
docs/audits/mercury-rust-conformance-2026-05-15.md by analysing a captured
SGW pcap.

The audit V2 left three items as DISPUTED-pending-wire-capture because spec and
Rust have competing claims and only a real client packet can decide:

  Dispute #2  — Sub-slot threshold: Rust uses 61 + sub_index = method_index - 61;
                spec says 62 + sub_index = method_index - 62.

  Dispute #12 — Flag-bit role assignments:
                  spec §1.2 says   bit 5 = HAS_SEQUENCE_NUMBER, bit 6 = HAS_REQUESTS,
                                   bit 7 = IS_FRAGMENT
                  Rust constants    bit 5 = FRAGMENTED,         bit 6 = HAS_SEQUENCE,
                                   bit 7 = INDEXED (unused-in-SGW per spec)

  Dispute on the off-by-one in sub_index — same evidence as #2.

This tool tests both hypotheses by:
  1. Decrypting every UDP packet in the pcap with the captured AES key.
  2. Trying each footer-parse interpretation against the plaintext.
  3. Tallying the histogram of flag bytes seen, the sub_index distribution
     for 0xBD-prefixed entity methods, and the success rate of each parse
     interpretation.

Usage:
    python3 tools/mercury_dispute_resolver.py <pcap_file> <keys_file>

Reuses the read_pcap / decrypt_aes256_cbc / strip_pkcs7 helpers from
tools/pcap_dissect.py (imported, not copied).
"""

from __future__ import annotations

import struct
import sys
from collections import Counter
from pathlib import Path

# Ensure pcap_dissect.py is importable (it lives in the same tools/ dir).
sys.path.insert(0, str(Path(__file__).parent))

import pcap_dissect as pd


# ── Named entity-method indices (from crates/services/src/mercury/mod.rs:135-213) ──
# These are the SGWPlayer flattened ClientMethod indices Rust emits/parses against.
# We use them to cross-reference observed sub_index bytes in the pcap against the
# two candidate encodings (sub_index = method - 61 vs method - 62).

METHOD_IDX = {
    # SGWSpawnableEntity own (0-11) — direct encoding 0x80..0x8B, not 0xBD
    1: "ON_SEQUENCE",
    3: "INTERACTION_TYPE",
    4: "ON_ENTITY_FLAGS",
    7: "ON_ENTITY_PROPERTY",
    8: "ON_VISIBLE",
    9: "ON_KISMET_EVENT_SET_UPDATE",
    10: "ON_ENTITY_TINT",
    11: "ON_BEING_NAME_ID_UPDATE",
    # SGWBeing interface (12-19)
    14: "ON_EFFECT_RESULTS",
    15: "ON_LEVEL_UPDATE",
    16: "ON_TARGET_UPDATE",
    17: "ON_BEING_NAME_UPDATE",
    19: "ON_STATE_FIELD_UPDATE",
    # SGWCombatant interface (20-26)
    20: "ON_STAT_UPDATE",
    21: "ON_STAT_BASE_UPDATE",
    23: "ON_ARCHETYPE_UPDATE",
    24: "ON_ALIGNMENT_UPDATE",
    25: "ON_FACTION_UPDATE",
    26: "BEING_APPEARANCE",
    # Communicator interface (27-33)
    28: "ON_PLAYER_COMMUNICATION",
    31: "ON_CHAT_JOINED",
    32: "ON_CHAT_LEFT",
    # GateTravel interface (65-68) — extended encoding starts here
    65: "SETUP_STARGATE_INFO",
    # SGWInventoryManager interface (69-75)
    69: "ON_BAG_INFO",
    70: "ON_ACTIVE_SLOT_UPDATE",
    71: "ON_REMOVE_ITEM",
    72: "ON_UPDATE_ITEM",
    75: "ON_CASH_CHANGED",
    # SGWMailManager interface (76-79)
    76: "ON_MAIL_HEADER_INFO",
    77: "ON_MAIL_HEADER_REMOVE",
    78: "ON_MAIL_READ",
    79: "SEND_MAIL_RESULT",
    # SGWVendorStore interface (80-81)
    80: "ON_STORE_OPEN",
    81: "ON_STORE_UPDATE",
    # SGWPlayer own methods
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
    # Extended-extended (>= 128)
    130: "ON_EXTRA_NAME_UPDATE",
    131: "ON_EXP_UPDATE",
    132: "ON_MAX_EXP_UPDATE",
    133: "ON_RING_TRANSPORTER_LIST",
    139: "ON_UPDATE_KNOWN_CRAFTS",
    141: "ON_ABILITY_TREE_INFO",
    155: "ON_PLAY_MOVIE",
}


# ── Two competing flag-bit interpretations ────────────────────────────────────

class FlagsRust:
    """Bit assignments per Rust crates/mercury/src/packet/mod.rs:54-78."""
    HAS_REQUESTS  = 0x01
    PIGGYBACK     = 0x02
    HAS_ACKS      = 0x04
    ON_CHANNEL    = 0x08
    RELIABLE      = 0x10
    FRAGMENTED    = 0x20  # bit 5 → fragment
    HAS_SEQUENCE  = 0x40  # bit 6 → seq id
    INDEXED       = 0x80  # bit 7 → indexed (unused in SGW per spec)
    NAME = "Rust"


class FlagsSpec:
    """Bit assignments per spec §1.2 of mercury-wire-format.md."""
    HAS_FIRST_REQUEST_OFFSET = 0x01
    HAS_PIGGYBACKS           = 0x02
    HAS_ACKS                 = 0x04
    ON_CHANNEL               = 0x08
    IS_RELIABLE              = 0x10
    HAS_SEQUENCE_NUMBER      = 0x20  # bit 5 → seq id
    HAS_REQUESTS             = 0x40  # bit 6 → requests
    IS_FRAGMENT              = 0x80  # bit 7 → fragment
    NAME = "Spec"


def parse_footer_with(flags_class, data):
    """Parse the Mercury footer using a specific bit-assignment interpretation.

    Returns (ok, body_len, parsed_dict) where:
      ok = True if every footer field popped cleanly (every advertised field had
           enough bytes) AND the resulting body length is non-negative
      body_len = remaining bytes between flags byte (offset 0) and the popped
                 footers
      parsed_dict = popped fields, for diagnostic printing
    """
    if len(data) < 1:
        return False, 0, {}

    flags = data[0]
    end = len(data)
    parsed = {"flags": flags}

    def pop(n):
        nonlocal end
        if end < n + 1:  # +1 because we cannot eat the flags byte
            return None
        end -= n
        return data[end:end + n]

    # FLAG_HAS_ACKS path is the same in both interpretations (bit 2)
    if flags & flags_class.HAS_ACKS:
        b = pop(1)
        if b is None: return False, 0, parsed
        ack_count = b[0]
        parsed["ack_count"] = ack_count
        acks = []
        for _ in range(ack_count):
            b = pop(4)
            if b is None: return False, 0, parsed
            acks.append(struct.unpack("<I", b)[0])
        parsed["acks"] = list(reversed(acks))

    # Sequence id — the bit differs between interpretations
    if flags_class.NAME == "Spec":
        seq_bit = flags_class.HAS_SEQUENCE_NUMBER
        frag_bit = flags_class.IS_FRAGMENT
        req_bit = flags_class.HAS_FIRST_REQUEST_OFFSET
    else:
        seq_bit = flags_class.HAS_SEQUENCE
        frag_bit = flags_class.FRAGMENTED
        req_bit = flags_class.HAS_REQUESTS

    if flags & seq_bit:
        b = pop(4)
        if b is None: return False, 0, parsed
        parsed["seq_id"] = struct.unpack("<I", b)[0]

    if flags & frag_bit:
        b = pop(4)
        if b is None: return False, 0, parsed
        parsed["frag_end"] = struct.unpack("<I", b)[0]
        b = pop(4)
        if b is None: return False, 0, parsed
        parsed["frag_begin"] = struct.unpack("<I", b)[0]
        # Sanity check: frag_begin should be ≤ frag_end and the difference
        # should be reasonable (≤ 64 per spec §1.6 MaxFragmentsPerBundle)
        diff = parsed["frag_end"] - parsed["frag_begin"]
        if diff < 0 or diff > 64:
            return False, end - 1, parsed  # fail this interpretation

    if flags & req_bit:
        b = pop(2)
        if b is None: return False, 0, parsed
        parsed["first_req_offset"] = struct.unpack("<H", b)[0]
        # Sanity check: first_req_offset should land inside the body
        body_len_so_far = end - 1
        if parsed["first_req_offset"] > body_len_so_far:
            return False, body_len_so_far, parsed

    body_len = end - 1
    return True, body_len, parsed


# ── Sub-slot scan ─────────────────────────────────────────────────────────────

def scan_sub_slot_bytes(body):
    """Walk a Mercury body looking for 0xBD-prefixed entity methods.

    Wire shape per spec §1.5: [0xBD][word_len:u16 LE][entity_id:u32 LE]
                              [sub_index:u8][args...]
    word_len = 4 (entity_id) + 1 (sub_index) + N (args)

    Yields (sub_index, word_len, entity_id, args_len) tuples for every
    0xBD-marked message in the body.
    """
    offset = 0
    while offset < len(body):
        msg_id = body[offset]
        offset += 1

        if msg_id == 0xBD:
            # Extended cell-method sentinel
            if offset + 2 > len(body):
                break
            word_len = struct.unpack("<H", body[offset:offset + 2])[0]
            offset += 2
            if offset + word_len > len(body):
                break
            if word_len < 5:
                # Malformed — sub-slot must have at least entity_id + sub_index
                offset += word_len
                continue
            entity_id = struct.unpack("<I", body[offset:offset + 4])[0]
            sub_index = body[offset + 4]
            args_len = word_len - 5
            yield ("BD", sub_index, word_len, entity_id, args_len)
            offset += word_len
            continue

        if msg_id == 0xFD:
            # Extended base-method sentinel (no entity_id; word_len = 1+args)
            if offset + 2 > len(body):
                break
            word_len = struct.unpack("<H", body[offset:offset + 2])[0]
            offset += 2
            if offset + word_len > len(body):
                break
            if word_len < 1:
                offset += word_len
                continue
            sub_index = body[offset]
            args_len = word_len - 1
            yield ("FD", sub_index, word_len, None, args_len)
            offset += word_len
            continue

        # Other entity methods (0x80..0xBC, 0xC0..0xFC) — skip past
        if 0x80 <= msg_id <= 0xFE and msg_id not in (0xBD, 0xFD):
            if offset + 2 > len(body):
                break
            word_len = struct.unpack("<H", body[offset:offset + 2])[0]
            offset += 2 + word_len
            continue

        # System messages — bail (we only care about 0xBD/0xFD here)
        # The dispute resolver is targeted; full parsing lives in pcap_dissect.
        return


# ── Per-packet diagnostic ─────────────────────────────────────────────────────

def analyse_packet(plaintext):
    """Try both flag interpretations on a plaintext Mercury packet.

    Returns (rust_ok, spec_ok, rust_body, spec_body, sub_indices, flags_byte).
    """
    if len(plaintext) < 1:
        return False, False, None, None, [], None

    flags_byte = plaintext[0]
    rust_ok, rust_body_len, _ = parse_footer_with(FlagsRust, plaintext)
    spec_ok, spec_body_len, _ = parse_footer_with(FlagsSpec, plaintext)

    rust_body = plaintext[1:1 + rust_body_len] if rust_ok else None
    spec_body = plaintext[1:1 + spec_body_len] if spec_ok else None

    # Scan for 0xBD/0xFD only on the body that the AGREED-OK interpretation
    # produced. If both agree, prefer the spec body. If only one parses, use
    # that one. If neither parses, no sub-indices.
    body = spec_body if spec_ok else rust_body
    sub_indices = list(scan_sub_slot_bytes(body)) if body else []

    return rust_ok, spec_ok, rust_body, spec_body, sub_indices, flags_byte


# ── Driver ────────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        sys.exit(2)

    pcap_path = sys.argv[1]
    keys_path = sys.argv[2]

    # Load the AES key from the keys file. Format from the AteraLoader sniffer
    # is one hex string (32 bytes = 64 hex chars), possibly prefixed.
    raw = Path(keys_path).read_text().strip()
    # Take the longest hex token in the file — robust to "key=..." prefixes etc.
    candidates = [t for t in raw.replace("\n", " ").split() if all(c in "0123456789abcdefABCDEF" for c in t)]
    if not candidates:
        print(f"No hex token found in {keys_path}", file=sys.stderr)
        sys.exit(2)
    key_hex = max(candidates, key=len)
    if len(key_hex) != 64:
        print(f"Expected 64-char hex key, got {len(key_hex)} chars: {key_hex[:32]}...", file=sys.stderr)
        sys.exit(2)

    print(f"# Mercury Dispute Resolver")
    print(f"# pcap : {pcap_path}")
    print(f"# keys : {keys_path}")
    print(f"# key  : {key_hex[:16]}...{key_hex[-16:]}")
    print()

    # Tally
    total_packets = 0
    decrypted = 0
    rust_ok_count = 0
    spec_ok_count = 0
    both_ok_count = 0
    neither_ok_count = 0
    flag_byte_histogram = Counter()
    sub_index_distribution = Counter()
    sub_index_examples = []  # (sub_index, args_len, hex_args) for first ~10
    fragment_flag_bytes = []  # flag bytes of fragmented packets, by both interps
    s2c_count = 0  # server-to-client (port 8000-9000 src)
    c2s_count = 0  # client-to-server (port 8000-9000 dst)

    # Walk the pcap
    for ts, src_port, dst_port, payload in pd.read_pcap(pcap_path):
        total_packets += 1
        # Skip cipher size errors
        if len(payload) < 16 or len(payload) % 16 != 0:
            # The HMAC tag adds 16 bytes; strip it and check again
            if len(payload) < 32:
                continue
            ciphertext = payload[:-16]
            if len(ciphertext) % 16 != 0:
                continue
        else:
            ciphertext = payload[:-16] if len(payload) >= 16 else payload

        if len(ciphertext) < 16 or len(ciphertext) % 16 != 0:
            continue

        plaintext = pd.decrypt_aes256_cbc(key_hex, ciphertext)
        if plaintext is None:
            continue

        plaintext = pd.strip_pkcs7(plaintext)
        if not plaintext:
            continue

        decrypted += 1
        rust_ok, spec_ok, rust_body, spec_body, sub_indices, flags_byte = analyse_packet(plaintext)
        flag_byte_histogram[flags_byte] += 1

        if rust_ok and spec_ok: both_ok_count += 1
        elif rust_ok: rust_ok_count += 1
        elif spec_ok: spec_ok_count += 1
        else: neither_ok_count += 1

        # If this packet has bits 5+6 OR bits 5+7 set with HAS_SEQUENCE not
        # explicitly required, it's a fragment under one interpretation or
        # the other. Pure fragments under spec set bit 7 + bit 5; under Rust,
        # bit 5 + bit 6.
        is_fragment_spec = bool(flags_byte & 0x80) and bool(flags_byte & 0x20)
        is_fragment_rust = bool(flags_byte & 0x20) and bool(flags_byte & 0x40)
        if is_fragment_spec or is_fragment_rust:
            fragment_flag_bytes.append(flags_byte)

        # Direction (heuristic: SGW BaseApp uses high port, client uses ephemeral)
        if 8000 <= src_port <= 65535 and dst_port < 8000:
            c2s_count += 1  # likely client→server
        elif 8000 <= dst_port <= 65535 and src_port < 8000:
            s2c_count += 1  # likely server→client
        # We don't actually need the direction split for the disputes; left as info

        for tag, sub_index, word_len, entity_id, args_len in sub_indices:
            sub_index_distribution[sub_index] += 1
            if len(sub_index_examples) < 20:
                args_hex = ""
                if spec_body or rust_body:
                    body = spec_body if spec_body else rust_body
                    # Find the message bytes for diagnostic
                    args_hex = f"args_len={args_len}"
                sub_index_examples.append((tag, sub_index, word_len, entity_id, args_hex))

    # ── Print findings ────────────────────────────────────────────────────────

    print(f"## Decryption summary")
    print(f"Total UDP packets in pcap : {total_packets}")
    print(f"Decrypted (PKCS#7-valid)  : {decrypted}")
    print()

    print(f"## Dispute #12 - Flag-bit role assignments")
    print(f"")
    print(f"Rust constants (mercury/src/packet/mod.rs:54-78):")
    print(f"  bit 5 (0x20) = FRAGMENTED")
    print(f"  bit 6 (0x40) = HAS_SEQUENCE")
    print(f"  bit 7 (0x80) = INDEXED (audit said: should mean FRAGMENT per spec)")
    print(f"")
    print(f"Spec section 1.2:")
    print(f"  bit 5 (0x20) = HAS_SEQUENCE_NUMBER")
    print(f"  bit 6 (0x40) = HAS_REQUESTS")
    print(f"  bit 7 (0x80) = IS_FRAGMENT")
    print(f"")
    print(f"Footer-parse success counts (real packets in this pcap):")
    print(f"  Both interpretations parse OK : {both_ok_count}")
    print(f"  Rust parses, Spec fails       : {rust_ok_count}")
    print(f"  Spec parses, Rust fails       : {spec_ok_count}")
    print(f"  Neither parses                : {neither_ok_count}")
    print(f"")
    # Stronger semantic test: spec section 1.3 says HAS_FIRST_REQUEST_OFFSET (bit 0)
    # and HAS_REQUESTS (bit 6) are "in practice always set together". Under spec
    # bit assignments, a packet with bit 6 set but bit 0 NOT set is malformed.
    # Tally how many packets violate this if we use spec bit assignments.
    spec_violation_count = 0
    for fbyte, n in flag_byte_histogram.items():
        bit_6 = bool(fbyte & 0x40)
        bit_0 = bool(fbyte & 0x01)
        if bit_6 and not bit_0:
            # Under spec: HAS_REQUESTS without HAS_FIRST_REQUEST_OFFSET = invalid per section 1.3
            spec_violation_count += n

    print(f"Spec section 1.3 invariant: HAS_REQUESTS (bit 6) and HAS_FIRST_REQUEST_OFFSET")
    print(f"(bit 0) 'in practice are always set together'. Packets where bit 6 is set but")
    print(f"bit 0 is NOT set would be malformed under spec interpretation:")
    print(f"  Spec-malformed packet count: {spec_violation_count} of {decrypted}")
    print(f"")
    if spec_violation_count > decrypted * 0.5:
        print(f"VERDICT for dispute #12: RUST bit assignments are correct.")
        print(f"  Under spec interpretation, {spec_violation_count} packets ({100*spec_violation_count/decrypted:.0f}%) would")
        print(f"  violate spec section 1.3 (HAS_REQUESTS without HAS_FIRST_REQUEST_OFFSET).")
        print(f"  Under Rust interpretation, those same packets are normal CHAN+REL+SEQ traffic")
        print(f"  (bit 6 = HAS_SEQUENCE in Rust, not HAS_REQUESTS).")
        print(f"")
        print(f"  Spec section 1.2 has the bit-to-role mapping WRONG for bits 5/6/7 (and bit 0).")
        print(f"  The actual wire-format bit assignments are:")
        print(f"    bit 0 (0x01) = HAS_REQUESTS")
        print(f"    bit 5 (0x20) = FRAGMENTED")
        print(f"    bit 6 (0x40) = HAS_SEQUENCE")
        print(f"    bit 7 (0x80) = unused/INDEXED")
        print(f"")
        print(f"  Audit finding #12 is INVERTED: Rust is correct, spec is wrong. Update spec")
        print(f"  section 1.2 bit table to match wire reality.")
    elif rust_ok_count > spec_ok_count * 2:
        print(f"VERDICT (preliminary): RUST bit assignments are correct.")
    elif spec_ok_count > rust_ok_count * 2:
        print(f"VERDICT (preliminary): SPEC bit assignments are correct.")
    elif both_ok_count > (rust_ok_count + spec_ok_count) * 5:
        print(f"VERDICT: AMBIGUOUS — most packets parse under both interpretations.")
        print(f"  Look at the fragment-flag-byte histogram below for a more targeted")
        print(f"  test (fragments are the rare wire shape that distinguishes the two).")
    else:
        print(f"VERDICT: MIXED — neither interpretation dominates. Manual inspection")
        print(f"  of fragment packets needed (see histogram below).")
    print()

    print(f"## Flag-byte histogram")
    print(f"Top flag bytes seen on the wire:")
    for fbyte, n in flag_byte_histogram.most_common(15):
        # Decode under both interpretations
        rust_parts = []
        if fbyte & FlagsRust.HAS_REQUESTS: rust_parts.append("HAS_REQ")
        if fbyte & FlagsRust.PIGGYBACK: rust_parts.append("PIG")
        if fbyte & FlagsRust.HAS_ACKS: rust_parts.append("ACK")
        if fbyte & FlagsRust.ON_CHANNEL: rust_parts.append("CHAN")
        if fbyte & FlagsRust.RELIABLE: rust_parts.append("REL")
        if fbyte & FlagsRust.FRAGMENTED: rust_parts.append("FRAG")
        if fbyte & FlagsRust.HAS_SEQUENCE: rust_parts.append("SEQ")
        if fbyte & FlagsRust.INDEXED: rust_parts.append("IDX")

        spec_parts = []
        if fbyte & FlagsSpec.HAS_FIRST_REQUEST_OFFSET: spec_parts.append("FRO")
        if fbyte & FlagsSpec.HAS_PIGGYBACKS: spec_parts.append("PIG")
        if fbyte & FlagsSpec.HAS_ACKS: spec_parts.append("ACK")
        if fbyte & FlagsSpec.ON_CHANNEL: spec_parts.append("CHAN")
        if fbyte & FlagsSpec.IS_RELIABLE: spec_parts.append("REL")
        if fbyte & FlagsSpec.HAS_SEQUENCE_NUMBER: spec_parts.append("SEQ")
        if fbyte & FlagsSpec.HAS_REQUESTS: spec_parts.append("REQ")
        if fbyte & FlagsSpec.IS_FRAGMENT: spec_parts.append("FRAG")
        print(f"  0x{fbyte:02X}  n={n:5d}  Rust=[{'|'.join(rust_parts)}]   Spec=[{'|'.join(spec_parts)}]")
    print()

    if fragment_flag_bytes:
        print(f"## Fragment packets (key evidence for #12)")
        print(f"Packets where bit 5 + (bit 6 or bit 7) are both set:")
        frag_hist = Counter(fragment_flag_bytes)
        for fbyte, n in frag_hist.most_common(10):
            spec_says_frag = bool(fbyte & 0x80)  # spec: bit 7 = IS_FRAGMENT
            rust_says_frag = bool(fbyte & 0x20)  # rust: bit 5 = FRAGMENTED
            print(f"  0x{fbyte:02X}  n={n:5d}  Spec interprets as fragment? {spec_says_frag}   Rust interprets as fragment? {rust_says_frag}")
    else:
        print(f"## Fragment packets")
        print(f"No fragmented packets observed in this pcap.")
    print()

    print(f"## Dispute #2 - Sub-slot threshold (sub_index distribution)")
    print(f"")
    print(f"Spec says sub_index = method_index - 62.")
    print(f"Rust says sub_index = method_index - 61.")
    print(f"")
    print(f"Cross-reference points (method indices the spec/code call out by name):")
    print(f"  Method 117 (onClientMapLoad)     -> spec sub_index = 55 (0x37)   Rust = 56 (0x38)")
    print(f"  Method 122 (setupWorldParameters) -> spec sub_index = 60 (0x3C)   Rust = 61 (0x3D)")
    print(f"  Method 115 (onPlayerDataLoaded)   -> spec sub_index = 53 (0x35)   Rust = 54 (0x36)")
    print(f"")
    print(f"sub_index values observed in 0xBD/0xFD-prefixed entity methods:")
    print(f"")
    if not sub_index_distribution:
        print(f"  No 0xBD/0xFD-prefixed messages found in the pcap.")
        print(f"  This is expected if the captured session is mostly login/character-")
        print(f"  select traffic (entity methods 0..60 use direct encoding 0x80..0xBC).")
        print(f"  To resolve dispute #2, capture a pcap that includes world entry")
        print(f"  (which emits setupWorldParameters/onClientMapLoad/onPlayerDataLoaded).")
    else:
        for sub_idx, n in sorted(sub_index_distribution.items()):
            spec_method = sub_idx + 62
            rust_method = sub_idx + 61
            note = ""
            if spec_method in (115, 117, 122):
                note = f"  <- if spec, this is method {spec_method}"
            if rust_method in (115, 117, 122):
                note = f"  <- if Rust, this is method {rust_method}"
            print(f"  sub_index = {sub_idx:3d} (0x{sub_idx:02X})  n={n:4d}  spec->method {spec_method}  rust->method {rust_method}{note}")
        print(f"")
        print(f"## First 20 0xBD/0xFD example messages")
        for tag, sub_index, word_len, entity_id, args_hex in sub_index_examples:
            eid_str = f"eid=0x{entity_id:08X}" if entity_id is not None else "no-eid"
            print(f"  {tag} sub_index={sub_index:3d} (0x{sub_index:02X}) word_len={word_len:3d} {eid_str} {args_hex}")
        print()

        # Cross-reference against Rust's known method_idx constants.
        # For each observed sub_index, compute candidate method indices under both
        # encoding hypotheses and check which produces a named method in METHOD_IDX.
        print(f"")
        print(f"## Cross-reference against Rust method_idx constants")
        print(f"For each observed sub_index, check which encoding maps to a NAMED method.")
        print(f"")
        print(f"{'sub_idx':>8}  {'count':>5}  {'spec(method=sub+62)':<35}  {'rust(method=sub+61)':<35}")
        print(f"{'-'*8}  {'-'*5}  {'-'*35}  {'-'*35}")
        spec_named_hits = 0
        rust_named_hits = 0
        for sub_idx, n in sorted(sub_index_distribution.items()):
            spec_method = sub_idx + 62
            rust_method = sub_idx + 61
            spec_name = METHOD_IDX.get(spec_method, "")
            rust_name = METHOD_IDX.get(rust_method, "")
            spec_marker = "<- NAMED" if spec_name else ""
            rust_marker = "<- NAMED" if rust_name else ""
            if spec_name: spec_named_hits += n
            if rust_name: rust_named_hits += n
            spec_disp = f"{spec_method:3d} {spec_name} {spec_marker}".strip()
            rust_disp = f"{rust_method:3d} {rust_name} {rust_marker}".strip()
            print(f"{sub_idx:>8}  {n:>5}  {spec_disp:<35}  {rust_disp:<35}")
        print(f"")
        print(f"Total observations of sub_index values that map to a NAMED Rust method:")
        print(f"  Under SPEC encoding (sub_index = method - 62): {spec_named_hits} packets")
        print(f"  Under RUST encoding (sub_index = method - 61): {rust_named_hits} packets")
        print(f"")
        print(f"## Disambiguating evidence")
        print(f"Cases where ONLY ONE interpretation maps to a named method (most decisive):")
        print(f"")
        print(f"{'sub_idx':>8}  {'count':>5}  {'evidence for':<10}  {'method':<6}  {'name'}")
        print(f"{'-'*8}  {'-'*5}  {'-'*10}  {'-'*6}  {'-'*30}")
        rust_uniq = 0
        spec_uniq = 0
        for sub_idx, n in sorted(sub_index_distribution.items()):
            spec_method = sub_idx + 62
            rust_method = sub_idx + 61
            spec_named = spec_method in METHOD_IDX
            rust_named = rust_method in METHOD_IDX
            if rust_named and not spec_named:
                print(f"{sub_idx:>8}  {n:>5}  RUST       {rust_method:<6}  {METHOD_IDX[rust_method]}")
                rust_uniq += n
            elif spec_named and not rust_named:
                print(f"{sub_idx:>8}  {n:>5}  SPEC       {spec_method:<6}  {METHOD_IDX[spec_method]}")
                spec_uniq += n
        print(f"")
        print(f"Disambiguating totals:")
        print(f"  Packets that ONLY make sense under RUST encoding: {rust_uniq}")
        print(f"  Packets that ONLY make sense under SPEC encoding: {spec_uniq}")
        print(f"")
        if rust_uniq > 0 and spec_uniq == 0:
            print(f"## VERDICT for dispute #2: RUST threshold (61) is correct.")
            print(f"  {rust_uniq} packets only make sense under Rust's encoding")
            print(f"  (sub_index = method_index - 61); zero packets only make sense under spec's.")
            print(f"  Spec sections 1.5/1.8/1.16 Q1 closure are wrong about the formula.")
        elif spec_uniq > 0 and rust_uniq == 0:
            print(f"## VERDICT for dispute #2: SPEC threshold (62) is correct.")
            print(f"  {spec_uniq} packets only make sense under spec's encoding; zero under Rust's.")
            print(f"  Audit finding #2 is Critical — Rust has the off-by-one bug.")
        elif rust_uniq > spec_uniq * 3:
            print(f"## VERDICT for dispute #2: RUST threshold (61) is overwhelmingly favored.")
            print(f"  {rust_uniq} Rust-only packets vs {spec_uniq} spec-only.")
        elif spec_uniq > rust_uniq * 3:
            print(f"## VERDICT for dispute #2: SPEC threshold (62) is overwhelmingly favored.")
            print(f"  {spec_uniq} spec-only packets vs {rust_uniq} Rust-only.")
        elif rust_named_hits > spec_named_hits * 2:
            print(f"## VERDICT for dispute #2: RUST threshold (61) is correct.")
            print(f"  Observed sub_index values overwhelmingly match named methods under the")
            print(f"  threshold-61 hypothesis ({rust_named_hits} hits vs {spec_named_hits} under spec).")
            print(f"  Spec sections 1.5/1.8/1.16 Q1 closure have an off-by-one error in the")
            print(f"  sub_index formula. The actual encoding is sub_index = method_index - 61.")
        elif spec_named_hits > rust_named_hits * 2:
            print(f"## VERDICT for dispute #2: SPEC threshold (62) is correct.")
            print(f"  Observed sub_index values overwhelmingly match named methods under the")
            print(f"  threshold-62 hypothesis ({spec_named_hits} hits vs {rust_named_hits} under Rust).")
            print(f"  Audit finding #2 is Critical — Rust has the off-by-one bug.")
        else:
            print(f"## VERDICT for dispute #2: AMBIGUOUS — close to a tie ({rust_named_hits} Rust vs {spec_named_hits} spec).")
            print(f"  The pcap may not contain enough extended-encoded entity-method calls to")
            print(f"  cleanly distinguish. Capture a longer session with more world-entry traffic.")


if __name__ == "__main__":
    main()
