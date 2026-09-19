# sqlx 0.9 rejects `&String` queries — share SQL with `concat!`, not `format!`

`sqlx::query()` takes `impl SqlSafeStr`, and **`SqlSafeStr` is implemented for
`&'static str` only** — not for `&str` in general, despite what the compiler's
"consider dereferencing here" hint suggests. Passing a `String` (or `&String`)
fails to compile with:

```
error[E0277]: dynamic SQL strings should be audited for possible injections
   = help: the trait `SqlSafeStr` is not implemented for `&std::string::String`
```

This bites the obvious way to dedupe a SELECT shared by two call sites that
differ only in their `WHERE`: a `fn entity_template_query(where_clause: &str)
-> String` does not compile. `AssertSqlSafe()` exists to bypass the check but
is the wrong tool when nothing dynamic is involved.

The shape that works — a `macro_rules!` with `concat!`, so the composed query
stays a compile-time literal:

```rust
macro_rules! entity_template_select {
    ($tail:literal) => {
        concat!("SELECT ... FROM resources.entity_templates t", $tail)
    };
}
pub(crate) use entity_template_select;   // path-addressable, order-independent
```

`pub(crate) use <macro_name>;` is what makes it reachable from another module
(`crate::cell::spawner::entity_template_select`) without `#[macro_export]`
dumping it at the crate root. Callers pass `""` or `" WHERE t.template_id = $1"`;
the bind parameter is still bound normally.

Live example: `crates/services/src/cell/spawner/templates.rs`, shared with
`crates/services/src/base/gm_spawn.rs` (PR #662 review, finding 3).
