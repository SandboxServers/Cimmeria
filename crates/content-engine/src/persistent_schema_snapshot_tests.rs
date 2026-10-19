//! Snapshot guard for the persistent player-data tables (PP-11, D-PP7).
//!
//! `db/persistent/persistent-schema.snapshot.json` records the columns,
//! constraints and index names of every table in `persistent-public-tables.txt`,
//! as `tools/persistent-schema-snapshot.ps1` writes them from a live database.
//! The live-DB test reads the live schema with the same catalog queries and
//! calls [`diff`]. A snapshot column, constraint or index that is dropped, or
//! whose type or nullability changed, is a violation. So is a live addition the
//! snapshot does not have: regenerate the snapshot and commit it (additive
//! changes only, D-PP7).
//!
//! [`diff`] is a pure function over two [`Snapshot`] values, so the unit tests
//! need no database.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Appended to every addition so the fix is in the failure output.
const REGENERATE: &str =
    "run tools/persistent-schema-snapshot.ps1 and commit; additive only (D-PP7)";

/// The whole persistent schema: one entry per listed table.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
struct Snapshot {
    tables: BTreeMap<String, TableShape>,
}

/// The shape of one table: its columns, constraints and index names.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
struct TableShape {
    columns: Vec<ColumnShape>,
    constraints: Vec<ConstraintShape>,
    indexes: Vec<String>,
}

/// One column from `information_schema.columns`. `is_nullable` is `YES` or `NO`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ColumnShape {
    name: String,
    data_type: String,
    is_nullable: String,
    column_default: String,
}

/// One constraint from `pg_constraint`; `kind` is its `contype` (`p`, `u`, `f`, `c`, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ConstraintShape {
    name: String,
    #[serde(rename = "type")]
    kind: String,
}

/// Compare the live schema with the snapshot. Returns one message per
/// violation, in table order.
fn diff(snapshot: &Snapshot, live: &Snapshot) -> Vec<String> {
    let mut out = Vec::new();
    for (table, want) in &snapshot.tables {
        let Some(have) = live.tables.get(table) else {
            out.push(format!("{table}: table is missing from the live schema"));
            continue;
        };
        for col in &want.columns {
            match have.columns.iter().find(|c| c.name == col.name) {
                None => out.push(format!("{table}.{}: column is missing", col.name)),
                Some(now) => {
                    if now.data_type != col.data_type {
                        out.push(format!(
                            "{table}.{}: type changed {} -> {}",
                            col.name, col.data_type, now.data_type
                        ));
                    }
                    if now.is_nullable != col.is_nullable {
                        out.push(format!(
                            "{table}.{}: nullability changed {} -> {}",
                            col.name, col.is_nullable, now.is_nullable
                        ));
                    }
                }
            }
        }
        for con in &want.constraints {
            match have.constraints.iter().find(|c| c.name == con.name) {
                None => out.push(format!("{table}: constraint {} is missing", con.name)),
                Some(now) if now.kind != con.kind => out.push(format!(
                    "{table}: constraint {} changed type {} -> {}",
                    con.name, con.kind, now.kind
                )),
                Some(_) => {}
            }
        }
        for index in &want.indexes {
            if !have.indexes.contains(index) {
                out.push(format!("{table}: index {index} is missing"));
            }
        }
        for col in &have.columns {
            if !want.columns.iter().any(|c| c.name == col.name) {
                out.push(format!(
                    "{table}.{}: column is not in the snapshot; {REGENERATE}",
                    col.name
                ));
            }
        }
        for con in &have.constraints {
            if !want.constraints.iter().any(|c| c.name == con.name) {
                out.push(format!(
                    "{table}: constraint {} is not in the snapshot; {REGENERATE}",
                    con.name
                ));
            }
        }
        for index in &have.indexes {
            if !want.indexes.contains(index) {
                out.push(format!(
                    "{table}: index {index} is not in the snapshot; {REGENERATE}"
                ));
            }
        }
    }
    for table in live.tables.keys() {
        if !snapshot.tables.contains_key(table) {
            out.push(format!(
                "{table}: table is not in the snapshot; {REGENERATE}"
            ));
        }
    }
    out
}

