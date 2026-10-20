---
name: Cellblock N05 interaction authority
description: Source-checked offer path for the first authored Cellblock mission
type: reference
---

2026-10-14: The CB-N05 design review found that `dialogButtonChoice` can forward a client-supplied ordinal, including `-1`, and `initialResponse` can name a stale pin. The current content conditions cannot authorize a new mission from the button ordinal. The native `interact` handler validates target existence, same space and range before dispatching `interact_tag`, so the proposed mission accepts through that server-checked path. See `docs/analysis/cellblock-continuation/worknotes/cb-n05-seal.md` for the exact source join, conditions, negative tests and remaining native-client proof. This is a design finding, not a completed client UAT result.
