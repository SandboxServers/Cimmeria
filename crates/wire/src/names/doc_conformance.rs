//! The name tables against `docs/protocol/*-dispatch-table.md`, row by row.
//!
//! Every table row whose first column is an index (or a message id) and that
//! has a method-name column must name what [`super`] resolves at that index,
//! and every index the tables hold must have a row. A disagreement fails with
//! the doc's path and line, so the losing side (doc or `.def`-generated
//! table) is one click away.

use std::collections::BTreeSet;

use super::defs;

use super::{
    base_method, client_method, client_msg_name, player_base_method, player_cell_method,
    server_msg_name, ACCOUNT_CLASS_ID, SGWGMPLAYER_CLASS_ID, SGWMOB_CLASS_ID, SGWPET_CLASS_ID,
    SGWPLAYER_CLASS_ID,
};

/// SGWBeing, the shared prefix SGWMob extends.
const SGWBEING_CLASS_ID: u8 = 0x01;

const CLIENT_DOC: &str = "client-method-dispatch-table.md";
const CELL_DOC: &str = "cell-method-dispatch-table.md";
const BASE_DOC: &str = "sgwplayer-base-method-dispatch-table.md";
const MESSAGE_DOC: &str = "message-dispatch-table.md";

/// One data row of a markdown table, with its header and the headings above.
#[derive(Debug)]
struct Row {
    line: usize,
    h2: String,
    header: Vec<String>,
    cells: Vec<String>,
}

impl Row {
    /// The cell under the first header that `pick` accepts.
    fn col(&self, pick: impl Fn(&str) -> bool) -> Option<&str> {
        let at = self.header.iter().position(|h| pick(h))?;
        self.cells.get(at).map(String::as_str)
    }
}

fn split_cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// Every table row in `docs/protocol/<name>`.
fn rows(name: &str) -> Vec<Row> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/protocol/").to_string() + name;
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut h2 = String::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(h) = line.strip_prefix("## ") {
            h2 = h.trim().to_string();
        }
        let is_table = line.trim_start().starts_with('|')
            && lines
                .get(i + 1)
                .is_some_and(|next| next.trim_start().starts_with("|-"));
        if !is_table {
            i += 1;
            continue;
        }
        let header = split_cells(line);
        i += 2;
        while let Some(row) = lines.get(i).filter(|l| l.trim_start().starts_with('|')) {
            out.push(Row {
                line: i + 1,
                h2: h2.clone(),
                header: header.clone(),
                cells: split_cells(row),
            });
            i += 1;
        }
    }
    out
}

/// The leading identifier of a method cell: `` `gmMissionAssign(WSTRING …)` ``
/// is `gmMissionAssign`.
fn ident(cell: &str) -> Option<&str> {
    let cell = cell.trim_start_matches(['`', '*']);
    let end = cell
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(cell.len());
    (end > 0).then(|| &cell[..end])
}

/// A generated table's length as an index bound.
fn len(table: &[&str]) -> u16 {
    u16::try_from(table.len()).expect("method tables are far below u16::MAX")
}

fn decimal(cell: &str) -> Option<u16> {
    cell.trim_matches('*').parse().ok()
}

fn hex(cell: &str) -> Option<u16> {
    u16::from_str_radix(cell.trim_matches('`').strip_prefix("0x")?, 16).ok()
}

/// Collects mismatches and the indices the doc covers, per table.
#[derive(Default)]
struct Check {
    errors: Vec<String>,
}

impl Check {
    fn expect(
        &mut self,
        doc: &str,
        row: &Row,
        what: &str,
        index: u16,
        doc_name: &str,
        table: Option<&str>,
    ) {
        if table != Some(doc_name) {
            self.errors.push(format!(
                "docs/protocol/{doc}:{}: {what} {index} is `{doc_name}` in the doc, {table:?} in crate::names",
                row.line
            ));
        }
    }

    /// The index and method of a row under an index header. In a table with
    /// a method column, a row whose index parses but whose method cell does
    /// not is an error: a typo must not drop a row out of the check.
    fn indexed_row<'r>(&mut self, doc: &str, row: &'r Row) -> Option<(u16, &'r str)> {
        let index = row.cells.first().and_then(|c| decimal(c))?;
        // A table with no method column (the crafting payload sizes) is not
        // a method table.
        let cell = row.col(is_method_header)?;
        // `*(see the SGWPlayer table above)*`: a pointer row, not a method.
        if cell.starts_with("*(") {
            return None;
        }
        match ident(cell) {
            Some(name) => Some((index, name)),
            None => {
                self.errors.push(format!(
                    "docs/protocol/{doc}:{}: index {index} has no readable method name",
                    row.line
                ));
                None
            }
        }
    }

    fn covers(&mut self, doc: &str, what: &str, seen: &BTreeSet<u16>, want: std::ops::Range<u16>) {
        let missing: Vec<u16> = want.filter(|i| !seen.contains(i)).collect();
        if !missing.is_empty() {
            self.errors.push(format!(
                "docs/protocol/{doc}: no row for {what} {missing:?}"
            ));
        }
    }

    fn finish(self) {
        assert!(
            self.errors.is_empty(),
            "dispatch-table docs and crate::names disagree:\n  {}",
            self.errors.join("\n  ")
        );
    }
}