/// The repository root: two hops up from the test crate's `CARGO_MANIFEST_DIR`.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// The table names in `persistent-public-tables.txt`: one per line, `#` starts a comment.
fn persistent_tables() -> Vec<String> {
    let path = workspace_root().join("db/persistent/persistent-public-tables.txt");
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let name = line.split('#').next().unwrap_or("").trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

/// Read the committed snapshot from `db/persistent/persistent-schema.snapshot.json`.
fn load_snapshot() -> Snapshot {
    let path = workspace_root().join("db/persistent/persistent-schema.snapshot.json");
    let text = fs::read_to_string(path).unwrap();
    serde_json::from_str(&text).expect("persistent-schema.snapshot.json parses")
}

/// Read the live schema of `tables` with the same catalog queries as
/// `tools/persistent-schema-snapshot.ps1`; keep the two in step.
async fn load_live(pool: &PgPool, tables: &[String]) -> Snapshot {
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        r#"SELECT * FROM (
            SELECT 'column', c.table_name::text, c.column_name::text, c.data_type::text,
                   c.is_nullable::text, COALESCE(c.column_default, '')::text
            FROM information_schema.columns c
            WHERE c.table_schema = 'public' AND c.table_name = ANY($1)
            UNION ALL
            SELECT 'constraint', rel.relname::text, con.conname::text, con.contype::text, '', ''
            FROM pg_constraint con
            JOIN pg_class rel ON rel.oid = con.conrelid
            JOIN pg_namespace n ON n.oid = rel.relnamespace
            WHERE n.nspname = 'public' AND rel.relname = ANY($1)
            UNION ALL
            SELECT 'index', i.tablename::text, i.indexname::text, '', '', ''
            FROM pg_indexes i
            WHERE i.schemaname = 'public' AND i.tablename = ANY($1)
        ) AS u(kind, tbl, nm, ty, nul, def)
        ORDER BY kind, tbl COLLATE "C", nm COLLATE "C""#,
    )
    .bind(tables.to_vec())
    .fetch_all(pool)
    .await
    .expect("read the live persistent schema");

    let mut live = Snapshot::default();
    for table in tables {
        live.tables.entry(table.clone()).or_default();
    }
    for (kind, table, name, ty, nullable, default) in rows {
        let shape = live.tables.entry(table).or_default();
        match kind.as_str() {
            "column" => shape.columns.push(ColumnShape {
                name,
                data_type: ty,
                is_nullable: nullable,
                column_default: default,
            }),
            "constraint" => shape.constraints.push(ConstraintShape { name, kind: ty }),
            _ => shape.indexes.push(name),
        }
    }
    live
}

#[tokio::test]
async fn live_db_persistent_schema_matches_snapshot() {
    let pool = cimmeria_test_support::require_db_or_skip!();
    let live = load_live(&pool, &persistent_tables()).await;
    let violations = diff(&load_snapshot(), &live);
    assert!(
        violations.is_empty(),
        "persistent schema drift:\n{}",
        violations.join("\n")
    );
}

/// A column shape with a default of `""` for "no default".
fn col(name: &str, data_type: &str, is_nullable: &str) -> ColumnShape {
    ColumnShape {
        name: name.to_string(),
        data_type: data_type.to_string(),
        is_nullable: is_nullable.to_string(),
        column_default: String::new(),
    }
}

/// A snapshot with one `account` table holding `columns`.
fn one_table(columns: Vec<ColumnShape>) -> Snapshot {
    let mut snapshot = Snapshot::default();
    snapshot.tables.insert(
        "account".to_string(),
        TableShape {
            columns,
            constraints: vec![ConstraintShape {
                name: "account_pkey".to_string(),
                kind: "p".to_string(),
            }],
            indexes: vec!["account_pkey".to_string()],
        },
    );
    snapshot
}

#[test]
fn diff_is_empty_when_the_schemas_match() {
    let want = one_table(vec![col("account_id", "integer", "NO")]);
    assert!(diff(&want, &want.clone()).is_empty());
}

#[test]
fn diff_flags_a_dropped_column() {
    let want = one_table(vec![
        col("account_id", "integer", "NO"),
        col("password", "character varying", "YES"),
    ]);
    let have = one_table(vec![col("account_id", "integer", "NO")]);
    let v = diff(&want, &have);
    assert!(
        v.iter().any(|m| m == "account.password: column is missing"),
        "{v:?}"
    );
}

#[test]
fn diff_flags_a_type_change() {
    let want = one_table(vec![col("account_id", "integer", "NO")]);
    let have = one_table(vec![col("account_id", "bigint", "NO")]);
    let v = diff(&want, &have);
    assert!(
        v.iter()
            .any(|m| m == "account.account_id: type changed integer -> bigint"),
        "{v:?}"
    );
}

#[test]
fn diff_flags_nullable_becoming_not_null() {
    let want = one_table(vec![col("password", "character varying", "YES")]);
    let have = one_table(vec![col("password", "character varying", "NO")]);
    let v = diff(&want, &have);
    assert!(
        v.iter()
            .any(|m| m == "account.password: nullability changed YES -> NO"),
        "{v:?}"
    );
}

#[test]
fn diff_flags_a_dropped_constraint_and_index() {
    let want = one_table(vec![col("account_id", "integer", "NO")]);
    let mut have = want.clone();
    let shape = have.tables.get_mut("account").unwrap();
    shape.constraints.clear();
    shape.indexes.clear();
    let v = diff(&want, &have);
    assert!(
        v.iter()
            .any(|m| m == "account: constraint account_pkey is missing"),
        "{v:?}"
    );
    assert!(
        v.iter()
            .any(|m| m == "account: index account_pkey is missing"),
        "{v:?}"
    );
}

#[test]
fn diff_flags_an_addition_with_the_regenerate_message() {
    let want = one_table(vec![col("account_id", "integer", "NO")]);
    let have = one_table(vec![
        col("account_id", "integer", "NO"),
        col("new_flag", "boolean", "YES"),
    ]);
    let v = diff(&want, &have);
    assert!(
        v.iter().any(|m| m.starts_with("account.new_flag: column is not in the snapshot; run tools/persistent-schema-snapshot.ps1")),
        "{v:?}"
    );
    assert!(
        v.iter().all(|m| m.contains("additive only (D-PP7)")),
        "{v:?}"
    );
}

#[test]
fn diff_flags_a_dropped_table() {
    let want = one_table(vec![col("account_id", "integer", "NO")]);
    let v = diff(&want, &Snapshot::default());
    assert_eq!(
        v,
        vec!["account: table is missing from the live schema".to_string()]
    );
}
