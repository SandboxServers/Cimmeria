---
name: lab_wait_bridge_retry
description: Waits retry transient Lua bridge transport failures until their original deadline.
type: project
---

A UI condition wait treats bridge transport errors as failed poll attempts, sleeps for the remaining poll interval, and retries until the original deadline. Deadline results retain the last bridge error with met: false, elapsed time and poll count; successful conditions and explicit fail values keep their existing outcomes.
