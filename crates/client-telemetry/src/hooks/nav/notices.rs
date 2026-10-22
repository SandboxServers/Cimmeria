//! One-shot notices from the poll: it attached, or it cannot read the
//! controller.
//!
//! The controller offsets are static RE and not yet confirmed live. Without
//! these rows a wrong offset looks the same as an idle player, so each notice
//! is emitted once per process, and the tick does no extra work after that.

use serde_json::json;

use crate::hooks::entity_trace::Fields;

/// A notice the poll emits once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The controller's words were read for the first time.
    Attached {
        /// The controller's address.
        pc: u32,
    },
    /// A controller was found, but its words cannot be read.
    Unavailable {
        /// The controller's address.
        pc: u32,
    },
}

/// Which notices have been given.
#[derive(Debug, Default)]
pub struct Notices {
    attached: bool,
    unavailable: bool,
}

impl Notices {
    /// No notice given yet. `const`, so it can seed a `static`.
    pub const fn new() -> Self {
        Self {
            attached: false,
            unavailable: false,
        }
    }

    /// Records that the controller at `pc` was read (`readable`) or not. The
    /// notice to emit, if this is the first time for its kind.
    pub fn on_read(&mut self, pc: u32, readable: bool) -> Option<Notice> {
        if readable {
            if std::mem::replace(&mut self.attached, true) {
                return None;
            }
            Some(Notice::Attached { pc })
        } else {
            if std::mem::replace(&mut self.unavailable, true) {
                return None;
            }
            Some(Notice::Unavailable { pc })
        }
    }
}

/// The target and fields of a notice's row.
pub fn notice_row(n: &Notice) -> (&'static str, Fields) {
    match *n {
        Notice::Attached { pc } => ("client.nav.attached", vec![("pc", json!(hex(pc)))]),
        Notice::Unavailable { pc } => (
            "client.nav.poll_unavailable",
            vec![
                ("reason", json!("controller_unreadable")),
                ("pc", json!(hex(pc))),
            ],
        ),
    }
}

/// An address as `0x`-prefixed hex.
fn hex(v: u32) -> String {
    format!("{v:#010x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_controller_is_noticed_once() {
        let mut n = Notices::new();
        assert_eq!(
            n.on_read(0x1234, false),
            Some(Notice::Unavailable { pc: 0x1234 })
        );
        assert_eq!(n.on_read(0x1234, false), None);
        assert_eq!(n.on_read(0x5678, false), None);
    }

    #[test]
    fn first_readable_controller_is_noticed_once() {
        let mut n = Notices::new();
        assert_eq!(n.on_read(0x10, true), Some(Notice::Attached { pc: 0x10 }));
        assert_eq!(n.on_read(0x10, true), None);
        // The two kinds are independent.
        assert_eq!(
            n.on_read(0x10, false),
            Some(Notice::Unavailable { pc: 0x10 })
        );
    }

    #[test]
    fn rows_carry_the_reason_and_hex_pc() {
        let (target, f) = notice_row(&Notice::Unavailable { pc: 0xab });
        assert_eq!(target, "client.nav.poll_unavailable");
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("reason"), Some(json!("controller_unreadable")));
        assert_eq!(get("pc"), Some(json!("0x000000ab")));
        let (target, _) = notice_row(&Notice::Attached { pc: 1 });
        assert_eq!(target, "client.nav.attached");
    }

    /// A controller whose header is unreadable: the real `read_controller` path
    /// feeds the notice, so the flag is set once and not on the next tick.
    #[test]
    fn an_unreadable_header_feeds_the_notice() {
        use crate::hooks::entity_trace::map::fake::FakeMem;
        let mem = FakeMem::default();
        let mut n = Notices::new();
        let pc = 0x4000_0000;
        let ok = super::super::controller::read_controller(&mem, pc).is_some();
        assert!(!ok);
        assert!(n.on_read(pc, ok).is_some());
        let ok = super::super::controller::read_controller(&mem, pc).is_some();
        assert!(n.on_read(pc, ok).is_none());
    }
}