fn is_index_header(h: &str) -> bool {
    h == "Index" || h == "Idx"
}

fn is_method_header(h: &str) -> bool {
    h.starts_with("Method")
}

/// SGWPlayer (0-156), the SGWGmPlayer tail (157-162), SGWMob (27-28) and
/// SGWPet (29-31) ClientMethods.
#[test]
fn client_method_doc_rows_match_the_tables() {
    let mut check = Check::default();
    let mut seen = [
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::new(),
    ];
    for row in rows(CLIENT_DOC) {
        if !row.header.first().is_some_and(|h| is_index_header(h)) {
            continue;
        }
        let Some((index, name)) = check.indexed_row(CLIENT_DOC, &row) else {
            continue;
        };
        let (slot, class_id, what) = if row.h2.starts_with("SGWGmPlayer") {
            (3, SGWGMPLAYER_CLASS_ID, "SGWGmPlayer ClientMethod")
        } else if row.h2.starts_with("SGWMob") {
            (1, SGWMOB_CLASS_ID, "SGWMob ClientMethod")
        } else if row.h2.starts_with("SGWPet") {
            (2, SGWPET_CLASS_ID, "SGWPet ClientMethod")
        } else {
            (0, SGWPLAYER_CLASS_ID, "SGWPlayer ClientMethod")
        };
        seen[slot].insert(index);
        check.expect(
            CLIENT_DOC,
            &row,
            what,
            index,
            name,
            client_method(class_id, index),
        );
    }
    // Each table's own range, from the generated tables: SGWPlayer's whole
    // table; SGWMob, SGWPet and SGWGmPlayer past the table they extend.
    let being = len(defs::client_methods(SGWBEING_CLASS_ID));
    let player = len(defs::client_methods(SGWPLAYER_CLASS_ID));
    let mob = len(defs::client_methods(SGWMOB_CLASS_ID));
    let pet = len(defs::client_methods(SGWPET_CLASS_ID));
    let gm = len(defs::client_methods(SGWGMPLAYER_CLASS_ID));
    check.covers(CLIENT_DOC, "SGWPlayer ClientMethod", &seen[0], 0..player);
    check.covers(CLIENT_DOC, "SGWMob ClientMethod", &seen[1], being..mob);
    check.covers(CLIENT_DOC, "SGWPet ClientMethod", &seen[2], mob..pet);
    check.covers(CLIENT_DOC, "SGWGmPlayer ClientMethod", &seen[3], player..gm);
    check.finish();
}

/// The `Wire` column of a cell-method row: direct `0xNN` below idbase 61,
/// `0xBD+k` from it.
fn cell_wire_index(cell: &str) -> Option<u16> {
    match cell.split_once('+') {
        Some((marker, sub)) if hex(marker) == Some(0xBD) => Some(61 + sub.parse::<u16>().ok()?),
        _ => hex(cell)?.checked_sub(0x80),
    }
}

/// SGWPlayer exposed CellMethods (0-108) and the SGWGmPlayer tail (109-225).
#[test]
fn cell_method_doc_rows_match_the_tables() {
    let mut check = Check::default();
    let mut seen = BTreeSet::new();
    for row in rows(CELL_DOC) {
        if !row.header.first().is_some_and(|h| is_index_header(h)) {
            continue;
        }
        let Some((index, name)) = check.indexed_row(CELL_DOC, &row) else {
            continue;
        };
        seen.insert(index);
        check.expect(
            CELL_DOC,
            &row,
            "CellMethod",
            index,
            name,
            player_cell_method(index),
        );
        if let Some(wire) = row.col(|h| h == "Wire") {
            if cell_wire_index(wire) != Some(index) {
                check.errors.push(format!(
                    "docs/protocol/{CELL_DOC}:{}: CellMethod {index} has wire `{wire}`",
                    row.line
                ));
            }
        }
    }
    let gm = len(defs::cell_methods(SGWGMPLAYER_CLASS_ID));
    check.covers(CELL_DOC, "CellMethod", &seen, 0..gm);
    check.finish();
}

