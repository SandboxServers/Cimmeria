---
name: feedback-terse-responses
description: User reads diff/output directly — skip trailing summaries at the end of responses
metadata:
  type: feedback
---

Do not add a trailing summary paragraph ("In summary, I have done X, Y, Z") at the end of responses. The user reads the tool output and diff directly.

**Why:** User preference — they are a senior engineer who processes output directly.

**How to apply:** End responses with the last substantive finding or recommendation. No recap paragraph.
