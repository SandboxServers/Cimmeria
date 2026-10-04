//! The gate motif and the side panel's gradient, drawn with egui's
//! painter so the launcher ships no image for them.
//!
//! The shapes follow the prototype's `.gate`: a dashed outer ring, a
//! thick blue-gray ring, a glowing event horizon, and the amber chevron
//! at the top.

use eframe::egui::{self, epaint, Color32, Pos2, Rect, Stroke, Vec2};

const RING: Color32 = Color32::from_rgb(0x30, 0x4e, 0x5c);
const RING_EDGE: Color32 = Color32::from_rgb(0x88, 0xd3, 0xde);
const DASH: Color32 = Color32::from_rgb(0x73, 0x96, 0xa1);
const HORIZON_DEEP: Color32 = Color32::from_rgb(0x10, 0x25, 0x2e);
const HORIZON_MID: Color32 = Color32::from_rgb(0x25, 0x6f, 0x83);
const HORIZON_LIGHT: Color32 = Color32::from_rgb(0xa5, 0xe2, 0xe5);
const CHEVRON: Color32 = Color32::from_rgb(0xff, 0xc7, 0x8e);

/// Allocate a square of `diameter` and draw the gate in it.
pub(super) fn gate(ui: &mut egui::Ui, diameter: f32) {
    // Room for the dashed ring and the chevron outside the gate itself.
    let pad = diameter * 0.16;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter + 2.0 * pad), egui::Sense::hover());
    paint_gate(ui.painter(), rect.center(), diameter / 2.0);
}

fn paint_gate(p: &egui::Painter, c: Pos2, r: f32) {
    let ring_w = r * 0.14;
    // Soft outer glow.
    for i in 0..6 {
        let a = 10 - i as u8;
        p.circle_stroke(
            c,
            r + 4.0 + i as f32 * 4.0,
            Stroke::new(4.0, Color32::from_rgba_unmultiplied(0x42, 0xae, 0xcb, a)),
        );
    }
    // Dashed outer ring.
    let dash_r = r + r * 0.12;
    let points: Vec<Pos2> = (0..=96)
        .map(|i| {
            let t = i as f32 / 96.0 * std::f32::consts::TAU + 0.26;
            c + Vec2::angled(t) * dash_r
        })
        .collect();
    p.extend(egui::Shape::dashed_line(
        &points,
        Stroke::new(1.5, DASH),
        5.0,
        4.0,
    ));
    // Event horizon: a radial gradient faked with offset discs, lighter
    // toward the upper left as in the prototype.
    let inner = r - ring_w;
    p.circle_filled(c, inner, HORIZON_DEEP);
    let steps = 14;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let colour = lerp(HORIZON_MID, HORIZON_LIGHT, t).gamma_multiply(0.18 + 0.05 * t);
        let rr = inner * (1.0 - t * 0.85);
        let shift = Vec2::splat(-inner * 0.18 * t);
        p.circle_filled(c + shift, rr, colour);
    }
    // The ring, with a thin bright edge outside and a dark one inside.
    p.circle_stroke(c, r - ring_w / 2.0, Stroke::new(ring_w, RING));
    p.circle_stroke(c, r + 1.0, Stroke::new(1.5, RING_EDGE.gamma_multiply(0.35)));
    p.circle_stroke(
        c,
        inner,
        Stroke::new(2.5, Color32::from_rgb(0x15, 0x27, 0x2f)),
    );
    // Chevron at the top, with a glow.
    let top = c - Vec2::new(0.0, r + r * 0.07);
    let w = r * 0.13;
    let h = r * 0.10;
    let chevron = [
        top + Vec2::new(-w, h * 0.5),
        top + Vec2::new(0.0, -h * 0.5),
        top + Vec2::new(w, h * 0.5),
    ];
    p.add(egui::Shape::line(
        chevron.to_vec(),
        Stroke::new(
            r * 0.07,
            Color32::from_rgba_unmultiplied(0xff, 0xa7, 0x69, 40),
        ),
    ));
    p.add(egui::Shape::line(
        chevron.to_vec(),
        Stroke::new((r * 0.035).max(2.0), CHEVRON),
    ));
}

/// Fill `rect` with a vertical gradient from `top` to `bottom`.
pub(super) fn vertical_gradient(p: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = epaint::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    p.add(egui::Shape::mesh(mesh));
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}
