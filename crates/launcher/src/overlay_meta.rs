//! The client-patches UI overlay's manifest identity: its patch id prefix
//! and the text the launcher's "Changes to your client" list shows for
//! it. Shared by the launcher and the `pack-client-overlay` tool, which
//! writes the text into the overlay's manifest entry.

/// Patch id prefix for the Black Market UI overlay.
pub const DEFAULT_ID_PREFIX: &str = "bm-ui-overlay";

/// The overlay's name in the "Changes to your client" list.
pub const OVERLAY_TITLE: &str = "Black Market window (UI files)";

/// What the overlay changes, for the same list.
pub const OVERLAY_DESCRIPTION: &str = "Adds the Black Market window's layout and Lua \
    under SGWGame/Content/UI. The client-patches DLL drives it; without the DLL the \
    window never opens.";
