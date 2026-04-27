# Pre-Commit Checklist for CodeRabbit

This checklist addresses the most common feedback patterns from CodeRabbit reviews. Following these guidelines will eliminate ~70% of review comments before they're generated.

## Quick Reference

| Category | Frequency | Priority |
|----------|-----------|----------|
| DB Error Handling | ~19% | 🔴 High |
| Magic Numbers | ~15% | 🔴 High |
| Race Conditions | ~13% | 🔴 High |
| Unchecked Mutations | ~10% | 🔴 High |
| Import Path Errors | ~8% | 🟠 Medium |
| Unused Code | ~7% | 🟠 Medium |
| Integer Overflow | ~5% | 🟠 Medium |
| Player ID Resolution | ~4% | 🟡 Low |
| Style Inconsistencies | ~4% | 🟡 Low |

---

## The Big 6 (Catches ~70% of Feedback)

### 1. DB Error Handling (~19%)

**Problem:** Using `.ok().flatten()` or `.unwrap_or_default()` on database queries silently converts errors into "not found" results, masking actual database failures.

**Fix:** Use explicit `match` statements on all database queries. Log errors with `tracing::error!` and handle the `Ok(None)` case separately from `Err`.

**Search for:** `.ok().flatten()` and `.unwrap_or_default()` anywhere near `query`, `fetch`, or database operations.

---

### 2. Magic Numbers (~15%)

**Problem:** Hardcoded numeric literals in method calls, container IDs, faction IDs, and method indices make code unreadable and error-prone.

**Fix:** Define named constants for all numeric values that have semantic meaning. Group related constants in a `constants.rs` module or at the top of the relevant file.

**Search for:** Numeric literals in struct fields like `method_index:`, `container_id:`, `faction:`, etc.

---

### 3. Race Conditions (~13%)

**Problem:** Reading a value and then writing based on that value without a transaction allows concurrent requests to corrupt data.

**Fix:** Wrap read-then-write sequences in a database transaction using `pool.begin()`. Use `FOR UPDATE` in SELECT queries to lock rows during the transaction.

**Search for:** SELECT queries followed by UPDATE/INSERT queries that aren't wrapped in a transaction block.

---

### 4. Unchecked Mutations (~10%)

**Problem:** Executing UPDATE/DELETE queries without checking `rows_affected()` means silent failures when the expected row doesn't exist.

**Fix:** After executing a mutation, verify `rows_affected() == 1` (or expected count). Rollback the transaction and log a warning if the count is unexpected.

**Search for:** `.execute()` calls that don't have a corresponding `rows_affected()` check.

---

### 5. Import Path Errors (~8%)

**Problem:** Wrong number of `super::` in import paths after refactoring causes compilation failures.

**Fix:** Count the actual module nesting depth and match the number of `super::` segments. After moving files, verify all imports resolve correctly.

**Search for:** `super::super::super::` chains — verify each one matches the actual file structure.

---

### 6. Unused Code (~7%)

**Problem:** Dead imports, unused variables, and unreachable code clutter the codebase.

**Fix:** Run Clippy before committing. It catches unused imports, dead code, and many other common issues.

---

## What is Clippy?

**Clippy** is Rust's official linter — a static analysis tool that catches common mistakes, style issues, and potential bugs before they become problems.

### Running Clippy

```bash
# Basic run
cargo clippy

# With specific warnings enabled
cargo clippy -- -W unused_imports -W dead_code

# Treat warnings as errors (for CI)
cargo clippy -- -D warnings
```

### What Clippy Catches

- Unused imports and variables
- Dead/unreachable code
- Redundant clones
- Inefficient patterns (e.g., `.iter().map().collect()` when `.iter().copied()` works)
- Common mistakes (e.g., comparing floats with `==`)
- Style violations

### IDE Integration

Most Rust IDE setups (VS Code with rust-analyzer, IntelliJ Rust) run Clippy automatically and show warnings inline.

---

## Pre-Commit Grep Commands

Quick searches to run before committing:

| What to Find | Search Pattern |
|--------------|----------------|
| Masked DB errors | `.ok().flatten()` in Rust files |
| Masked DB errors | `.unwrap_or_default()` near query/fetch calls |
| Magic numbers | `method_index:` followed by a digit |
| Unchecked mutations | `.execute(` without nearby `rows_affected` |
| Deep super chains | `super::super::super::` |

---

## Secondary Checks (~30% of Feedback)

### Integer Overflow in SQL (~5%)

When multiplying values in SQL queries, cast to BIGINT before the operation to prevent overflow on large values.

### Player ID Resolution (~4%)

Don't re-query player_id by account_id when you already have it. Cache player_id on the connection state and pass it through.

### UNIMPLEMENTED Stubs (~3%)

Stub functions that return `true` or silently succeed are dangerous. Either forward to the real implementation or return `false` / log a warning.

### Test Assertions (~3%)

When changing message enum variants or function signatures, update the corresponding test assertions.

### Duplicate Definitions (~2%)

If the same SQL query or struct appears in multiple files, extract it to a shared module.

---

## Summary

**Before every commit:**

1. ✅ Search for `.ok().flatten()` — replace with explicit match
2. ✅ Search for hardcoded numbers — extract to constants
3. ✅ Check read-then-write patterns — wrap in transactions
4. ✅ Verify `.execute()` calls — check `rows_affected()`
5. ✅ Run `cargo clippy` — fix all warnings
6. ✅ Verify import paths — count your `super::`s

This catches the majority of CodeRabbit feedback before it's generated.
