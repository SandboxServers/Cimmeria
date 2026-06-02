---
name: pattern-checked-alloc-size
description: Canonical idiom for safely sizing a Vec from attacker-influenced header counts and strides; lives in crates/navmesh-extractor/src/nav_roundtrip.rs
metadata:
  type: reference
---

The canonical pattern for safely sizing a `Vec` from attacker-influenced binary input (file format header counts, wire payload lengths, etc.) lives in `crates/navmesh-extractor/src/nav_roundtrip.rs`:

```rust
fn check_count(value: u32, max: u32, field: &'static str) -> Result<u32, ExtractError> {
    if value > max {
        return Err(ExtractError::NavHeaderOutOfRange {
            field, value: value as u64, reason: "exceeds extractor sanity cap",
        });
    }
    Ok(value)
}

fn checked_alloc_size(count: u32, stride: u32, field: &'static str) -> Result<usize, ExtractError> {
    let product = (count as u64).checked_mul(stride as u64)
        .ok_or(ExtractError::NavHeaderOutOfRange { field, value: count as u64, reason: "count * stride overflows u64" })?;
    usize::try_from(product).map_err(|_| ExtractError::NavHeaderOutOfRange { field, value: product, reason: "count * stride does not fit in usize on this target" })
}
```

**Why this is the right shape:**

1. Two-stage validation: per-field sanity cap (`check_count`) PLUS overflow-safe multiplication (`checked_alloc_size`). The cap alone isn't enough — even with `MAX_NVERTS = 1M`, you still need to defend `1M * 64` from the next field's stride.
2. `u32 * u32` arithmetic done as `u64` so no intermediate truncation hides the overflow.
3. `usize::try_from` catches 32-bit-host edge cases where `u64 → usize` would truncate.
4. Typed error variant with `field: &'static str` carries the violating header name to the log/diagnostic.

**Where to apply this pattern next:**

- `crates/entity/src/navigation.rs::NavMesh::load` — same XRC nav format, runtime path, currently UNGUARDED. The fix would be a direct port of the helpers above.
- Anywhere else in the codebase where a Vec is sized from a (count, stride) pair read off the wire or off disk.

**Regression-guard template** (also from PR #426, `read_rejects_oversized_*`): synthesise a header with `value = 0xFFFFFFFF`, pad zeros for the rest of the header so the cap fires before any subsequent read, then `assert_eq!(field, "<field_name>")` on the typed error variant. Failure mode if the bounds check is reverted: the multiplication wraps to a small `Vec` and the read loop consumes only a few elements while the claimed huge region goes unread — leaving every downstream offset wrong. Test must fail when the cap is removed.
