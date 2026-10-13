---
name: lab_wait_bridge_retry
description: Waits retry transient Lua bridge transport failures until their original deadline.
type: project
---

A UI condition wait retries only recognized bridge dispatch timeout/queue-full and reset/EOF errors. Each read is bounded by the original deadline; its timeout result includes failed_polls and last_error. Lua, malformed-response and revoked-lease errors stop immediately. The current bridge closes on a bad token without a distinct rejection ack, so auth EOF remains ambiguous.