/// SGWPlayer exposed BaseMethods (0-29), sent as `0xC0 | index`.
#[test]
fn base_method_doc_rows_match_the_table() {
    let mut check = Check::default();
    let mut seen = BTreeSet::new();
    for row in rows(BASE_DOC) {
        if !row.header.first().is_some_and(|h| is_index_header(h)) {
            continue;
        }
        let Some((index, name)) = check.indexed_row(BASE_DOC, &row) else {
            continue;
        };
        seen.insert(index);
        check.expect(
            BASE_DOC,
            &row,
            "BaseMethod",
            index,
            name,
            player_base_method(index),
        );
        if let Some(wire) = row.col(|h| h == "Wire") {
            if hex(wire) != Some(0xC0 + index) {
                check.errors.push(format!(
                    "docs/protocol/{BASE_DOC}:{}: BaseMethod {index} has wire `{wire}`",
                    row.line
                ));
            }
        }
    }
    let player = len(defs::base_methods(SGWPLAYER_CLASS_ID));
    check.covers(BASE_DOC, "BaseMethod", &seen, 0..player);
    check.finish();
}

/// Both Mercury interface tables, and the Account entity's methods.
#[test]
fn message_doc_rows_match_the_tables() {
    let mut check = Check::default();
    let (mut to_client, mut to_server) = (BTreeSet::new(), BTreeSet::new());
    let (mut account_base, mut account_client) = (BTreeSet::new(), BTreeSet::new());
    for row in rows(MESSAGE_DOC) {
        let first = row.header.first().map(String::as_str);
        if first == Some("Msg ID") {
            let (Some(id), Some(name)) = (
                row.cells.first().and_then(|c| hex(c)),
                row.col(|h| h == "Name").and_then(ident),
            ) else {
                continue;
            };
            // Message ids are bytes; the hex parser is wider.
            let byte = u8::try_from(id).expect("message id fits a byte");
            if row.h2.starts_with("Server-to-Client") {
                to_client.insert(id);
                check.expect(
                    MESSAGE_DOC,
                    &row,
                    "server-to-client message",
                    id,
                    name,
                    client_msg_name(byte),
                );
            } else if row.h2.starts_with("Client-to-Server") {
                to_server.insert(id);
                check.expect(
                    MESSAGE_DOC,
                    &row,
                    "client-to-server message",
                    id,
                    name,
                    server_msg_name(byte),
                );
            }
        } else if first == Some("Wire ID") {
            let (Some(wire), Some(index), Some(name), Some(dir)) = (
                row.cells.first().and_then(|c| hex(c)),
                row.col(|h| h == "Method Index").and_then(decimal),
                row.col(|h| h == "Method Name").and_then(ident),
                row.col(|h| h == "Direction"),
            ) else {
                continue;
            };
            let (base, table, what, seen) = if dir == "C->S" {
                (
                    0xC0,
                    base_method(ACCOUNT_CLASS_ID, index),
                    "Account BaseMethod",
                    &mut account_base,
                )
            } else {
                (
                    0x80,
                    client_method(ACCOUNT_CLASS_ID, index),
                    "Account ClientMethod",
                    &mut account_client,
                )
            };
            seen.insert(index);
            check.expect(MESSAGE_DOC, &row, what, index, name, table);
            if wire != base + index {
                check.errors.push(format!(
                    "docs/protocol/{MESSAGE_DOC}:{}: {what} {index} has wire {wire:#04x}",
                    row.line
                ));
            }
        }
    }
    check.covers(MESSAGE_DOC, "server-to-client message", &to_client, 0..0x39);
    check.covers(MESSAGE_DOC, "client-to-server message", &to_server, 0..0x0D);
    let base = len(defs::base_methods(ACCOUNT_CLASS_ID));
    let client = len(defs::client_methods(ACCOUNT_CLASS_ID));
    check.covers(MESSAGE_DOC, "Account BaseMethod", &account_base, 0..base);
    check.covers(
        MESSAGE_DOC,
        "Account ClientMethod",
        &account_client,
        0..client,
    );
    check.finish();
}

/// The parser itself: a mismatching row is reported with its line, so a
/// broken doc cannot pass by being skipped.
#[test]
fn a_wrong_doc_row_is_reported_with_its_line() {
    let row = Row {
        line: 42,
        h2: String::new(),
        header: vec!["Index".into(), "Method".into()],
        cells: vec!["1".into(), "`notOnSequence`".into()],
    };
    let mut check = Check::default();
    let name = row.col(is_method_header).and_then(ident).unwrap();
    check.expect(
        CLIENT_DOC,
        &row,
        "SGWPlayer ClientMethod",
        1,
        name,
        client_method(SGWPLAYER_CLASS_ID, 1),
    );
    assert_eq!(check.errors.len(), 1);
    assert!(check.errors[0].contains(":42:"), "{}", check.errors[0]);
    assert_eq!(
        ident("`gmMissionAssign(WSTRING DesignID)`"),
        Some("gmMissionAssign")
    );
    assert_eq!(cell_wire_index("0xBD+7"), Some(68));
    assert_eq!(cell_wire_index("0xA3"), Some(35));
}
