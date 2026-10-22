//! Button edges and click classes (`client.input.button`).
//!
//! One row per LMB or RMB edge, from the controller's button word. A release
//! is classified the way the client does (RE § 6.2): drag counters under
//! [`DRAG_THRESHOLD`] make it a click, a select for LMB and an interact for RMB.
//! Pure apart from the tracker's static and the emit call in [`step`].

use std::sync::{Mutex, MutexGuard};

use serde_json::json;

use super::controller::{ControllerState, BUTTON_LMB, BUTTON_RMB};
use super::frame::NavFrame;
use super::ui;
use crate::hooks::entity_trace::map::Mem;
use crate::hooks::entity_trace::Fields;

/// RE § 6.2: the client treats a release as a click when |dx| < 5 and |dy| < 5.
pub const DRAG_THRESHOLD: u32 = 5;

/// A mouse button the poll reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// The left button.
    Lmb,
    /// The right button.
    Rmb,
}

/// Whether a button went down or came up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// The button was pressed this tick.
    Down,
    /// The button was released this tick.
    Up,
}

/// The class of a release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickClass {
    /// A left click: a select.
    Select,
    /// A right click: an interact.
    Interact,
    /// A release after a drag past the threshold.
    Drag,
    /// A release while the other button was held too.
    Chord,
}

/// One button edge, with the fields a row needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonEvent {
    /// Which button.
    pub button: Button,
    /// Down or up.
    pub edge: Edge,
    /// Whether the other button was held at this edge.
    pub other_held: bool,
    /// Milliseconds from the down edge, when the down was seen (up only).
    pub hold_ms: Option<u64>,
    /// The drag counters at release (up only).
    pub drag: Option<[i32; 2]>,
    /// The release's class (up only).
    pub class: Option<ClickClass>,
    /// The hover id at release, for a select or an interact (up only).
    pub hover_id: Option<i32>,
}

/// The per-button state between ticks: the last button word, when each button
/// went down, and whether it was part of a chord.
#[derive(Debug, Default)]
pub struct ButtonTracker {
    initialized: bool,
    prev: u32,
    down_at: [Option<u64>; 2],
    chord: [bool; 2],
}

impl ButtonTracker {
    /// A tracker that has seen no tick yet. `const`, so it can seed a `static`.
    pub const fn new() -> Self {
        Self {
            initialized: false,
            prev: 0,
            down_at: [None; 2],
            chord: [false; 2],
        }
    }

    /// Forgets the previous tick, the press times and the chords, so a new
    /// controller's first tick only records its starting state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// The edges between the previous tick's buttons and `ctrl.buttons`. The
    /// first call only records the starting state and returns nothing.
    pub fn observe(&mut self, now_ms: u64, ctrl: &ControllerState) -> Vec<ButtonEvent> {
        let cur = ctrl.buttons;
        if !self.initialized {
            self.initialized = true;
            self.prev = cur;
            return Vec::new();
        }
        let mut out = Vec::new();
        let buttons = [
            (0, Button::Lmb, BUTTON_LMB, BUTTON_RMB),
            (1, Button::Rmb, BUTTON_RMB, BUTTON_LMB),
        ];
        for (i, button, bit, other_bit) in buttons {
            let was = self.prev & bit != 0;
            let is = cur & bit != 0;
            let other = cur & other_bit != 0;
            if !was && is {
                self.down_at[i] = Some(now_ms);
                self.chord[i] = other;
                out.push(ButtonEvent {
                    button,
                    edge: Edge::Down,
                    other_held: other,
                    hold_ms: None,
                    drag: None,
                    class: None,
                    hover_id: None,
                });
            } else if was && is {
                self.chord[i] |= other;
            } else if was && !is {
                let class = classify(button, ctrl.drag, self.chord[i] || other);
                let hover_id = (matches!(class, ClickClass::Select | ClickClass::Interact)
                    && ctrl.hover_id != 0)
                    .then_some(ctrl.hover_id);
                out.push(ButtonEvent {
                    button,
                    edge: Edge::Up,
                    other_held: other,
                    hold_ms: self.down_at[i].map(|t| now_ms.saturating_sub(t)),
                    drag: Some(ctrl.drag),
                    class: Some(class),
                    hover_id,
                });
                self.down_at[i] = None;
                self.chord[i] = false;
            }
        }
        self.prev = cur;
        out
    }
}

