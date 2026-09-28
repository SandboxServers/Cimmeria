---
name: login-handshake-acks-and-pre-channel-sends
description: Login reply (seq 1) + time-sync (seq 2) are in the channel TX window since #842; real clients ack them in only 3 of 5 captures; how to decode the captures and test a pre-channel packet drop
metadata:
  type: project
---

**Fact (2026-09-28, #842).** `handle_login` sends seq 1 and 2 raw, then builds the session channel with `new_client_channel_with_handshake` so both datagrams sit in the TX window and the tick loop's retransmit scan resends them. Legacy C++ sent both through the channel's reliable bundle, so this is parity.

**Client ack shapes** (five logins in `debug/*/` pcaps, decoded with `python tools/pcap_to_session.py <pcap> <keys> --out x.jsonl`; Windows python needs `C:/...` paths, not `/c/...`): 3 of 5 send an ack-only `0x4C` packet, nub seq 2, `acks [2,1]` ~10 ms after; 2 of 5 never ack seqs 1/2 at all. For those, one resend of each ~1.5 s in (initial RTO) is expected; the client acks duplicates (`queueAckForPacket` before `inSeqAt`). Inferred, not seen on the wire yet.

**Why it matters:** before registering any pre-channel reliable send, check the captures for whether the client ever acks it. An unacked TX entry resends forever (base never reaps on `MAX_RETRIES`; `is_timed_out` is not called by base) and opens a tx_hole (`TX_HOLE_WARN_MS` = 2 s vs initial RTO 1.5 s).

**How to apply:** harness peers and wireclient sessions start after the handshake, so they cannot see a lost seq 1/2. Test pre-channel drops by running `run_connect_loop` on a loopback socket behind a `Transport` wrapper that drops one reliable seq (`crates/base/src/base/login/tests/handshake_retransmit.rs`). Wireclient now acks 1/2 via `LoopbackPeer::queue_ack`. Related: [[wireclient-passive-session-dies]].
