# Revert-verifying DB-bound code without a live DB

When fixing a bug in an async DB-touching function (slot reservation, balance
checks, etc.) and the user can't run live Postgres locally, the
revert-verification problem is: the live-DB test is the canonical guard but
auto-skips locally, so you can't observe a revert failing.

The pattern that works:

1. **Split the production async function into a pure sync helper + thin async
   shell.** The sync helper takes the raw DB data as a slice and returns the
   algorithmic answer. The async shell does the DB query, calls the helper,
   maps the result to the production error type.
2. **Unit-test the pure helper.** Since the production async function delegates
   to it, a revert in the helper trips both the unit test (locally) and the
   live-DB test (in CI's `ci-live-db` profile).
3. **Keep the live-DB test as well.** It's the integration coverage; the unit
   test is the local revert-verifier. Document both layers in the test commit.

Example shape (from trade slot reservation):

```rust
// Async shell — production entry point
async fn reserve_main_slots_excluding(tx, recipient_id, needed, vacating) -> Result<Vec<i32>> {
    let raw = sqlx::query_as::<...>("SELECT slot_id FROM ... FOR UPDATE")
        .fetch_all(&mut **tx).await?;
    match pick_free_main_slots_excluding(&raw, vacating, needed) {
        Some(slots) => Ok(slots),
        None => Err(TradeAbort::NotEnoughSlots { ... }),
    }
}

// Pure sync helper — unit-testable
fn pick_free_main_slots_excluding(raw_occupied: &[i32], vacating: &[i32], needed: usize) -> Option<Vec<i32>> {
    let excluding: HashSet<i32> = vacating.iter().copied().collect();
    let after: Vec<i32> = raw_occupied.iter().copied().filter(|s| !excluding.contains(s)).collect();
    free_inventory_slots(min, max, &after, needed)
}
```

Tip: write the test to demonstrate BOTH the pre-fix and post-fix shape side-by-side. Call the helper with `vacating: &[]` (pre-fix mimic) AND with `vacating: &[39]` (post-fix). Assert `None` for the former and `Some(_)` for the latter. The single test then "shows its work" — any reviewer can see the discriminator.

Revert-verification protocol: temporarily revert the fix (drop the `.filter()`, or comment out the exclusion line), `cargo test -p cimmeria-services --lib trade::execute::slot_exclusion_accounting`, confirm the discriminating tests fail with the expected error. Restore the fix.
