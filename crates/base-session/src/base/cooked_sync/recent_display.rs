//! The last dialog each client was shown, for correlating a refused
//! cooked-data miss with the display that asked for it.
//!
//! A dialog the client was just shown and then cannot resolve (its cooked
//! entry is refused) shows nothing; the refusal row alone reads as noise.
//! The base notes each `onDialogDisplay` it forwards, and the miss path asks
//! how long ago the dialog it refused was displayed to that client.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// `onDialogDisplay` on SGWPlayer, flat method index 105 (the table in
/// `docs/protocol/client-method-dispatch-table.md`).
pub const ON_DIALOG_DISPLAY_INDEX: u16 = 105;

/// The cooked-data category of dialogs (`CookedDataDialogs.pak`).
pub const DIALOGS_CATEGORY: u32 = 5;

/// Where the dialog id sits in the `onDialogDisplay` args: after the
/// little-endian `EntityId` (`send_dialog_display`, `args[0..4]`).
const DIALOG_ID_OFFSET: usize = 4;

/// The most clients whose last display is kept. Past it the map is cleared
/// on insert: a correlation that is lost only costs one missing row.
const MAX_TRACKED: usize = 4096;

static LAST: LazyLock<Mutex<HashMap<SocketAddr, (u32, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The dialog id in the args of an `onDialogDisplay`, or `None` when the
/// args are too short to hold it.
pub fn dialog_id_from_args(args: &[u8]) -> Option<u32> {
    let bytes: [u8; 4] = args
        .get(DIALOG_ID_OFFSET..DIALOG_ID_OFFSET + 4)?
        .try_into()
        .ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// Record that `dialog_id` was just displayed to the client at `addr`.
pub fn note_dialog_display(addr: SocketAddr, dialog_id: u32, now: Instant) {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.len() > MAX_TRACKED {
        last.clear();
    }
    last.insert(addr, (dialog_id, now));
}

/// Milliseconds since `dialog_id` was last displayed to `addr`, when that
/// display is the most recent one to `addr`; `None` otherwise.
pub fn display_age_ms(addr: SocketAddr, dialog_id: u32, now: Instant) -> Option<u64> {
    let last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let &(shown, at) = last.get(&addr)?;
    if shown != dialog_id {
        return None;
    }
    now.saturating_duration_since(at)
        .as_millis()
        .try_into()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    #[test]
    fn the_same_dialog_gives_its_age() {
        let (a, now) = (addr(47_701), Instant::now());
        note_dialog_display(a, 2576, now);
        let later = now + Duration::from_millis(250);
        assert_eq!(display_age_ms(a, 2576, later), Some(250));
    }

    #[test]
    fn another_dialog_gives_none() {
        let (a, now) = (addr(47_702), Instant::now());
        note_dialog_display(a, 2576, now);
        assert_eq!(display_age_ms(a, 2574, now), None);
    }

    #[test]
    fn an_unknown_client_gives_none() {
        let now = Instant::now();
        assert_eq!(display_age_ms(addr(47_703), 2576, now), None);
    }

    #[test]
    fn the_latest_display_replaces_the_earlier_one() {
        let (a, now) = (addr(47_704), Instant::now());
        note_dialog_display(a, 2574, now);
        note_dialog_display(a, 2576, now);
        assert_eq!(display_age_ms(a, 2574, now), None);
        assert_eq!(display_age_ms(a, 2576, now), Some(0));
    }

    #[test]
    fn the_dialog_id_is_read_after_the_entity_id() {
        let mut args = 100i32.to_le_bytes().to_vec();
        args.extend_from_slice(&2576i32.to_le_bytes());
        args.extend_from_slice(&[0; 8]);
        assert_eq!(dialog_id_from_args(&args), Some(2576));
    }

    #[test]
    fn short_args_have_no_dialog_id() {
        assert_eq!(dialog_id_from_args(&[0; 7]), None);
        assert_eq!(dialog_id_from_args(&[]), None);
    }
}
