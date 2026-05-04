# Tests — `defs`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 5  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Parses entity definitions from `entities/defs/` XML into Rust types.

## All tests (5)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [registry_constructs_without_panic](../../../crates/defs/src/registry.rs#L103) | unit | Registry | 2026-03-03 | Registry constructs without panic | smell: no_assert_or_question_mark |
| [registry_default_is_same_as_new](../../../crates/defs/src/registry.rs#L111) | unit | Registry | 2026-03-03 | Asserts equality on `a.len()` |  |
| [lookup_by_name_and_id_are_consistent](../../../crates/defs/src/registry.rs#L118) | unit | Registry | 2026-03-03 | Asserts equality on `by_name.name` |  |
| [missing_name_returns_none](../../../crates/defs/src/registry.rs#L130) | unit | Registry | 2026-03-03 | Asserts on `registry.lookup_by_name("NonExistentEntity").is_none()` |  |
| [missing_id_returns_none](../../../crates/defs/src/registry.rs#L136) | unit | Registry | 2026-03-03 | Asserts on `registry.lookup_by_id(u16::MAX).is_none()` |  |
