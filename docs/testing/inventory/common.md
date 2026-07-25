# Tests — `common`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-07-25 *(links repaired; catalogue rows still the 2026-05-04 snapshot)*  
> **Total tests**: 31  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Shared types, config loading, error handling, math primitives. No deps on other crates.

## All tests (31)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [default_config_has_expected_ports](../../../crates/common/src/config.rs#L203) | unit | Config | 2026-03-03 | Asserts equality on `config.auth_port` |  |
| [default_config_developer_mode_off](../../../crates/common/src/config.rs#L213) | unit | Config | 2026-03-03 | Asserts on `!config.developer_mode` |  |
| [load_config_returns_default_for_now](../../../crates/common/src/config.rs#L257) | unit | Config | 2026-03-03 | Asserts equality on `config.auth_port` |  |
| [config_error_display](../../../crates/common/src/error.rs#L108) | unit | Error | 2026-03-03 | Asserts equality on `format!("{}", err)` |  |
| [network_error_display](../../../crates/common/src/error.rs#L114) | unit | Error | 2026-03-03 | Asserts equality on `format!("{}", err)` |  |
| [protocol_error_display](../../../crates/common/src/error.rs#L120) | unit | Error | 2026-03-03 | Asserts equality on `format!("{}", err)` |  |
| [database_error_display](../../../crates/common/src/error.rs#L126) | unit | Error | 2026-03-03 | Asserts equality on `format!("{}", err)` |  |
| [entity_error_display](../../../crates/common/src/error.rs#L132) | unit | Error | 2026-03-03 | Asserts equality on `format!("{}", err)` |  |
| [auth_error_display](../../../crates/common/src/error.rs#L138) | unit | Error | 2026-03-03 | Asserts equality on `format!("{}", err)` |  |
| [io_error_from_std](../../../crates/common/src/error.rs#L147) | unit | Error | 2026-03-03 | Asserts on `format!("{}", err).contains("file not found")` |  |
| [vector3_zero](../../../crates/common/src/math.rs#L139) | unit | Math | 2026-03-03 | Asserts equality on `v.x` |  |
| [vector3_new](../../../crates/common/src/math.rs#L147) | unit | Math | 2026-03-03 | Asserts equality on `v.x` |  |
| [vector3_add](../../../crates/common/src/math.rs#L155) | unit | Math | 2026-03-03 | Asserts equality on `c` |  |
| [vector3_sub](../../../crates/common/src/math.rs#L163) | unit | Math | 2026-03-03 | Asserts equality on `c` |  |
| [vector3_mul_scalar](../../../crates/common/src/math.rs#L171) | unit | Math | 2026-03-03 | Asserts equality on `scaled` |  |
| [vector3_length](../../../crates/common/src/math.rs#L178) | unit | Math | 2026-03-03 | Asserts on `(v.length() - 5.0).abs() < f32::EPSILON` |  |
| [vector3_distance](../../../crates/common/src/math.rs#L184) | unit | Math | 2026-03-03 | Asserts on `(a.distance_to(&b) - 5.0).abs() < f32::EPSILON` |  |
| [vector3_distance_squared](../../../crates/common/src/math.rs#L191) | unit | Math | 2026-03-03 | Asserts on `(a.distance_squared_to(&b) - 25.0).abs() < f32::EPSILON` |  |
| [vector3_normalized](../../../crates/common/src/math.rs#L198) | unit | Math | 2026-03-03 | Asserts on `(n.x - 1.0).abs() < f32::EPSILON` |  |
| [vector3_normalized_zero](../../../crates/common/src/math.rs#L207) | unit | Math | 2026-03-03 | Asserts equality on `n` |  |
| [quaternion_identity](../../../crates/common/src/math.rs#L214) | unit | Math | 2026-03-03 | Asserts equality on `q.x` |  |
| [quaternion_new](../../../crates/common/src/math.rs#L223) | unit | Math | 2026-03-03 | Asserts equality on `q.x` |  |
| [default_vector3_is_zero](../../../crates/common/src/math.rs#L232) | unit | Math | 2026-03-03 | Asserts equality on `v` |  |
| [default_quaternion_is_zero_components](../../../crates/common/src/math.rs#L238) | unit | Math | 2026-03-03 | Asserts equality on `q.x` |  |
| [entity_id_display](../../../crates/common/src/types.rs#L127) | unit | Types | 2026-03-03 | Asserts equality on `format!("{}", id)` |  |
| [space_id_display](../../../crates/common/src/types.rs#L133) | unit | Types | 2026-03-03 | Asserts equality on `format!("{}", id)` |  |
| [distribution_flags_contains](../../../crates/common/src/types.rs#L139) | unit | Types | 2026-03-03 | Asserts on `DistributionFlags::ALL_CLIENTS.contains(DistributionFlags::OWN_CLIENT)` |  |
| [distribution_flags_cell_private_is_empty](../../../crates/common/src/types.rs#L146) | unit | Types | 2026-03-03 | Asserts on `DistributionFlags::CELL_PRIVATE.is_empty()` |  |
| [distribution_flags_union](../../../crates/common/src/types.rs#L151) | unit | Types | 2026-03-03 | Asserts equality on `combined` |  |
| [entity_id_equality](../../../crates/common/src/types.rs#L157) | unit | Types | 2026-03-03 | Asserts equality on `EntityId(1)` |  |
| [message_id_value](../../../crates/common/src/types.rs#L163) | unit | Types | 2026-03-03 | Asserts equality on `msg.0` |  |
