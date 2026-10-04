//! The launcher's look: dark only, blue-gray surfaces, a cyan primary
//! action (from the approved prototype at `e70b076a9`).
//!
//! The palette is the prototype's CSS variables. [`apply`] locks egui to
//! its dark theme and overwrites the dark visuals with this palette, so
//! the window stays dark when Windows uses light mode.

use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Stroke};

/// `--bg`: the main surface.
pub(super) const BG: Color32 = Color32::from_rgb(0x15, 0x19, 0x1e);
/// The side panel's top and bottom gradient stops (`aside`).
pub(super) const ASIDE_TOP: Color32 = Color32::from_rgb(0x20, 0x37, 0x44);
pub(super) const ASIDE_BOTTOM: Color32 = Color32::from_rgb(0x16, 0x24, 0x2d);
/// The settings panel and cards.
pub(super) const CARD: Color32 = Color32::from_rgb(0x1b, 0x22, 0x2b);
/// `--panel`: inputs and raised controls.
pub(super) const PANEL: Color32 = Color32::from_rgb(0x22, 0x27, 0x2e);
/// `--line`: hairlines (white at ~9%).
pub(super) const LINE: Color32 = Color32::from_rgb(0x2a, 0x2e, 0x33);
/// `--muted`: secondary text.
pub(super) const MUTED: Color32 = Color32::from_rgb(0xa4, 0xae, 0xbc);
/// `--ink`: body text.
pub(super) const INK: Color32 = Color32::from_rgb(0xf4, 0xf5, 0xf7);
/// `--accent`: the primary action and selection.
pub(super) const ACCENT: Color32 = Color32::from_rgb(0x87, 0xd5, 0xe7);
/// Text on an accent fill.
pub(super) const ON_ACCENT: Color32 = Color32::from_rgb(0x10, 0x26, 0x2e);
/// Ready / running.
pub(super) const GOOD: Color32 = Color32::from_rgb(0xa6, 0xde, 0xc0);
/// Notices that need attention.
pub(super) const WARN: Color32 = Color32::from_rgb(0xe6, 0xc5, 0x94);
/// Errors and destructive actions.
pub(super) const DANGER: Color32 = Color32::from_rgb(0xff, 0xaa, 0xa5);

/// Lock the dark theme and install the palette. Cheap; called once at
/// startup and safe to call again.
pub(super) fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.spacing.interact_size.y = 30.0;
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(INK);
        v.panel_fill = BG;
        v.window_fill = CARD;
        v.window_stroke = Stroke::new(1.0, LINE);
        v.window_corner_radius = CornerRadius::same(12);
        v.extreme_bg_color = PANEL;
        v.faint_bg_color = CARD;
        v.code_bg_color = PANEL;
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = WARN;
        v.error_fg_color = DANGER;
        v.selection.bg_fill = ACCENT.linear_multiply(0.35);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        // Small enough that a checkbox stays square; the theme's own
        // buttons set their larger radius explicitly.
        let radius = CornerRadius::same(4);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = BG;
        w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        w.noninteractive.fg_stroke = Stroke::new(1.0, INK);
        for (state, fill, stroke) in [
            (&mut w.inactive, PANEL, LINE),
            (&mut w.hovered, Color32::from_rgb(0x2b, 0x32, 0x3b), MUTED),
            (&mut w.active, Color32::from_rgb(0x31, 0x3a, 0x44), ACCENT),
            (&mut w.open, PANEL, LINE),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.corner_radius = radius;
            state.fg_stroke = Stroke::new(1.5, INK);
        }
    });
}

/// The cyan primary button.
pub(super) fn primary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.to_owned())
            .color(ON_ACCENT)
            .strong()
            .size(15.0),
    )
    .fill(ACCENT)
    .corner_radius(CornerRadius::same(9))
    .min_size(egui::vec2(200.0, 42.0))
}

/// A quiet outlined button.
pub(super) fn secondary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(9))
        .min_size(egui::vec2(0.0, 42.0))
}

/// A destructive action's button.
pub(super) fn danger_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).color(DANGER))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, DANGER.linear_multiply(0.4)))
        .corner_radius(CornerRadius::same(9))
}

/// A bordered card, the prototype's `.status` box.
pub(super) fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::same(16))
}

/// Small uppercase label in the accent colour (`.eyebrow`).
pub(super) fn eyebrow(text: &str) -> RichText {
    RichText::new(spaced_caps(text))
        .size(10.0)
        .strong()
        .color(ACCENT)
}

/// Secondary text.
pub(super) fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(MUTED).size(12.5)
}

/// Upper-case, one space between letters and three between words: egui
/// has no letter-spacing, and this approximates the prototype's tracking.
pub(super) fn spaced_caps(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            word.to_uppercase()
                .chars()
                .map(String::from)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("   ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bug shape: a light Windows theme flipped the window to egui's light
    // visuals. The launcher is dark-only whatever the OS says.
    #[test]
    fn apply_locks_the_dark_theme_with_the_palette() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Light);
        apply(&ctx);
        assert_eq!(
            ctx.options(|o| o.theme_preference),
            egui::ThemePreference::Dark
        );
        let style = ctx.style_of(egui::Theme::Dark);
        assert!(style.visuals.dark_mode);
        assert_eq!(style.visuals.panel_fill, BG);
        assert_eq!(style.visuals.hyperlink_color, ACCENT);
    }

    #[test]
    fn spaced_caps_spaces_letters_but_not_words() {
        assert_eq!(spaced_caps("ab c"), "A B   C");
    }
}
