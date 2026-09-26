//! The four rule predicates, and the allowlist R1 carries until the
//! fixing packets land.
//!
//! R1 and R2 are about WHERE the buttons sit; R3 and R4 are about
//! whether the window can draw them at all. All four end in the same
//! failure for the player — a screen with nothing to press whose close
//! sends nothing — because the client decides whether to emit
//! `dialogButtonChoice(id, -1)` by counting COOKED buttons, not
//! rendered ones.
//!
//! Each predicate takes its policy (allowlist, protected list) as a
//! parameter rather than reading the constant directly. That is what
//! lets [`super::rule_guards`] drive these exact functions on synthetic
//! data: a guard that re-implemented a rule in its own body would keep
//! passing after someone loosened the rule here, which is the failure
//! mode `interact_tag_linter.rs` documents on `region_key_violations`.

use std::collections::BTreeSet;

use super::seed_model::{Button, ChainRefs, DialogSeed};
use super::sql_scan::{sql_statements, unquote, value_tuples};

/// Dialogs that must never gain a button (R2).
///
/// Inherited hard rule from the Cellblock and Castle rebuild campaigns.
/// Every one of these is the key of a `dialog_choice` chain and ships
/// with zero buttons, so the player's close emits
/// `dialogButtonChoice(id, -1)` (F8) and the chain fires. A button —
/// any button, on any screen, including the final one — replaces that
/// `-1` with silence and strands the mission step. R1 cannot catch it,
/// because a button on the final screen satisfies R1.
pub(crate) const NEVER_ADD_A_BUTTON: [i32; 11] = [
    2300, 5021, 5020, 2574, 2575, 2577, 2581, 5003, 5004, 5008, 5009,
];

/// A dialog that violates R1 today and is temporarily tolerated.
pub(crate) struct R1Exemption {
    pub(crate) dialog_id: i32,
    /// The shipped button layout that makes it a violation right now.
    pub(crate) reason: &'static str,
    /// The packet that must delete this entry.
    pub(crate) removed_by: &'static str,
}

/// The dialogs that ship in the soft-locking shape.
///
/// It held three (3999, 5861, 2576). DU-02a stripped every button from
/// 3999; DU-02b moved 5861's Accept onto final screen 96789 and 2576's
/// Take Missions onto final screen 96825. It is empty now and should stay
/// that way.
///
/// This is a debt register, not a policy: each entry names the packet
/// that empties it, and [`r1_violations`] fails on a STALE entry as well
/// as on an unlisted violator. DU-02a and DU-02b therefore cannot fix a
/// dialog and leave its exemption behind — the linter turns red until
/// the entry is deleted. The list is expected to be empty at the end of
/// Wave 1.
///
/// A fourth violator has never been found: a full scan of the four chain
/// seeds on 2026-09-21 turned up exactly nineteen `dialog_choice`-keyed
/// dialogs, of which those three and no others broke R1.
pub(crate) const R1_ALLOWLIST: [R1Exemption; 0] = [];

/// R1 — a chain-keyed dialog has zero buttons, or a button on its final
/// screen. Also reports allowlist entries that have gone stale.
pub(crate) fn r1_violations(
    seed: &DialogSeed,
    refs: &ChainRefs,
    allowlist: &[R1Exemption],
) -> Vec<String> {
    let exempt: BTreeSet<i32> = allowlist.iter().map(|e| e.dialog_id).collect();
    let mut out = Vec::new();

    for &dialog in refs.keyed.keys() {
        if !breaks_r1(seed, dialog) || exempt.contains(&dialog) {
            continue;
        }
        out.push(format!(
            "  R1 dialog {dialog}: keyed by {chains}. Buttons on screen(s) \
             {with_buttons:?}, but final screen {final_screen:?} has none. A player who \
             pages to the end sees only Done; Done is a close, and closing a dialog that \
             HAS buttons sends nothing (F8), so the chain never fires and the player is \
             soft-locked. Move a button onto the final screen, or strip every button so \
             the close emits dialogButtonChoice({dialog}, -1).",
            chains = refs.chains_for(dialog),
            with_buttons = seed.screens_with_buttons(dialog),
            final_screen = seed.final_screen(dialog),
        ));
    }

    for exemption in allowlist {
        let dialog = exemption.dialog_id;
        if !refs.keyed.contains_key(&dialog) {
            out.push(format!(
                "  R1 allowlist entry for dialog {dialog} is STALE: nothing in the Castle \
                 or Cellblock seeds keys a dialog_choice chain on it any more, so the \
                 exemption can never suppress anything. Delete the entry ({removed_by}).",
                removed_by = exemption.removed_by,
            ));
        } else if !breaks_r1(seed, dialog) {
            out.push(format!(
                "  R1 allowlist entry for dialog {dialog} is STALE: it now satisfies R1 \
                 (buttons on screen(s) {with_buttons:?}, final screen {final_screen:?}). \
                 The breakage it recorded was: {reason}. Deleting the entry is the last \
                 step of {removed_by}.",
                with_buttons = seed.screens_with_buttons(dialog),
                final_screen = seed.final_screen(dialog),
                reason = exemption.reason,
                removed_by = exemption.removed_by,
            ));
        }
    }

    out.sort();
    out
}

