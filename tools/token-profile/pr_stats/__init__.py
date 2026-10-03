"""Per-PR stats comments (TP-10): one idempotent cimmeria-pr-stats/1 comment per PR.

block.py builds and renders it from the profiler database and gh, github.py
finds, edits or creates the one comment, backfill.py walks merged PRs.
Usage: cli.py.
"""
