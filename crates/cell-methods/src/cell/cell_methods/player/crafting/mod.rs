//! Crafting cell methods 95-100: argument parsing and the forward to the
//! base.
//!
//! The cell parses every argument per `entities/defs/SGWPlayer.def:916-948`
//! and forwards one [`CraftRequest`] per request ([`forward`]), in the
//! `CellToBaseMsg::Plugin` envelope (#962 step 5). The base owns the rules,
//! the database and the feedback (`cimmeria-base-crafting`, a base plugin).
//!
//! [`CraftRequest`]: crate::cell::messages::CraftRequest Campaign ledger:
//! `docs/analysis/crafting/`.
//!
//! A request whose bytes do not parse is dropped with a WARN at target
//! `crafting` and no feedback: the 2009 client always sends the def's exact
//! shape, so only a forged or corrupted packet lands there.

use tokio::sync::mpsc;

use crate::cell::client_methods::player::ON_UPDATE_DISCIPLINE;
use crate::cell::messages::{CellToBaseMsg, CraftVerb};
use crate::cell::space_manager::SpaceManager;

use super::constants::*;

mod forward;

#[cfg(test)]
mod tests;

/// Route one crafting method. Returns `false` only for an index outside
/// 95-100, so the caller can report it as unhandled.
pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    if !(SPEND_APPLIED_SCIENCE_POINTS..=RESPEC_CRAFTING).contains(&method_index) {
        return false;
    }
    match parse_verb(method_index, args) {
        Ok(verb) => forward::forward(entity_id, verb, tx, space_mgr).await,
        Err(error) => {
            tracing::warn!(
                target: "crafting",
                event = "malformed",
                entity_id,
                method_index,
                method_name = cimmeria_wire::names::player_cell_method(method_index),
                args_len = args.len(),
                ?error,
                "crafting request arguments did not parse; dropped"
            );
        }
    }
    true
}

/// Why a crafting method's arguments did not parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgsError {
    /// The payload ended inside the argument starting at `offset`.
    Truncated { offset: usize },
    /// Bytes were left over after the last argument.
    TrailingBytes { extra: usize },
    /// `method_index` is not one of 95-100.
    NotACraftingMethod,
}

/// Parse the arguments of crafting method `method_index` into its verb.
///
/// `ItemID` is an `INT32` alias (`alias.xml`), and an `ARRAY<ItemID>` is a
/// `u32` count followed by that many `INT32`s. The whole payload must be
/// consumed.
pub fn parse_verb(method_index: u16, args: &[u8]) -> Result<CraftVerb, ArgsError> {
    let mut r = ArgReader { args, offset: 0 };
    let verb = match method_index {
        SPEND_APPLIED_SCIENCE_POINTS => CraftVerb::Spend {
            discipline_id: r.i32()?,
        },
        CRAFT => CraftVerb::Craft {
            blueprint_id: r.i32()?,
            items: r.i32_array()?,
            quantity: r.i32()?,
        },
        RESEARCH => CraftVerb::Research {
            item_id: r.i32()?,
            kickers: r.i32_array()?,
        },
        REVERSE_ENGINEER => CraftVerb::ReverseEngineer { item_id: r.i32()? },
        ALLOYING => CraftVerb::Alloy {
            blueprint_id: r.i32()?,
            current_tier_item_id: r.i32()?,
            lower_tier_items: r.i32_array()?,
        },
        RESPEC_CRAFTING => CraftVerb::Respec,
        _ => return Err(ArgsError::NotACraftingMethod),
    };
    match args.len() - r.offset {
        0 => Ok(verb),
        extra => Err(ArgsError::TrailingBytes { extra }),
    }
}

/// A little-endian cursor over one method's arguments.
struct ArgReader<'a> {
    args: &'a [u8],
    offset: usize,
}

impl ArgReader<'_> {
    fn i32(&mut self) -> Result<i32, ArgsError> {
        let bytes = self
            .args
            .get(self.offset..self.offset + 4)
            .ok_or(ArgsError::Truncated {
                offset: self.offset,
            })?;
        self.offset += 4;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32_array(&mut self) -> Result<Vec<i32>, ArgsError> {
        let start = self.offset;
        let count = self.i32()? as u32 as usize;
        // Bound the count by the bytes present before allocating: a forged
        // count of u32::MAX must not reserve 16 GiB.
        if count.saturating_mul(4) > self.args.len() - self.offset {
            return Err(ArgsError::Truncated { offset: start });
        }
        (0..count).map(|_| self.i32()).collect()
    }
}

/// Emit an `onUpdateDiscipline` callback to the client.
///
/// Sends a `CellToBaseMsg::EntityMethodCall` with method index 136 and the
/// 8-byte payload from `cimmeria_wire::crafting::update_discipline_args`.
/// The BaseApp encodes the extended-encoding wire bytes and ships the packet.
///
/// Nothing calls this yet: the expertise-changing verbs run on the base and
/// send 136 from there. The wire shape stays pinned by
/// [`tests::send_on_update_discipline_emits_correct_message`].
#[allow(dead_code)] // Kept for a cell-side expertise change; `.allcraft` sends 136 from the base.
pub async fn send_on_update_discipline(
    entity_id: u32,
    discipline_id: i32,
    expertise: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let args = cimmeria_wire::crafting::update_discipline_args(discipline_id, expertise);
    // mpsc::Sender::send().await returns Err only when the receiver has been
    // dropped — i.e., the base task is shutting down. A dropped client at
    // this point isn't actionable from the cell, so we log-and-continue.
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_UPDATE_DISCIPLINE,
            args,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            discipline_id,
            expertise,
            error = %e,
            "onUpdateDiscipline send dropped — base receiver gone",
        );
    }
}
