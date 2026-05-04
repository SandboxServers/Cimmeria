# Tests — `upk-objects`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 11  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

UPK (Unreal Package) object type definitions — readers and writers for the engine's serialized object graphs.

## All tests (11)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [parse_unused_bulk_data](../../../crates/upk-objects/src/bulk_data.rs#L237) | unit | Bulk Data | 2026-03-18 | Asserts equality on `result.flags` |  |
| [parse_inline_bulk_data](../../../crates/upk-objects/src/bulk_data.rs#L256) | unit | Bulk Data | 2026-03-18 | Parse inline bulk data |  |
| [parse_empty_lazy_array](../../../crates/upk-objects/src/bulk_data.rs#L275) | unit | Bulk Data | 2026-03-18 | Asserts equality on `result.element_count` |  |
| [parse_lazy_array_with_data](../../../crates/upk-objects/src/bulk_data.rs#L288) | unit | Bulk Data | 2026-03-18 | Asserts equality on `result.element_count` |  |
| [unpack_normal_center](../../../crates/upk-objects/src/static_mesh.rs#L459) | unit | Static Mesh | 2026-03-18 | Asserts on `n[0].abs() < 0.01` |  |
| [unpack_normal_positive](../../../crates/upk-objects/src/static_mesh.rs#L468) | unit | Static Mesh | 2026-03-18 | Asserts on `(n[0] - 1.0).abs() < 0.01` |  |
| [unpack_normal_negative](../../../crates/upk-objects/src/static_mesh.rs#L475) | unit | Static Mesh | 2026-03-18 | Asserts on `(n[0] + 1.0).abs() < 0.01` |  |
| [parse_bounds_roundtrip](../../../crates/upk-objects/src/static_mesh.rs#L482) | unit | Static Mesh | 2026-03-18 | Parse bounds roundtrip |  |
| [skip_empty_kdop_tree](../../../crates/upk-objects/src/static_mesh.rs#L504) | unit | Static Mesh | 2026-03-18 | Asserts equality on `pos` |  |
| [skip_small_kdop_tree](../../../crates/upk-objects/src/static_mesh.rs#L516) | unit | Static Mesh | 2026-03-18 | Asserts equality on `pos` |  |
| [parse_single_vertex_40byte](../../../crates/upk-objects/src/static_mesh.rs#L528) | unit | Static Mesh | 2026-03-18 | Parse single vertex 40byte |  |
