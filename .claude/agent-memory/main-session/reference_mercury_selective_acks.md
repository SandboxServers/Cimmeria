---
name: reference-mercury-selective-acks
description: "SGW client ACKs are per-packet and include packets buffered behind a gap; the server's old cumulative ACK drain retired lost packets and wedged the client's reliable stream (fixed 2026-09-27). Read before touching ACK, retransmit or visibility code."
metadata:
  type: reference
---

**Fact (verified 2026-09-27).** The SGW client queues an ACK for every reliable packet with a valid sequence *before* its `inSeqAt` check (`UnAckedHandler::queueAckForPacket`, `0x0158cba0`, ack-set insert `FUN_0157ac40(this+0x9c, seq)`). So it ACKs packets it is buffering behind a gap. Wire proof: `debug/lomiada-broke-in-hallway02/` — #1148 never arrived, the client acked #1149..#1358 one by one starting ~100 ms later; its log says `Buffering packet #1149..#1358 above #1148`. Decode with `python tools/pcap_to_session.py <pcap> <keys> --out x.jsonl`.

**Consequence.** Any cumulative reading of client ACKs retires the lost packet, it is never resent, and the client holds every later reliable message forever (no creates/leaves/methods; unreliable movement still flows). This was the leading explanation for "players can't reliably see each other". Fixed by `Channel::process_ack` / `process_ack_footer` in `crates/mercury/src/channel/ack.rs`; details in `docs/protocol/mercury-wire-format.md` § "SGW Client and Rust Server".

**Diagnosing a stuck client now:** SigNoz target `mercury.tx_hole` (`tx_hole_stall` WARN = client stuck behind a missing server packet), counters `mercury_tx_holes_total` / `mercury_tx_hole_stalls_total`.

**Test gotcha.** Harness and wireclient peers that stay silent until the sender's RTO cannot catch ACK-semantics bugs; the real client sends ~6 pkt/s with ACKs. Use `chaos/gap_acked_past_by_prompt_client.rs` as the pattern (TESTING.md type 10).

**Also verified:** the stock client's reorder buffer is 512 slots (`Channel+0x2c = 0x200`, read at `0x0158C801`), not 32; `TX_WINDOW_SIZE = 32` has no client reason (#353).

**Still open after the fix (2026-09-27):** disconnect teardown `try_send(DisconnectEntity)` drops silently and frees the entity id before the cell tears down; ring-transport hide has no server-side hidden state. Retest the invisible Cellblock guard ([[project-invisible-cellblock-guard]]) once the ACK fix is deployed.
