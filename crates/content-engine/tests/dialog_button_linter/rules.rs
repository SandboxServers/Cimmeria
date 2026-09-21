//! The three rule predicates, and the allowlist R1 carries until the
//! fixing packets land.
//!
//! Each predicate takes its policy (allowlist, protected list) as a
//! parameter rather than reading the constant directly. That is what
//! lets [`super::rule_guards`] drive these exact functions on synthetic
//! data: a guard that re-implemented a rule in its own body would keep
//! passing after someone loosened the rule here, which is the failure
//! mode `interact_tag_linter.rs` documents on `region_key_violations`.

use std::collections::BTreeSet;

use super::seed_model::{Button, ChainRefs, DialogSeed};

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

/// The three dialogs that ship in the soft-locking shape.
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
/// dialogs, of which these three and no others break R1.
pub(crate) const R1_ALLOWLIST: [R1Exemption; 3] = [
    R1Exemption {
        dialog_id: 3999,
        reason: "Receive Item (type 4, id 70) on screens 96252-96258; final 96260 is bare",
        removed_by: "DU-02a (StripAll)",
    },
    R1Exemption {
        dialog_id: 5861,
        reason: "Accept (type 2, id 8) on screens 96782-96786; final 96789 is bare",
        removed_by: "DU-02b (OnlyOn final 96789)",
    },
    R1Exemption {
        dialog_id: 2576,
        reason: "Take Missions (type 4, id 71) on screens 96821-96823; final 96825 is bare",
        removed_by: "DU-02b (OnlyOn final 96825)",
    },
];

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

/// R3 — a Blurb referenced by these chain files uses only More Info (1)
/// and Accept (2), the only two types `BlurbWin` draws (F7).
pub(crate) fn r3_violations(seed: &DialogSeed, refs: &ChainRefs) -> Vec<String> {
    let mut out = Vec::new();
    for dialog in refs.referenced() {
        if !seed.is_blurb(dialog) {
            continue;
        }
        for screen in seed.screens_in_order(dialog) {
            for button in seed.buttons_on(screen) {
                if matches!(button.button_type, 1 | 2) {
                    continue;
                }
                out.push(format!(
                    "  R3 dialog {dialog} (DUIST_DefaultBlurb, referenced by \
                     {chains}) screen {screen}: button type {ty} (id {id}, {text:?}). \
                     BlurbWin renders only More Info (1) and Accept (2) (F7), so this \
                     button is never drawn and can never be clicked — and BlurbWin has no \
                     Next either, so nothing else reaches it.",
                    chains = refs.chains_for(dialog),
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