/// The R1 predicate itself, shared by the violation scan and the
/// stale-allowlist scan so the two can never disagree about what
/// "violates" means.
fn breaks_r1(seed: &DialogSeed, dialog_id: i32) -> bool {
    if seed.button_count(dialog_id) == 0 {
        return false;
    }
    match seed.final_screen(dialog_id) {
        Some(final_screen) => seed.buttons_on(final_screen).is_empty(),
        // Buttons but no screens cannot occur in the seed and would mean
        // the screen scan lost rows. Report it rather than call it
        // compliant.
        None => true,
    }
}

/// R2 — the never-add-a-button list stays zero-button.
pub(crate) fn r2_violations(seed: &DialogSeed, refs: &ChainRefs, protected: &[i32]) -> Vec<String> {
    let mut out = Vec::new();
    for &dialog in protected {
        let with_buttons = seed.screens_with_buttons(dialog);
        if with_buttons.is_empty() {
            continue;
        }
        let layout: Vec<(i32, &[Button])> = with_buttons
            .iter()
            .map(|s| (*s, seed.buttons_on(*s)))
            .collect();
        out.push(format!(
            "  R2 dialog {dialog}: on the never-add-a-button list but now carries buttons \
             — {layout:?}. It is keyed by {chains} and advances ONLY through the \
             zero-button close, which sends dialogButtonChoice({dialog}, -1) (F8). Any \
             button at all replaces that -1 with silence and strands the step. Remove the \
             button row(s).",
            chains = refs.chains_for(dialog),
        ));
    }
    out.sort();
    out
}

/// The cooked `button_type` values each window actually draws.
///
/// `None` means the linter has not been taught this `ui_screen_type` and
/// cannot reason about it; that is itself reported, so a seventh enum
/// value cannot slip through as "no violations".
///
/// Why this is a correctness rule and not a style rule: the client
/// decides whether to emit the close event by counting the dialog's
/// COOKED buttons, not the ones it managed to draw. A button the window
/// cannot render is therefore invisible AND suppresses
/// `dialogButtonChoice(id, -1)` — the player sees a bare screen, closes
/// it, and nothing reaches the server. That is the same soft-lock R1
/// catches, arriving by a different route.
///
/// This table is the executable form of the drawable-button matrix in
/// `docs/content/dialog-ui-client-contract.md` § Buttons. Note that
/// Decline (3) is chrome the client draws for itself and is never
/// authored, so it is absent from every row here — a cooked type-3
/// button is a violation on any window.
///
/// Sources: `Dialog/Dialog.lua:3-8` (DialogWin draws Accept 2 and
/// Generic1-3 = 4, 5, 6), `Dialog/Blurb.lua:3-6` (BlurbWin draws More
/// Info 1 and Accept 2), `Dialog/Blurb.lua:35-38` (F5 — type 0
/// `DUIST_None` is registered to BlurbWin under a "TEMP HACK" comment),
/// `Dialog/Dialog.lua:71-73` (F4 — Radio and Realization register the
/// same window and init function as Dialog), `TutorialScreen.lua`
/// (Tutorial renders no cooked buttons at all).
fn drawable_button_types(ui_screen_type: &str) -> Option<&'static [i32]> {
    match ui_screen_type {
        // BlurbWin.
        "DUIST_DefaultBlurb" | "DUIST_None" => Some(&[1, 2]),
        // DialogWin. Radio and Realization are the same window (F4).
        "DUIST_DefaultDialog" | "DUIST_DefaultRadio" | "DUIST_DefaultRealization" => {
            Some(&[2, 4, 5, 6])
        }
        // Draws no cooked buttons whatsoever.
        "DUIST_DefaultTutorial" => Some(&[]),
        _ => None,
    }
}

/// Window types a `dialog_choice` chain may key (R4).
///
/// Tutorial never draws a cooked button, and `DUIST_None` is the type-0
/// "TEMP HACK" Blurb that the redesign does not use. Keying a chain on
/// either is a wiring mistake, not a layout one, so it is reported
/// separately rather than squeezed into R1 or R3.
fn may_key_a_chain(ui_screen_type: &str) -> bool {
    matches!(
        ui_screen_type,
        "DUIST_DefaultBlurb"
            | "DUIST_DefaultDialog"
            | "DUIST_DefaultRadio"
            | "DUIST_DefaultRealization"
    )
}

