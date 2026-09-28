---
name: baseapp-login-retry-train
description: 41-byte "ciphertext length 25" decrypt failures at 300 ms on an established session are the client re-sending its plaintext baseAppLogin; client acked the reply but never completed its reply handler (colo 2026-09-26, unresolved client-side)
metadata:
  type: project
---

Colo 2026-09-26 (one tester): two logins from the same client process hung before char select. The server accepted the ticket, sent connect_reply (seq 1) and time_sync (seq 2), and the client **acked both** (`acks [1,2]`, ack-only packet, flags 0x4C). The client then re-sent its 41-byte plaintext baseAppLogin every 300 ms. Those datagrams hit the encrypted path and failed the v1 length check (41 - 16 tag = 25). A restarted client on a new port logged in fine.

**Correction (2026-09-28, #842):** "a healthy login never acks seq 1 and 2" below is wrong. Of the five logins decoded from `debug/` captures, three healthy ones send exactly that `0x4C` `acks [2,1]` packet on nub seq 2; two never ack them. The ack packet is not a failure signature. Since #842 both packets are in the channel TX window and resent on RTO, and the retry row carries `reply_outstanding` (true = reply unacked, likely lost). See `docs/protocol/login-handshake.md` § "Reliability of the reply and the time-sync bundle".

**Why:** the client's login timer (`ServerSelectSuccess` handler `SGW.exe@0x00ddfd00`, 300 ms timer `0x493e0` µs, tick `@0x00de10b0`) keeps sending attempts until its `BaseAppLoginHandler` reply completes. The reply reached the client and was acked, but it was never delivered to the handler. A healthy login never acks seq 1 and 2, and its first client packet is channel seq 0 (AUTHENTICATE + ENABLE_ENTITIES). The failing client's ack packet used the nub sequence counter (seq 2, then 18 on the retry), so the client's state was different. The client-side cause is still unknown and needs an x64dbg session on a client that reproduces it.

**Refuted:** "the reply was lost, so re-send it". The client received and acked it. Re-sending the same seq-1 bytes would be dropped again as a duplicate. The legacy C++ server also dropped these retries (EncryptionFilter length check, TRACE).

**How to apply:** do not treat `decrypt_fail` spikes from one addr at 300 ms as a key or HMAC problem. After branch fix/colo-login-plaintext-resend, look for `reason = login_retry_on_channel` instead. Also, connect_reply and time_sync are sent with raw `send_to` and are **not** in the session Channel's tx_window, so a truly lost seq 1 or 2 is never retransmitted. That is a separate latent bug, not what happened here; tracked as #842.
