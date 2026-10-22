//! Whether the UI owns the mouse (`ui_owned` on `client.input.button`). LN-10
//! fills this in; until then the answer is unknown.

use crate::hooks::entity_trace::map::Mem;

/// Whether the UI owns the mouse, or `None` when it is not known. A stub until LN-10.
pub(super) fn read_ui_owned(_mem: &impl Mem) -> Option<bool> {
    None
}
