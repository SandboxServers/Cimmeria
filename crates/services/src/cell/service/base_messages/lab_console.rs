//! `BaseToCellMsg::LabConsoleExec` handler — run a GM `.`-console line on
//! behalf of an entity and capture its feedback output for the live-research
//! lab MCP endpoint (issue #687), instead of sending it to the player as chat.
//!
//! The in-world `.`-console path (`crate::cell::chat`) routes every command's
//! output through the single-recipient `send_gm_feedback` seam, which pushes a
//! `CellToBaseMsg::EntityMethodCall { method_index: ON_PLAYER_COMMUNICATION }`
//! back to the acting player's client. This handler reuses that path verbatim
//! but *tees* the cell→base channel: it runs the command against a private
//! capture channel, then drains it — decoding the caller-addressed feedback
//! lines into the reply while forwarding every other message (spawn
//! round-trips, teleports, witness fan-out) to the real base channel so the
//! command's side effects still happen. No console handler is touched; the
//! in-world path is unchanged.

use tokio::sync::{mpsc, oneshot};

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::console;
use crate::cell::messages::{CellToBaseMsg, LabConsoleResult};
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;

/// Capacity of the private capture channel. A single console command emits a
/// handful of feedback lines plus at most a couple of side-effect messages;
/// 256 mirrors the real base↔cell channel and leaves generous headroom so a
/// verbose command (`.help`, `.players`) cannot block on a full buffer before
/// we drain it.
const CAPTURE_CAPACITY: usize = 256;

/// Handle `BaseToCellMsg::LabConsoleExec`.
pub(super) async fn handle_lab_console_exec(
    entity_id: u32,
    line: String,
    reply_tx: oneshot::Sender<LabConsoleResult>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // GM access-level gate — identical trust model to the in-world
    // `.`-console path in `crate::cell::chat`: authorization is on the
    // server-side `access_level` (from `account.accesslevel`), never a
    // client-asserted byte. An unknown entity is treated as access level 0.
    let access_level = space_mgr
        .get_entity(entity_id)
        .map_or(0, |e| e.access_level);
    if !console::is_gm(access_level) {
        tracing::warn!(
            entity_id,
            access_level,
            "LabConsoleExec rejected: acting entity is not a GameMaster"
        );
        let _ = reply_tx.send(Err(format!(
            "entity {entity_id} is not authorized (requires GameMaster access level)"
        )));
        return;
    }

    // Capture tee: run against a private channel, then drain it.
    let (cap_tx, mut cap_rx) = mpsc::channel::<CellToBaseMsg>(CAPTURE_CAPACITY);
    console::handle_console_command(entity_id, &line, &cap_tx, space_mgr, engine).await;
    drop(cap_tx);

    let mut lines = Vec::new();
    while let Ok(msg) = cap_rx.try_recv() {
        match msg {
            // Single-recipient GM feedback addressed to the acting entity is
            // captured as output — this is what the in-world path would render
            // as a chat line to the player.
            CellToBaseMsg::EntityMethodCall {
                entity_id: target,
                method_index,
                ref args,
            } if target == entity_id && method_index == ON_PLAYER_COMMUNICATION => {
                if let Some(text) = decode_feedback_text(args) {
                    lines.push(text);
                }
            }
            // Everything else is a genuine side effect (spawn round-trip,
            // teleport, witness fan-out): forward to base so the command's
            // effects happen exactly as the in-world path would produce them.
            other => {
                if tx.send(other).await.is_err() {
                    tracing::warn!(
                        entity_id,
                        "LabConsoleExec: base channel closed while forwarding a side-effect message"
                    );
                    break;
                }
            }
        }
    }

    let _ = reply_tx.send(Ok(lines));
}

/// Decode the `Text` WSTRING out of an `onPlayerCommunication(Speaker,
/// SpeakerFlags, Channel, Text)` arg buffer. Mirrors the serializer in
/// [`crate::cell::cell_methods::gm::feedback`]. Returns `None` on a truncated
/// buffer rather than panicking — a malformed capture must not take the
/// endpoint down.
fn decode_feedback_text(args: &[u8]) -> Option<String> {
    // Speaker WSTRING (u32 char_count + N×2B UTF-16LE).
    let speaker_len = u32::from_le_bytes(args.get(0..4)?.try_into().ok()?) as usize;
    let mut off = 4usize.checked_add(speaker_len.checked_mul(2)?)?;
    // SpeakerFlags (1) + Channel (1).
    off = off.checked_add(2)?;
    // Text WSTRING (u32 char_count + N×2B UTF-16LE).
    let text_len =
        u32::from_le_bytes(args.get(off..off.checked_add(4)?)?.try_into().ok()?) as usize;
    off += 4;
    let end = off.checked_add(text_len.checked_mul(2)?)?;
    let text_bytes = args.get(off..end)?;
    let (chunks, _rest) = text_bytes.as_chunks::<2>();
    let utf16: Vec<u16> = chunks
        .iter()
        .map(|&[a, b]| u16::from_le_bytes([a, b]))
        .collect();
    String::from_utf16(&utf16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip the feedback serializer → decoder so the tee's text
    /// extraction stays byte-compatible with `send_gm_feedback`. A drift in
    /// either serializer or decoder would silently drop captured output.
    #[test]
    fn decode_feedback_text_round_trips_serializer() {
        // Reproduce the on-wire feedback layout: speaker "SYSTEM", flags 0,
        // channel 9, text "spawned npc 42".
        let text = "spawned npc 42";
        let speaker: Vec<u16> = "SYSTEM".encode_utf16().collect();
        let text_u16: Vec<u16> = text.encode_utf16().collect();
        let mut args = Vec::new();
        args.extend_from_slice(&(speaker.len() as u32).to_le_bytes());
        for c in &speaker {
            args.extend_from_slice(&c.to_le_bytes());
        }
        args.push(0); // flags
        args.push(9); // channel
        args.extend_from_slice(&(text_u16.len() as u32).to_le_bytes());
        for c in &text_u16 {
            args.extend_from_slice(&c.to_le_bytes());
        }

        assert_eq!(decode_feedback_text(&args).as_deref(), Some(text));
    }

    /// A truncated buffer must return `None`, never panic — the drain loop
    /// runs on every lab console call and a malformed capture must not take
    /// the cell loop down.
    #[test]
    fn decode_feedback_text_tolerates_truncation() {
        assert_eq!(decode_feedback_text(&[]), None);
        assert_eq!(decode_feedback_text(&[6, 0, 0, 0]), None); // claims 6 speaker chars, none present
    }
}
