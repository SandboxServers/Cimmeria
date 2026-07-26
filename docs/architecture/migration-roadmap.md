---
title: "C++ Dependency Migration Roadmap (historical)"
type: explanation
audience: engineers
last_updated: 2026-07-25
---

# C++ Dependency Migration Roadmap (historical)

> [!IMPORTANT]
> **Historical record. Every migration below is a dependency of the deprecated
> C++ server, not of anything Cimmeria ships.**
>
> The active Rust server under [`crates/`](../../crates/) shares **none** of
> these dependencies — it manages everything through Cargo. The tree these
> migrations target now sits under [`deprecated/cpp/`](../../deprecated/cpp/)
> and is not maintained, so most rows below will never be executed.
>
> **Looking for the actual roadmap?** See
> [project-status.md](../project-status.md) for where the project is and where
> it is going, and [gap-analysis.md](../gap-analysis.md) for per-system
> completeness. This page is not that.

## Do not cite the OpenSSL row as a live finding

This is the single most misread line in the repo, so it gets its own heading.

The roadmap below marks **OpenSSL 0.9.8i → 3.5.x** as "Pending — CRITICAL
(active CVEs)". That describes OpenSSL **statically linked into the deprecated
C++ server and into the 2009 game client**. It is **not an open vulnerability
in anything Cimmeria runs**:

- The Rust auth server terminates TLS with `tokio-rustls` and links no OpenSSL.
- No crate in the workspace depends on OpenSSL.

Quoting this row in a security review, an issue, or an audit as though it were
a finding against Cimmeria is wrong. For the crypto work that actually shipped
and the work that is actually proposed, see
[encryption-modernization.md](encryption-modernization.md).

## What this was

From the project's C++ era, this page tracked the dependency-upgrade plan for
the original server: which third-party libraries were how far behind, in what
order they should be upgraded, and what each upgrade would involve. It was
extracted from CLAUDE.md to keep that file short.

Two of its migrations completed before the Rust rewrite made the rest moot:

| Migration | Path | Outcome |
|---|---|---|
| MSVC Toolchain | v120 (VS2012) → v145 (VS2026) | **Complete** — all 6 C++ projects build under v145 |
| PostgreSQL | 9.2.3 → 17.9 | **Complete** — still the live database; compatibility fixes applied (dropped `default_with_oids`, `EXECUTE PROCEDURE` → `EXECUTE FUNCTION`), pgdata version-mismatch detection added to bootstrap |

The PostgreSQL upgrade is the one piece of this roadmap with ongoing relevance:
Cimmeria still runs PostgreSQL 17.9 (EOL November 2029), and the bootstrap
tooling that came out of that migration is still what sets up a local database.

## The rest, unexecuted

Recorded for completeness. Each of these was a real plan against
`deprecated/cpp/`; none is scheduled, and the Rust server needs none of them.

| Migration | Path | Original priority | Why it no longer matters |
|---|---|---|---|
| OpenSSL | 0.9.8i → 3.5.x | CRITICAL | Rust uses `tokio-rustls`; no OpenSSL anywhere in the workspace |
| Boost | 1.55.0 → 1.90.0 | HIGH | Boost.Asio / .Python / .Thread / .Filesystem all replaced by `tokio` + std |
| Python (embedded) | 3.4.1 → 3.12+ | MEDIUM | No embedded interpreter — see [python-console.md](python-console.md) |
| Build system | `.sln`/`.vcxproj` → CMake + vcpkg | MEDIUM | Cargo |
| SOCI | 3.2.1 → 4.1.2 | MEDIUM | Replaced by `sqlx` |
| TinyXML2 | ~1.x → 11.x | LOW | Rust XML parsing in [`crates/defs/`](../../crates/defs/) |
| Qt | 5.x → 6.10 | LOW (ServerEd only) | ServerEd replaced by Tauri tools — see [tauri-rewrite.md](tauri-rewrite.md) |
| Recast/Detour | ~2013 → 1.6.0 | LOW | Navmesh handled in [`crates/navmesh-extractor/`](../../crates/navmesh-extractor/) |

The original page also carried eight "migration agent" definitions — per-library
expertise briefs describing the breaking changes each upgrade would hit. They
were written for a rewrite path the project did not take. If a C++ migration is
ever revived, `git log` on this file recovers them.

## Related documents

- [project-status.md](../project-status.md) — the actual forward-looking
  roadmap.
- [gap-analysis.md](../gap-analysis.md) — per-system feature completeness.
- [tech-stack-replacement.md](tech-stack-replacement.md) — the decision
  document that chose the rewrite over this roadmap.
- [encryption-modernization.md](encryption-modernization.md) — the real crypto
  work, shipped and proposed.
- [`deprecated/cpp/src/README.md`](../../deprecated/cpp/src/README.md) — the
  deprecated tree's own overview.