/// The class of a release of `button` with drag counters `drag`. A drag past
/// the threshold wins, then a chord, then the button's own click.
pub fn classify(button: Button, drag: [i32; 2], chord: bool) -> ClickClass {
    if drag[0].unsigned_abs() >= DRAG_THRESHOLD || drag[1].unsigned_abs() >= DRAG_THRESHOLD {
        ClickClass::Drag
    } else if chord {
        ClickClass::Chord
    } else {
        match button {
            Button::Lmb => ClickClass::Select,
            Button::Rmb => ClickClass::Interact,
        }
    }
}

/// The fields of one `client.input.button` row. `ui_owned` comes last, and only
/// when it is known.
pub fn button_fields(ev: &ButtonEvent, ui_owned: Option<bool>) -> Fields {
    let mut f: Fields = vec![
        ("button", json!(button_name(ev.button))),
        ("edge", json!(edge_name(ev.edge))),
    ];
    match ev.edge {
        Edge::Down => f.push(("other_held", json!(ev.other_held))),
        Edge::Up => {
            if let Some(ms) = ev.hold_ms {
                f.push(("hold_ms", json!(ms)));
            }
            if let Some([dx, dy]) = ev.drag {
                f.push(("drag_x", json!(dx)));
                f.push(("drag_y", json!(dy)));
            }
            if let Some(class) = ev.class {
                f.push(("class", json!(class_name(class))));
            }
            if let Some(id) = ev.hover_id {
                f.push(("hover_id", json!(id)));
            }
        }
    }
    if let Some(owned) = ui_owned {
        f.push(("ui_owned", json!(owned)));
    }
    f
}

/// The wire name of a button.
fn button_name(button: Button) -> &'static str {
    match button {
        Button::Lmb => "lmb",
        Button::Rmb => "rmb",
    }
}

/// The wire name of an edge.
fn edge_name(edge: Edge) -> &'static str {
    match edge {
        Edge::Down => "down",
        Edge::Up => "up",
    }
}

/// The wire name of a click class.
fn class_name(class: ClickClass) -> &'static str {
    match class {
        ClickClass::Select => "select",
        ClickClass::Interact => "interact",
        ClickClass::Drag => "drag",
        ClickClass::Chord => "chord",
    }
}

/// The tracker between ticks.
static TRACKER: Mutex<ButtonTracker> = Mutex::new(ButtonTracker::new());

/// Locks `m`, recovering a poisoned lock: a panic in another tick must not stop the poll.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Drops the tracker state, for a new controller.
pub(super) fn reset() {
    lock(&TRACKER).reset();
}

/// The `client.input.button` rows for this tick's edges. `ui_owned` costs
/// several memory reads, so it is read once per tick that has an edge, and not
/// at all on the (usual) tick without one.
fn rows(tracker: &mut ButtonTracker, mem: &impl Mem, frame: &NavFrame) -> Vec<Fields> {
    let events = tracker.observe(frame.now_ms, &frame.ctrl);
    if events.is_empty() {
        return Vec::new();
    }
    let ui_owned = ui::read_ui_owned(mem);
    events
        .iter()
        .map(|ev| button_fields(ev, ui_owned))
        .collect()
}

