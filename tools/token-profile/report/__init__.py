"""Token profiler reports (TP-01b): stdlib-only reports over the profiler's SQLite database.

Layers are kept apart: raw tokens, estimated list-price USD (a plan-usage
proxy, not a bill), context pressure and exposure, cost per merged PR, and a
cache-policy simulator. Every report is version-stamped and passes the
privacy gate in scrub.py before it is written. Usage: cli.py.
"""
