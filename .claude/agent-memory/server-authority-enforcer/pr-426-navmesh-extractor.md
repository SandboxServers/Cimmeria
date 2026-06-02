---
name: pr-426-navmesh-extractor
description: Build-time UE3 navmesh extractor; XrcNav::read hardened with check_count + checked_alloc_size before any allocation
metadata:
  type: project
---

PR #426 (cimmeria-navmesh-extractor Phase 0) verdict: SHIP.

**Threat model:** Build-time tool that consumes `data/spaces/*.nav` files. Only attackers with filesystem-write access to the repo can craft hostile inputs; if they have that, they own much more than the build tool. Still, defense-in-depth applied is the right call.

**Where the hardening lives** (origin tip of `feat/navmesh-extractor-46-phase0`, commit `21b2b681`):

- `crates/navmesh-extractor/src/nav_roundtrip.rs::check_count` validates header counts (`MAX_NVERTS=1M`, `MAX_NPOLYS=1M`, `MAX_NVP=64`, `MAX_DETAIL_NMESHES=1M`, `MAX_DETAIL_NVERTS=10M`, `MAX_DETAIL_NTRIS=10M`) before any allocation.
- `crates/navmesh-extractor/src/nav_roundtrip.rs::checked_alloc_size` computes `count * stride` as `u64`, returns `ExtractError::NavHeaderOutOfRange` on overflow or `usize::try_from` failure.
- Regression tests `read_rejects_oversized_nverts` + `read_rejects_oversized_npolys` synthesise hostile headers and assert the typed error before allocation.

**Minor inconsistency** (advisory only): `regs`, `flags`, `areas` allocations still use `vec![0u16; npolys as usize]` directly. Safe because `npolys ≤ MAX_NPOLYS = 1M`, but stylistically inconsistent with the `checked_alloc_size` discipline. Not a fix-required item; flag if a future maintainer copies this and forgets the cap.

**Open follow-up** (out of #426 scope): `crates/entity/src/navigation.rs::NavMesh::load` is the runtime navmesh loader and has the EXACT same unguarded `(nverts * 3) as usize`, `(npolys * nvp * 2) as usize` etc. patterns. This is loaded at runtime by the server from `data/spaces/*.nav`. An operator who manages to drop a malicious `.nav` into the data dir could trigger the same overflow there. Worth a follow-up issue to port `check_count` + `checked_alloc_size` to the runtime loader.

**Pattern worth promoting:** see [pattern-checked-alloc-size.md](pattern-checked-alloc-size.md) — the helper shape from this PR is the right canonical idiom for any attacker-influenced count-and-stride Vec allocation in the codebase.