/// R3 — every button on a chain-referenced dialog is one its own window
/// can draw.
pub(crate) fn r3_violations(seed: &DialogSeed, refs: &ChainRefs) -> Vec<String> {
    let mut out = Vec::new();
    for dialog in refs.referenced() {
        let Some(window) = seed.ui_screen_type.get(&dialog) else {
            out.push(format!(
                "  R3 dialog {dialog}: referenced by {chains} but has no row in \
                 dialogs.sql, so its window type — and therefore which buttons it can \
                 draw — is unknown. A dangling dialog reference displays nothing.",
                chains = refs.references_for(dialog),
            ));
            continue;
        };
        let Some(drawable) = drawable_button_types(window) else {
            out.push(format!(
                "  R3 dialog {dialog} (ui_screen_type {window}, referenced by {chains}): \
                 the linter does not know which buttons this window draws. Teach \
                 drawable_button_types() in tests/it/dialog_button_linter/rules.rs — until \
                 then no button on this dialog is being checked.",
                chains = refs.references_for(dialog),
            ));
            continue;
        };
        for screen in seed.screens_in_order(dialog) {
            for button in seed.buttons_on(screen) {
                if drawable.contains(&button.button_type) {
                    continue;
                }
                out.push(format!(
                    "  R3 dialog {dialog} ({window}, referenced by {chains}) screen \
                     {screen}: button type {ty} (id {id}, {text:?}) is not one of \
                     {drawable:?}, the types that window draws. This is a SOFT-LOCK, not a \
                     cosmetic issue: the client counts COOKED buttons to decide whether to \
                     send dialogButtonChoice({dialog}, -1) on close (F8), so an undrawable \
                     button is invisible to the player AND silences the close event. The \
                     screen looks like it has nothing to press, and pressing Done sends \
                     nothing.",
                    chains = refs.references_for(dialog),
                    ty = button.button_type,
                    id = button.button_id,
                    text = button.text,
                ));
            }
        }
    }
    out.sort();
    out
}

/// R4 — a chain-keyed dialog is a window that can actually carry the
/// interaction.
pub(crate) fn r4_violations(seed: &DialogSeed, refs: &ChainRefs) -> Vec<String> {
    let mut out = Vec::new();
    for &dialog in refs.keyed.keys() {
        let window = seed.ui_screen_type.get(&dialog).map(String::as_str);
        match window {
            Some(w) if may_key_a_chain(w) => {}
            Some(w) => out.push(format!(
                "  R4 dialog {dialog}: keyed by {chains} but its ui_screen_type is {w}. \
                 {why} Re-point the chain at a Blurb, Dialog, Radio or Realization \
                 dialog, or drop the trigger.",
                chains = refs.chains_for(dialog),
                why = why_it_cannot_key(w),
            )),
            None => out.push(format!(
                "  R4 dialog {dialog}: keyed by {chains} but has no row in dialogs.sql — \
                 the trigger can never fire because the dialog does not exist.",
                chains = refs.chains_for(dialog),
            )),
        }
    }
    out.sort();
    out
}

fn why_it_cannot_key(ui_screen_type: &str) -> &'static str {
    match ui_screen_type {
        "DUIST_DefaultTutorial" => {
            "TutorialScreen.lua renders no cooked buttons at all, so the player has nothing \
             to press and the chain has nothing to fire it."
        }
        "DUIST_None" => {
            "Type 0 is the \"TEMP HACK\" Blurb registration (F5) and is not a window the \
             redesign drives chains from."
        }
        _ => "That window type is not one a dialog_choice chain may key.",
    }
}

/// Every label of the `EDialogUIScreenType` enum must be known to
/// [`drawable_button_types`].
///
/// Without this, adding a seventh enum value and using it in the Castle
/// seeds would make R3 report "unknown window" per dialog rather than
/// checking anything — and if the unknown-window arm were ever softened
/// to a `continue`, it would report nothing at all. Reading the enum
/// from the schema keeps the table honest against the real type.
pub(crate) fn untaught_enum_labels(enum_sql: &str) -> Vec<String> {
    enum_labels(enum_sql)
        .into_iter()
        .filter(|label| drawable_button_types(label).is_none())
        .collect()
}

/// Pull the labels out of `CREATE TYPE ... AS ENUM ('a', 'b', ...)`.
pub(crate) fn enum_labels(enum_sql: &str) -> Vec<String> {
    for stmt in sql_statements(enum_sql) {
        let Some(at) = stmt.find("AS ENUM") else {
            continue;
        };
        if let Some(tuple) = value_tuples(&stmt[at + "AS ENUM".len()..])
            .into_iter()
            .next()
        {
            return tuple.iter().filter_map(|f| unquote(f)).collect();
        }
    }
    Vec::new()
}
