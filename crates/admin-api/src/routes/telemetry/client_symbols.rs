//! SGW.exe symbol names for native addresses in client telemetry (NT-40).
//!
//! The injected DLL reports raw addresses (`address = "0x01576f90"` on a hook
//! install or an OS exception). The ingest names them from
//! `client_symbols.tsv`, a committed table of function entry points seeded
//! from the DLL's own hook table and the addresses documented in
//! `docs/reverse-engineering/` and `docs/protocol/`. Only an exact entry
//! point is named: an address inside a function stays unnamed rather than
//! guessed from the nearest symbol below it, since the table lists a small
//! fraction of the binary's functions.

use std::collections::HashMap;
use std::sync::LazyLock;

/// The table, compiled in so the server and the container image always
/// carry the same one.
const TABLE: &str = include_str!("client_symbols.tsv");

static SYMBOLS: LazyLock<HashMap<u32, &'static str>> = LazyLock::new(|| parse(TABLE));

/// Rows of `src`: `address<TAB>name<TAB>source`, `#` comments and blank
/// lines skipped. A malformed row is skipped too; the table's own test
/// keeps the committed file clean.
fn parse(src: &'static str) -> HashMap<u32, &'static str> {
    src.lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.starts_with('#') {
                return None;
            }
            let mut cols = line.split('\t');
            let address = parse_address(cols.next()?)?;
            let name = cols.next().filter(|n| !n.trim().is_empty())?;
            Some((address, name.trim()))
        })
        .collect()
}

/// `0x01576f90` (any case, with or without `0x`) as a 32-bit address.
fn parse_address(text: &str) -> Option<u32> {
    let text = text.trim();
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u32::from_str_radix(hex, 16).ok()
}

/// Load the table now rather than on the first upload, and return how many
/// symbols it holds, for the boot log.
pub fn load_client_symbols() -> usize {
    SYMBOLS.len()
}

/// The function whose entry point is `address`, given as the DLL writes it
/// (`"0x01576f90"`). `None` for an unknown or unparsable address.
pub(super) fn symbol_name(address: &str) -> Option<&'static str> {
    SYMBOLS.get(&parse_address(address)?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_table_parses_every_row_once() {
        let mut seen = HashMap::new();
        for (n, line) in TABLE.lines().enumerate() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            assert_eq!(cols.len(), 3, "line {}: want address, name, source", n + 1);
            let address = parse_address(cols[0])
                .unwrap_or_else(|| panic!("line {}: bad address {:?}", n + 1, cols[0]));
            assert!(!cols[1].trim().is_empty(), "line {}: empty name", n + 1);
            assert!(!cols[2].trim().is_empty(), "line {}: empty source", n + 1);
            if let Some(first) = seen.insert(address, n + 1) {
                panic!(
                    "line {}: address {address:#010x} already on line {first}",
                    n + 1
                );
            }
        }
        assert_eq!(seen.len(), load_client_symbols());
    }

    #[test]
    fn names_documented_entry_points() {
        assert_eq!(symbol_name("0x01576f90"), Some("Mercury::Channel::send"));
        assert_eq!(symbol_name("0x013A96E0"), Some("curl_easy_setopt"));
    }

    #[test]
    fn unknown_or_malformed_address_is_unnamed() {
        // One byte past Channel::send's entry: inside the function, not named.
        assert_eq!(symbol_name("0x01576f91"), None);
        assert_eq!(symbol_name("not an address"), None);
        assert_eq!(symbol_name(""), None);
    }
}
