# spec-lint

`cimmeria-spec-lint`, the Cimmeria Bible symbol-resolution lint. It scans every chapter under `docs/spec/` and warns about `evidence_refs.rust:` entries and `crates/...::Type::method` references that do not resolve to a Rust symbol, and about bare `.rs:N` line citations in sections 4 and 5. It always exits 0; under GitHub Actions it emits inline `::warning` annotations.

```powershell
pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-spec-lint --bin spec-lint
```

CI runs it in `spec-lint.yml` (warn-only). The citation grammar it enforces: [docs/spec/conventions.md](../../docs/spec/conventions.md).
