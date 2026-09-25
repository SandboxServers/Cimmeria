//! Castle_CellBlock dialog patches.
//!
//! Owned by DU-02a. Ships empty from DU-01 so the two zone tables can be
//! filled by two packets in parallel without touching the same file.
//!
//! Adding a row, in one commit:
//!
//!   1. Add the [`DialogPatch`] here.
//!   2. Make `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` agree.
//!      The client renders from this patch; the seed is the committed
//!      record and what the DU-L linter reads. They are kept in sync by
//!      hand, exactly as the full-regeneration overrides already require.
//!   3. Check the two hard rules from the client contract:
//!      * a dialog that keys a `dialog_choice` chain must end with either
//!        zero buttons or a button on its FINAL screen, or a player who
//!        reads to the end and presses Done soft-locks (fact F8);
//!      * never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581,
//!        5003, 5004 or 5009.
//!
//! Syntax:
//!
//! ```ignore
//! DialogPatch {
//!     dialog_id: 3999,
//!     ui_screen_type: None,
//!     buttons: ButtonPlan::StripAll,
//! },
//! ```

#[allow(unused_imports)] // named by the doc comment above; used once a row lands
use super::patch::{ButtonPlan, DialogPatch};

/// Castle_CellBlock patch rows. Empty until DU-02a.
pub const CELLBLOCK_DIALOG_PATCHES: &[DialogPatch] = &[];