/// Runs once per tick: emits one `client.input.button` row per edge. The lock
/// is released before the rows are emitted.
pub(super) fn step(mem: &impl Mem, frame: &NavFrame) {
    let fields = rows(&mut lock(&TRACKER), mem, frame);
    for f in fields {
        crate::hooks::emit::emit("client.input.button", "info", f);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::hooks::entity_trace::map::fake::FakeMem;

    /// A controller with only the button word, drag and hover set.
    fn ctrl(buttons: u32, drag: [i32; 2], hover_id: i32) -> ControllerState {
        ControllerState {
            buttons,
            drag,
            hover_id,
            ..Default::default()
        }
    }

    /// The value of field `key` in `f`, if present.
    fn field(f: &Fields, key: &str) -> Option<Value> {
        f.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn first_call_emits_nothing() {
        let mut t = ButtonTracker::new();
        assert!(t.observe(0, &ctrl(BUTTON_LMB, [0, 0], 0)).is_empty());
    }

    #[test]
    fn lmb_click_is_select_with_hover() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(0, [0, 0], 0));
        let down = t.observe(100, &ctrl(BUTTON_LMB, [0, 0], 0));
        assert_eq!(down.len(), 1);
        assert_eq!(down[0].edge, Edge::Down);
        assert!(!down[0].other_held);
        let up = t.observe(180, &ctrl(0, [2, -3], 42));
        assert_eq!(up.len(), 1);
        assert_eq!(up[0].hold_ms, Some(80));
        assert_eq!(up[0].drag, Some([2, -3]));
        assert_eq!(up[0].class, Some(ClickClass::Select));
        assert_eq!(up[0].hover_id, Some(42));
    }

    #[test]
    fn rmb_drag_has_no_hover() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(0, [0, 0], 0));
        t.observe(10, &ctrl(BUTTON_RMB, [0, 0], 0));
        let up = t.observe(20, &ctrl(0, [0, 7], 42));
        assert_eq!(up.len(), 1);
        assert_eq!(up[0].class, Some(ClickClass::Drag));
        assert_eq!(up[0].hover_id, None);
    }

    #[test]
    fn chord_gives_two_chord_ups() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(0, [0, 0], 0));
        t.observe(10, &ctrl(BUTTON_LMB, [0, 0], 0));
        let rmb = t.observe(20, &ctrl(BUTTON_LMB | BUTTON_RMB, [0, 0], 0));
        assert_eq!(rmb.len(), 1);
        assert_eq!(rmb[0].button, Button::Rmb);
        assert!(rmb[0].other_held);
        let ups = t.observe(30, &ctrl(0, [0, 0], 0));
        assert_eq!(ups.len(), 2);
        assert_eq!(ups[0].button, Button::Lmb);
        assert_eq!(ups[0].class, Some(ClickClass::Chord));
        assert_eq!(ups[1].button, Button::Rmb);
        assert_eq!(ups[1].class, Some(ClickClass::Chord));
    }

    #[test]
    fn zero_hover_has_no_hover_id() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(0, [0, 0], 0));
        t.observe(10, &ctrl(BUTTON_LMB, [0, 0], 0));
        let up = t.observe(20, &ctrl(0, [0, 0], 0));
        assert_eq!(up[0].class, Some(ClickClass::Select));
        assert_eq!(up[0].hover_id, None);
    }

    #[test]
    fn up_without_seen_down_has_no_hold_ms() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(BUTTON_LMB, [0, 0], 0));
        let up = t.observe(50, &ctrl(0, [0, 0], 0));
        assert_eq!(up.len(), 1);
        assert_eq!(up[0].hold_ms, None);
    }

    /// A memory that counts every word read, to prove what a tick touches.
    struct Counting {
        inner: FakeMem,
        reads: std::cell::Cell<u32>,
    }

    impl Mem for Counting {
        fn u32_at(&self, addr: u32) -> Option<u32> {
            self.reads.set(self.reads.get() + 1);
            self.inner.u32_at(addr)
        }
    }

    fn frame(now_ms: u64, buttons: u32) -> NavFrame {
        NavFrame {
            now_ms,
            ctrl: ctrl(buttons, [0, 0], 0),
        }
    }

    #[test]
    fn ui_owned_is_read_only_for_a_tick_with_an_edge() {
        let mem = Counting {
            inner: FakeMem::default(),
            reads: std::cell::Cell::new(0),
        };
        let mut t = ButtonTracker::new();
        assert!(rows(&mut t, &mem, &frame(0, 0)).is_empty());
        assert!(rows(&mut t, &mem, &frame(10, 0)).is_empty());
        assert_eq!(
            mem.reads.get(),
            0,
            "a no-edge tick must not read the UI chain"
        );
        let emitted = rows(&mut t, &mem, &frame(20, BUTTON_LMB));
        assert_eq!(emitted.len(), 1);
        assert!(mem.reads.get() > 0, "an edge reads the UI chain");
    }

    #[test]
    fn reset_drops_a_held_button_without_an_up_edge() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(0, [0, 0], 0));
        t.observe(10, &ctrl(BUTTON_LMB, [0, 0], 0));
        // The controller is replaced while LMB is held.
        t.reset();
        assert!(t.observe(500, &ctrl(0, [0, 0], 0)).is_empty(), "no up edge");
        // A later press carries no hold time from the old one.
        t.observe(600, &ctrl(BUTTON_RMB, [0, 0], 0));
        let up = t.observe(650, &ctrl(0, [0, 0], 0));
        assert_eq!(up[0].hold_ms, Some(50));
    }

    #[test]
    fn chord_released_first_button_first_is_still_a_chord() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(0, [0, 0], 0));
        t.observe(10, &ctrl(BUTTON_LMB, [0, 0], 0));
        t.observe(20, &ctrl(BUTTON_LMB | BUTTON_RMB, [0, 0], 0));
        // LMB, the first pressed, goes up first while RMB stays held.
        let first = t.observe(30, &ctrl(BUTTON_RMB, [0, 0], 0));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].button, Button::Lmb);
        assert_eq!(first[0].class, Some(ClickClass::Chord));
        // RMB comes up alone later but was part of the chord.
        let second = t.observe(40, &ctrl(0, [0, 0], 0));
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].button, Button::Rmb);
        assert_eq!(second[0].class, Some(ClickClass::Chord));
        assert_eq!(second[0].hold_ms, Some(20));
    }

    #[test]
    fn a_down_and_an_up_in_one_tick_both_report() {
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(BUTTON_RMB, [0, 0], 0));
        let evs = t.observe(10, &ctrl(BUTTON_LMB, [0, 0], 0));
        assert_eq!(evs.len(), 2);
        assert_eq!((evs[0].button, evs[0].edge), (Button::Lmb, Edge::Down));
        assert_eq!((evs[1].button, evs[1].edge), (Button::Rmb, Edge::Up));
        // A press that starts and ends between two ticks is invisible to a poll.
        let mut t = ButtonTracker::new();
        t.observe(0, &ctrl(0, [0, 0], 0));
        assert!(t.observe(10, &ctrl(0, [0, 0], 0)).is_empty());
    }

    #[test]
    fn classify_table() {
        assert_eq!(classify(Button::Lmb, [4, -4], false), ClickClass::Select);
        assert_eq!(classify(Button::Lmb, [-5, 0], false), ClickClass::Drag);
        assert_eq!(
            classify(Button::Lmb, [i32::MIN, 0], false),
            ClickClass::Drag
        );
        assert_eq!(classify(Button::Lmb, [0, 0], true), ClickClass::Chord);
        assert_eq!(classify(Button::Rmb, [0, 0], false), ClickClass::Interact);
    }

    #[test]
    fn button_fields_shape() {
        let down = ButtonEvent {
            button: Button::Lmb,
            edge: Edge::Down,
            other_held: false,
            hold_ms: None,
            drag: None,
            class: None,
            hover_id: None,
        };
        let f = button_fields(&down, None);
        assert_eq!(field(&f, "edge"), Some(json!("down")));
        assert_eq!(field(&f, "other_held"), Some(json!(false)));
        assert_eq!(field(&f, "hold_ms"), None);
        assert!(!f.iter().any(|(k, _)| *k == "ui_owned"));

        let up = ButtonEvent {
            button: Button::Lmb,
            edge: Edge::Up,
            other_held: false,
            hold_ms: Some(80),
            drag: Some([1, 2]),
            class: Some(ClickClass::Select),
            hover_id: Some(42),
        };
        let f = button_fields(&up, Some(true));
        assert_eq!(field(&f, "class"), Some(json!("select")));
        assert_eq!(field(&f, "hover_id"), Some(json!(42)));
        assert_eq!(field(&f, "ui_owned"), Some(json!(true)));
        assert!(f.iter().any(|(k, _)| *k == "ui_owned"));
    }
}
