# Security audits

Time-stamped reference records of security audits run against the Cimmeria
server. Each audit directory contains the findings as committed at audit
completion. **Finding bodies are never rewritten post-audit** — the audit is a
point-in-time snapshot, and fixes are tracked via the linked GitHub issues.

Status *does* get layered on top, as a dated blockquote banner above the
finding (or at the top of the category file) reading
`> **Status re-verification (YYYY-MM-DD)**` or
`> **Resolved (#NNN).**`. A banner records what re-verification against the
tree found; it never edits the original text. Two rules for anyone adding one:

- Cite `file:line` evidence for a status change, and say which branch it was
  verified against — "fixed on a feature branch" is not "fixed on `main`".
- When in doubt, leave the finding open. A finding wrongly marked fixed is far
  more damaging than one wrongly left open.

## Audits

- [`2026-05-31-server-authority/`](2026-05-31-server-authority/) — Server-authority / anti-cheat / anti-replay audit across all 15 player-facing categories. 177 findings. See [`2026-05-31-server-authority/UMBRELLA.md`](2026-05-31-server-authority/UMBRELLA.md) for the executive summary + triage tiers. Tracking umbrella issue: [#459](https://github.com/SandboxServers/Cimmeria/issues/459). Per-category issues: [#460](https://github.com/SandboxServers/Cimmeria/issues/460)-[#474](https://github.com/SandboxServers/Cimmeria/issues/474). P0 foundational issues: [#475](https://github.com/SandboxServers/Cimmeria/issues/475)-[#479](https://github.com/SandboxServers/Cimmeria/issues/479).
