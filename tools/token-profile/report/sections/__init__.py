"""Report sections. Each module's build(db, sc, ...) returns one JSON-ready dict.

The layers stay separate on purpose: `tokens` is raw counts, `cost` is the
only section with USD besides the per-PR and cache-policy sections, and
`context` and `tools` measure context pressure and exposure, which are never
priced.
"""
